use crate::agent::agent_state::AgentState;
use crate::lang_err::LangErr;
use crate::primitive::{Node, Primitive};
use crate::sexp::{HeapSexp, Sexp};


pub type Ret = Result<Sexp, LangErr>;

/// Meaning of Structures.
pub trait Eval {
    fn eval(&mut self, structure: HeapSexp) -> Ret;
}

pub trait Model {
    /// Model -> structure according to (possibly implicit) metamodel.
    fn reify(&self, state: &mut AgentState) -> HeapSexp;

    /// Model <- structure according to (possibly implicit) metamodel.
    ///
    /// |process_primitive| is used so that reflect code can be written
    /// uniformly in the face of, say, a structure made of unresolved Symbols
    /// vs one made of resolved Nodes.
    fn reflect<F>(
        structure: HeapSexp,
        state: &mut AgentState,
        process_primitive: F,
    ) -> Result<Self, LangErr>
    where
        Self: Sized,
        F: FnMut(&mut AgentState, &Primitive) -> Result<Node, LangErr>;
}
