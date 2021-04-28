use std::borrow::Cow;

use super::agent::Agent;
use super::env_state::EnvState;
use crate::builtin::BUILTINS;
use crate::function::{
    EvalErr::{self, *},
    ExpectedCount, Func, Ret,
};
use crate::model::{Designation, Eval};
use crate::parser::parse_sexp;
use crate::primitive::Primitive;
use crate::sexp::Sexp;
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
                return Err(InvalidSexp(Sexp::Primitive(primitive.clone())));
            }

            Sexp::Cons(cons) => {
                let mut iter = cons.iter();
                let name = if let Some(Sexp::Primitive(Primitive::Symbol(symbol))) = iter.next() {
                    symbol.to_string()
                } else {
                    return Err(InvalidArgument {
                        given: Sexp::Cons(cons.clone()),
                        expected: Cow::Borrowed("symbol"),
                    });
                };

                return match iter.next() {
                    None => {
                        let identifier = self.env_state().identifier();
                        let env = self.env_state().env();

                        let name_sexp = Sexp::Primitive(Primitive::Symbol(name));
                        let node = env.insert_atom();
                        let node_name = env.insert_structure(name_sexp.clone());
                        env.insert_triple(node, identifier, node_name);

                        for triple in env.match_all() {
                            println!("    {}", self.env_state().triple_identifiers(triple));
                        }
                        Ok(name_sexp)
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
                let value = BUILTINS.lookup(symbol);
                match value {
                    Some(builtin) => Ok(Sexp::Primitive(Primitive::BuiltIn(builtin))),
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
            _ => Ok(Sexp::Primitive(designator.clone())),
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
                    None => return Err(InvalidSexp(Sexp::Cons(cons.clone()))),
                };

                if let Sexp::Primitive(Primitive::Symbol(first)) = car {
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

                if let Sexp::Primitive(Primitive::BuiltIn(builtin)) = self.eval(car)? {
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
