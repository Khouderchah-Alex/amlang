//! Environment abstraction.

use std::collections::BTreeSet;
use std::fmt;

use super::local_node::{LocalNode, LocalTriple};
use crate::sexp::Sexp;


pub type EnvObject = dyn Environment;

// TODO(flex) Use newtype w/trait impls? In future could be enum w/static dispatch.
pub type NodeSet = BTreeSet<LocalNode>;
pub type TripleSet = BTreeSet<LocalTriple>;

/// Triple store of Nodes, which can be atoms, structures, or triples.
/// Always contains at least one node, which represents itself.
pub trait Environment: EnvClone {
    fn all_nodes(&self) -> NodeSet;

    fn insert_atom(&mut self) -> LocalNode;
    fn insert_structure(&mut self, structure: Sexp) -> LocalNode;
    fn insert_triple(
        &mut self,
        subject: LocalNode,
        predicate: LocalNode,
        object: LocalNode,
    ) -> LocalTriple;
    fn get_or_insert_triple(
        &mut self,
        subject: LocalNode,
        predicate: LocalNode,
        object: LocalNode,
    ) -> LocalTriple {
        if let Some(triple) = self.match_triple(subject, predicate, object) {
            triple
        } else {
            self.insert_triple(subject, predicate, object)
        }
    }

    fn match_subject(&self, subject: LocalNode) -> TripleSet;
    fn match_predicate(&self, predicate: LocalNode) -> TripleSet;
    fn match_object(&self, object: LocalNode) -> TripleSet;

    fn match_but_subject(&self, predicate: LocalNode, object: LocalNode) -> TripleSet;
    fn match_but_predicate(&self, subject: LocalNode, object: LocalNode) -> TripleSet;
    fn match_but_object(&self, subject: LocalNode, predicate: LocalNode) -> TripleSet;

    fn match_triple(
        &self,
        subject: LocalNode,
        predicate: LocalNode,
        object: LocalNode,
    ) -> Option<LocalTriple>;
    fn match_all(&self) -> TripleSet;

    fn match_any(&self, node: LocalNode) -> TripleSet {
        let mut triples = self.match_subject(node);
        triples = triples
            .union(&self.match_predicate(node))
            .cloned()
            .collect();
        triples.union(&self.match_object(node)).cloned().collect()
    }

    fn node_structure(&self, node: LocalNode) -> Option<&Sexp>;
    fn node_structure_mut(&mut self, node: LocalNode) -> Option<&mut Sexp>;
    fn node_as_triple(&self, node: LocalNode) -> Option<LocalTriple>;

    fn triple_subject(&self, triple: LocalTriple) -> LocalNode;
    fn triple_predicate(&self, triple: LocalTriple) -> LocalNode;
    fn triple_object(&self, triple: LocalTriple) -> LocalNode;
    fn triple_index(&self, triple: LocalTriple) -> usize;
    fn triple_from_index(&self, index: usize) -> LocalTriple;
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
