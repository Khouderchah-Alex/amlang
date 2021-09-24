use log::{debug, info, warn};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path as StdPath;

use super::agent::Agent;
use super::agent_state::{AgentState, AMLANG_DESIGNATION};
use super::amlang_context::AmlangContext;
use super::amlang_wrappers::quote_wrapper;
use super::env_policy::EnvPolicy;
use crate::builtins::generate_builtin_map;
use crate::environment::environment::EnvObject;
use crate::environment::LocalNode;
use crate::lang_err;
use crate::model::{Eval, Model, Ret};
use crate::parser::{self, parse_sexp};
use crate::primitive::symbol_policies::{policy_admin, AdminSymbolInfo};
use crate::primitive::{
    AmString, BuiltIn, Node, Path, Primitive, Procedure, Symbol, SymbolTable, ToSymbol,
};
use crate::sexp::{Cons, HeapSexp, Sexp, SexpIntoIter};
use crate::token::file_stream::{self, FileStream, FileStreamError};

use DeserializeError::*;


pub struct EnvManager<Policy: EnvPolicy> {
    state: AgentState,
    #[allow(dead_code)]
    policy: Policy,
}

#[derive(Debug)]
pub enum DeserializeError {
    FileStreamError(file_stream::FileStreamError),
    ParseError(parser::ParseError),
    MissingNodeSection,
    MissingTripleSection,
    ExtraneousSection,
    UnexpectedCommand(Sexp),
    ExpectedSymbol,
    UnrecognizedBuiltIn(Symbol),
    LangErr(lang_err::LangErr),
}

// Consumes manager and replaces placeholder'd context nodes through
// AmlangDesignation lookups.
macro_rules! bootstrap_context {
    (
        $manager:expr,
        $($node:ident : $query:expr),+
        $(,)?
    ) => {
        let ($($node,)+) = {
            let desig_node = $manager.state().context().designation();
            let table = if let Ok(table) =
                <&SymbolTable>::try_from(
                    $manager.state_mut().env().node_structure(desig_node)
                ) {
                table
            } else {
                panic!("Env designation isn't a symbol table");
            };

            let lookup = |s: &str| -> Result<LocalNode, DeserializeError> {
                if let Some(node) = table.lookup(s) {
                    Ok(node.local())
                } else {
                    err!(UnboundSymbol(s.to_symbol_or_panic(policy_admin))).map_err(|e| LangErr(e))
                }
            };
            (
                $(lookup($query)?,)+
            )
        };

        // Fill in placeholder'd context nodes.
        let context = $manager.state_mut().context_mut();
        $(context.$node = $node;)+
    };
}

impl<Policy: EnvPolicy> EnvManager<Policy> {
    pub fn bootstrap<P: AsRef<StdPath>>(meta_path: P) -> Result<Self, DeserializeError> {
        let mut meta = Policy::DefaultEnv::default();
        EnvManager::<Policy>::initialize_env(LocalNode::default(), &mut meta);

        let policy = Policy::from_root_env(meta);
        let context = AmlangContext::new(
            policy.new_overlay(),
            LocalNode::new(0), // self
            LocalNode::new(1), // designation
        );

        // Bootstrap meta env.
        let meta_state = AgentState::new(
            Node::new(LocalNode::default(), context.self_node()),
            context,
        );
        let mut manager = Self {
            state: meta_state,
            policy: policy,
        };
        manager.deserialize_curr_env(meta_path)?;
        bootstrap_context!(manager,
                           imports: "__imports",
                           import_table: "__import_table",
                           serialize_path: "__serialize_path",
        );
        info!("Meta env bootstrapping complete.");

        // Make context usable for bootstrapping lang.
        {
            let lang_env = manager.state().find_env("lang.env").unwrap();
            manager.initialize_env_node(lang_env);
            let context = manager.state_mut().context_mut();
            context.lang_env = lang_env;

            // TODO(flex) Find more flexible approch to bootstrapping Procedure
            // nodes. It's worth noting that these are only needed for
            // deserializing Procedures in the lang env (since otherwise the
            // context is already fully bootstrapped).
            context.lambda = LocalNode::new(13);
            context.apply = LocalNode::new(33);
            context.branch = LocalNode::new(41);
            context.fexpr = LocalNode::new(45);
            context.progn = LocalNode::new(49);
        }

        // Bootstrap lang env.
        let lang_env = manager.state().context().lang_env();
        let serialize_path = manager.state().context().serialize_path;
        manager.state_mut().jump_env(lang_env);
        let lang_path = {
            let meta = manager.state().context().meta();
            let lang_path_triple = meta
                .match_but_object(lang_env, serialize_path)
                .iter()
                .next()
                .unwrap()
                .clone();
            let lang_path_node = meta.triple_object(lang_path_triple);

            <&Path>::try_from(meta.node_structure(lang_path_node))
                .unwrap()
                .clone()
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
                           env_find: "env_find",
                           t: "true",
                           f: "false",
        );
        info!("Lang env bootstrapping complete.");

        // Since we've bootstrapped the lang by here, we can clone the context
        // without worrying about using stale/placeholder values.
        let context = manager.state().context().clone();
        let env_triples = context.meta().match_predicate(serialize_path);
        // TODO(func) Allow for delayed loading of environments.
        for triple in env_triples {
            let subject_node = context.meta().triple_subject(triple);
            if subject_node == context.lang_env {
                continue;
            }

            let object_node = context.meta().triple_object(triple);
            let object = context.meta().node_structure(object_node).unwrap();
            let env_path = <&Path>::try_from(&*object).unwrap();

            manager.initialize_env_node(subject_node);
            manager
                .state_mut()
                .jump(Node::new(subject_node, LocalNode::default()));
            manager.deserialize_curr_env(env_path.as_std_path())?;
        }

        manager.state_mut().jump_env(lang_env);
        Ok(manager)
    }

    pub fn create_env<P: AsRef<StdPath>>(&mut self, path: P) -> LocalNode {
        let serialize_path = self.state().context().serialize_path;
        let meta = self.state_mut().context_mut().meta_mut();
        let env_node = meta.insert_structure(Box::new(Policy::DefaultEnv::default()).into());

        let path_node = meta.insert_structure(Path::new(path.as_ref().to_path_buf()).into());
        meta.insert_triple(env_node, serialize_path, path_node);

        let env =
            if let Some(Sexp::Primitive(Primitive::Env(env))) = meta.node_structure_mut(env_node) {
                env
            } else {
                panic!()
            };
        EnvManager::<Policy>::initialize_env(env_node, &mut **env);
        env_node
    }


    fn initialize_env(env_node: LocalNode, env: &mut EnvObject) {
        // Set up self node.
        let _self_node = env.insert_structure(Node::new(LocalNode::default(), env_node).into());

        // Set up designation node.
        let designation = env.insert_structure(SymbolTable::default().into());
        if let Ok(table) = <&mut SymbolTable>::try_from(env.node_structure_mut(designation)) {
            table.insert(
                AMLANG_DESIGNATION.to_symbol_or_panic(policy_admin),
                Node::new(env_node, designation),
            );
        } else {
            panic!("Env designation isn't a symbol table");
        }
    }

    fn initialize_env_node(&mut self, env_node: LocalNode) {
        let meta = self.state_mut().context_mut().meta_mut();
        if let Some(sexp) = meta.node_structure_mut(env_node) {
            *sexp = Box::new(Policy::DefaultEnv::default()).into();
            let env = if let Some(Sexp::Primitive(Primitive::Env(env))) =
                meta.node_structure_mut(env_node)
            {
                env
            } else {
                panic!()
            };
            EnvManager::<Policy>::initialize_env(env_node, &mut **env);
        } else {
            panic!()
        }
    }
}


// {,De}serialization functionality.
impl<Policy: EnvPolicy> EnvManager<Policy> {
    pub fn serialize_full<P: AsRef<StdPath>>(&mut self, out_path: P) -> std::io::Result<()> {
        let original_pos = self.state().pos();

        self.state_mut()
            .jump(Node::new(LocalNode::default(), LocalNode::default()));
        self.serialize_curr_env(out_path)?;

        // Serialize recursively.
        let serialize_path = self.state().context().serialize_path;
        let env_triples = self
            .state()
            .context()
            .meta()
            .match_predicate(serialize_path);
        for triple in env_triples {
            let subject_node = self.state().context().meta().triple_subject(triple);
            let path = {
                let object_node = self.state().context().meta().triple_object(triple);
                let object = self
                    .state()
                    .context()
                    .meta()
                    .node_structure(object_node)
                    .unwrap();
                <&Path>::try_from(object).unwrap().clone()
            };

            self.state_mut().jump_env(subject_node);
            self.serialize_curr_env(path.as_std_path())?;
        }

        self.state_mut().jump(original_pos);
        Ok(())
    }

    pub fn serialize_curr_env<P: AsRef<StdPath>>(&mut self, out_path: P) -> std::io::Result<()> {
        let file = File::create(out_path.as_ref())?;
        let mut w = BufWriter::new(file);

        write!(&mut w, "(nodes")?;
        for node in self.state_mut().env().all_nodes() {
            write!(&mut w, "\n    ")?;

            // Subtle: Cloning of Env doesn't actually copy data. In this case,
            // the resulting Env object will be invalid and should only stand as
            // a placeholder to determine typing.
            //
            // TODO(func) SharedEnv impl.
            let s = self.state_mut().env().node_structure(node).cloned();
            let (write_structure, add_quote) = match &s {
                Some(sexp) => match sexp {
                    Sexp::Primitive(Primitive::SymbolTable(_)) => (false, false),
                    // Don't quote structures with special deserialize ops.
                    Sexp::Primitive(Primitive::BuiltIn(_)) => (true, false),
                    Sexp::Primitive(Primitive::Procedure(_)) => (true, false),
                    Sexp::Primitive(Primitive::Node(_)) => (true, false),
                    Sexp::Primitive(Primitive::Env(_)) => (true, false),
                    Sexp::Primitive(Primitive::Path(_)) => (true, false),
                    Sexp::Primitive(Primitive::AmString(_)) => (true, false),
                    _ => (true, true),
                },
                _ => (false, false),
            };

            if write_structure {
                write!(&mut w, "(")?;
            }
            self.serialize_list_internal(&mut w, &node.globalize(self.state()).into(), 0)?;
            if write_structure {
                write!(&mut w, "\t")?;
                if add_quote {
                    write!(&mut w, "'")?;
                }
                self.serialize_list_internal(&mut w, &s.unwrap(), 1)?;
                write!(&mut w, ")")?;
            }
        }
        write!(&mut w, "\n)\n\n")?;

        write!(&mut w, "(triples")?;
        for triple in self.state_mut().env().match_all() {
            write!(&mut w, "\n    ")?;
            let s = triple.reify(self.state_mut());
            self.serialize_list_internal(&mut w, &s, 1)?;
        }
        writeln!(&mut w, "\n)")?;
        info!(
            "Serialized env {} @ \"{}\".",
            self.state().pos().env(),
            out_path.as_ref().to_string_lossy()
        );
        Ok(())
    }

    pub fn deserialize_curr_env<P: AsRef<StdPath>>(
        &mut self,
        in_path: P,
    ) -> Result<(), DeserializeError> {
        let stream = match FileStream::new(in_path.as_ref(), policy_admin) {
            Ok(stream) => stream,
            Err(FileStreamError::IoError(ref e)) if e.kind() == std::io::ErrorKind::NotFound => {
                warn!("Env file not found: {}", in_path.as_ref().to_string_lossy());
                warn!(
                    "Leaving env {} unchanged. If this is intended, then all is well.",
                    self.state().pos().env()
                );
                return Ok(());
            }
            Err(err) => return Err(FileStreamError(err)),
        };
        let mut peekable = stream.peekable();

        match parse_sexp(&mut peekable, 0) {
            Ok(Some(parsed)) => self.deserialize_nodes(parsed)?,
            Ok(None) => return Err(MissingNodeSection),
            Err(err) => return Err(ParseError(err)),
        };
        match parse_sexp(&mut peekable, 0) {
            Ok(Some(parsed)) => self.deserialize_triples(parsed)?,
            Ok(None) => return Err(MissingTripleSection),
            Err(err) => return Err(ParseError(err)),
        };
        match parse_sexp(&mut peekable, 0) {
            Ok(Some(_)) => return Err(ExtraneousSection),
            Ok(None) => {}
            Err(err) => return Err(ParseError(err)),
        };

        info!(
            "Loaded env {} from \"{}\".",
            self.state().pos().env(),
            in_path.as_ref().to_string_lossy()
        );
        debug!(
            "  Node count:    {}",
            self.state_mut().env().all_nodes().len()
        );
        debug!(
            "  Triple count:  {}",
            self.state_mut().env().match_all().len()
        );
        Ok(())
    }


    fn serialize_list_internal<W: std::io::Write>(
        &mut self,
        w: &mut W,
        structure: &Sexp,
        depth: usize,
    ) -> std::io::Result<()> {
        structure.write_list(w, depth, &mut |writer, primitive, depth| {
            self.serialize_primitive(writer, primitive, depth)
        })
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
                let proc_sexp = proc.reify(self.state_mut());
                self.serialize_list_internal(w, &proc_sexp, depth + 1)
            }
            Primitive::Node(node) => {
                if let Some(triple) = self
                    .state_mut()
                    .access_env(node.env())
                    .unwrap()
                    .node_as_triple(node.local())
                {
                    if node.env() != self.state().pos().env() {
                        write!(w, "^{}", node.env().id())?;
                    }
                    return write!(w, "^t{}", self.state_mut().env().triple_index(triple));
                }

                if node.env() != self.state().pos().env() {
                    write!(w, "^{}", node.env().id())?;
                }
                write!(w, "^{}", node.local().id())
            }
            Primitive::Env(_) => write!(w, "(__env)"),
            _ => write!(w, "{}", primitive),
        }
    }

    fn deserialize_nodes(&mut self, structure: HeapSexp) -> Result<(), DeserializeError> {
        let builtins = generate_builtin_map();
        let (command, remainder) =
            break_by_types!(*structure, Symbol; remainder).map_err(|e| LangErr(e))?;
        if command.as_str() != "nodes" {
            return Err(UnexpectedCommand(command.into()));
        }

        let iter = SexpIntoIter::try_from(remainder).map_err(|e| LangErr(e))?;
        for entry in iter.skip(2) {
            match *entry {
                Sexp::Primitive(primitive) => {
                    if let Primitive::Symbol(_sym) = primitive {
                        self.state_mut().env().insert_atom();
                    } else {
                        return Err(ExpectedSymbol);
                    }
                }
                Sexp::Cons(cons) => {
                    let (_name, command) =
                        break_by_types!(cons.into(), Symbol, Sexp).map_err(|e| LangErr(e))?;
                    let structure = self.eval_structure(command, &builtins)?;
                    self.state_mut().env().insert_structure(structure);
                }
            }
        }
        Ok(())
    }

    fn parse_symbol(&mut self, sym: &Symbol) -> Result<Node, DeserializeError> {
        EnvManager::<Policy>::parse_symbol_inner(self.state_mut(), sym).map_err(|e| LangErr(e))
    }

    fn parse_symbol_inner(state: &mut AgentState, sym: &Symbol) -> Result<Node, lang_err::LangErr> {
        match policy_admin(sym.as_str()).unwrap() {
            AdminSymbolInfo::Identifier => err!(UnboundSymbol(sym.clone())),
            AdminSymbolInfo::LocalNode(node) => Ok(node.globalize(state)),
            AdminSymbolInfo::LocalTriple(idx) => {
                let triple = state.env().triple_from_index(idx);
                Ok(triple.node().globalize(state))
            }
            AdminSymbolInfo::GlobalNode(env, node) => Ok(Node::new(env, node)),
            AdminSymbolInfo::GlobalTriple(env, idx) => {
                Ok(Node::new(env, state.env().triple_from_index(idx).node()))
            }
        }
    }

    fn eval_structure(
        &mut self,
        structure: Sexp,
        builtins: &HashMap<&'static str, BuiltIn>,
    ) -> Result<Sexp, DeserializeError> {
        if let Ok(sym) = <&Symbol>::try_from(&structure) {
            return Ok(self.parse_symbol(sym)?.into());
        } else if let Ok(s) = <&AmString>::try_from(&structure) {
            return Ok(s.clone().into());
        }

        let (command, cdr) =
            break_by_types!(structure, Symbol; remainder).map_err(|e| LangErr(e))?;

        if let Ok(node) = self.parse_symbol(&command) {
            let context = self.state().context();

            // Note(subtle): during the initial deserialization of the lang env,
            // Procedure context nodes are only valid because they're specially set
            // before actual context bootstrapping occurs.
            if node.env() == context.lang_env() {
                return Ok(Procedure::reflect(
                    Sexp::Cons(Cons::new(Some(HeapSexp::new(command.into())), cdr)).into(),
                    self.state_mut(),
                    |state, p| match p {
                        Primitive::Node(n) => Ok(*n),
                        Primitive::Symbol(s) => {
                            Ok(EnvManager::<Policy>::parse_symbol_inner(state, &s)?)
                        }
                        _ => panic!(),
                    },
                )
                .map_err(|e| LangErr(e))?
                .into());
            }
        }

        match command.as_str() {
            "quote" => Ok(quote_wrapper(cdr).map_err(|e| LangErr(e))?),
            "__builtin" => {
                if let Ok(sym) = <Symbol>::try_from(quote_wrapper(cdr).map_err(|e| LangErr(e))?) {
                    if let Some(builtin) = builtins.get(sym.as_str()) {
                        Ok(builtin.clone().into())
                    } else {
                        Err(UnrecognizedBuiltIn(sym.clone()))
                    }
                } else {
                    Err(ExpectedSymbol)
                }
            }
            // TODO(func) Remove and load as atom once atomic nodes can be
            // turned into structured nodes.
            "__env" => Ok(Sexp::default()),
            "__path" => {
                let (path,) = break_by_types!(*cdr.unwrap(), AmString).map_err(|e| LangErr(e))?;
                Ok(Path::new(path.as_str().into()).into())
            }
            _ => panic!("{}", command),
        }
    }

    fn deserialize_triples(&mut self, structure: HeapSexp) -> Result<(), DeserializeError> {
        let (command, remainder) =
            break_by_types!(*structure, Symbol; remainder).map_err(|e| LangErr(e))?;
        if command.as_str() != "triples" {
            return Err(UnexpectedCommand(command.into()));
        }

        let iter = match SexpIntoIter::try_from(remainder) {
            Ok(iter) => iter,
            Err(lang_err::LangErr {
                kind: lang_err::ErrKind::WrongArgumentCount { .. },
                ..
            }) => return Ok(()),
            Err(err) => return Err(DeserializeError::LangErr(err)),
        };

        for entry in iter {
            let (s, p, o) =
                break_by_types!(*entry, Symbol, Symbol, Symbol).map_err(|e| LangErr(e))?;

            let subject = self.parse_symbol(&s)?;
            let predicate = self.parse_symbol(&p)?;
            let object = self.parse_symbol(&o)?;

            self.state_mut().env().insert_triple(
                subject.local(),
                predicate.local(),
                object.local(),
            );

            let designation = self.state().context().designation();
            if predicate.local() == designation && object.local() != designation {
                let name = if let Ok(sym) =
                    <Symbol>::try_from(self.state_mut().designate(Primitive::Node(object)))
                {
                    sym
                } else {
                    println!(
                        "{} {} {:?}",
                        subject,
                        object,
                        self.state_mut().designate(Primitive::Node(object)).unwrap()
                    );
                    return Err(ExpectedSymbol);
                };

                if let Ok(table) = <&mut SymbolTable>::try_from(
                    self.state_mut().env().node_structure_mut(designation),
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


impl<Policy: EnvPolicy> Agent for EnvManager<Policy> {
    fn state(&self) -> &AgentState {
        &self.state
    }
    fn state_mut(&mut self) -> &mut AgentState {
        &mut self.state
    }
}

impl<Policy: EnvPolicy> Eval for EnvManager<Policy> {
    fn eval(&mut self, _structure: HeapSexp) -> Ret {
        Ok(Sexp::default())
    }
}
