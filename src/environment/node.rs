//! Abstraction of nodes, to be connected by pairs of unlabeled, directed
//! edges (triples). Can be used for implementations of in-memory or IO-based
//! Environments.

use std::fmt;


pub type LocalId = u64;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialOrd, PartialEq)]
pub struct NodeId(LocalId);

#[derive(Clone, Copy, Debug, Eq, Ord, PartialOrd, PartialEq)]
pub struct TripleId(NodeId);


impl NodeId {
    pub(super) const fn new(id: LocalId) -> NodeId {
        NodeId(id)
    }

    pub fn id(&self) -> LocalId {
        self.0
    }
}

impl TripleId {
    pub(super) const fn new(id: LocalId) -> TripleId {
        TripleId(NodeId::new(id))
    }

    pub fn node(&self) -> NodeId {
        self.0
    }
}


impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NodeId {}", self.id())
    }
}

impl fmt::Display for TripleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TripleId {}", self.node())
    }
}
