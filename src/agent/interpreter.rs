use std::fmt;

use super::Agent;
use crate::error::Error;
use crate::sexp::Sexp;


/// Meaning of Structure, according to (possibly implicit) metamodel.
///
/// The meaning of Structures in the methods below can be represented by the
/// Structures returned, the state of the Interpreter itself, and possibly
/// how it modifies the state of its Environment.
pub trait Interpreter {
    fn interpret(&mut self, structure: Sexp) -> Result<Sexp, Error>;
}


/// State which can borrow Execution to create an Interpreter.
/// Can be stored in Continuation and facilitates reifying metacontinuations.
// TODO(func) Allow storage in Env.
pub trait InterpreterState: fmt::Debug {
    fn borrow_agent<'a>(&'a mut self, agent: &'a mut Agent) -> Box<dyn Interpreter + 'a>;
}


/// Base metacontinuation state for non-running (i.e. manually driven) Agent.
#[derive(Debug, Default)]
pub struct NullInterpreter {}
impl Interpreter for NullInterpreter {
    fn interpret(&mut self, structure: Sexp) -> Result<Sexp, Error> {
        Ok(structure)
    }
}
impl InterpreterState for NullInterpreter {
    fn borrow_agent<'a>(&'a mut self, _agent: &'a mut Agent) -> Box<dyn Interpreter + 'a> {
        Box::new(NullInterpreter {})
    }
}
