//! Abstraction of nodes, to be connected by pairs of unlabeled, directed
//! edges (triples). Can be used for implementations of in-memory or IO-based
//! Environments.

use std::fmt;

use crate::agent::amlang_context::EnvPrelude;
use crate::agent::Agent;
use crate::error::Error;
use crate::model::Reflective;
use crate::primitive::Node;
use crate::sexp::Sexp;


pub type LocalId = u64;

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialOrd, PartialEq)]
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

    pub const fn as_prelude(&self) -> Option<EnvPrelude> {
        match self.id() {
            0 => Some(EnvPrelude::SelfEnv),
            1 => Some(EnvPrelude::Designation),
            2 => Some(EnvPrelude::TellHandler),
            3 => Some(EnvPrelude::Reserved3),
            4 => Some(EnvPrelude::Reserved4),
            5 => Some(EnvPrelude::Reserved5),
            6 => Some(EnvPrelude::Reserved6),
            7 => Some(EnvPrelude::Reserved7),
            8 => Some(EnvPrelude::Reserved8),
            9 => Some(EnvPrelude::Reserved9),
            _ => None,
        }
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


impl Reflective for LocalTriple {
    fn reify(&self, agent: &mut Agent) -> Sexp {
        let e = agent.pos().env();
        let env = agent.env();
        let s = Node::new(e, env.triple_subject(*self));
        let p = Node::new(e, env.triple_predicate(*self));
        let o = Node::new(e, env.triple_object(*self));
        list!(s, p, o,)
    }

    fn reflect<F>(
        _structure: Sexp,
        _agent: &mut Agent,
        _process_primitive: F,
    ) -> Result<Self, Error> {
        unimplemented!();
    }

    fn valid_discriminator(_node: Node, _agent: &Agent) -> bool {
        return false;
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
