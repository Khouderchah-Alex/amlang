use log::{debug, info, warn};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path as StdPath;

use super::amlang_context::AmlangContext;
use super::amlang_wrappers::quote_wrapper;
use super::deserialize_error::DeserializeError::*;
use super::env_header::EnvHeader;
use super::env_policy::EnvPolicy;
use super::env_prelude::EnvPrelude;
use super::Agent;
use crate::agent::lang_error::LangError;
use crate::builtins::generate_builtin_map;
use crate::env::local_node::{LocalId, LocalNode};
use crate::env::meta_env::MetaEnv;
use crate::env::Environment;
use crate::error::Error;
use crate::model::Reflective;
use crate::parser::Parser;
use crate::primitive::prelude::*;
use crate::primitive::symbol_policies::policy_env_serde;
use crate::primitive::table::Table;
use crate::sexp::{HeapSexp, Sexp, SexpIntoIter};
use crate::stream::input::FileReader;
use crate::token::Tokenizer;


pub struct EnvManager<Policy: EnvPolicy> {
    agent: Agent,
    policy: Policy,
}

/// Sanity check bootstrapped context nodes match env.
macro_rules! verify_context {
    (
        $manager:expr,
        $($node:ident : $query:expr),+
        $(,)?
    ) => {
        let ($($node,)+) = {
            let desig_node = $manager.agent().context().designation();
            let entry = $manager.agent().env().entry(desig_node);
            let table = if let Ok(table) =
                <&SymNodeTable>::try_from(entry.as_option()) {
                    table
                } else {
                    panic!("Env designation isn't a symbol table");
                };

            let lookup = |s: &str| -> Result<LocalNode, Error> {
                if let Some(node) = table.lookup(s) {
                    Ok(node.local())
                } else {
                    Err(Error::no_cont(
                        Box::new(LangError::UnboundSymbol(s.to_symbol_or_panic(policy_admin)))))?
                }
            };
            (
                $(lookup($query)?,)+
            )
        };

        // Verify context nodes.
        let context = $manager.agent_mut().context_mut();
        $(assert_eq!(context.$node(), $node);)+
    };
}

impl<Policy: EnvPolicy> EnvManager<Policy> {
    pub fn bootstrap<P: AsRef<StdPath>>(in_path: P) -> Result<Self, Error> {
        let mut policy = Policy::default();
        let meta = MetaEnv::new(EnvManager::create_env(&mut policy, LocalNode::default()));

        let mut context_path = in_path.as_ref().to_path_buf();
        context_path.push("context.bootstrap");
        let context = EnvManager::<Policy>::bootstrap_context(meta, context_path)?;
        info!("Context bootstrapping complete.");

        // Bootstrap meta env.
        let meta_agent = Agent::new(
            Node::new(LocalNode::default(), context.self_node()),
            context.clone(),
        );
        let mut manager = Self {
            agent: meta_agent,
            policy: policy,
        };

        let mut meta_path = in_path.as_ref().to_path_buf();
        meta_path.push("meta.env");
        manager.deserialize_curr_env(meta_path)?;
        verify_context!(manager,
                        imports: "__imports",
                        import_table: "__import_table",
                        serialize_path: "__serialize_path",
        );
        info!("Meta env bootstrapping complete.");

        // Bootstrap lang env.
        let lang_env = context.lang_env();
        manager.initialize_env_node(lang_env);
        let serialize_path = context.serialize_path();
        manager.agent_mut().jump_env(lang_env);
        let lang_path = {
            let meta = context.meta();
            let lang_path_node = meta
                .match_but_object(lang_env, serialize_path)
                .objects()
                .next()
                .unwrap()
                .clone();

            Path::try_from(meta.entry(lang_path_node).owned()).unwrap()
        };
        manager.deserialize_curr_env(lang_path.as_std_path())?;
        verify_context!(manager,
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
                        sym_node_table: "table-sym-node",
                        sym_sexp_table: "table-sym-sexp",
                        local_node_table: "table-lnode",
                        label: "label",
                        vector: "vector",
                        set: "set!",
                        node: "node",
                        anon: "$"
        );
        info!("Lang env bootstrapping complete.");

        // Load all other envs.
        // TODO(func) Allow for delayed loading of environments.
        let env_triples = context.meta().match_predicate(serialize_path).triples();
        for triple in env_triples {
            let subject_node = context.meta().triple_subject(triple);
            if subject_node == context.lang_env() {
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
        let serialize_path = self.agent().context().serialize_path();
        let env_node = self.agent_mut().context_mut().meta_mut().insert_atom();
        self.initialize_env_node(env_node);

        let meta = self.agent_mut().context_mut().meta_mut();
        let path_node = meta.insert_structure(Path::new(path.as_ref().to_path_buf()).into());
        meta.insert_triple(env_node, serialize_path, path_node);

        env_node
    }

    fn bootstrap_context<P: AsRef<StdPath>>(
        meta: MetaEnv,
        in_path: P,
    ) -> Result<AmlangContext, Error> {
        let placeholder_context = AmlangContext::new(meta);
        let mut placeholder_agent = Agent::new(
            Node::new(LocalNode::default(), placeholder_context.self_node()),
            placeholder_context,
        );

        let input = match FileReader::new(in_path) {
            Ok(input) => input,
            Err(err) => return err!(placeholder_agent, IoError(err)),
        };
        let mut stream = pull_transform!(input
                                         =>> Tokenizer::new(policy_env_serde)
                                         =>. Parser::new());

        let s = match stream.next() {
            Some(Ok(parsed)) => parsed,
            None => return err!(placeholder_agent, MissingNodeSection),
            Some(Err(err)) => return Err(err),
        };
        AmlangContext::reflect(s, &mut placeholder_agent, |agent, p| {
            EnvManager::<Policy>::parse_node_inner(
                agent,
                &Symbol::try_from(Sexp::from(p.clone())).unwrap(),
            )
        })
    }

    fn create_env(policy: &mut Policy, env_node: LocalNode) -> Box<Policy::StoredEnv> {
        let mut env = Policy::BaseEnv::default();

        // Create nodes.
        let self_env = env.insert_structure(Node::new(LocalNode::default(), env_node).into());
        let designation = env.insert_structure(SymNodeTable::default().into());
        let tell_handler = env.insert_atom();
        let mut reserved_id = env.all_nodes().len() as LocalId;
        while let Some(_) = LocalNode::new(reserved_id).as_prelude() {
            env.insert_structure("RESERVED".to_symbol_or_panic(policy_admin).into());
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
        self.agent_mut()
            .context_mut()
            .meta_mut()
            .insert_env(env_node, env);
    }
}


// {,De}serialization functionality.
impl<Policy: EnvPolicy> EnvManager<Policy> {
    pub fn serialize_full<P: AsRef<StdPath>>(&mut self, out_path: P) -> std::io::Result<()> {
        let original_pos = self.agent().pos();

        // Serialize meta env.
        self.agent_mut()
            .jump(Node::new(LocalNode::default(), LocalNode::default()));
        let mut meta_path = out_path.as_ref().to_path_buf();
        meta_path.push("meta.env");
        self.serialize_curr_env(meta_path)?;

        // Serialize context.
        {
            let mut path = out_path.as_ref().to_path_buf();
            path.push("context.bootstrap");
            let file = File::create(path)?;
            let mut w = BufWriter::new(file);

            let context = self.agent().context().clone().reify(self.agent_mut());
            write!(&mut w, "(\n\n")?;
            for (sublist, _) in context {
                self.serialize_list_internal(&mut w, &sublist, 0)?;
            }
            write!(&mut w, ")\n")?;
        }

        // Serialize envs in meta env.
        let serialize_path = self.agent().context().serialize_path();
        let env_triples = self
            .agent()
            .context()
            .meta()
            .match_predicate(serialize_path)
            .triples();
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

    pub fn serialize_curr_env<P: AsRef<StdPath>>(&self, out_path: P) -> std::io::Result<()> {
        let file = File::create(out_path.as_ref())?;
        let mut w = BufWriter::new(file);

        let env = self.agent().env();
        let header = EnvHeader::from_env(env).reify(self.agent());
        self.serialize_list_internal(&mut w, &header, 0)?;

        write!(&mut w, "(nodes")?;
        for (i, node) in env.all_nodes().into_iter().enumerate() {
            write!(&mut w, "\n    ")?;

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
                    Sexp::Primitive(Primitive::Path(_)) => (true, false),
                    Sexp::Primitive(Primitive::LangString(_)) => (true, false),
                    _ => (true, true),
                },
                _ => (false, false),
            };

            let node = node.globalize(self.agent());
            let line = if write_structure {
                let mut structure = s.unwrap();
                if add_quote {
                    structure = list!("quote".to_symbol_or_panic(policy_admin), structure);
                }
                list!(node, structure)
            } else {
                write!(&mut w, " ")?; // Add space to align with structured lines.
                node.into()
            };
            self.serialize_list_internal(&mut w, &line, 1)?;
        }
        write!(&mut w, "\n)\n\n")?;

        write!(&mut w, "(triples")?;
        for triple in env.match_all().triples() {
            write!(&mut w, "\n    ")?;
            let s = triple.reify(self.agent());
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
        let input = match FileReader::new(in_path.as_ref()) {
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
        let mut stream = pull_transform!(input
                                         =>> Tokenizer::new(policy_env_serde)
                                         =>. Parser::new());

        let _header = match stream.next() {
            Some(Ok(parsed)) => EnvHeader::reflect(parsed, self.agent_mut(), |_agent, p| {
                if let Primitive::Node(n) = p {
                    Ok(*n)
                } else {
                    panic!();
                }
            })?,
            None => return err!(self.agent(), MissingHeaderSection),
            Some(Err(mut err)) => {
                err.set_cont(self.agent().exec_state().clone());
                return Err(err);
            }
        };
        match stream.next() {
            Some(Ok(parsed)) => self.deserialize_nodes(parsed)?,
            None => return err!(self.agent(), MissingNodeSection),
            Some(Err(mut err)) => {
                err.set_cont(self.agent().exec_state().clone());
                return Err(err);
            }
        };
        match stream.next() {
            Some(Ok(parsed)) => self.deserialize_triples(parsed)?,
            None => return err!(self.agent(), MissingTripleSection),
            Some(Err(mut err)) => {
                err.set_cont(self.agent().exec_state().clone());
                return Err(err);
            }
        };
        match stream.next() {
            Some(Ok(_)) => return err!(self.agent(), ExtraneousSection),
            None => {}
            Some(Err(mut err)) => {
                err.set_cont(self.agent().exec_state().clone());
                return Err(err);
            }
        };

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
        &self,
        w: &mut W,
        primitive: &Primitive,
        depth: usize,
    ) -> std::io::Result<()> {
        match primitive {
            Primitive::LangString(s) => write!(w, "\"{}\"", s.clone().to_escaped()),
            Primitive::Symbol(symbol) => write!(w, "{}", symbol.as_str()),
            Primitive::Path(path) => {
                write!(w, "(__path \"{}\")", path.as_std_path().to_string_lossy())
            }
            Primitive::BuiltIn(builtin) => write!(w, "(__builtin {})", builtin.name()),
            Primitive::Procedure(proc) => {
                let proc_sexp = proc.reify(self.agent());
                self.serialize_list_internal(w, &proc_sexp, depth)
            }
            Primitive::SymNodeTable(table) => {
                let sexp = table.reify(self.agent());
                self.serialize_list_internal(w, &sexp, depth)
            }
            Primitive::LocalNodeTable(table) => {
                let sexp = table.reify(self.agent());
                self.serialize_list_internal(w, &sexp, depth)
            }
            Primitive::Node(node) => {
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
                        self.agent_mut().define(None)?;
                    } else {
                        return err!(self.agent(), ExpectedSymbol);
                    }
                }
                Sexp::Cons(_) => {
                    let (_name, command) = break_sexp!(entry => (Symbol, HeapSexp), self.agent())?;
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
        hsexp: HeapSexp,
        builtins: &HashMap<&'static str, BuiltIn>,
    ) -> Result<Sexp, Error> {
        match *hsexp {
            Sexp::Primitive(Primitive::Symbol(sym)) => return Ok(self.parse_node(&sym)?.into()),
            Sexp::Primitive(Primitive::LangString(s)) => return Ok(s.into()),
            _ => {}
        }

        let (command, _) = break_sexp!(hsexp.iter() => (&Symbol; remainder), self.agent())?;
        // Note(subtle): during the initial deserialization of the meta & lang
        // envs, Reflective context nodes are only valid because they're specially
        // set before actual context bootstrapping occurs.
        if let Ok(node) = self.parse_node(&command) {
            let resolve = |agent: &Agent, p: &Primitive| match p {
                Primitive::Node(n) => Ok(*n),
                Primitive::Symbol(s) => EnvManager::<Policy>::parse_node_inner(agent, &s),
                _ => panic!(),
            };

            if Procedure::valid_discriminator(node, self.agent()) {
                return Ok(Procedure::reflect(*hsexp, self.agent(), resolve)?.into());
            } else if LocalNodeTable::valid_discriminator(node, self.agent()) {
                return Ok(LocalNodeTable::reflect(*hsexp, self.agent(), resolve)?.into());
            } else if SymNodeTable::valid_discriminator(node, self.agent()) {
                return Ok(SymNodeTable::reflect(*hsexp, self.agent(), resolve)?.into());
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
                let (path,) = break_sexp!(cdr.unwrap() => (LangString), self.agent())?;
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

            self.agent_mut().tell(subject, predicate, object)?;

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

                if let Ok(table) = <&mut SymNodeTable>::try_from(
                    self.agent_mut()
                        .env_mut()
                        .entry_mut(designation)
                        .as_option(),
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
