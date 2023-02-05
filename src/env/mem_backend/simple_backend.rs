use super::{index_id_conv::*, Edges, MemBackend, Node, Triple};
use crate::env::local_node::LocalNode;


/// Simple MemBackend implementation.
///
/// Not thread-safe or production-ready, but could be used for
/// local testing, simple local deployments, or as a fake in tests.
#[derive(Debug, Default)]
pub struct SimpleBackend {
    nodes: Vec<Node>,
    triples: Vec<Triple>,

    node_edges: Vec<Edges>,
    triple_edges: Vec<Edges>,
}

impl MemBackend for SimpleBackend {
    fn edges(&self, node: LocalNode) -> &Edges {
        if is_triple_id(node.id()) {
            &self.triple_edges[triple_index_unchecked(node.id())]
        } else {
            &self.node_edges[node_index_unchecked(node.id())]
        }
    }
    fn edges_mut(&mut self, node: LocalNode) -> &mut Edges {
        if is_triple_id(node.id()) {
            &mut self.triple_edges[triple_index_unchecked(node.id())]
        } else {
            &mut self.node_edges[node_index_unchecked(node.id())]
        }
    }

    fn node_unchecked(&self, node: LocalNode) -> &Node {
        &self.nodes[node_index_unchecked(node.id())]
    }
    fn node_mut_unchecked(&mut self, node: LocalNode) -> &mut Node {
        &mut self.nodes[node_index_unchecked(node.id())]
    }

    fn triple_unchecked(&self, triple: LocalNode) -> &Triple {
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
