//! Implementation of Environment based on underlying MemBackend.

use std::fmt::Debug;

use super::environment::{Environment, NodeSet, TripleSet};
use super::local_node::{LocalId, LocalNode, LocalTriple};
use super::mem_backend::root_backend::RootBackend;
use super::mem_backend::{index_id_conv::*, Edges, MemBackend, Node, Triple};
use crate::sexp::Sexp;


#[derive(Debug)]
pub struct MemEnvironment {
    backend: Box<dyn MemBackend>,
}

impl MemEnvironment {
    pub fn new() -> Self {
        // TODO(perf) Could either use compile-time generics, or rely on dynamic
        // dispatch to impl lazy-loading through stub backend.
        Self {
            backend: Box::new(RootBackend::default()),
        }
    }
}

impl Environment for MemEnvironment {
    fn all_nodes(&self) -> NodeSet {
        (0..self.backend.node_count())
            .map(|x| LocalNode::new(x as LocalId))
            .collect()
    }

    fn insert_atom(&mut self) -> LocalNode {
        let id = self.backend.next_node_id();
        self.backend.push_node(Node::Atomic);
        self.backend.push_node_edges(Edges::default());
        id
    }
    fn insert_structure(&mut self, structure: Sexp) -> LocalNode {
        let id = self.backend.next_node_id();
        self.backend.push_node(Node::Structured(structure));
        self.backend.push_node_edges(Edges::default());
        id
    }
    fn insert_triple(
        &mut self,
        subject: LocalNode,
        predicate: LocalNode,
        object: LocalNode,
    ) -> LocalTriple {
        let id = self.backend.next_triple_id();

        self.backend.edges_mut(subject).as_subject.insert(id);
        self.backend.edges_mut(predicate).as_predicate.insert(id);
        self.backend.edges_mut(object).as_object.insert(id);

        self.backend.push_triple(Triple {
            subject,
            predicate,
            object,
        });
        self.backend.push_triple_edges(Edges::default());
        id
    }


    fn match_subject(&self, subject: LocalNode) -> TripleSet {
        let set = self.backend.edges(subject).as_subject.iter();
        set.cloned().collect()
    }
    fn match_predicate(&self, predicate: LocalNode) -> TripleSet {
        let set = self.backend.edges(predicate).as_predicate.iter();
        set.cloned().collect()
    }
    fn match_object(&self, object: LocalNode) -> TripleSet {
        let set = self.backend.edges(object).as_object.iter();
        set.cloned().collect()
    }

    fn match_but_subject(&self, predicate: LocalNode, object: LocalNode) -> TripleSet {
        let set = self
            .backend
            .edges(predicate)
            .as_predicate
            .intersection(&self.backend.edges(object).as_object);
        set.cloned().collect()
    }
    fn match_but_predicate(&self, subject: LocalNode, object: LocalNode) -> TripleSet {
        let set = self
            .backend
            .edges(subject)
            .as_subject
            .intersection(&self.backend.edges(object).as_object);
        set.cloned().collect()
    }
    fn match_but_object(&self, subject: LocalNode, predicate: LocalNode) -> TripleSet {
        let set = self
            .backend
            .edges(subject)
            .as_subject
            .intersection(&self.backend.edges(predicate).as_predicate);
        set.cloned().collect()
    }

    fn match_triple(
        &self,
        subject: LocalNode,
        predicate: LocalNode,
        object: LocalNode,
    ) -> Option<LocalTriple> {
        self.match_but_object(subject, predicate)
            .iter()
            .find(|&&triple| self.triple_object(triple) == object)
            .cloned()
    }
    fn match_all(&self) -> TripleSet {
        (0..self.backend.triple_count())
            .map(|x| index_to_triple_id(x))
            .collect()
    }

    fn node_structure(&self, node: LocalNode) -> Option<&Sexp> {
        if is_triple_id(node.id()) {
            return None;
        }

        match &self.backend.node_unchecked(node) {
            Node::Atomic => None,
            Node::Structured(structure) => Some(structure),
        }
    }
    fn node_structure_mut(&mut self, node: LocalNode) -> Option<&mut Sexp> {
        if is_triple_id(node.id()) {
            return None;
        }

        match self.backend.node_mut_unchecked(node) {
            Node::Atomic => None,
            Node::Structured(structure) => Some(structure),
        }
    }
    fn node_as_triple(&self, node: LocalNode) -> Option<LocalTriple> {
        if !is_triple_id(node.id()) {
            return None;
        }

        Some(LocalTriple::new(node.id()))
    }

    fn triple_subject(&self, triple: LocalTriple) -> LocalNode {
        self.backend.triple_unchecked(triple.node()).subject
    }
    fn triple_predicate(&self, triple: LocalTriple) -> LocalNode {
        self.backend.triple_unchecked(triple.node()).predicate
    }
    fn triple_object(&self, triple: LocalTriple) -> LocalNode {
        self.backend.triple_unchecked(triple.node()).object
    }
    fn triple_index(&self, triple: LocalTriple) -> usize {
        triple_index_unchecked(triple.node().id())
    }
    fn triple_from_index(&self, index: usize) -> LocalTriple {
        index_to_triple_id(index)
    }
}

// We need this for dyn Environment to be cloneable. Just return a new env.
impl Clone for MemEnvironment {
    fn clone(&self) -> Self {
        MemEnvironment::new()
    }
}


#[cfg(test)]
#[path = "./mem_environment_test.rs"]
mod mem_environment_test;
