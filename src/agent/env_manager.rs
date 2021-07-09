use std::collections::HashMap;
use std::convert::TryFrom;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::sync::Arc;

use super::agent::Agent;
use super::amlang_context::AmlangContext;
use super::amlang_wrappers::quote_wrapper;
use super::env_state::{EnvState, AMLANG_DESIGNATION};
use crate::builtins::generate_builtin_map;
use crate::environment::environment::{EnvObject, Environment};
use crate::environment::mem_environment::MemEnvironment;
use crate::environment::LocalNode;
use crate::function::{self, Ret};
use crate::model::{Eval, Model};
use crate::parser::{self, parse_sexp};
use crate::primitive::symbol_policies::{policy_admin, AdminSymbolInfo};
use crate::primitive::{BuiltIn, Node, Primitive, Procedure, Symbol, SymbolTable, ToSymbol};
use crate::sexp::{Cons, HeapSexp, Sexp, SexpIntoIter};
use crate::token::file_stream::{self, FileStream};

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
                <&mut SymbolTable>::try_from(
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
    pub fn bootstrap<P: AsRef<Path>>(base_path: P) -> Result<Self, DeserializeError> {
        // Initially create meta as MemEnvironment.
        let mut meta = Box::new(MemEnvironment::new());
        EnvManager::initialize_env(LocalNode::default(), &mut *meta);
        let imports_name = meta.insert_structure("imports".to_symbol_or_panic(policy_admin).into());
        let imports = meta.insert_atom();
        let import_table_name =
            meta.insert_structure("__import_table".to_symbol_or_panic(policy_admin).into());
        let import_table = meta.insert_atom();

        let (lang_env_node, self_node, designation) = EnvManager::create_env_internal(&mut *meta);
        let mut context = Arc::new(AmlangContext::new(
            meta,
            lang_env_node,
            imports,
            import_table,
            self_node,
            designation,
        ));
        // TODO(flex) Find more flexible approch to bootstrapping required lang nodes.
        {
            let mut c = Arc::get_mut(&mut context).unwrap();
            c.lambda = LocalNode::new(13);
            c.apply = LocalNode::new(33);
        }

        {
            let mut meta_state =
                EnvState::new(LocalNode::default(), context.self_node(), context.clone());
            meta_state
                .name_node(imports_name, imports)
                .map_err(|e| EvalErr(e))?;
            meta_state
                .name_node(import_table_name, import_table)
                .map_err(|e| EvalErr(e))?;
        }

        // Start in lang env. Ditto for below.
        let env_state = EnvState::new(context.lang_env(), context.self_node(), context.clone());
        let mut manager = Self { env_state };
        manager.deserialize(base_path)?;

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
        );

        Ok(Self {
            env_state: EnvState::new(context.lang_env(), context.self_node(), context),
        })
    }

    pub fn create_env(&mut self) -> LocalNode {
        EnvManager::create_env_internal(self.env_state().context().meta()).0
    }

    pub fn serialize<P: AsRef<Path>>(&mut self, out_path: P) -> std::io::Result<()> {
        let file = File::create(out_path)?;
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
        Ok(())
    }

    pub fn deserialize<P: AsRef<Path>>(&mut self, in_path: P) -> Result<(), DeserializeError> {
        let stream = match FileStream::new(in_path, policy_admin) {
            Ok(stream) => stream,
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
            Ok(None) => return Ok(()),
            Err(err) => return Err(ParseError(err)),
        };
    }


    // Returns (Env Meta Node, Self Base Node, Designation Base Node).
    fn create_env_internal(meta: &mut EnvObject) -> (LocalNode, LocalNode, LocalNode) {
        // Initially create as MemEnvironment.
        let env_node = meta.insert_structure(Box::new(MemEnvironment::new()).into());
        let env = if let Some(Sexp::Primitive(Primitive::Env(env))) = meta.node_structure(env_node)
        {
            env
        } else {
            panic!()
        };

        let (self_node, designation) = EnvManager::initialize_env(env_node, &mut **env);
        (env_node, self_node, designation)
    }

    // Returns (Env Meta Node, Self Base Node, Designation Base Node).
    fn initialize_env(env_node: LocalNode, env: &mut EnvObject) -> (LocalNode, LocalNode) {
        // Set up self node.
        let self_node = env.insert_structure(Node::new(LocalNode::default(), env_node).into());

        // Set up designation node.
        let designation = env.insert_structure(SymbolTable::default().into());
        if let Ok(table) = <&mut SymbolTable>::try_from(env.node_structure(designation)) {
            table.insert(
                AMLANG_DESIGNATION.to_symbol_or_panic(policy_admin),
                Node::new(env_node, designation),
            );
        } else {
            panic!("Env designation isn't a symbol table");
        }
        (self_node, designation)
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
            Primitive::Symbol(symbol) => {
                write!(w, "{}", symbol.as_str())
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
            _ => panic!("{}", command),
        }
    }

    fn deserialize_triples(&mut self, structure: HeapSexp) -> Result<(), DeserializeError> {
        let (command, remainder) =
            break_by_types!(*structure, Symbol; remainder).map_err(|e| EvalErr(e))?;
        if command.as_str() != "triples" {
            return Err(UnexpectedCommand(command.into()));
        }

        let iter = SexpIntoIter::try_from(remainder).map_err(|e| EvalErr(e))?;
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

                if let Ok(table) =
                    <&mut SymbolTable>::try_from(self.env_state().env().node_structure(designation))
                {
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
