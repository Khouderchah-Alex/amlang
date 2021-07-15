//! Global Node reference for use in structures.

use std::convert::TryFrom;
use std::fmt;

use crate::environment::LocalNode;
use crate::primitive::Primitive;
use crate::sexp::Sexp;


#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialOrd, PartialEq)]
pub struct Node {
    env: LocalNode,
    node: LocalNode,
}


impl Node {
    pub const fn new(env: LocalNode, node: LocalNode) -> Self {
        Self { env, node }
    }

    pub const fn env(&self) -> LocalNode {
        self.env
    }

    pub const fn local(&self) -> LocalNode {
        self.node
    }
}


impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[Node_{}_{}]", self.env.id(), self.node.id())
    }
}


impl_try_from!(Sexp, Node, Node;
               // Prefer not to use this but need consistency for sexp_conversion.
               ref Sexp, ref Node, Node;
               Option<Sexp>, Node, Node;
               Result<Sexp>, Node, Node;);

impl<'a> TryFrom<&'a Sexp> for Node {
    type Error = ();

    fn try_from(value: &'a Sexp) -> Result<Self, Self::Error> {
        if let Sexp::Primitive(Primitive::Node(node)) = value {
            Ok(*node)
        } else {
            Err(())
        }
    }
}

impl<'a> TryFrom<Option<&'a Sexp>> for Node {
    type Error = ();

    fn try_from(value: Option<&'a Sexp>) -> Result<Self, Self::Error> {
        if let Some(Sexp::Primitive(Primitive::Node(node))) = value {
            Ok(*node)
        } else {
            Err(())
        }
    }
}

impl<'a, E> TryFrom<&'a Result<Sexp, E>> for Node {
    type Error = ();

    fn try_from(value: &'a Result<Sexp, E>) -> Result<Self, Self::Error> {
        if let Ok(Sexp::Primitive(Primitive::Node(node))) = value {
            Ok(*node)
        } else {
            Err(())
        }
    }
}
