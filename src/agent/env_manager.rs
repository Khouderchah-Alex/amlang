use log::{debug, info, warn};
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path as StdPath;

use super::amlang_context::{AmlangContext, EnvPrelude};
use super::amlang_wrappers::quote_wrapper;
use super::deserialize_error::DeserializeError::*;
use super::env_policy::EnvPolicy;
use super::Agent;
use crate::agent::lang_error::LangError;
use crate::builtins::generate_builtin_map;
use crate::environment::entry::EntryMutKind;
use crate::environment::environment::Environment;
use crate::environment::local_node::{LocalId, LocalNode};
use crate::error::Error;
use crate::model::Reflective;
use crate::parser::parse_sexp;
use crate::primitive::prelude::*;
use crate::primitive::symbol_policies::{policy_admin, AdminSymbolInfo};
use crate::primitive::table::Table;
use crate::sexp::{Cons, HeapSexp, Sexp, SexpIntoIter};
use crate::token::file_stream::{FileStream, FileStreamError};


pub struct EnvManager<Policy: EnvPolicy> {
    agent: Agent,
    policy: Policy,
}

/// Replace placeholder'd context nodes through AmlangDesignation lookups.
macro_rules! bootstrap_context {
    (
        $manager:expr,
        $($node:ident : $query:expr),+
        $(,)?
    ) => {
        let ($($node,)+) = {
            let desig_node = $manager.agent().context().designation();
            let entry = $manager.agent_mut().env().entry(desig_node);
            let table = if let Ok(table) =
                <&SymbolTable>::try_from(entry.as_option()) {
                    table
                } else {
                    panic!("Env designation isn't a symbol table");
                };

            let lookup = |s: &str| -> Result<LocalNode, Error> {
                if let Some(node) = table.lookup(s) {
                    Ok(node.local())
                } else {
                    Err(Error::no_agent(
                        Box::new(LangError::UnboundSymbol(s.to_symbol_or_panic(policy_admin)))))?
                }
            };
            (
                $(lookup($query)?,)+
            )
        };

        // Fill in placeholder'd context nodes.
        let context = $manager.agent_mut().context_mut();
        $(context.$node = $node;)+
    };
}

impl<Policy: EnvPolicy> EnvManager<Policy> {
    pub fn bootstrap<P: AsRef<StdPath>>(meta_path: P) -> Result<Self, Error> {
        let mut policy = Policy::default();
        let meta = EnvManager::create_env(&mut policy, LocalNode::default());

        let mut context = AmlangContext::new(meta);
        // Make context usable for reflecting Reflectives during bootstrapping.
        // TODO(flex) Find more flexible approch to bootstrapping Reflective
        // nodes, like {,de}serializing this as a bootstrap kernel.
        context.lang_env = LocalNode::new(16);
        // Procedure nodes.
        context.lambda = LocalNode::new(17);
        context.apply = LocalNode::new(31);
        context.branch = LocalNode::new(39);
        context.fexpr = LocalNode::new(43);
        context.progn = LocalNode::new(45);
        // Table nodes.
        context.symbol_table = LocalNode::new(65);
        context.local_node_table = LocalNode::new(67);

        // Bootstrap meta env.
        let meta_state = Agent::new(
            Node::new(LocalNode::default(), context.self_node()),
            context,
            // Subtle: Can't use history_env until meta env has been bootstrapped.
            LocalNode::default(),
        );
        let mut manager = Self {
            agent: meta_state,
            policy: policy,
        };
        manager.deserialize_curr_env(meta_path)?;
        bootstrap_context!(manager,
                           imports: "__imports",
                           import_table: "__import_table",
                           serialize_path: "__serialize_path",
        );
        let history_env = manager.agent().find_env("history.env").unwrap();
        manager.agent_mut().set_history_env(history_env);
        info!("Meta env bootstrapping complete.");

        // Bootstrap lang env.
        let lang_env = manager.agent().context().lang_env();
        manager.initialize_env_node(lang_env);
        let serialize_path = manager.agent().context().serialize_path;
        manager.agent_mut().jump_env(lang_env);
        let lang_path = {
            let meta = manager.agent().context().meta();
            let lang_path_triple = meta
                .match_but_object(lang_env, serialize_path)
                .iter()
                .next()
                .unwrap()
                .clone();
            let lang_path_node = meta.triple_object(lang_path_triple);

            Path::try_from(meta.entry(lang_path_node).owned()).unwrap()
        };
        manager.deserialize_curr_env(lang_path.as_std_path())?;
        bootstrap_context!(manager,
                           quote: "quote",
                           lambda: "lambda",
                           fexpr: "fexpr",
                           def: "def",
                           tell: "tell",
                           curr: "curr",
                           jump: "jump",
                           ask: "ask",
                           placeholder: "_",
                           apply: "apply",
                           eval: "eval",
                           exec: "exec",
                           import: "import",
                           branch: "if",
                           progn: "progn",
                           let_basic: "let",
                           let_rec: "letrec",
                           env_find: "env-find",
                           t: "true",
                           f: "false",
                           eq: "eq",
                           symbol_table: "table-sym-node",
                           local_node_table: "table-lnode",
                           label: "label",
        );
        info!("Lang env bootstrapping complete.");

        // Since we've bootstrapped the lang by here, we can clone the context
        // without worrying about using stale/placeholder values.
        let context = manager.agent().context().clone();
        let env_triples = context.meta().match_predicate(serialize_path);
        // TODO(func) Allow for delayed loading of environments.
        for triple in env_triples {
            let subject_node = context.meta().triple_subject(triple);
            if subject_node == context.lang_env {
                continue;
            }

            let object_node = context.meta().triple_object(triple);
            let entry = context.meta().entry(object_node);
            let object = entry.structure();
            let env_path = <&Path>::try_from(&*object).unwrap();

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

    pub fn insert_new_env<P: AsRef<StdPath>>(&mut self, path: P) -> LocalNode {
        let serialize_path = self.agent().context().serialize_path;
        let env_node = self
            .agent_mut()
            .context_mut()
            .meta_mut()
            .insert_structure(Sexp::default());
        self.initialize_env_node(env_node);

        let meta = self.agent_mut().context_mut().meta_mut();
        let path_node = meta.insert_structure(Path::new(path.as_ref().to_path_buf()).into());
        meta.insert_triple(env_node, serialize_path, path_node);

        env_node
    }

    fn create_env(policy: &mut Policy, env_node: LocalNode) -> Box<Policy::StoredEnv> {
        let mut env = Policy::BaseEnv::default();

        // Create nodes.
        let self_env = env.insert_structure(Node::new(LocalNode::default(), env_node).into());
        let designation = env.insert_structure(SymbolTable::default().into());
        let tell_handler = env.insert_atom();
        let mut reserved_id = env.all_nodes().len() as LocalId;
        while let Some(_) = LocalNode::new(reserved_id).as_prelude() {
            env.insert_structure("RESERVED".to_symbol_or_panic(policy_admin).into());
            reserved_id += 1;
        }

        // Name nodes.
        if let Ok(table) = <&mut SymbolTable>::try_from(env.entry_mut(designation).as_option()) {
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
        let meta = self.agent_mut().context_mut().meta_mut();
        *meta.entry_mut(env_node).kind_mut() = EntryMutKind::Owned(env.into());
    }
}


// {,De}serialization functionality.
impl<Policy: EnvPolicy> EnvManager<Policy> {
    pub fn serialize_full<P: AsRef<StdPath>>(&mut self, out_path: P) -> std::io::Result<()> {
        let original_pos = self.agent().pos();

        self.agent_mut()
            .jump(Node::new(LocalNode::default(), LocalNode::default()));
        self.serialize_curr_env(out_path)?;

        // Serialize recursively.
        let serialize_path = self.agent().context().serialize_path;
        let env_triples = self
            .agent()
            .context()
            .meta()
            .match_predicate(serialize_path);
        for triple in env_triples {
            let subject_node = self.agent().context().meta().triple_subject(triple);
            let path = {
                let object_node = self.agent().context().meta().triple_object(triple);
                let entry = self.agent().context().meta().entry(object_node);
                Path::try_from(entry.owned()).unwrap()
            };

            self.agent_mut().jump_env(subject_node);
            self.serialize_curr_env(path.as_std_path())?;
        }

        self.agent_mut().jump(original_pos);
        Ok(())
    }

    pub fn serialize_curr_env<P: AsRef<StdPath>>(&mut self, out_path: P) -> std::io::Result<()> {
        let file = File::create(out_path.as_ref())?;
        let mut w = BufWriter::new(file);

        // TODO(func) Use Header class & reify + reflect.
        let node_count = self.agent_mut().env().all_nodes().into_iter().count();
        let triple_count = self.agent_mut().env().match_all().into_iter().count();
        let header = list!(
            "header".to_symbol_or_panic(policy_admin),
            Cons::new(
                Some("version".to_symbol_or_panic(policy_admin).into()),
                Some(Number::Integer(1).into())
            ),
            Cons::new(
                Some("node-count".to_symbol_or_panic(policy_admin).into()),
                Some(Number::Integer(node_count.try_into().unwrap()).into())
            ),
            Cons::new(
                Some("triple-count".to_symbol_or_panic(policy_admin).into()),
                Some(Number::Integer(triple_count.try_into().unwrap()).into())
            ),
        );
        self.serialize_list_internal(&mut w, &header, 0)?;

        write!(&mut w, "(nodes")?;
        for (i, node) in self.agent_mut().env().all_nodes().into_iter().enumerate() {
            write!(&mut w, "\n    ")?;

            let s = self.agent_mut().env().entry(node).owned();
            let (write_structure, add_quote) = match &s {
                // Serialize self_des as ^1 since it can be reconstructed.
                _ if i == 1 => (false, false),
                // Don't quote structures with special deserialize ops.
                Some(sexp) => match sexp {
                    Sexp::Primitive(Primitive::SymbolTable(_)) => (true, false),
                    Sexp::Primitive(Primitive::LocalNodeTable(_)) => (true, false),
                    Sexp::Primitive(Primitive::BuiltIn(_)) => (true, false),
                    Sexp::Primitive(Primitive::Procedure(_)) => (true, false),
                    Sexp::Primitive(Primitive::Node(_)) => (true, false),
                    Sexp::Primitive(Primitive::Env(_)) => (false, false),
                    Sexp::Primitive(Primitive::Path(_)) => (true, false),
                    Sexp::Primitive(Primitive::AmString(_)) => (true, false),
                    _ => (true, true),
                },
                _ => (false, false),
            };

            let node = node.globalize(self.agent());
            let line = if write_structure {
                let structure = s.unwrap();
                if add_quote {
                    list!(node, ("quote".to_symbol_or_panic(policy_admin), structure,),)
                } else {
                    list!(node, structure,)
                }
            } else {
                write!(&mut w, " ")?; // Add space to align with structured lines.
                node.into()
            };
            self.serialize_list_internal(&mut w, &line, 1)?;
        }
        write!(&mut w, "\n)\n\n")?;

        write!(&mut w, "(triples")?;
        for triple in self.agent_mut().env().match_all() {
            write!(&mut w, "\n    ")?;
            let s = triple.reify(self.agent_mut());
            self.serialize_list_internal(&mut w, &s, 1)?;
        }
        writeln!(&mut w, "\n)")?;
        info!(
            "Serialized env {} @ \"{}\".",
            self.agent().pos().env(),
            out_path.as_ref().to_string_lossy()
        );
        Ok(())
    }

    pub fn deserialize_curr_env<P: AsRef<StdPath>>(&mut self, in_path: P) -> Result<(), Error> {
        let stream = match FileStream::new(in_path.as_ref(), policy_admin) {
            Ok(stream) => stream,
            Err(FileStreamError::IoError(ref e)) if e.kind() == std::io::ErrorKind::NotFound => {
                warn!("Env file not found: {}", in_path.as_ref().to_string_lossy());
                warn!(
                    "Leaving env {} unchanged. If this is intended, then all is well.",
                    self.agent().pos().env()
                );
                return Ok(());
            }
            Err(err) => return err!(self.agent(), FileStreamError(err)),
        };
        let mut peekable = stream.peekable();

        match parse_sexp(&mut peekable, 0) {
            Ok(Some(parsed)) => self.deserialize_header(parsed)?,
            Ok(None) => return err!(self.agent(), MissingHeaderSection),
            Err(err) => return err!(self.agent(), ParseError(err)),
        };
        match parse_sexp(&mut peekable, 0) {
            Ok(Some(parsed)) => self.deserialize_nodes(parsed)?,
            Ok(None) => return err!(self.agent(), MissingNodeSection),
            Err(err) => return err!(self.agent(), ParseError(err)),
        };
        match parse_sexp(&mut peekable, 0) {
            Ok(Some(parsed)) => self.deserialize_triples(parsed)?,
            Ok(None) => return err!(self.agent(), MissingTripleSection),
            Err(err) => return err!(self.agent(), ParseError(err)),
        };
        match parse_sexp(&mut peekable, 0) {
            Ok(Some(_)) => return err!(self.agent(), ExtraneousSection),
            Ok(None) => {}
            Err(err) => return err!(self.agent(), ParseError(err)),
        };

        info!(
            "Loaded env {} from \"{}\".",
            self.agent().pos().env(),
            in_path.as_ref().to_string_lossy()
        );
        debug!(
            "  Node count:    {}",
            self.agent_mut().env().all_nodes().len()
        );
        debug!(
            "  Triple count:  {}",
            self.agent_mut().env().match_all().len()
        );
        Ok(())
    }


    fn serialize_list_internal<W: std::io::Write>(
        &mut self,
        w: &mut W,
        structure: &Sexp,
        depth: usize,
    ) -> std::io::Result<()> {
        structure.write(
            w,
            depth,
            &mut |writer, primitive, depth| self.serialize_primitive(writer, primitive, depth),
            &mut |writer, paren, depth| {
                let prefix = if paren == ")" && depth == 0 { "\n" } else { "" };
                let suffix = if paren == ")" && depth == 0 {
                    "\n\n"
                } else {
                    ""
                };
                write!(writer, "{}{}{}", prefix, paren, suffix)
            },
            &mut |writer, depth| {
                let s = match depth {
                    0 => "\n    ",
                    _ => " ",
                };
                write!(writer, "{}", s)
            },
            None,
            None,
        )
    }

    fn serialize_primitive<W: std::io::Write>(
        &mut self,
        w: &mut W,
        primitive: &Primitive,
        depth: usize,
    ) -> std::io::Result<()> {
        match primitive {
            Primitive::AmString(s) => write!(w, "\"{}\"", s.clone().to_escaped()),
            Primitive::Symbol(symbol) => write!(w, "{}", symbol.as_str()),
            Primitive::Path(path) => {
                write!(w, "(__path \"{}\")", path.as_std_path().to_string_lossy())
            }
            Primitive::BuiltIn(builtin) => write!(w, "(__builtin {})", builtin.name()),
            Primitive::Procedure(proc) => {
                let proc_sexp = proc.reify(self.agent_mut());
                self.serialize_list_internal(w, &proc_sexp, depth)
            }
            Primitive::SymbolTable(table) => {
                let sexp = table.reify(self.agent_mut());
                self.serialize_list_internal(w, &sexp, depth)
            }
            Primitive::LocalNodeTable(table) => {
                let sexp = table.reify(self.agent_mut());
                self.serialize_list_internal(w, &sexp, depth)
            }
            Primitive::Node(node) => {
                if let Some(triple) = self
                    .agent_mut()
                    .access_env(node.env())
                    .unwrap()
                    .node_as_triple(node.local())
                {
                    if node.env() != self.agent().pos().env() {
                        write!(w, "^{}", node.env().id())?;
                    }
                    return write!(w, "^t{}", self.agent_mut().env().triple_index(triple));
                }

                if node.env() != self.agent().pos().env() {
                    write!(w, "^{}", node.env().id())?;
                }
                write!(w, "^{}", node.local().id())
            }
            Primitive::Env(_) => Ok(()),
            _ => write!(w, "{}", primitive),
        }
    }

    fn deserialize_header(&mut self, structure: Sexp) -> Result<(), Error> {
        // TODO(func) Use Header class & reify + reflect.
        let (command, _) = break_sexp!(structure => (Symbol; remainder), self.agent())?;
        if command.as_str() != "header" {
            return err!(self.agent(), UnexpectedCommand(command.into()));
        }
        Ok(())
    }

    fn deserialize_nodes(&mut self, structure: Sexp) -> Result<(), Error> {
        let builtins = generate_builtin_map();
        let (command, remainder) = break_sexp!(structure => (Symbol; remainder), self.agent())?;
        if command.as_str() != "nodes" {
            return err!(self.agent(), UnexpectedCommand(command.into()));
        }

        // Ensure prelude nodes are as expected.
        let iter = SexpIntoIter::from(remainder);
        let (first, second, third, _r0, _r1, _r2, _r3, _r4, _r5, _r6, remainder) = break_sexp!(
            iter => (HeapSexp,
                     Symbol,
                     Symbol,
                     HeapSexp,
                     HeapSexp,
                     HeapSexp,
                     HeapSexp,
                     HeapSexp,
                     HeapSexp,
                     HeapSexp; remainder), self.agent())?;
        let (self_node_id, self_val_node) = break_sexp!(first => (Symbol, Symbol), self.agent())?;
        let self_id = EnvPrelude::SelfEnv.local();
        if self.parse_node(&self_node_id)?.local() != self_id
            || self.parse_node(&self_val_node)? != Node::new(self_id, self.agent().pos().env())
        {
            return err!(self.agent(), InvalidNodeEntry(self_val_node.into()));
        }
        if self.parse_node(&second)?.local() != EnvPrelude::Designation.local() {
            return err!(self.agent(), InvalidNodeEntry(second.into()));
        }
        if self.parse_node(&third)?.local() != EnvPrelude::TellHandler.local() {
            return err!(self.agent(), InvalidNodeEntry(third.into()));
        }

        for (entry, proper) in SexpIntoIter::from(remainder) {
            if !proper {
                return err!(self.agent(), LangError::InvalidSexp(*entry))?;
            }
            match *entry {
                Sexp::Primitive(primitive) => {
                    if let Primitive::Symbol(_sym) = primitive {
                        self.agent_mut().env().insert_atom();
                    } else {
                        return err!(self.agent(), ExpectedSymbol);
                    }
                }
                Sexp::Cons(_) => {
                    let (_name, command) = break_sexp!(entry => (Symbol, HeapSexp), self.agent())?;
                    let structure = self.eval_structure(command, &builtins)?;
                    self.agent_mut().env().insert_structure(structure);
                }
            }
        }
        Ok(())
    }

    fn parse_node(&mut self, sym: &Symbol) -> Result<Node, Error> {
        EnvManager::<Policy>::parse_node_inner(self.agent_mut(), sym)
    }

    fn parse_node_inner(agent: &mut Agent, sym: &Symbol) -> Result<Node, Error> {
        match policy_admin(sym.as_str()).unwrap() {
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
        hsexp: HeapSexp,
        builtins: &HashMap<&'static str, BuiltIn>,
    ) -> Result<Sexp, Error> {
        match *hsexp {
            Sexp::Primitive(Primitive::Symbol(sym)) => return Ok(self.parse_node(&sym)?.into()),
            Sexp::Primitive(Primitive::AmString(s)) => return Ok(s.into()),
            _ => {}
        }

        let (command, _) = break_sexp!(hsexp.iter() => (&Symbol; remainder), self.agent())?;
        // Note(subtle): during the initial deserialization of the meta & lang
        // envs, Reflective context nodes are only valid because they're specially
        // set before actual context bootstrapping occurs.
        if let Ok(node) = self.parse_node(&command) {
            let process_primitive = |agent: &mut Agent, p: &Primitive| match p {
                Primitive::Node(n) => Ok(*n),
                Primitive::Symbol(s) => EnvManager::<Policy>::parse_node_inner(agent, &s),
                _ => panic!(),
            };

            if Procedure::valid_discriminator(node, self.agent()) {
                return Ok(Procedure::reflect(*hsexp, self.agent_mut(), process_primitive)?.into());
            } else if LocalNodeTable::valid_discriminator(node, self.agent()) {
                return Ok(
                    LocalNodeTable::reflect(*hsexp, self.agent_mut(), process_primitive)?.into(),
                );
            } else if SymbolTable::valid_discriminator(node, self.agent()) {
                return Ok(
                    SymbolTable::reflect(*hsexp, self.agent_mut(), process_primitive)?.into(),
                );
            }
        }

        let (command, cdr) = break_sexp!(hsexp => (Symbol; remainder), self.agent())?;
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
                let (path,) = break_sexp!(cdr.unwrap() => (AmString), self.agent())?;
                Ok(Path::new(path.as_str().into()).into())
            }
            _ => panic!("{}", command),
        }
    }

    fn deserialize_triples(&mut self, structure: Sexp) -> Result<(), Error> {
        let (command, remainder) = break_sexp!(structure => (Symbol; remainder), self.agent())?;
        if command.as_str() != "triples" {
            return err!(self.agent(), UnexpectedCommand(command.into()));
        }

        for (entry, proper) in SexpIntoIter::from(remainder) {
            if !proper {
                return err!(self.agent(), LangError::InvalidSexp(*entry))?;
            }
            let (s, p, o) = break_sexp!(entry => (Symbol, Symbol, Symbol), self.agent())?;

            let subject = self.parse_node(&s)?;
            let predicate = self.parse_node(&p)?;
            let object = self.parse_node(&o)?;

            self.agent_mut().env().insert_triple(
                subject.local(),
                predicate.local(),
                object.local(),
            );

            let designation = self.agent().context().designation();
            if predicate.local() == designation && object.local() != designation {
                let name = if let Ok(sym) =
                    <Symbol>::try_from(self.agent_mut().designate(object.into()))
                {
                    sym
                } else {
                    println!(
                        "{} {} {:?}",
                        subject,
                        object,
                        self.agent_mut().designate(object.into()).unwrap()
                    );
                    return err!(self.agent(), ExpectedSymbol);
                };

                if let Ok(table) = <&mut SymbolTable>::try_from(
                    self.agent_mut().env().entry_mut(designation).as_option(),
                ) {
                    table.insert(name, subject);
                } else {
                    panic!("Env designation isn't a symbol table");
                }
            }
        }
        Ok(())
    }
}
