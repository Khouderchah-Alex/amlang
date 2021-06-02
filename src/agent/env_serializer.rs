use std::collections::HashMap;
use std::convert::TryFrom;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use super::agent::Agent;
use super::amlang_wrappers::quote_wrapper;
use super::env_state::{EnvState, AMLANG_DESIGNATION};
use crate::builtins::generate_builtin_map;
use crate::function::Ret;
use crate::model::{Eval, Model};
use crate::parser::{self, parse_sexp};
use crate::primitive::{BuiltIn, Primitive, Symbol, SymbolTable};
use crate::sexp::{HeapSexp, Sexp};
use crate::token::file_stream::{self, FileStream};

use DeserializeError::*;


pub struct EnvSerializer {
    env_state: EnvState,
}

#[derive(Debug)]
pub enum DeserializeError {
    FileStreamError(file_stream::FileStreamError),
    ParseError(parser::ParseError),
    MissingNodeSection,
    MissingTripleSection,
    ExtraneousSection,
    InvalidSexp(Sexp),
    UnexpectedCommand(Sexp),
    MissingCommand,
    ExpectedSymbol,
    ExtraneousNodeInfo,
    UnrecognizedBuiltIn(Symbol),
}

impl EnvSerializer {
    pub fn new() -> Self {
        let env_state = EnvState::new();
        Self { env_state }
    }

    // TODO(func) Only using this until we have shared env functionality.
    pub fn from_env(env_state: EnvState) -> Self {
        Self { env_state }
    }

    // TODO(func) Only using this until we have shared env functionality.
    pub fn extract_env(self) -> EnvState {
        self.env_state
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
        let cons = match *structure {
            Sexp::Primitive(primitive) => {
                return Err(InvalidSexp(primitive.clone().into()));
            }
            Sexp::Cons(cons) => cons,
        };

        let mut iter = cons.into_iter();
        match iter.next() {
            Some(hsexp) => match *hsexp {
                Sexp::Primitive(Primitive::Symbol(symbol)) => {
                    if symbol.as_str() != "nodes" {
                        return Err(UnexpectedCommand(symbol.clone().into()));
                    }
                }
                s => return Err(UnexpectedCommand(s.clone())),
            },
            _ => return Err(MissingCommand),
        }

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
                    let mut iter = cons.into_iter();
                    let sym = if let Ok(symbol) = <Symbol>::try_from(iter.next()) {
                        symbol
                    } else {
                        return Err(ExpectedSymbol);
                    };

                    let command = if let Some(sexp) = iter.next() {
                        sexp
                    } else {
                        return Err(MissingCommand);
                    };

                    if let Some(_) = iter.next() {
                        return Err(ExtraneousNodeInfo);
                    }

                    let structure = self.eval_structure(command, &builtins)?;
                    node_table.insert(sym, self.env_state().env().insert_structure(structure));
                }
            }
        }
        Ok(node_table)
    }

    fn eval_structure(
        &mut self,
        structure: HeapSexp,
        builtins: &HashMap<&'static str, BuiltIn>,
    ) -> Result<Sexp, DeserializeError> {
        let cons = match *structure {
            Sexp::Primitive(primitive) => {
                return Err(InvalidSexp(primitive.clone().into()));
            }
            Sexp::Cons(cons) => cons,
        };
        let (car, cdr) = cons.consume();
        let command = if let Ok(sym) = <Symbol>::try_from(car) {
            sym
        } else {
            return Err(ExpectedSymbol);
        };

        match command.as_str() {
            // TODO unwraps.
            "quote" => Ok(quote_wrapper(cdr).unwrap()),
            "__builtin" => {
                if let Ok(sym) = <Symbol>::try_from(quote_wrapper(cdr).unwrap()) {
                    if let Some(builtin) = builtins.get(sym.as_str()) {
                        Ok(builtin.clone().into())
                    } else {
                        Err(UnrecognizedBuiltIn(sym.clone()))
                    }
                } else {
                    Err(ExpectedSymbol)
                }
            }
            "apply" | "lambda" => Ok(Sexp::default()),
            _ => panic!("{}", command),
        }
    }

    fn deserialize_triples(
        &mut self,
        structure: HeapSexp,
        node_table: SymbolTable,
    ) -> Result<(), DeserializeError> {
        let cons = match *structure {
            Sexp::Primitive(primitive) => {
                return Err(InvalidSexp(primitive.clone().into()));
            }
            Sexp::Cons(cons) => cons,
        };

        let mut iter = cons.into_iter();
        match iter.next() {
            Some(hsexp) => match *hsexp {
                Sexp::Primitive(Primitive::Symbol(symbol)) => {
                    if symbol.as_str() != "triples" {
                        return Err(UnexpectedCommand(symbol.clone().into()));
                    }
                }
                s => return Err(UnexpectedCommand(s.clone())),
            },
            _ => return Err(MissingCommand),
        }

        for entry in iter {
            let cons = match *entry {
                Sexp::Primitive(primitive) => {
                    return Err(InvalidSexp(primitive.clone().into()));
                }
                Sexp::Cons(cons) => cons,
            };
            let mut iter = cons.into_iter();
            // TODO unwraps
            let subject = if let Ok(symbol) = <Symbol>::try_from(iter.next()) {
                node_table.lookup(&symbol).unwrap()
            } else {
                return Err(ExpectedSymbol);
            };
            let predicate = if let Ok(symbol) = <Symbol>::try_from(iter.next()) {
                node_table.lookup(&symbol).unwrap()
            } else {
                return Err(ExpectedSymbol);
            };
            let object = if let Ok(symbol) = <Symbol>::try_from(iter.next()) {
                node_table.lookup(&symbol).unwrap()
            } else {
                return Err(ExpectedSymbol);
            };
            self.env_state()
                .env()
                .insert_triple(subject, predicate, object);

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
            self.serialize_list_internal(&mut w, &node.into(), 0)?;
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
                // TODO(func) Rm this hack once lambda is a node.
                if symbol.as_str() == "lambda" {
                    write!(w, "{}", symbol)
                } else {
                    write!(w, "'{}", symbol)
                }
            }
            Primitive::BuiltIn(builtin) => write!(w, "(__builtin {})", builtin.name()),
            Primitive::Procedure(proc) => {
                let proc_sexp = proc.generate_structure(self.env_state());
                self.serialize_list_internal(w, &proc_sexp, depth + 1)
            }
            Primitive::Node(node) => {
                let s = self.env_state().env().node_structure(*node).cloned();
                let write_structure = depth == 0
                    && match &s {
                        Some(sexp) => match sexp {
                            Sexp::Primitive(Primitive::SymbolTable(_)) => false,
                            _ => true,
                        },
                        _ => false,
                    };
                if write_structure {
                    write!(w, "(")?;
                }

                // Output Nodes as their designators if possible.
                if let Some(designator) = self.env_state().node_designator(*node) {
                    write!(w, "{}", designator)?;
                } else {
                    write!(w, "^{}", node.id())?;
                }

                if write_structure {
                    write!(w, "\t")?;
                    self.serialize_list_internal(w, &s.unwrap(), depth + 1)?;
                    write!(w, ")")?;
                }
                Ok(())
            }
            _ => write!(w, "{}", primitive),
        }
    }
}

impl Agent for EnvSerializer {
    fn run(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn env_state(&mut self) -> &mut EnvState {
        &mut self.env_state
    }
}

impl Eval for EnvSerializer {
    fn eval(&mut self, _structure: HeapSexp) -> Ret {
        Ok(Sexp::default())
    }
}
