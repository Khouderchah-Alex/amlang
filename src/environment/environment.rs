//! Environment abstraction.

use std::collections::BTreeSet;
use std::fmt;

use super::node::{NodeId, TripleId};
use crate::sexp::Sexp;


pub type EnvObject = dyn Environment;

// TODO(flex) Use newtype w/trait impls? In future could be enum w/static dispatch.
pub type NodeSet = BTreeSet<NodeId>;
pub type TripleSet = BTreeSet<TripleId>;

/// Triple store of Nodes, which can be atoms, structures, or triples.
/// Always contains at least one node, which represents itself.
pub trait Environment: EnvClone {
    // Portal to self node.
    fn self_node(&self) -> NodeId;
    fn all_nodes(&self) -> NodeSet;

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

    fn match_any(&self, node: NodeId) -> TripleSet {
        let mut triples = self.match_subject(node);
        triples = triples
            .union(&self.match_predicate(node))
            .cloned()
            .collect();
        triples.union(&self.match_object(node)).cloned().collect()
    }

    fn node_structure(&mut self, node: NodeId) -> Option<&mut Sexp>;
    fn node_as_triple(&self, node: NodeId) -> Option<TripleId>;

    fn triple_subject(&self, triple: TripleId) -> NodeId;
    fn triple_predicate(&self, triple: TripleId) -> NodeId;
    fn triple_object(&self, triple: TripleId) -> NodeId;
    fn triple_index(&self, triple: TripleId) -> usize;
}


pub trait EnvClone {
    fn clone_box(&self) -> Box<EnvObject>;
}

impl<T> EnvClone for T
where
    T: 'static + Environment + Clone,
{
    fn clone_box(&self) -> Box<EnvObject> {
        Box::new(self.clone())
    }
}

impl Clone for Box<EnvObject> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}


impl fmt::Debug for Box<EnvObject> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[Env @ {:p}]", self)
    }
}
