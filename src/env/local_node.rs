//! Abstraction of nodes, to be connected by pairs of unlabeled, directed
//! edges (triples). Can be used for implementations of in-memory or IO-based
//! Environments.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::agent::Agent;
use crate::primitive::Node;
use crate::sexp::Sexp;


pub type LocalId = u64;

#[derive(
    Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialOrd, PartialEq, Serialize, Deserialize,
)]
pub struct LocalNode(LocalId);

#[derive(Clone, Copy, Debug, Eq, Ord, PartialOrd, PartialEq)]
pub struct LocalTriple(LocalNode);


impl LocalNode {
    pub const fn new(id: LocalId) -> LocalNode {
        LocalNode(id)
    }

    pub const fn id(&self) -> LocalId {
        self.0
    }

    /// Globalize relative to current env of agent.
    pub fn globalize(self, agent: &Agent) -> Node {
        agent.globalize(self)
    }
}

impl LocalTriple {
    pub const fn new(id: LocalId) -> LocalTriple {
        LocalTriple(LocalNode::new(id))
    }

    pub fn node(&self) -> LocalNode {
        self.0
    }

    pub fn reify(&self, agent: &Agent) -> Sexp {
        let e = agent.pos().env();
        let env = agent.access_env(e).unwrap();
        let s = Node::new(e, env.triple_subject(*self));
        let p = Node::new(e, env.triple_predicate(*self));
        let o = Node::new(e, env.triple_object(*self));
        list!(s, p, o)
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
