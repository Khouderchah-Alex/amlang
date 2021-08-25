use std::collections::BTreeSet;

use super::local_node::{LocalId, LocalNode, LocalTriple};
use crate::primitive::Primitive;
use crate::sexp::Sexp;


// TODO(perf, scale) Allow for Edges to be pushed on-disk?
#[derive(Debug, Default)]
pub struct Edges {
    pub as_subject: BTreeSet<LocalTriple>,
    pub as_predicate: BTreeSet<LocalTriple>,
    pub as_object: BTreeSet<LocalTriple>,
}

#[derive(Debug)]
pub enum Node {
    Atomic,
    Structured(Sexp),
}

#[derive(Debug)]
pub struct Triple {
    pub object: LocalNode,
    pub predicate: LocalNode,
    pub subject: LocalNode,
}


pub trait MemBackend {
    fn edges(&self, node: LocalNode) -> &Edges;
    fn edges_mut(&mut self, node: LocalNode) -> &mut Edges;

    fn node_unchecked(&self, node: LocalNode) -> &Node;
    fn node_mut_unchecked(&mut self, node: LocalNode) -> &mut Node;

    fn triple_unchecked(&self, triple: LocalNode) -> &Triple;

    fn push_node(&mut self, node: Node);
    fn push_triple(&mut self, triple: Triple);
    fn push_node_edges(&mut self, edges: Edges);
    fn push_triple_edges(&mut self, edges: Edges);

    fn node_count(&self) -> usize;
    fn triple_count(&self) -> usize;


    fn next_node_id(&self) -> LocalNode {
        let num: LocalId = self.node_count() as LocalId;
        // TODO(scale, sec) Any problems with crash upon exhaustion? Probably
        // not a concern unless/until parts of an Environment can be offloaded.
        assert!(!self.is_triple_id(num));
        LocalNode::new(num)
    }
    fn next_triple_id(&self) -> LocalTriple {
        // TODO(scale, sec) Any problems with crash upon exhaustion? Probably
        // not a concern unless/until parts of an Environment can be offloaded.
        assert!(!self.is_triple_id(self.triple_count() as LocalId));

        self.index_to_triple_id(self.triple_count())
    }

    fn is_triple_id(&self, id: LocalId) -> bool {
        id.leading_ones() > 0
    }
    fn index_to_triple_id(&self, index: usize) -> LocalTriple {
        LocalTriple::new((index as LocalId) | !(LocalId::MAX >> 1))
    }

    fn env_id(&self) -> LocalId {
        // Technically, this is a bit of a layer violation, but by assuming the
        // self node exists, the env can identify itself at this layer.
        if let Node::Structured(Sexp::Primitive(Primitive::Node(node))) =
            self.node_unchecked(LocalNode::default())
        {
            node.env().id()
        } else {
            panic!();
        }
    }
}
