use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use super::agent::Agent;
use super::env_state::EnvState;
use crate::function::Ret;
use crate::model::{Eval, Model};
// use crate::parser::parse_sexp;
use crate::primitive::Primitive;
use crate::sexp::{HeapSexp, Sexp};


pub struct EnvSerializer {
    env_state: EnvState,
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

    // fn deserialize<P: AsRef<Path>>(&mut self, in_path: P) {}

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
            let s = self.env_state().triple_structure(triple);
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
                    write!(w, "(__symbol {})", symbol)
                }
            }
            Primitive::BuiltIn(builtin) => write!(w, "(__builtin {})", builtin.name()),
            Primitive::Procedure(proc) => {
                self.serialize_list_internal(w, &proc.generate_structure(), depth + 1)
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
