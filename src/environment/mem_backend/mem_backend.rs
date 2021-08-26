use std::fmt::Debug;

use super::{index_id_conv::*, Edges, Node, Triple};
use crate::environment::local_node::{LocalId, LocalNode, LocalTriple};
use crate::primitive::Primitive;
use crate::sexp::Sexp;


pub trait MemBackend: Debug {
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
        assert!(!is_triple_id(num));
        LocalNode::new(num)
    }
    fn next_triple_id(&self) -> LocalTriple {
        // TODO(scale, sec) Any problems with crash upon exhaustion? Probably
        // not a concern unless/until parts of an Environment can be offloaded.
        assert!(!is_triple_id(self.triple_count() as LocalId));

        index_to_triple_id(self.triple_count())
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
