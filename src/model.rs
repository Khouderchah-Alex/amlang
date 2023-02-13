use crate::agent::Agent;
use crate::error::Error;
use crate::primitive::{Node, Primitive};
use crate::sexp::Sexp;


/// Structure of compiled meaning, according to (possibly implicit) metamodel.
pub trait Reflective {
    /// Compiled meaning -> Structure.
    fn reify(&self, agent: &Agent) -> Sexp;

    /// Structure -> compiled meaning.
    ///
    /// |resolve| is used so that reflect code can be written uniformly in the
    /// face of, say, a Structure made of unresolved Symbols vs one made of
    /// resolved Nodes.
    fn reflect<F>(structure: Sexp, agent: &Agent, resolve: F) -> Result<Self, Error>
    where
        Self: Sized,
        F: Fn(&Agent, &Primitive) -> Result<Node, Error>;

    /// Whether the Structure's discriminator corresponds to this impl.
    ///
    /// If this returns false, calling reflect should return an Error for sane
    /// impls.
    fn valid_discriminator(node: Node, agent: &Agent) -> bool;
}
