//! Global Node reference for use in structures.

use std::convert::TryFrom;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::env::LocalNode;
use crate::primitive::Primitive;
use crate::sexp::{HeapSexp, Sexp};


#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialOrd, PartialEq, Serialize, Deserialize)]
pub struct Node {
    env: LocalNode,
    local: LocalNode,
}


impl Node {
    pub const fn new(env: LocalNode, local: LocalNode) -> Self {
        Self { env, local }
    }

    pub const fn env(&self) -> LocalNode {
        self.env
    }

    pub const fn local(&self) -> LocalNode {
        self.local
    }
}


impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[Node_{}_{}]", self.env.id(), self.local.id())
    }
}


impl_try_from!(Node;
               Primitive     -> Node,
               Sexp          -> Node,
               HeapSexp      -> Node,
               // Prefer not to use this but need consistency for sexp_conversion.
               ref Sexp      ->  ref Node,
               Option<Sexp>  ->  Node,
               Result<Sexp>  ->  Node,
);

impl<'a> TryFrom<&'a Sexp> for Node {
    type Error = &'a Sexp;

    fn try_from(value: &'a Sexp) -> Result<Self, Self::Error> {
        if let Sexp::Primitive(Primitive::Node(node)) = value {
            Ok(*node)
        } else {
            Err(value)
        }
    }
}

impl<'a> TryFrom<Option<&'a Sexp>> for Node {
    type Error = Option<&'a Sexp>;

    fn try_from(value: Option<&'a Sexp>) -> Result<Self, Self::Error> {
        if let Some(Sexp::Primitive(Primitive::Node(node))) = value {
            Ok(*node)
        } else {
            Err(value)
        }
    }
}

impl<'a, E> TryFrom<&'a Result<Sexp, E>> for Node {
    type Error = &'a Result<Sexp, E>;

    fn try_from(value: &'a Result<Sexp, E>) -> Result<Self, Self::Error> {
        if let Ok(Sexp::Primitive(Primitive::Node(node))) = value {
            Ok(*node)
        } else {
            Err(value)
        }
    }
}
