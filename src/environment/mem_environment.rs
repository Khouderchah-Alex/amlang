//! Thread-unsafe in-memory Environment.

use log::trace;
use std::fmt::Debug;

use super::environment::{Environment, NodeSet, TripleSet};
use super::local_node::{LocalId, LocalNode, LocalTriple};
use super::mem_backend::{Edges, MemBackend, Node, Triple};
use crate::sexp::Sexp;


#[derive(Debug)]
pub struct MemEnvironment {
    nodes: Vec<Node>,
    triples: Vec<Triple>,

    node_edges: Vec<Edges>,
    triple_edges: Vec<Edges>,
}

impl MemEnvironment {
    pub fn new() -> MemEnvironment {
        Self {
            nodes: vec![],
            triples: vec![],

            node_edges: vec![],
            triple_edges: vec![],
        }
    }
}

impl MemBackend for MemEnvironment {
    fn edges(&self, node: LocalNode) -> &Edges {
        // TODO(sec) Under what conditions could IDs be faked?
        trace!("Env {}: edge lookup: {}", self.env_id(), node.id());
        if self.is_triple_id(node.id()) {
            &self.triple_edges[triple_index_unchecked(node.id())]
        } else {
            &self.node_edges[node_index_unchecked(node.id())]
        }
    }
    fn edges_mut(&mut self, node: LocalNode) -> &mut Edges {
        // TODO(sec) Under what conditions could IDs be faked?
        trace!("Env {}: edge mut lookup: {}", self.env_id(), node.id());
        if self.is_triple_id(node.id()) {
            &mut self.triple_edges[triple_index_unchecked(node.id())]
        } else {
            &mut self.node_edges[node_index_unchecked(node.id())]
        }
    }

    fn node_unchecked(&self, node: LocalNode) -> &Node {
        trace!("Env {}: node lookup: {}", self.env_id(), node.id());
        &self.nodes[node_index_unchecked(node.id())]
    }
    fn node_mut_unchecked(&mut self, node: LocalNode) -> &mut Node {
        trace!("Env {}: node mut lookup: {}", self.env_id(), node.id());
        &mut self.nodes[node_index_unchecked(node.id())]
    }

    fn triple_unchecked(&self, triple: LocalNode) -> &Triple {
        trace!("Env {}: triple lookup: {}", self.env_id(), triple.id());
        &self.triples[triple_index_unchecked(triple.id())]
    }

    fn push_node(&mut self, node: Node) {
        self.nodes.push(node);
    }

    fn push_triple(&mut self, triple: Triple) {
        self.triples.push(triple);
    }

    fn push_node_edges(&mut self, edges: Edges) {
        self.node_edges.push(edges);
    }

    fn push_triple_edges(&mut self, edges: Edges) {
        self.triple_edges.push(edges);
    }

    fn node_count(&self) -> usize {
        self.nodes.len()
    }

    fn triple_count(&self) -> usize {
        self.triples.len()
    }
}

impl Environment for MemEnvironment {
    fn all_nodes(&self) -> NodeSet {
        (0..self.node_count())
            .map(|x| LocalNode::new(x as LocalId))
            .collect()
    }

    fn insert_atom(&mut self) -> LocalNode {
        let id = self.next_node_id();
        self.push_node(Node::Atomic);
        self.push_node_edges(Edges::default());
        id
    }
    fn insert_structure(&mut self, structure: Sexp) -> LocalNode {
        let id = self.next_node_id();
        self.push_node(Node::Structured(structure));
        self.push_node_edges(Edges::default());
        id
    }
    fn insert_triple(
        &mut self,
        subject: LocalNode,
        predicate: LocalNode,
        object: LocalNode,
    ) -> LocalTriple {
        let id = self.next_triple_id();

        self.edges_mut(subject).as_subject.insert(id);
        self.edges_mut(predicate).as_predicate.insert(id);
        self.edges_mut(object).as_object.insert(id);

        self.push_triple(Triple {
            subject,
            predicate,
            object,
        });
        self.push_triple_edges(Edges::default());
        id
    }


    fn match_subject(&self, subject: LocalNode) -> TripleSet {
        let set = self.edges(subject).as_subject.iter();
        set.cloned().collect()
    }
    fn match_predicate(&self, predicate: LocalNode) -> TripleSet {
        let set = self.edges(predicate).as_predicate.iter();
        set.cloned().collect()
    }
    fn match_object(&self, object: LocalNode) -> TripleSet {
        let set = self.edges(object).as_object.iter();
        set.cloned().collect()
    }

    fn match_but_subject(&self, predicate: LocalNode, object: LocalNode) -> TripleSet {
        let set = self
            .edges(predicate)
            .as_predicate
            .intersection(&self.edges(object).as_object);
        set.cloned().collect()
    }
    fn match_but_predicate(&self, subject: LocalNode, object: LocalNode) -> TripleSet {
        let set = self
            .edges(subject)
            .as_subject
            .intersection(&self.edges(object).as_object);
        set.cloned().collect()
    }
    fn match_but_object(&self, subject: LocalNode, predicate: LocalNode) -> TripleSet {
        let set = self
            .edges(subject)
            .as_subject
            .intersection(&self.edges(predicate).as_predicate);
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
        (0..self.triple_count())
            .map(|x| self.index_to_triple_id(x))
            .collect()
    }

    fn node_structure(&self, node: LocalNode) -> Option<&Sexp> {
        if self.is_triple_id(node.id()) {
            return None;
        }

        match &self.node_unchecked(node) {
            Node::Atomic => None,
            Node::Structured(structure) => Some(structure),
        }
    }
    fn node_structure_mut(&mut self, node: LocalNode) -> Option<&mut Sexp> {
        if self.is_triple_id(node.id()) {
            return None;
        }

        match self.node_mut_unchecked(node) {
            Node::Atomic => None,
            Node::Structured(structure) => Some(structure),
        }
    }
    fn node_as_triple(&self, node: LocalNode) -> Option<LocalTriple> {
        if !self.is_triple_id(node.id()) {
            return None;
        }

        Some(LocalTriple::new(node.id()))
    }

    fn triple_subject(&self, triple: LocalTriple) -> LocalNode {
        self.triple_unchecked(triple.node()).subject
    }
    fn triple_predicate(&self, triple: LocalTriple) -> LocalNode {
        self.triple_unchecked(triple.node()).predicate
    }
    fn triple_object(&self, triple: LocalTriple) -> LocalNode {
        self.triple_unchecked(triple.node()).object
    }
    fn triple_index(&self, triple: LocalTriple) -> usize {
        triple_index_unchecked(triple.node().id())
    }
    fn triple_from_index(&self, index: usize) -> LocalTriple {
        self.index_to_triple_id(index)
    }
}

// We need this for dyn Environment to be cloneable. Just return a new env.
impl Clone for MemEnvironment {
    fn clone(&self) -> Self {
        MemEnvironment::new()
    }
}


fn triple_index_unchecked(id: LocalId) -> usize {
    (id & (LocalId::MAX >> 1)) as usize
}
// Note CANNOT be used for Nodes of Triples.
fn node_index_unchecked(id: LocalId) -> usize {
    id as usize
}


#[cfg(test)]
#[path = "./mem_environment_test.rs"]
mod mem_environment_test;
