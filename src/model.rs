use crate::agent::agent_state::AgentState;
use crate::primitive::{Error, Node, Primitive};
use crate::sexp::Sexp;


/// Meaning of Structure, according to (possibly implicit) metamodel.
///
/// The meaning of Structures in the methods below can be represented by the
/// Structures returned, the state of the Interpretation itself, and possibly
/// how it modifies the state of its Environment.
///
/// Note the distinction between "internal" Structures the Interpretation uses
/// to communicate with itself and "external" Structures it uses to communicate
/// broadly. This inherently recursible notion represents abstraction in the
/// process of metamodelling. In some sense, we can look at the idea of
/// encapsulation in traditional programming languages (e.g. WRT objects,
/// modules, etc) as implicitly embodying a similar notion.
///
/// In a sense, adding contemplate() allows for what would normally just be some
/// form of construe (or eval/call/etc) to reify some form of internal theory.
pub trait Interpretation {
    /// Meaning of internal Structure.
    fn contemplate(&mut self, structure: Sexp) -> Result<Sexp, Error>;

    /// Meaning of external Structure.
    ///
    /// While the notion of abstraction means there will likely be some
    /// connection to |contemplate| (i.e. that there is some direct connection
    /// between external and internal Structures), implementations are not
    /// constrained to comply with this.
    ///
    /// The default implementation directly passing to |contemplate| represents
    /// an abstractive base case, where external and internal Structures are the
    /// same.
    fn construe(&mut self, structure: Sexp) -> Result<Sexp, Error> {
        self.contemplate(structure)
    }
}

/// Structure of compiled meaning, according to (possibly implicit) metamodel.
pub trait Reflective {
    /// Compiled meaning -> Structure.
    fn reify(&self, state: &mut AgentState) -> Sexp;

    /// Structure -> compiled meaning.
    ///
    /// |process_primitive| is used so that reflect code can be written
    /// uniformly in the face of, say, a Structure made of unresolved Symbols
    /// vs one made of resolved Nodes.
    fn reflect<F>(
        structure: Sexp,
        state: &mut AgentState,
        process_primitive: F,
    ) -> Result<Self, Error>
    where
        Self: Sized,
        F: FnMut(&mut AgentState, &Primitive) -> Result<Node, Error>;

    /// Whether the Structure's discriminator corresponds to this impl.
    ///
    /// If this returns false, calling reflect should return an Error for sane
    /// impls.
    fn valid_discriminator(node: Node, state: &AgentState) -> bool;
}
