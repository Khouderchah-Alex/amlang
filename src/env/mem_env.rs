//! Implementation of Environment based on underlying MemBackend.

use log::warn;
use std::fmt::Debug;

use super::entry::{Entry, EntryKind, EntryMut, EntryMutKind};
use super::local_node::{LocalId, LocalNode, LocalTriple};
use super::mem_backend::{index_id_conv::*, Edges, MemBackend, Node, Triple};
use super::{Environment, NodeSet, TripleSet};
use crate::primitive::Node as PrimitiveNode;
use crate::primitive::Symbol;
use crate::sexp::Sexp;


#[derive(Debug, Default)]
pub struct MemEnv<Backend: MemBackend + 'static> {
    backend: Backend,
}

impl<Backend: MemBackend> MemEnv<Backend> {
    pub fn new() -> Self {
        Default::default()
    }
}

impl<Backend: MemBackend> Environment for MemEnv<Backend> {
    fn type_name(&self) -> &'static str {
        "MemEnv"
    }

    fn all_nodes(&self) -> NodeSet {
        (0..self.backend.node_count())
            .map(|x| LocalNode::new(x as LocalId))
            .collect()
    }

    fn insert_node(&mut self, structure: Option<Sexp>) -> LocalNode {
        let id = self.backend.next_node_id();
        match structure {
            Some(structure) => self.backend.push_node(Node::Structured(structure)),
            None => self.backend.push_node(Node::Atomic),
        }
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


    fn insert_designation(&mut self, node: PrimitiveNode, designation: Symbol, context: LocalNode) {
        self.backend
            .designator_mut(context)
            .insert(designation, node);
    }

    fn match_designation(&self, designation: &Symbol, context: LocalNode) -> Option<PrimitiveNode> {
        if let Some(designator) = self.backend.designator(context) {
            return designator.get_by_left(designation).copied();
        }
        None
    }

    fn find_designation(&self, node: PrimitiveNode, context: LocalNode) -> Option<&Symbol> {
        if let Some(designator) = self.backend.designator(context) {
            return designator.get_by_right(&node);
        }
        None
    }

    fn designation_pairs(&self, context: LocalNode) -> Vec<(&Symbol, &PrimitiveNode)> {
        match self.backend.designator(context) {
            Some(designator) => designator.iter().collect(),
            None => vec![],
        }
    }


    fn match_subject(&self, subject: LocalNode) -> TripleSet {
        let set = self.backend.edges(subject).as_subject.iter();
        TripleSet::new(self, set.cloned().collect())
    }
    fn match_predicate(&self, predicate: LocalNode) -> TripleSet {
        let set = self.backend.edges(predicate).as_predicate.iter();
        TripleSet::new(self, set.cloned().collect())
    }
    fn match_object(&self, object: LocalNode) -> TripleSet {
        let set = self.backend.edges(object).as_object.iter();
        TripleSet::new(self, set.cloned().collect())
    }

    fn match_but_subject(&self, predicate: LocalNode, object: LocalNode) -> TripleSet {
        let set = self
            .backend
            .edges(predicate)
            .as_predicate
            .intersection(&self.backend.edges(object).as_object);
        TripleSet::new(self, set.cloned().collect())
    }
    fn match_but_predicate(&self, subject: LocalNode, object: LocalNode) -> TripleSet {
        let set = self
            .backend
            .edges(subject)
            .as_subject
            .intersection(&self.backend.edges(object).as_object);
        TripleSet::new(self, set.cloned().collect())
    }
    fn match_but_object(&self, subject: LocalNode, predicate: LocalNode) -> TripleSet {
        let set = self
            .backend
            .edges(subject)
            .as_subject
            .intersection(&self.backend.edges(predicate).as_predicate);
        TripleSet::new(self, set.cloned().collect())
    }

    fn match_triple(
        &self,
        subject: LocalNode,
        predicate: LocalNode,
        object: LocalNode,
    ) -> TripleSet {
        let option = self
            .match_but_object(subject, predicate)
            .triples()
            .find(|&triple| self.triple_object(triple) == object);
        TripleSet::from_option(self, option)
    }
    fn match_all(&self) -> TripleSet {
        // TODO(feat) Watch out if backends can ever have gaps here.
        let elements = (0..self.backend.triple_count())
            .map(|x| index_to_triple_id(x))
            .collect();
        TripleSet::new(self, elements)
    }

    fn entry(&self, node: LocalNode) -> Entry {
        let kind = if is_triple_id(node.id()) {
            // TODO(func) Add Triple to EntryKind?
            EntryKind::Atomic
        } else {
            match self.backend.node_unchecked(node) {
                Node::Atomic => EntryKind::Atomic,
                Node::Structured(structure) => EntryKind::Borrowed(structure),
            }
        };
        Entry::new(kind)
    }
    fn entry_mut(&mut self, node: LocalNode) -> EntryMut {
        let p = self as *mut _;
        let kind = if is_triple_id(node.id()) {
            // TODO(func) Add Triple to EntryMutKind. Leaving this here can lead
            // to undefined behavior if we call entry_mut on a Triple.
            EntryMutKind::Atomic
        } else {
            match self.backend.node_mut_unchecked(node) {
                Node::Atomic => EntryMutKind::Atomic,
                Node::Structured(structure) => EntryMutKind::Borrowed(structure),
            }
        };
        EntryMut::new(node, kind, p)
    }
    fn entry_update(&mut self, entry: EntryMut) -> LocalNode {
        let (node, kind, env) = entry.consume();
        assert_eq!(self as *mut dyn Environment, env.unwrap());

        let stored = self.backend.node_mut_unchecked(node);
        match kind {
            EntryMutKind::Atomic => {
                *stored = Node::Atomic;
            }
            EntryMutKind::Borrowed(_) => { /* Already set this. */ }
            EntryMutKind::Owned(sexp) => {
                *stored = Node::Structured(sexp);
            }
        }
        node
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

// We need this for Environment: DynClone. Just return a new env.
impl<Backend: MemBackend> Clone for MemEnv<Backend> {
    fn clone(&self) -> Self {
        warn!(
            "Env @ {} being empty-cloned.",
            self.entry(LocalNode::default()).structure(),
        );
        MemEnv::new()
    }
}


#[cfg(test)]
#[path = "./mem_env_test.rs"]
mod mem_env_test;
