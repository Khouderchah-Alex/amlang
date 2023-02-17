use std::fmt;

use super::Agent;
use crate::error::Error;
use crate::sexp::Sexp;


/// Meaning of Structure, according to (possibly implicit) metamodel.
///
/// The meaning of Structures in the methods below can be represented by the
/// Structures returned, the state of the Interpreter itself, and possibly
/// how it modifies the state of its Environment.
///
/// Note the distinction between "internal" Structures the Interpreter uses
/// to communicate with itself and "external" Structures it uses to communicate
/// broadly. This inherently recursible notion represents abstraction in the
/// process of metamodelling. In some sense, we can look at the idea of
/// encapsulation in traditional programming languages (e.g. WRT objects,
/// modules, etc) as implicitly embodying a similar notion.
///
/// In a sense, adding internalize() allows for what would normally just be some
/// form of contemplate (or eval/call/etc) to reify some form of internal theory.
/// This is similar to how Transform forces control state to be explicit and
/// consequently can dictate policy (push vs pull streams, Write vs
/// BufWrite type policy using same underlying code, etc).
pub trait Interpreter {
    /// Meaning of external Structure as internal Structure.
    fn internalize(&mut self, structure: Sexp) -> Result<Sexp, Error>;

    /// Meaning of internal Structure.
    fn contemplate(&mut self, structure: Sexp) -> Result<Sexp, Error>;
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
    fn internalize(&mut self, structure: Sexp) -> Result<Sexp, Error> {
        Ok(structure)
    }
    fn contemplate(&mut self, structure: Sexp) -> Result<Sexp, Error> {
        Ok(structure)
    }
}
impl InterpreterState for NullInterpreter {
    fn borrow_agent<'a>(&'a mut self, _agent: &'a mut Agent) -> Box<dyn Interpreter + 'a> {
        Box::new(NullInterpreter {})
    }
}
