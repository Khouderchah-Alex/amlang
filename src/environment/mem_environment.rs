//! Thread-unsafe in-memory Environment.

use std::collections::BTreeSet;

use super::environment::{Environment, Resolver, TripleSet};
use super::node::{LocalId, NodeId, TripleId};
use crate::sexp::Sexp;


#[derive(Debug)]
pub struct MemEnvironment {
    nodes: Vec<Node>,
    triples: Vec<Triple>,
}

// TODO(perf, scale) Allow for Edges to be pushed on-disk?
#[derive(Debug, Default)]
struct Edges {
    as_subject: BTreeSet<TripleId>,
    as_predicate: BTreeSet<TripleId>,
    as_object: BTreeSet<TripleId>,
}

#[derive(Debug)]
struct Node {
    kind: NodeKind,
    edges: Edges,
}

#[derive(Debug)]
enum NodeKind {
    Atomic,
    Structured(Sexp),
}

#[derive(Debug)]
struct Triple {
    object: NodeId,
    predicate: NodeId,
    subject: NodeId,

    edges: Edges,
}


impl MemEnvironment {
    pub fn new() -> MemEnvironment {
        let mut env = MemEnvironment {
            nodes: vec![],
            triples: vec![],
        };
        // Set up SELF node.
        // TODO(func) This should be a Portal rather than Atom.
        env.insert_atom();
        env
    }

    fn edges(&self, node: NodeId) -> &Edges {
        // TODO(sec) Under what conditions could IDs be faked?
        if is_triple_id(node.id()) {
            &self.triple_unchecked(node).edges
        } else {
            &self.node_unchecked(node).edges
        }
    }
    fn edges_mut(&mut self, node: NodeId) -> &mut Edges {
        // TODO(sec) Under what conditions could IDs be faked?
        if is_triple_id(node.id()) {
            &mut self.triple_mut_unchecked(node).edges
        } else {
            &mut self.node_mut_unchecked(node).edges
        }
    }

    fn node_unchecked(&self, node: NodeId) -> &Node {
        &self.nodes[node_index_unchecked(node.id())]
    }
    fn node_mut_unchecked(&mut self, node: NodeId) -> &mut Node {
        &mut self.nodes[node_index_unchecked(node.id())]
    }

    fn triple_unchecked(&self, triple: NodeId) -> &Triple {
        &self.triples[triple_index_unchecked(triple.id())]
    }
    fn triple_mut_unchecked(&mut self, triple: NodeId) -> &mut Triple {
        &mut self.triples[triple_index_unchecked(triple.id())]
    }


    fn next_node_id(&self) -> NodeId {
        let num: LocalId = self.nodes.len() as LocalId;
        // TODO(scale, sec) Any problems with crash upon exhaustion? Probably
        // not a concern unless/until parts of an Environment can be offloaded.
        assert!(!is_triple_id(num));
        NodeId::new(num)
    }

    fn next_triple_id(&self) -> TripleId {
        // TODO(scale, sec) Any problems with crash upon exhaustion? Probably
        // not a concern unless/until parts of an Environment can be offloaded.
        assert!(!is_triple_id(self.triples.len() as LocalId));

        index_to_triple_id(self.triples.len())
    }
}

impl Environment for MemEnvironment {
    const SELF: NodeId = NodeId::new(0);

    fn insert_atom(&mut self) -> NodeId {
        let id = self.next_node_id();
        self.nodes.push(Node::new(NodeKind::Atomic));
        id
    }
    fn insert_structure(&mut self, structure: Sexp) -> NodeId {
        let id = self.next_node_id();
        self.nodes.push(Node::new(NodeKind::Structured(structure)));
        id
    }
    fn insert_triple(&mut self, subject: NodeId, predicate: NodeId, object: NodeId) -> TripleId {
        let id = self.next_triple_id();

        self.edges_mut(subject).as_subject.insert(id);
        self.edges_mut(predicate).as_predicate.insert(id);
        self.edges_mut(object).as_object.insert(id);

        self.triples.push(Triple {
            subject,
            predicate,
            object,
            edges: Edges::default(),
        });
        id
    }

    fn match_subject(&self, subject: NodeId) -> TripleSet {
        let set = self.edges(subject).as_subject.iter();
        set.cloned().collect()
    }
    fn match_predicate(&self, predicate: NodeId) -> TripleSet {
        let set = self.edges(predicate).as_predicate.iter();
        set.cloned().collect()
    }
    fn match_object(&self, object: NodeId) -> TripleSet {
        let set = self.edges(object).as_object.iter();
        set.cloned().collect()
    }

    fn match_but_subject(&self, predicate: NodeId, object: NodeId) -> TripleSet {
        let set = self
            .edges(predicate)
            .as_predicate
            .intersection(&self.edges(object).as_object);
        set.cloned().collect()
    }
    fn match_but_predicate(&self, subject: NodeId, object: NodeId) -> TripleSet {
        let set = self
            .edges(subject)
            .as_subject
            .intersection(&self.edges(object).as_object);
        set.cloned().collect()
    }
    fn match_but_object(&self, subject: NodeId, predicate: NodeId) -> TripleSet {
        let set = self
            .edges(subject)
            .as_subject
            .intersection(&self.edges(predicate).as_predicate);
        set.cloned().collect()
    }

    fn match_triple(&self, subject: NodeId, predicate: NodeId, object: NodeId) -> Option<TripleId> {
        self.match_but_object(subject, predicate)
            .iter()
            .find(|&&triple| self.triple_object(triple) == object)
            .cloned()
    }
    fn match_all(&self) -> TripleSet {
        (0..self.triples.len())
            .map(|x| index_to_triple_id(x))
            .collect()
    }
}

impl Resolver for MemEnvironment {
    fn node_structure(&self, node: NodeId) -> Option<&Sexp> {
        if is_triple_id(node.id()) {
            return None;
        }

        match &self.node_unchecked(node).kind {
            NodeKind::Atomic => None,
            NodeKind::Structured(structure) => Some(structure),
        }
    }
    fn node_as_triple(&self, node: NodeId) -> Option<TripleId> {
        if !is_triple_id(node.id()) {
            return None;
        }

        Some(TripleId::new(node.id()))
    }

    fn triple_subject(&self, triple: TripleId) -> NodeId {
        self.triple_unchecked(triple.node()).subject
    }
    fn triple_predicate(&self, triple: TripleId) -> NodeId {
        self.triple_unchecked(triple.node()).predicate
    }
    fn triple_object(&self, triple: TripleId) -> NodeId {
        self.triple_unchecked(triple.node()).object
    }
}


impl Node {
    fn new(kind: NodeKind) -> Node {
        Node {
            kind,
            edges: Edges::default(),
        }
    }
}

fn is_triple_id(id: LocalId) -> bool {
    id.leading_ones() > 0
}
fn index_to_triple_id(index: usize) -> TripleId {
    TripleId::new((index as LocalId) | !(LocalId::MAX >> 1))
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
