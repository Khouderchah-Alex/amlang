use crate::agent::agent_state::AgentState;
use crate::lang_err::LangErr;
use crate::sexp::{HeapSexp, Sexp};


pub type Ret = Result<Sexp, LangErr>;

/// Meaning of Structures.
pub trait Eval {
    fn eval(&mut self, structure: HeapSexp) -> Ret;
}

pub trait Model {
    /// Model -> structure according to (possibly implicit) metamodel.
    fn reify(&self, state: &mut AgentState) -> HeapSexp;
}
