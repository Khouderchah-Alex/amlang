use std::cell::UnsafeCell;
use std::sync::Arc;

use super::environment::{Environment, NodeSet, TripleSet};
use super::local_node::{LocalNode, LocalTriple};
use crate::sexp::Sexp;


/// Concurrency-unsafe Environment overlay.
///
/// Useful for local single-{threaded,process} deployments or tests, where
/// multiple objects/Agents can share ownership of an Environment hierarchy.
///
/// SAFETY: Clients must ensure read-write exclusion (at whatever granularity is
/// relevant). This is trivial for serial execution; concurrent deployments
/// should look into different overlay implementations.
#[derive(Clone)]
pub struct SerialOverlay<T: 'static + Environment + Clone> {
    base: Arc<UnsafeCell<T>>,
}

impl<T: Environment + Clone> SerialOverlay<T> {
    pub fn new(base: Box<T>) -> Self {
        Self {
            base: Arc::new(UnsafeCell::new(*base)),
        }
    }

    fn base(&self) -> &mut T {
        unsafe { &mut *self.base.get() }
    }
}

impl<T: Environment + Clone> Environment for SerialOverlay<T> {
    fn all_nodes(&self) -> NodeSet {
        self.base().all_nodes()
    }
    fn insert_atom(&mut self) -> LocalNode {
        self.base().insert_atom()
    }
    fn insert_structure(&mut self, structure: Sexp) -> LocalNode {
        self.base().insert_structure(structure)
    }
    fn insert_triple(
        &mut self,
        subject: LocalNode,
        predicate: LocalNode,
        object: LocalNode,
    ) -> LocalTriple {
        self.base().insert_triple(subject, predicate, object)
    }


    fn match_subject(&self, subject: LocalNode) -> TripleSet {
        self.base().match_subject(subject)
    }
    fn match_predicate(&self, predicate: LocalNode) -> TripleSet {
        self.base().match_predicate(predicate)
    }
    fn match_object(&self, object: LocalNode) -> TripleSet {
        self.base().match_object(object)
    }
    fn match_but_subject(&self, predicate: LocalNode, object: LocalNode) -> TripleSet {
        self.base().match_but_subject(predicate, object)
    }
    fn match_but_predicate(&self, subject: LocalNode, object: LocalNode) -> TripleSet {
        self.base().match_but_predicate(subject, object)
    }
    fn match_but_object(&self, subject: LocalNode, predicate: LocalNode) -> TripleSet {
        self.base().match_but_object(subject, predicate)
    }
    fn match_triple(
        &self,
        subject: LocalNode,
        predicate: LocalNode,
        object: LocalNode,
    ) -> Option<LocalTriple> {
        self.base().match_triple(subject, predicate, object)
    }
    fn match_all(&self) -> TripleSet {
        self.base().match_all()
    }

    fn node_structure(&self, node: LocalNode) -> Option<&Sexp> {
        self.base().node_structure(node)
    }
    fn node_structure_mut(&mut self, node: LocalNode) -> Option<&mut Sexp> {
        self.base().node_structure_mut(node)
    }
    fn node_as_triple(&self, node: LocalNode) -> Option<LocalTriple> {
        self.base().node_as_triple(node)
    }

    fn triple_subject(&self, triple: LocalTriple) -> LocalNode {
        self.base().triple_subject(triple)
    }
    fn triple_predicate(&self, triple: LocalTriple) -> LocalNode {
        self.base().triple_predicate(triple)
    }
    fn triple_object(&self, triple: LocalTriple) -> LocalNode {
        self.base().triple_object(triple)
    }
    fn triple_index(&self, triple: LocalTriple) -> usize {
        self.base().triple_index(triple)
    }
    fn triple_from_index(&self, index: usize) -> LocalTriple {
        self.base().triple_from_index(index)
    }
}
