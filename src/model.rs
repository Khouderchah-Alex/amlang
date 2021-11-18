use crate::agent::Agent;
use crate::error::Error;
use crate::primitive::{Node, Primitive};
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
/// In a sense, adding construe() allows for what would normally just be some
/// form of contemplate (or eval/call/etc) to reify some form of internal theory.
pub trait Interpreter {
    /// Meaning of external Structure as internal Structure.
    fn construe(&mut self, structure: Sexp) -> Result<Sexp, Error>;

    /// Meaning of internal Structure.
    fn contemplate(&mut self, structure: Sexp) -> Result<Sexp, Error>;
}

/// Structure of compiled meaning, according to (possibly implicit) metamodel.
pub trait Reflective {
    /// Compiled meaning -> Structure.
    fn reify(&self, agent: &mut Agent) -> Sexp;

    /// Structure -> compiled meaning.
    ///
    /// |resolve| is used so that reflect code can be written uniformly in the
    /// face of, say, a Structure made of unresolved Symbols vs one made of
    /// resolved Nodes.
    fn reflect<F>(structure: Sexp, agent: &mut Agent, resolve: F) -> Result<Self, Error>
    where
        Self: Sized,
        F: Fn(&mut Agent, &Primitive) -> Result<Node, Error>;

    /// Whether the Structure's discriminator corresponds to this impl.
    ///
    /// If this returns false, calling reflect should return an Error for sane
    /// impls.
    fn valid_discriminator(node: Node, agent: &Agent) -> bool;
}
