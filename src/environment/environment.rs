//! Environment abstraction.

use std::collections::BTreeSet;

use super::node::{NodeId, TripleId};
use crate::sexp::Sexp;

// TODO(flex) Use newtype w/trait impls? In future could be enum w/static dispatch.
pub type TripleSet = BTreeSet<TripleId>;

/// Triple store of Nodes, which can be atoms, structures, or triples.
/// Always contains at least one node, which represents itself.
pub trait Environment {
    const SELF: NodeId; // Portal to SELF.

    fn insert_atom(&mut self) -> NodeId;
    fn insert_structure(&mut self, structure: Sexp) -> NodeId;
    fn insert_triple(&mut self, subject: NodeId, predicate: NodeId, object: NodeId) -> TripleId;

    fn match_subject(&self, subject: NodeId) -> TripleSet;
    fn match_predicate(&self, predicate: NodeId) -> TripleSet;
    fn match_object(&self, object: NodeId) -> TripleSet;

    fn match_but_subject(&self, predicate: NodeId, object: NodeId) -> TripleSet;
    fn match_but_predicate(&self, subject: NodeId, object: NodeId) -> TripleSet;
    fn match_but_object(&self, subject: NodeId, predicate: NodeId) -> TripleSet;

    fn match_triple(&self, subject: NodeId, predicate: NodeId, object: NodeId) -> Option<TripleId>;
    fn match_all(&self) -> TripleSet;
}

/// Means of accessing structure of nodes.
pub trait Resolver {
    fn node_structure(&self, node: NodeId) -> Option<&Sexp>;
    fn node_as_triple(&self, node: NodeId) -> Option<TripleId>;

    fn triple_subject(&self, triple: TripleId) -> NodeId;
    fn triple_object(&self, triple: TripleId) -> NodeId;
    fn triple_predicate(&self, triple: TripleId) -> NodeId;
}
