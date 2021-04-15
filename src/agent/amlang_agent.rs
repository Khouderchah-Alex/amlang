use super::agent::Agent;
use super::env_state::EnvState;
use crate::builtin::BUILTINS;
use crate::function::{
    EvalErr::{self, *},
    Func, Ret,
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
