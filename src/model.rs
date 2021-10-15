use crate::agent::agent_state::AgentState;
use crate::primitive::{Error, Node, Primitive};
use crate::sexp::Sexp;


pub type Ret = Result<Sexp, Error>;

/// Meaning of Structures.
pub trait Eval {
    fn eval(&mut self, structure: Sexp) -> Ret;
}

pub trait Model {
    /// Model -> Structure according to (possibly implicit) metamodel.
    fn reify(&self, state: &mut AgentState) -> Sexp;

    /// Structure -> Model according to (possibly implicit) metamodel.
    ///
    /// |process_primitive| is used so that reflect code can be written
    /// uniformly in the face of, say, a structure made of unresolved Symbols
    /// vs one made of resolved Nodes.
    fn reflect<F>(
        structure: Sexp,
        state: &mut AgentState,
        process_primitive: F,
    ) -> Result<Self, Error>
    where
        Self: Sized,
        F: FnMut(&mut AgentState, &Primitive) -> Result<Node, Error>;

    fn valid_discriminator(node: Node, state: &AgentState) -> bool;
}
