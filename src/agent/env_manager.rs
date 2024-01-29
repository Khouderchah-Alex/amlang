use log::{debug, info, warn};
use std::borrow::Borrow;
use std::collections::{BTreeSet, HashMap};
use std::convert::TryFrom;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use super::amlang_wrappers::quote_wrapper;
use super::context::{Context, MetaEnvContext};
use super::deserialize_error::DeserializeError::*;
use super::env_header::EnvHeader;
use super::env_policy::EnvPolicy;
use super::Agent;
use crate::agent::lang_error::LangError;
use crate::builtins::generate_builtin_map;
use crate::env::local_node::{LocalId, LocalNode};
use crate::env::meta_env::MetaEnv;
use crate::env::Environment;
use crate::error::Error;
use crate::model::Reflective;
use crate::primitive::prelude::*;
use crate::primitive::symbol_policies::policy_env_serde;
use crate::primitive::table::Table;
use crate::sexp::Sexp;
use crate::stream::input::FileReader;


pub struct EnvManager<Policy: EnvPolicy> {
    agent: Agent,
    policy: Policy,
}

impl<Policy: EnvPolicy> EnvManager<Policy> {
    pub fn bootstrap<P: AsRef<Path>>(in_path: P) -> Result<Self, Error> {
        let mut policy = Policy::default();
        let meta = MetaEnv::new(EnvManager::create_env(&mut policy, LocalNode::default()));

        let amlang_base = Path::new(env!("CARGO_MANIFEST_DIR"))
            .canonicalize()
            .unwrap();

        // Bootstrap meta env.
        let meta_agent = Agent::new(
            Node::new(LocalNode::default(), LocalNode::default()),
            meta.clone(),
            MetaEnvContext::placeholder(),
        );
        let mut manager = Self {
            agent: meta_agent,
            policy: policy,
        };

        let mut meta_path = in_path.as_ref().canonicalize().unwrap().to_path_buf();
        meta_path.push("meta.env");
        manager.deserialize_curr_env(meta_path)?;
        manager.agent.context_metaenv =
            MetaEnvContext::load(manager.agent().pos(), manager.agent_mut())?;
        info!("Meta env bootstrapping complete.");

        // Bootstrap lang env.
        let lang_env = manager.agent().find_env("lang.env").unwrap();
        manager.initialize_env_node(lang_env);
        manager.agent_mut().jump_env(lang_env);
        let lang_path = amlang_base.join("envs/lang.env");
        manager.deserialize_curr_env(lang_path)?;
        info!("Lang env bootstrapping complete.");

        // Load all other envs.
        // TODO(func) Allow for delayed loading of environments.
        let env_triples = meta
            .base()
            .match_predicate(*manager.agent.context_metaenv.serialize_path())
            .triples();
        for triple in env_triples {
            let subject_node = meta.base().triple_subject(triple);
            if subject_node == lang_env {
                continue;
            }

            let object_node = meta.base().triple_object(triple);
            let entry = meta.base().entry(object_node);
            let object = entry.structure();
            let env_path = <&LangPath>::try_from(&*object).unwrap();

            manager.initialize_env_node(subject_node);
            manager
                .agent_mut()
                .jump(Node::new(subject_node, LocalNode::default()));
            manager.deserialize_curr_env(env_path.as_std_path())?;
        }

        manager.agent_mut().jump_env(lang_env);
        Ok(manager)
    }

    pub fn agent(&self) -> &Agent {
        &self.agent
    }
    pub fn agent_mut(&mut self) -> &mut Agent {
        &mut self.agent
    }

    pub fn insert_new_env<P: AsRef<Path>>(&mut self, path: P) -> LocalNode {
        let serialize_path = *self.agent.context_metaenv.serialize_path();
        let env_node = self.agent_mut().meta_mut().base_mut().insert_node(None);
        self.initialize_env_node(env_node);

        let meta = self.agent_mut().meta_mut();
        let path_node = meta
            .base_mut()
            .insert_node(Some(LangPath::new(path.as_ref().to_path_buf()).into()));
        meta.base_mut()
            .insert_triple(env_node, serialize_path, path_node);

        env_node
    }

    fn create_env(policy: &mut Policy, env_node: LocalNode) -> Box<Policy::StoredEnv> {
        let mut env = Policy::BaseEnv::default();

        // Create nodes.
        let self_env = env.insert_node(Some(Node::new(LocalNode::default(), env_node).into()));
        let designation = env.insert_node(Some(SymNodeTable::default().into()));
        let tell_handler = env.insert_node(None);
        let mut reserved_id = env.all_nodes().len() as LocalId;
        while let Some(_) = LocalNode::new(reserved_id).as_prelude() {
            env.insert_node(Some("RESERVED".to_symbol_or_panic(policy_admin).into()));
            reserved_id += 1;
        }

        // Name nodes.
        if let Ok(table) = <&mut SymNodeTable>::try_from(env.entry_mut(designation).as_option()) {
            table.insert(
                self_env
                    .as_prelude()
                    .unwrap()
                    .name()
                    .to_symbol_or_panic(policy_admin),
                Node::new(env_node, self_env),
            );
            table.insert(
                designation
                    .as_prelude()
                    .unwrap()
                    .name()
                    .to_symbol_or_panic(policy_admin),
                Node::new(env_node, designation),
            );
            table.insert(
                tell_handler
                    .as_prelude()
                    .unwrap()
                    .name()
                    .to_symbol_or_panic(policy_admin),
                Node::new(env_node, tell_handler),
            );
        } else {
            panic!("Env designation isn't a symbol table");
        }

        policy.new_stored_env(env)
    }

    fn initialize_env_node(&mut self, env_node: LocalNode) {
        let env = EnvManager::create_env(&mut self.policy, env_node);
        self.agent_mut().meta_mut().insert_env(env_node, env);
    }
}


// {,De}serialization functionality.
impl<Policy: EnvPolicy> EnvManager<Policy> {
    pub fn serialize_full<P: AsRef<Path>>(
        &mut self,
        out_path: P,
        blacklist: BTreeSet<&str>,
    ) -> std::io::Result<()> {
        let original_pos = self.agent().pos();

        // Serialize meta env.
        self.agent_mut()
            .jump(Node::new(LocalNode::default(), LocalNode::default()));
        let mut meta_path = out_path.as_ref().to_path_buf();
        meta_path.push("meta.env");
        self.serialize_curr_env(meta_path)?;

        // Serialize envs in meta env.
        let serialize_path = *self.agent.context_metaenv.serialize_path();
        let env_triples = self
            .agent()
            .meta()
            .base()
            .match_predicate(serialize_path)
            .triples();
        for triple in env_triples {
            let env = self.agent().meta().base().triple_subject(triple);
            let path = {
                let object_node = self.agent().meta().base().triple_object(triple);
                let entry = self.agent().meta().base().entry(object_node);
                LangPath::try_from(entry.owned()).unwrap()
            };

            if blacklist.contains(&path.as_std_path().as_os_str().to_string_lossy().borrow()) {
                continue;
            }

            self.agent_mut().jump_env(env);
            self.serialize_curr_env(path.as_std_path())?;
        }

        self.agent_mut().jump(original_pos);
        Ok(())
    }

    pub fn serialize_curr_env<P: AsRef<Path>>(&self, out_path: P) -> std::io::Result<()> {
        let file = File::create(out_path.as_ref())?;
        let mut w = BufWriter::new(file);

        let env = self.agent().env();
        let header = *self.agent().reify(&EnvHeader::from_env(env)).unwrap();
        self.serialize_list_internal(&mut w, &header, 0)?;
        writeln!(&mut w, "")?;

        writeln!(&mut w, "(section nodes)")?;
        for (i, node) in env.all_nodes().into_iter().enumerate() {
            let s = env.entry(node).owned();
            let (write_structure, add_quote) = match &s {
                // Serialize self_des as ^1 since it can be reconstructed.
                _ if i == 1 => (false, false),
                // Don't quote structures with special deserialize ops.
                Some(sexp) => match sexp {
                    Sexp::Primitive(Primitive::SymNodeTable(_)) => (true, false),
                    Sexp::Primitive(Primitive::LocalNodeTable(_)) => (true, false),
                    Sexp::Primitive(Primitive::BuiltIn(_)) => (true, false),
                    Sexp::Primitive(Primitive::Procedure(_)) => (true, false),
                    Sexp::Primitive(Primitive::Node(_)) => (true, false),
                    Sexp::Primitive(Primitive::LangPath(_)) => (true, false),
                    Sexp::Primitive(Primitive::LangString(_)) => (true, false),
                    _ => (true, true),
                },
                _ => (false, false),
            };

            let node = node.globalize(self.agent());
            if write_structure {
                let mut structure = s.unwrap();
                if add_quote {
                    structure = list!("quote".to_symbol_or_panic(policy_admin), structure);
                }
                self.serialize_list_internal(&mut w, &list!(node, structure), 0)?;
            } else {
                write!(&mut w, " ")?; // Add space to align with structured lines.
                self.serialize_list_internal(&mut w, &node.into(), 0)?;
            };
        }
        writeln!(&mut w, "")?;

        writeln!(&mut w, "(section triples)")?;
        for triple in env.match_all().triples() {
            let s = triple.reify(self.agent());
            self.serialize_list_internal(&mut w, &s, 0)?;
        }
        writeln!(&mut w, "")?;

        // TODO(func) Generalize to arbitrary designations.
        let des = env.designation_pairs(LocalNode::default());
        let name = if self.agent().pos().env().id() != 0 {
            "amlang"
        } else {
            "meta"
        };
        if !des.is_empty() {
            writeln!(&mut w, "(section designation {})", name)?;
            for (sym, node) in des {
                writeln!(&mut w, "(^{} {})", node.id(), sym)?;
            }
            writeln!(&mut w, "")?;
        }
        info!(
            "Serialized env {} @ \"{}\".",
            self.agent().pos().env(),
            out_path.as_ref().to_string_lossy()
        );
        Ok(())
    }

    pub fn deserialize_curr_env<P: AsRef<Path>>(&mut self, in_path: P) -> Result<(), Error> {
        debug!("Deserializing env {}", in_path.as_ref().to_string_lossy());
        let mut input = match FileReader::new(in_path.as_ref()) {
            Ok(input) => input,
            Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => {
                warn!("Env file not found: {}", in_path.as_ref().to_string_lossy());
                warn!(
                    "Leaving env {} unchanged. If this is intended, then all is well.",
                    self.agent().pos().env()
                );
                return Ok(());
            }
            Err(err) => return err!(self.agent(), IoError(err)),
        };

        let header = if let Some(line) = input.next() {
            let header = Sexp::parse_with(line?.as_str(), policy_env_serde)?;
            self.agent_mut().reflect::<EnvHeader>(header)?
        } else {
            return err!(self.agent(), MissingHeaderSection);
        };

        // Deserialize designations first in case we need it for nodes/triples.
        let mut context_input = FileReader::new(in_path.as_ref()).unwrap();
        context_input.seek_line(6 + header.node_count() + header.triple_count())?;
        self.deserialize_designations(&mut context_input)?;

        input.next();
        self.deserialize_nodes(&mut input, header.node_count())?;
        input.next();
        self.deserialize_triples(&mut input)?;

        info!(
            "Loaded env {} from \"{}\".",
            self.agent().pos().env(),
            in_path.as_ref().to_string_lossy()
        );
        debug!("  Node count:    {}", self.agent().env().all_nodes().len());
        debug!(
            "  Triple count:  {}",
            self.agent().ask(None, None, None).unwrap().len()
        );
        Ok(())
    }


    fn serialize_list_internal<W: std::io::Write>(
        &self,
        w: &mut W,
        structure: &Sexp,
        depth: usize,
    ) -> std::io::Result<()> {
        structure.write(
            w,
            depth,
            &mut |writer, primitive, depth| self.serialize_primitive(writer, primitive, depth),
            &mut |writer, paren, _depth| write!(writer, "{}", paren),
            &mut |writer, _depth| write!(writer, " "),
            None,
            None,
        )?;
        if depth == 0 {
            write!(w, "\n")?;
        }
        Ok(())
    }

    fn serialize_primitive<W: std::io::Write>(
        &self,
        w: &mut W,
        primitive: &Primitive,
        depth: usize,
    ) -> std::io::Result<()> {
        match primitive {
            Primitive::LangString(s) => write!(w, "\"{}\"", s.clone().to_escaped()),
            Primitive::Symbol(symbol) => write!(w, "{}", symbol.as_str()),
            Primitive::LangPath(path) => {
                write!(w, "(__path \"{}\")", path.as_std_path().to_string_lossy())
            }
            Primitive::BuiltIn(builtin) => write!(w, "(__builtin {})", builtin.name()),
            Primitive::Procedure(proc) => {
                let sexp = *self.agent().reify(&proc).unwrap();
                self.serialize_list_internal(w, &sexp, depth)
            }
            Primitive::SymNodeTable(table) => {
                let sexp = *self.agent().reify(&table).unwrap();
                self.serialize_list_internal(w, &sexp, depth)
            }
            Primitive::LocalNodeTable(table) => {
                let sexp = *self.agent().reify(&table).unwrap();
                self.serialize_list_internal(w, &sexp, depth)
            }
            Primitive::Node(node) => {
                // Write Nodes as their designation if possible.
                if let Some(sym) = self.agent().lookup_designation(*node) {
                    return write!(w, "{}", sym.as_str());
                }

                if let Some(triple) = self
                    .agent()
                    .access_env(node.env())
                    .unwrap()
                    .node_as_triple(node.local())
                {
                    if node.env() != self.agent().pos().env() {
                        write!(w, "^{}", node.env().id())?;
                    }
                    return write!(w, "^t{}", self.agent().env().triple_index(triple));
                }

                if node.env() != self.agent().pos().env() {
                    write!(w, "^{}", node.env().id())?;
                }
                write!(w, "^{}", node.local().id())
            }
            _ => write!(w, "{}", primitive),
        }
    }

    fn deserialize_nodes(
        &mut self,
        reader: &mut FileReader,
        mut node_count: usize,
    ) -> Result<(), Error> {
        debug!("Deserializing nodes");
        let section_line = if let Some(line) = reader.next() {
            line?
        } else {
            return err!(self.agent(), MissingNodeSection);
        };

        let header = Sexp::parse_with(section_line.as_str(), policy_env_serde)?;
        let (command, section) = break_sexp!(header => (Symbol, Symbol), self.agent())?;
        if command.as_str() != "section" || section.as_str() != "nodes" {
            return err!(self.agent(), UnexpectedCommand(list!(command, section)));
        }

        // Skip prelude nodes.
        reader.nth(9);
        node_count -= 10;

        let builtins = generate_builtin_map();
        for _i in 0..node_count {
            let line = reader.next().unwrap()?;
            let entry = Sexp::parse_with(line.as_str(), policy_env_serde)?;
            match entry {
                Sexp::Primitive(primitive) => {
                    if let Primitive::Symbol(_sym) = primitive {
                        self.agent_mut().define(None)?;
                    } else {
                        return err!(self.agent(), ExpectedSymbol);
                    }
                }
                Sexp::Cons(_) => {
                    let (_name, command) = break_sexp!(entry => (Symbol, Sexp), self.agent())?;
                    let structure = self.eval_structure(command, &builtins)?;
                    self.agent_mut().define(Some(structure))?;
                }
            }
        }
        Ok(())
    }

    fn parse_node(&self, sym: &Symbol) -> Result<Node, Error> {
        EnvManager::<Policy>::parse_node_inner(self.agent(), sym)
    }

    fn parse_node_inner(agent: &Agent, sym: &Symbol) -> Result<Node, Error> {
        match policy_env_serde(sym.as_str()).unwrap() {
            AdminSymbolInfo::Identifier => {
                err!(agent, LangError::UnboundSymbol(sym.clone()))
            }
            AdminSymbolInfo::LocalNode(node) => Ok(node.globalize(agent)),
            AdminSymbolInfo::LocalTriple(idx) => {
                let triple = agent.env().triple_from_index(idx);
                Ok(triple.node().globalize(agent))
            }
            AdminSymbolInfo::GlobalNode(env, node) => Ok(Node::new(env, node)),
            AdminSymbolInfo::GlobalTriple(env, idx) => {
                Ok(Node::new(env, agent.env().triple_from_index(idx).node()))
            }
        }
    }

    fn eval_structure(
        &mut self,
        sexp: Sexp,
        builtins: &HashMap<&'static str, BuiltIn>,
    ) -> Result<Sexp, Error> {
        match sexp {
            Sexp::Primitive(Primitive::Symbol(sym)) => return Ok(self.parse_node(&sym)?.into()),
            Sexp::Primitive(Primitive::LangString(s)) => return Ok(s.into()),
            _ => {}
        }

        let (command, _) = break_sexp!(sexp.iter() => (&Symbol; remainder), self.agent())?;
        if let Some(t) = Primitive::type_from_discriminator(command.as_str()) {
            return Ok(match t {
                "Procedure" => self.agent_mut().reflect::<Procedure>(sexp)?.into(),
                "LocalNodeTable" => self.agent_mut().reflect::<LocalNodeTable>(sexp)?.into(),
                "SymNodeTable" => self.agent_mut().reflect::<SymNodeTable>(sexp)?.into(),
                _ => panic!(),
            });
        }

        let (command, cdr) = break_sexp!(sexp => (Symbol; remainder), self.agent())?;
        match command.as_str() {
            "quote" => Ok(*quote_wrapper(cdr, self.agent())?),
            "__builtin" => {
                if let Ok(sym) = <Symbol>::try_from(*quote_wrapper(cdr, self.agent())?) {
                    if let Some(builtin) = builtins.get(sym.as_str()) {
                        Ok(builtin.clone().into())
                    } else {
                        err!(self.agent(), UnrecognizedBuiltIn(sym.clone()))
                    }
                } else {
                    err!(self.agent(), ExpectedSymbol)
                }
            }
            "__path" => {
                let (path,) = break_sexp!(cdr.unwrap() => (LangString), self.agent())?;
                Ok(LangPath::new(path.as_str().into()).into())
            }
            _ => panic!("{}", command),
        }
    }

    fn deserialize_triples(&mut self, reader: &mut FileReader) -> Result<(), Error> {
        debug!("Deserializing triples");
        let section_line = if let Some(line) = reader.next() {
            line?
        } else {
            return err!(self.agent(), MissingTripleSection);
        };

        let header = Sexp::parse_with(section_line.as_str(), policy_env_serde)?;
        let (command, section) = break_sexp!(header => (Symbol, Symbol), self.agent())?;
        if command.as_str() != "section" || section.as_str() != "triples" {
            return err!(self.agent(), UnexpectedCommand(list!(command, section)));
        }

        let mut line = reader.next().unwrap()?;
        while !line.is_empty() {
            let triple = Sexp::parse_with(line.as_str(), policy_env_serde)?;
            let (s, p, o) = break_sexp!(triple => (Symbol, Symbol, Symbol), self.agent())?;

            let subject = self.parse_node(&s)?;
            let predicate = self.parse_node(&p)?;
            let object = self.parse_node(&o)?;

            self.agent_mut().tell(subject, predicate, object)?;

            line = reader.next().unwrap()?;
        }
        Ok(())
    }

    fn deserialize_designations(&mut self, reader: &mut FileReader) -> Result<(), Error> {
        debug!("Deserializing designations");
        while let Some(section_line) = reader.next() {
            let header = Sexp::parse_with(section_line?.as_str(), policy_env_serde)?;
            let (command, section, designator) =
                break_sexp!(header => (Symbol, Symbol, Symbol), self.agent())?;
            if command.as_str() != "section" || section.as_str() != "designation" {
                return err!(
                    self.agent(),
                    UnexpectedCommand(list!(command, section, designator))
                );
            }

            let mut line = reader.next().unwrap()?;
            while !line.is_empty() {
                let pair = Sexp::parse_with(line.as_str(), policy_env_serde)?;
                let (node_id, name) = break_sexp!(pair => (Symbol, Symbol), self.agent())?;

                let node = self.parse_node(&node_id)?;
                self.agent_mut().env_mut().insert_designation(
                    node.local(),
                    name,
                    LocalNode::default(),
                );

                line = reader.next().unwrap()?;
            }

            // TODO(func) Generic handling of desired d-chain/context-state.
            if designator.as_str() == "amlang" {
                let env_node = self.agent().pos().env();
                self.agent
                    .designation_chain_mut()
                    .push_front(Node::new(env_node, LocalNode::default()));
            }
        }

        Ok(())
    }
}
