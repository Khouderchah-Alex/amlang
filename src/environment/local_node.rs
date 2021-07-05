//! Abstraction of nodes, to be connected by pairs of unlabeled, directed
//! edges (triples). Can be used for implementations of in-memory or IO-based
//! Environments.

use std::fmt;

use crate::agent::env_state::EnvState;
use crate::model::Model;
use crate::primitive::Node;
use crate::sexp::HeapSexp;


pub type LocalId = u64;

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialOrd, PartialEq)]
pub struct LocalNode(LocalId);

#[derive(Clone, Copy, Debug, Eq, Ord, PartialOrd, PartialEq)]
pub struct LocalTriple(LocalNode);


impl LocalNode {
    pub const fn new(id: LocalId) -> LocalNode {
        LocalNode(id)
    }

    pub fn id(&self) -> LocalId {
        self.0
    }

    /// Globalize relative to current env of env_state.
    pub fn globalize(self, env_state: &EnvState) -> Node {
        env_state.globalize(self)
    }
}

impl LocalTriple {
    pub const fn new(id: LocalId) -> LocalTriple {
        LocalTriple(LocalNode::new(id))
    }

    pub fn node(&self) -> LocalNode {
        self.0
    }
}


impl Model for LocalTriple {
    fn generate_structure(&self, env_state: &mut EnvState) -> HeapSexp {
        let e = env_state.pos().env();
        let env = env_state.env();
        let s = Node::new(e, env.triple_subject(*self));
        let p = Node::new(e, env.triple_predicate(*self));
        let o = Node::new(e, env.triple_object(*self));
        list!(s, p, o,)
    }
}

impl fmt::Display for LocalNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[LocalNode_{}]", self.id())
    }
}

impl fmt::Display for LocalTriple {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[LocalTriple_{}]", self.node())
    }
}
