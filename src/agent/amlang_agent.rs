use std::borrow::Cow;
use std::convert::TryFrom;

use super::agent::Agent;
use super::env_state::EnvState;
use crate::builtin::{BuiltIn, BUILTINS};
use crate::function::{
    EvalErr::{self, *},
    ExpectedCount, Func, Ret,
};
use crate::model::{Designation, Eval};
use crate::parser::parse_sexp;
use crate::primitive::Primitive;
use crate::sexp::Sexp;
use crate::symbol::Symbol;
use crate::syntax;
use crate::token::interactive_stream::InteractiveStream;


pub struct AmlangAgent {
    env_state: EnvState,
}

impl AmlangAgent {
    pub fn new() -> Self {
        let env_state = EnvState::new();
        Self { env_state }
    }

    fn env_insert(&mut self, args: Option<&Sexp>) -> Ret {
        if args.is_none() {
            return Err(WrongArgumentCount {
                given: 0,
                expected: ExpectedCount::AtLeast(1),
            });
        }

        match args.unwrap() {
            Sexp::Primitive(primitive) => {
                return Err(InvalidSexp(primitive.clone().into()));
            }

            Sexp::Cons(cons) => {
                let mut iter = cons.iter();
                let name = if let Ok(symbol) = <&Symbol>::try_from(iter.next()) {
                    symbol
                } else {
                    return Err(InvalidArgument {
                        given: cons.clone().into(),
                        expected: Cow::Borrowed("symbol"),
                    });
                };

                return match iter.next() {
                    None => {
                        let designation = self.env_state().designation();
                        let env = self.env_state().env();

                        if let Some(Sexp::Primitive(Primitive::SymbolTable(table))) =
                            env.node_structure(designation)
                        {
                            if table.contains_key(&name) {
                                return Err(AlreadyBoundSymbol(name.clone()));
                            }
                        } else {
                            panic!("Env designation isn't a symbol table");
                        }

                        let name_node = env.insert_structure(name.clone().into());
                        if let Some(Sexp::Primitive(Primitive::SymbolTable(table))) =
                            env.node_structure(designation)
                        {
                            table.insert(name.clone(), name_node);
                        } else {
                            panic!("Env designation isn't a symbol table");
                        }

                        let node = env.insert_atom();
                        env.insert_triple(node, designation, name_node);

                        for triple in env.match_all() {
                            println!("    {}", self.env_state().triple_inner_designators(triple));
                        }
                        Ok(name.clone().into())
                    }
                    _ => Err(WrongArgumentCount {
                        given: iter.count() + 2,
                        expected: ExpectedCount::AtMost(2),
                    }),
                };
            }
        }
    }
}


impl Default for AmlangAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl Agent for AmlangAgent {
    fn run(&mut self) -> Result<(), String> {
        let stream = InteractiveStream::new();
        let mut peekable = stream.peekable();

        loop {
            let sexp = match parse_sexp(&mut peekable, 0) {
                Ok(Some(parsed)) => parsed,
                Ok(None) => return Ok(()),
                Err(err) => {
                    println!(" {}", err);
                    println!("");
                    continue;
                }
            };

            let result = self.eval(&sexp);
            match result {
                Ok(val) => {
                    println!("-> {}", val);
                }
                Err(err) => {
                    println!(" {}", err);
                }
            }
            println!();
        }
    }

    fn env_state(&mut self) -> &mut EnvState {
        &mut self.env_state
    }
}

impl Designation for AmlangAgent {
    fn designate(&mut self, designator: &Primitive) -> Ret {
        return match designator {
            Primitive::Symbol(symbol) => {
                let value = BUILTINS.lookup(symbol.as_str());
                match value {
                    Some(builtin) => Ok(builtin.into()),
                    None => Err(EvalErr::UnboundSymbol(symbol.clone())),
                }
            }
            Primitive::Node(node) => Ok(self
                .env_state()
                .env()
                .node_structure(*node)
                .cloned()
                .unwrap_or_else(Default::default)),
            // Base case for self-designating.
            _ => Ok(designator.clone().into()),
        };
    }
}

impl Eval for AmlangAgent {
    fn eval(&mut self, structure: &Sexp) -> Ret {
        match structure {
            Sexp::Primitive(primitive) => {
                return self.designate(primitive);
            }

            Sexp::Cons(cons) => {
                let car = match cons.car() {
                    Some(car) => car,
                    None => return Err(InvalidSexp(cons.clone().into())),
                };

                if let Ok(first) = <&Symbol>::try_from(car) {
                    match first.as_str() {
                        "quote" => {
                            return syntax::quote(cons.cdr());
                        }
                        "new" => {
                            return self.env_insert(cons.cdr());
                        }
                        _ => { /* Fallthrough */ }
                    }
                }

                if let Ok(builtin) = <&BuiltIn>::try_from(self.eval(car)?) {
                    let args = syntax::evlis(cons.cdr(), self)?;
                    return builtin.call(&args);
                }
                panic!(
                    "`{}` did not match functional application catchall",
                    structure
                );
            }
        }
    }
}
