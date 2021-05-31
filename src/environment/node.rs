//! Abstraction of nodes, to be connected by pairs of unlabeled, directed
//! edges (triples). Can be used for implementations of in-memory or IO-based
//! Environments.

use std::convert::TryFrom;
use std::fmt;

use crate::agent::env_state::EnvState;
use crate::model::Model;
use crate::primitive::Primitive;
use crate::sexp::{cons, HeapSexp, Sexp};


pub type LocalId = u64;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialOrd, PartialEq)]
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


impl Model for TripleId {
    fn generate_structure(&self, env_state: &mut EnvState) -> HeapSexp {
        let env = env_state.env();
        let s = env.triple_subject(*self);
        let p = env.triple_predicate(*self);
        let o = env.triple_object(*self);
        cons(
            Some(Box::new(s.into())),
            cons(
                Some(Box::new(p.into())),
                cons(Some(Box::new(o.into())), None),
            ),
        )
        .unwrap()
    }
}


impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[Node_{}]", self.id())
    }
}

impl fmt::Display for TripleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[Triple_{}]", self.node())
    }
}

impl TryFrom<Sexp> for NodeId {
    type Error = ();

    fn try_from(value: Sexp) -> Result<Self, Self::Error> {
        if let Sexp::Primitive(Primitive::Node(node)) = value {
            Ok(node)
        } else {
            Err(())
        }
    }
}

impl<'a> TryFrom<&'a Sexp> for NodeId {
    type Error = ();

    fn try_from(value: &'a Sexp) -> Result<Self, Self::Error> {
        if let Sexp::Primitive(Primitive::Node(node)) = value {
            Ok(*node)
        } else {
            Err(())
        }
    }
}

impl<'a> TryFrom<Option<&'a Sexp>> for NodeId {
    type Error = ();

    fn try_from(value: Option<&'a Sexp>) -> Result<Self, Self::Error> {
        if let Some(Sexp::Primitive(Primitive::Node(node))) = value {
            Ok(*node)
        } else {
            Err(())
        }
    }
}

impl<E> TryFrom<Result<Sexp, E>> for NodeId {
    type Error = ();

    fn try_from(value: Result<Sexp, E>) -> Result<Self, Self::Error> {
        if let Ok(Sexp::Primitive(Primitive::Node(node))) = value {
            Ok(node)
        } else {
            Err(())
        }
    }
}

impl<'a, E> TryFrom<&'a Result<Sexp, E>> for NodeId {
    type Error = ();

    fn try_from(value: &'a Result<Sexp, E>) -> Result<Self, Self::Error> {
        if let Ok(Sexp::Primitive(Primitive::Node(node))) = value {
            Ok(*node)
        } else {
            Err(())
        }
    }
}
