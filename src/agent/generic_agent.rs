use super::agent::Agent;
use super::designation::Designation;
use super::env_state::EnvState;
use crate::builtin::BUILTINS;
use crate::function::{EvalErr, Ret};
use crate::interpreter;
use crate::parser;
use crate::primitive::Primitive;
use crate::sexp::Sexp;
use crate::token::interactive_stream::InteractiveStream;


pub struct GenericAgent {
    env_state: EnvState,
}

impl GenericAgent {
    pub fn new() -> Self {
        Self {
            env_state: EnvState::new(),
        }
    }
}

impl Default for GenericAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl Agent for GenericAgent {
    fn run(&mut self) -> Result<(), String> {
        let stream = InteractiveStream::new();
        let mut peekable = stream.peekable();

        loop {
            let sexp = match parser::parse_sexp(&mut peekable, 0) {
                Ok(Some(parsed)) => parsed,
                Ok(None) => return Ok(()),
                Err(err) => {
                    println!(" {}", err);
                    println!("");
                    continue;
                }
            };

            let result = interpreter::eval(&sexp, self);
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

impl Designation for GenericAgent {
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
