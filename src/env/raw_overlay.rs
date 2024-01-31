use std::cell::UnsafeCell;
use std::sync::Arc;

use super::entry::{Entry, EntryMut};
use super::local_node::{LocalNode, LocalTriple};
use super::{Environment, NodeSet, TripleSet};
use crate::primitive::{Node, Symbol};
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
pub struct RawOverlay<T: Environment> {
    base: Arc<UnsafeCell<T>>,
}

impl<T: Environment> RawOverlay<T> {
    pub fn new(base: T) -> Self {
        Self {
            base: Arc::new(UnsafeCell::new(base)),
        }
    }

    fn base(&self) -> &mut T {
        unsafe { &mut *self.base.get() }
    }
}

impl<T: Environment + Clone> Environment for RawOverlay<T> {
    fn type_name(&self) -> &'static str {
        "RawOverlay"
    }

    fn all_nodes(&self) -> NodeSet {
        self.base().all_nodes()
    }
    fn insert_node(&mut self, structure: Option<Sexp>) -> LocalNode {
        self.base().insert_node(structure)
    }
    fn insert_triple(
        &mut self,
        subject: LocalNode,
        predicate: LocalNode,
        object: LocalNode,
    ) -> LocalTriple {
        self.base().insert_triple(subject, predicate, object)
    }


    fn insert_designation(&mut self, node: Node, designation: Symbol, context: LocalNode) {
        self.base().insert_designation(node, designation, context)
    }

    fn match_designation(&self, designation: &Symbol, context: LocalNode) -> Option<Node> {
        self.base().match_designation(designation, context)
    }

    fn find_designation(&self, node: Node, context: LocalNode) -> Option<&Symbol> {
        self.base().find_designation(node, context)
    }

    fn designation_pairs(&self, context: LocalNode) -> Vec<(&Symbol, &Node)> {
        self.base().designation_pairs(context)
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
    ) -> TripleSet {
        self.base().match_triple(subject, predicate, object)
    }
    fn match_all(&self) -> TripleSet {
        self.base().match_all()
    }

    fn entry(&self, node: LocalNode) -> Entry {
        self.base().entry(node)
    }
    fn entry_mut(&mut self, node: LocalNode) -> EntryMut {
        self.base().entry_mut(node)
    }
    fn entry_update(&mut self, entry: EntryMut) -> LocalNode {
        self.base().entry_update(entry)
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
