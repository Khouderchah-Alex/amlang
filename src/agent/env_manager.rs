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
use crate::environment::environment::Environment;
use crate::environment::mem_environment::MemEnvironment;
use crate::environment::meta_environment::{MetaEnvStructure, MetaEnvironment};
use crate::function::{self, Ret};
use crate::model::{Eval, Model};
use crate::parser::{self, parse_sexp};
use crate::primitive::{BuiltIn, NodeId, Primitive, Procedure, Symbol, SymbolTable, ToSymbol};
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
            let lookup = |s: &str| -> Result<NodeId, DeserializeError> {
                Ok(table
                    .lookup(&s.to_symbol_or_panic())
                    .map_err(|e| EvalErr(e))?
                    .clone())
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
        let mut meta = MetaEnvironment::new();
        let base_env_node =
            meta.insert_structure(MetaEnvStructure::Env(Box::new(MemEnvironment::new())));
        let base_env = meta.access_env(base_env_node);

        let pos = base_env.self_node();
        let designation = base_env.insert_structure(SymbolTable::default().into());

        if let Ok(table) = <&mut SymbolTable>::try_from(base_env.node_structure(designation)) {
            table.insert(AMLANG_DESIGNATION.to_symbol_or_panic(), designation);
        } else {
            panic!("Env designation isn't a symbol table");
        }

        let mut context = Arc::new(AmlangContext::new(meta, base_env_node, designation));
        let env_state = EnvState::new(context.clone(), pos);

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
        );

        Ok(Self {
            env_state: EnvState::new(context, pos),
        })
    }

    pub fn deserialize<P: AsRef<Path>>(&mut self, in_path: P) -> Result<(), DeserializeError> {
        let stream = match FileStream::new(in_path) {
            Ok(stream) => stream,
            Err(err) => return Err(FileStreamError(err)),
        };
        let mut peekable = stream.peekable();

        let node_table = match parse_sexp(&mut peekable, 0) {
            Ok(Some(parsed)) => self.deserialize_nodes(parsed)?,
            Ok(None) => return Err(MissingNodeSection),
            Err(err) => return Err(ParseError(err)),
        };
        match parse_sexp(&mut peekable, 0) {
            Ok(Some(parsed)) => self.deserialize_triples(parsed, node_table)?,
            Ok(None) => return Err(MissingTripleSection),
            Err(err) => return Err(ParseError(err)),
        };
        match parse_sexp(&mut peekable, 0) {
            Ok(Some(_)) => return Err(ExtraneousSection),
            Ok(None) => return Ok(()),
            Err(err) => return Err(ParseError(err)),
        };
    }

    fn deserialize_nodes(&mut self, structure: HeapSexp) -> Result<SymbolTable, DeserializeError> {
        let builtins = generate_builtin_map();
        let mut node_table = SymbolTable::default();
        let (command, remainder) =
            break_by_types!(*structure, Symbol; remainder).map_err(|e| EvalErr(e))?;
        if command.as_str() != "nodes" {
            return Err(UnexpectedCommand(command.into()));
        }

        let iter = SexpIntoIter::try_from(remainder).map_err(|e| EvalErr(e))?;
        for entry in iter.skip(1) {
            match *entry {
                Sexp::Primitive(primitive) => {
                    if let Primitive::Symbol(sym) = primitive {
                        // TODO(func) Create designation here rather than in EnvState.
                        if sym.as_str() == AMLANG_DESIGNATION {
                            node_table.insert(sym, self.env_state().designation());
                        } else {
                            node_table.insert(sym, self.env_state().env().insert_atom());
                        }
                    } else {
                        return Err(ExpectedSymbol);
                    }
                }
                Sexp::Cons(cons) => {
                    let (name, command) =
                        break_by_types!(cons.into(), Symbol, Sexp).map_err(|e| EvalErr(e))?;
                    let structure = self.eval_structure(command, &builtins, &node_table)?;
                    node_table.insert(name, self.env_state().env().insert_structure(structure));
                }
            }
        }
        Ok(node_table)
    }

    fn eval_structure(
        &mut self,
        structure: Sexp,
        builtins: &HashMap<&'static str, BuiltIn>,
        node_table: &SymbolTable,
    ) -> Result<Sexp, DeserializeError> {
        if let Ok(sym) = <&Symbol>::try_from(&structure) {
            return Ok(node_table.lookup(sym).map_err(|e| EvalErr(e))?.into());
        }

        let (command, cdr) =
            break_by_types!(structure, Symbol; remainder).map_err(|e| EvalErr(e))?;

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
            "apply" => {
                if cdr.is_none() {
                    return Err(EvalErr(function::EvalErr::WrongArgumentCount {
                        given: 0,
                        expected: function::ExpectedCount::Exactly(2),
                    }));
                }

                let (func, args) =
                    break_by_types!(*cdr.unwrap(), Symbol, Cons).map_err(|e| EvalErr(e))?;
                let fnode = node_table.lookup(&func).map_err(|e| EvalErr(e))?;
                let mut arg_nodes = Vec::with_capacity(args.iter().count());
                for arg in args {
                    if let Ok(sym) = <&Symbol>::try_from(&*arg) {
                        arg_nodes.push(node_table.lookup(sym).map_err(|e| EvalErr(e))?);
                    } else {
                        return Err(EvalErr(function::EvalErr::InvalidSexp(*arg)));
                    }
                }
                Ok(Procedure::Application(fnode, arg_nodes).into())
            }
            "lambda" => {
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
                        param_nodes.push(node_table.lookup(sym).map_err(|e| EvalErr(e))?);
                    } else {
                        return Err(EvalErr(function::EvalErr::InvalidSexp(*param)));
                    }
                }
                let body_node = node_table.lookup(&body).map_err(|e| EvalErr(e))?;
                Ok(Procedure::Abstraction(param_nodes, body_node).into())
            }
            _ => panic!("{}", command),
        }
    }

    fn deserialize_triples(
        &mut self,
        structure: HeapSexp,
        mut node_table: SymbolTable,
    ) -> Result<(), DeserializeError> {
        let (command, remainder) =
            break_by_types!(*structure, Symbol; remainder).map_err(|e| EvalErr(e))?;
        if command.as_str() != "triples" {
            return Err(UnexpectedCommand(command.into()));
        }

        let iter = SexpIntoIter::try_from(remainder).map_err(|e| EvalErr(e))?;
        for entry in iter {
            let (s, p, o) =
                break_by_types!(*entry, Symbol, Symbol, Symbol).map_err(|e| EvalErr(e))?;

            let subject = node_table.lookup(&s).map_err(|e| EvalErr(e))?;
            let predicate = node_table.lookup(&p).map_err(|e| EvalErr(e))?;
            let object = node_table.lookup(&o).map_err(|e| EvalErr(e))?;

            let triple = self
                .env_state()
                .env()
                .insert_triple(subject, predicate, object);
            node_table.insert(
                format!("^t{}", self.env_state().env().triple_index(triple)).to_symbol_or_panic(),
                triple.node(),
            );

            let designation = self.env_state().designation();
            if predicate == designation && object != designation {
                let name = if let Ok(sym) =
                    <Symbol>::try_from(self.env_state().designate(Primitive::Node(object)))
                {
                    sym
                } else {
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
            self.serialize_list_internal(&mut w, &node.into(), 0)?;
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
                if let Some(triple) = self.env_state().env().node_as_triple(*node) {
                    return write!(w, "^t{}", self.env_state().env().triple_index(triple));
                }

                // Output Nodes as their designators if possible.
                if let Ok(designator) = <Symbol>::try_from(self.env_state().node_designator(*node))
                {
                    write!(w, "{}", designator.as_str())?;
                } else {
                    write!(w, "^{}", node.id())?;
                }

                Ok(())
            }
            _ => write!(w, "{}", primitive),
        }
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
