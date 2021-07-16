use log::{info, warn};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path as StdPath;
use std::sync::Arc;

use super::agent::Agent;
use super::amlang_context::AmlangContext;
use super::amlang_wrappers::quote_wrapper;
use super::env_state::{EnvState, AMLANG_DESIGNATION};
use crate::builtins::generate_builtin_map;
use crate::environment::environment::EnvObject;
use crate::environment::mem_environment::MemEnvironment;
use crate::environment::LocalNode;
use crate::function::{self, Ret};
use crate::model::{Eval, Model};
use crate::parser::{self, parse_sexp};
use crate::primitive::symbol_policies::{policy_admin, AdminSymbolInfo};
use crate::primitive::{
    AmString, BuiltIn, Node, Path, Primitive, Procedure, Symbol, SymbolTable, ToSymbol,
};
use crate::sexp::{Cons, HeapSexp, Sexp, SexpIntoIter};
use crate::token::file_stream::{self, FileStream, FileStreamError};

use DeserializeError::*;


pub struct EnvManager {
    env_state: EnvState,
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
    EvalErr(function::EvalErr),
}

// Consumes manager and replaces placeholder'd context nodes through
// AmlangDesignation lookups.
macro_rules! bootstrap_context {
    (
        $manager:expr,
        $context:expr,
        $($node:ident : $query:expr),+
        $(,)?
    ) => {
        let ($($node,)+) = {
            let table = if let Ok(table) =
                <&SymbolTable>::try_from(
                    $manager.env_state().env().node_structure($context.designation())
                ) {
                table
            } else {
                panic!("Env designation isn't a symbol table");
            };

            let lookup = |s: &str| -> Result<LocalNode, DeserializeError> {
                if let Some(node) = table.lookup(s) {
                    Ok(node.local())
                } else {
                    Err(EvalErr(function::EvalErr::UnboundSymbol(s.to_symbol_or_panic(policy_admin))))
                }
            };
            (
                $(lookup($query)?,)+
            )
        };

        // To gain mutable access to context.
        ::std::mem::drop($manager);

        // Fill in placeholder'd context nodes.
        let c = Arc::get_mut(&mut $context).unwrap();
        {
            $(c.$node = $node;)+
        }
    };
}

impl EnvManager {
    pub fn bootstrap<P: AsRef<StdPath>>(meta_path: P) -> Result<Self, DeserializeError> {
        // Initially create meta as MemEnvironment.
        let mut meta = Box::new(MemEnvironment::new());
        EnvManager::initialize_env(LocalNode::default(), &mut *meta);

        let mut context = Arc::new(AmlangContext::new(
            meta,
            LocalNode::new(0), // self
            LocalNode::new(1), // designation
        ));

        // Bootstrap meta env.
        let meta_state = EnvState::new(LocalNode::default(), context.self_node(), context.clone());
        let mut manager = Self {
            env_state: meta_state,
        };
        manager.deserialize_curr_env(meta_path)?;
        bootstrap_context!(manager, context,
                           imports: "__imports",
                           import_table: "__import_table",
                           serialize_path: "__serialize_path",
        );
        info!("Meta env bootstrapping complete.");

        // Make context usable for bootstrapping lang.
        {
            let meta_state =
                EnvState::new(LocalNode::default(), context.self_node(), context.clone());
            let l = meta_state.find_env("lang.env").unwrap();
            std::mem::drop(meta_state);
            let mut c = Arc::get_mut(&mut context).unwrap();

            EnvManager::initialize_env_node(&mut *c.meta(), l);
            c.lang_env = l;

            // TODO(flex) Find more flexible approch to bootstrapping these nodes.
            c.lambda = LocalNode::new(13);
            c.apply = LocalNode::new(33);
            c.branch = LocalNode::new(41);
        }

        // Bootstrap lang env.
        let lang_state = EnvState::new(context.lang_env(), context.self_node(), context.clone());
        let mut manager = Self {
            env_state: lang_state,
        };
        let meta = manager.env_state().context().meta();
        let lang_path_triple = meta
            .match_but_object(context.lang_env(), context.serialize_path)
            .iter()
            .next()
            .unwrap()
            .clone();
        let lang_path_node = meta.triple_object(lang_path_triple);
        let lang_path = <&Path>::try_from(meta.node_structure(lang_path_node))
            .unwrap()
            .clone();
        std::mem::drop(meta);
        manager.deserialize_curr_env(lang_path.as_std_path())?;
        bootstrap_context!(manager, context,
                           quote: "quote",
                           lambda: "lambda",
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
        );
        info!("Lang env bootstrapping complete.");

        let mut bootstrapped = Self {
            env_state: EnvState::new(context.lang_env(), context.self_node(), context.clone()),
        };

        // TODO(func) Allow for delayed loading of environments.
        let meta = context.meta();
        let env_triples = meta.match_predicate(context.serialize_path);
        for triple in env_triples {
            let subject_node = meta.triple_subject(triple);
            if subject_node == context.lang_env {
                continue;
            }
            let object_node = meta.triple_object(triple);
            let object = meta.node_structure(object_node).unwrap();
            let path = <&Path>::try_from(&*object).unwrap();

            EnvManager::initialize_env_node(&mut *context.meta(), subject_node);
            bootstrapped
                .env_state()
                .jump(Node::new(subject_node, LocalNode::default()));
            bootstrapped.deserialize_curr_env(path.as_std_path())?;
        }

        bootstrapped.env_state().jump_env(context.lang_env);
        Ok(bootstrapped)
    }

    pub fn create_env<P: AsRef<StdPath>>(&mut self, path: P) -> LocalNode {
        let serialize_path = self.env_state().context().serialize_path;
        let meta = self.env_state().context().meta();
        // Initially create as MemEnvironment.
        let env_node = meta.insert_structure(Box::new(MemEnvironment::new()).into());

        let path_node = meta.insert_structure(Path::new(path.as_ref().to_path_buf()).into());
        meta.insert_triple(env_node, serialize_path, path_node);

        let env =
            if let Some(Sexp::Primitive(Primitive::Env(env))) = meta.node_structure_mut(env_node) {
                env
            } else {
                panic!()
            };
        EnvManager::initialize_env(env_node, &mut **env);
        env_node
    }

    pub fn serialize_full<P: AsRef<StdPath>>(&mut self, out_path: P) -> std::io::Result<()> {
        let original_pos = self.env_state().pos();

        self.env_state()
            .jump(Node::new(LocalNode::default(), LocalNode::default()));
        self.serialize_curr_env(out_path)?;

        // Serialize recursively.
        let serialize_path = self.env_state().context().serialize_path;
        let env_triples = self
            .env_state()
            .context()
            .meta()
            .match_predicate(serialize_path);
        for triple in env_triples {
            let subject_node = self.env_state().context().meta().triple_subject(triple);
            let path = {
                let object_node = self.env_state().context().meta().triple_object(triple);
                let object = self
                    .env_state()
                    .context()
                    .meta()
                    .node_structure(object_node)
                    .unwrap();
                <&Path>::try_from(object).unwrap().clone()
            };

            self.env_state().jump_env(subject_node);
            self.serialize_curr_env(path.as_std_path())?;
        }

        self.env_state().jump(original_pos);
        Ok(())
    }

    pub fn serialize_curr_env<P: AsRef<StdPath>>(&mut self, out_path: P) -> std::io::Result<()> {
        let file = File::create(out_path.as_ref())?;
        let mut w = BufWriter::new(file);

        write!(&mut w, "(nodes")?;
        for node in self.env_state().env().all_nodes() {
            write!(&mut w, "\n    ")?;

            let s = self.env_state().env().node_structure(node).cloned();
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
            self.serialize_list_internal(&mut w, &node.globalize(&self.env_state).into(), 0)?;
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
        for triple in self.env_state().env().match_all() {
            write!(&mut w, "\n    ")?;
            let s = triple.generate_structure(self.env_state());
            self.serialize_list_internal(&mut w, &s, 1)?;
        }
        writeln!(&mut w, "\n)")?;
        info!(
            "Serialized env {} @ \"{}\".",
            self.env_state().pos().env(),
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
                    self.env_state().pos().env()
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
            self.env_state().pos().env(),
            in_path.as_ref().to_string_lossy()
        );
        Ok(())
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

    fn initialize_env_node(meta: &mut EnvObject, env_node: LocalNode) {
        if let Some(sexp) = meta.node_structure_mut(env_node) {
            // Initially create as MemEnvironment.
            *sexp = Box::new(MemEnvironment::new()).into();
            let env = if let Some(Sexp::Primitive(Primitive::Env(env))) =
                meta.node_structure_mut(env_node)
            {
                env
            } else {
                panic!()
            };
            EnvManager::initialize_env(env_node, &mut **env);
        } else {
            panic!()
        }
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
            Primitive::Symbol(symbol) => write!(w, "{}", symbol.as_str()),
            Primitive::Path(path) => {
                write!(w, "(__path \"{}\")", path.as_std_path().to_string_lossy())
            }
            Primitive::BuiltIn(builtin) => write!(w, "(__builtin {})", builtin.name()),
            Primitive::Procedure(proc) => {
                let proc_sexp = proc.generate_structure(self.env_state());
                self.serialize_list_internal(w, &proc_sexp, depth + 1)
            }
            Primitive::Node(node) => {
                if let Some(triple) = self
                    .env_state()
                    .access_env(node.env())
                    .unwrap()
                    .node_as_triple(node.local())
                {
                    if node.env() != self.env_state().pos().env() {
                        write!(w, "^{}", node.env().id())?;
                    }
                    return write!(w, "^t{}", self.env_state().env().triple_index(triple));
                }

                if node.env() != self.env_state().pos().env() {
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
            break_by_types!(*structure, Symbol; remainder).map_err(|e| EvalErr(e))?;
        if command.as_str() != "nodes" {
            return Err(UnexpectedCommand(command.into()));
        }

        let iter = SexpIntoIter::try_from(remainder).map_err(|e| EvalErr(e))?;
        for entry in iter.skip(2) {
            match *entry {
                Sexp::Primitive(primitive) => {
                    if let Primitive::Symbol(_sym) = primitive {
                        self.env_state().env().insert_atom();
                    } else {
                        return Err(ExpectedSymbol);
                    }
                }
                Sexp::Cons(cons) => {
                    let (_name, command) =
                        break_by_types!(cons.into(), Symbol, Sexp).map_err(|e| EvalErr(e))?;
                    let structure = self.eval_structure(command, &builtins)?;
                    self.env_state().env().insert_structure(structure);
                }
            }
        }
        Ok(())
    }

    fn parse_symbol(&mut self, sym: &Symbol) -> Result<Node, DeserializeError> {
        match policy_admin(sym.as_str()).unwrap() {
            AdminSymbolInfo::Identifier => {
                Err(EvalErr(function::EvalErr::UnboundSymbol(sym.clone())))
            }
            AdminSymbolInfo::LocalNode(node) => Ok(node.globalize(self.env_state())),
            AdminSymbolInfo::LocalTriple(idx) => {
                let triple = self.env_state().env().triple_from_index(idx);
                Ok(triple.node().globalize(self.env_state()))
            }
            AdminSymbolInfo::GlobalNode(env, node) => Ok(Node::new(env, node)),
            AdminSymbolInfo::GlobalTriple(env, idx) => Ok(Node::new(
                env,
                self.env_state().env().triple_from_index(idx).node(),
            )),
        }
    }

    fn eval_structure(
        &mut self,
        structure: Sexp,
        builtins: &HashMap<&'static str, BuiltIn>,
    ) -> Result<Sexp, DeserializeError> {
        if let Ok(sym) = <&Symbol>::try_from(&structure) {
            return Ok(self.parse_symbol(sym)?.into());
        }

        let (command, cdr) =
            break_by_types!(structure, Symbol; remainder).map_err(|e| EvalErr(e))?;

        if let Ok(node) = self.parse_symbol(&command) {
            let context = self.env_state().context();

            // Note(subtle): during the initial deserialization of the lang env,
            // these context nodes are only valid because they're specially set
            // before actual context bootstrapping occurs.
            if node.env() == context.lang_env() {
                if node.local() == context.apply {
                    if cdr.is_none() {
                        return Err(EvalErr(function::EvalErr::WrongArgumentCount {
                            given: 0,
                            expected: function::ExpectedCount::Exactly(2),
                        }));
                    }

                    let (func, args) =
                        break_by_types!(*cdr.unwrap(), Symbol, Cons).map_err(|e| EvalErr(e))?;
                    let fnode = self.parse_symbol(&func)?;
                    let mut arg_nodes = Vec::with_capacity(args.iter().count());
                    for arg in args {
                        if let Ok(sym) = <&Symbol>::try_from(&*arg) {
                            arg_nodes.push(self.parse_symbol(sym)?);
                        } else {
                            return Err(EvalErr(function::EvalErr::InvalidSexp(*arg)));
                        }
                    }
                    return Ok(Procedure::Application(fnode, arg_nodes).into());
                } else if node.local() == context.lambda {
                    if cdr.is_none() {
                        return Err(EvalErr(function::EvalErr::WrongArgumentCount {
                            given: 0,
                            expected: function::ExpectedCount::AtLeast(2),
                        }));
                    }

                    let (params, body) =
                        break_by_types!(*cdr.unwrap(), Cons, Symbol).map_err(|e| EvalErr(e))?;
                    let mut param_nodes = Vec::with_capacity(params.iter().count());
                    for param in params {
                        if let Ok(sym) = <&Symbol>::try_from(&*param) {
                            param_nodes.push(self.parse_symbol(sym)?);
                        } else {
                            return Err(EvalErr(function::EvalErr::InvalidSexp(*param)));
                        }
                    }
                    let body_node = self.parse_symbol(&body)?;
                    return Ok(Procedure::Abstraction(param_nodes, body_node).into());
                } else if node.local() == context.branch {
                    if cdr.is_none() {
                        return Err(EvalErr(function::EvalErr::WrongArgumentCount {
                            given: 0,
                            expected: function::ExpectedCount::Exactly(3),
                        }));
                    }

                    let (pred, a, b) = break_by_types!(*cdr.unwrap(), Symbol, Symbol, Symbol)
                        .map_err(|e| EvalErr(e))?;
                    return Ok(Procedure::Branch(
                        self.parse_symbol(&pred)?,
                        self.parse_symbol(&a)?,
                        self.parse_symbol(&b)?,
                    )
                    .into());
                }
            }
        }

        match command.as_str() {
            "quote" => Ok(quote_wrapper(cdr).map_err(|e| EvalErr(e))?),
            "__builtin" => {
                if let Ok(sym) = <Symbol>::try_from(quote_wrapper(cdr).map_err(|e| EvalErr(e))?) {
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
                let (path,) = break_by_types!(*cdr.unwrap(), AmString).map_err(|e| EvalErr(e))?;
                Ok(Path::new(path.as_str().into()).into())
            }
            _ => panic!("{}", command),
        }
    }

    fn deserialize_triples(&mut self, structure: HeapSexp) -> Result<(), DeserializeError> {
        let (command, remainder) =
            break_by_types!(*structure, Symbol; remainder).map_err(|e| EvalErr(e))?;
        if command.as_str() != "triples" {
            return Err(UnexpectedCommand(command.into()));
        }

        let iter = match SexpIntoIter::try_from(remainder) {
            Ok(iter) => iter,
            Err(function::EvalErr::WrongArgumentCount { .. }) => return Ok(()),
            Err(err) => return Err(DeserializeError::EvalErr(err)),
        };

        for entry in iter {
            let (s, p, o) =
                break_by_types!(*entry, Symbol, Symbol, Symbol).map_err(|e| EvalErr(e))?;

            let subject = self.parse_symbol(&s)?;
            let predicate = self.parse_symbol(&p)?;
            let object = self.parse_symbol(&o)?;

            self.env_state().env().insert_triple(
                subject.local(),
                predicate.local(),
                object.local(),
            );

            let designation = self.env_state().designation();
            if predicate.local() == designation && object.local() != designation {
                let name = if let Ok(sym) =
                    <Symbol>::try_from(self.env_state().designate(Primitive::Node(object)))
                {
                    sym
                } else {
                    println!(
                        "{} {} {:?}",
                        subject,
                        object,
                        self.env_state().designate(Primitive::Node(object)).unwrap()
                    );
                    return Err(ExpectedSymbol);
                };

                if let Ok(table) = <&mut SymbolTable>::try_from(
                    self.env_state().env().node_structure_mut(designation),
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

impl Agent for EnvManager {
    fn run(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn env_state(&mut self) -> &mut EnvState {
        &mut self.env_state
    }
}

impl Eval for EnvManager {
    fn eval(&mut self, _structure: HeapSexp) -> Ret {
        Ok(Sexp::default())
    }
}
