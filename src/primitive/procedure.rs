use std::convert::TryFrom;

use super::{BuiltIn, NodeId, Primitive};
use crate::sexp::Sexp;


/*
pub struct SProcedure {
    args: Vec<NodeId>,
    ret: Sexp,
}
*/

#[derive(Clone, Debug, PartialEq)]
pub enum Procedure {
    Application(NodeId, Vec<Sexp>),
    Sequence(Vec<Procedure>),
    Branch(Box<Branch>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Branch {
    cond: Procedure,
    a: Procedure,
    b: Procedure,
}


impl TryFrom<Sexp> for Procedure {
    type Error = ();

    fn try_from(value: Sexp) -> Result<Self, Self::Error> {
        if let Sexp::Primitive(Primitive::Procedure(procedure)) = value {
            Ok(procedure)
        } else {
            Err(())
        }
    }
}

impl<'a> TryFrom<&'a Sexp> for &'a Procedure {
    type Error = ();

    fn try_from(value: &'a Sexp) -> Result<Self, Self::Error> {
        if let Sexp::Primitive(Primitive::Procedure(procedure)) = value {
            Ok(procedure)
        } else {
            Err(())
        }
    }
}

impl<'a> TryFrom<Option<&'a Sexp>> for &'a Procedure {
    type Error = ();

    fn try_from(value: Option<&'a Sexp>) -> Result<Self, Self::Error> {
        if let Some(Sexp::Primitive(Primitive::Procedure(procedure))) = value {
            Ok(procedure)
        } else {
            Err(())
        }
    }
}

impl<E> TryFrom<Result<Sexp, E>> for Procedure {
    type Error = ();

    fn try_from(value: Result<Sexp, E>) -> Result<Self, Self::Error> {
        if let Ok(Sexp::Primitive(Primitive::Procedure(procedure))) = value {
            Ok(procedure)
        } else {
            Err(())
        }
    }
}

impl<'a, E> TryFrom<&'a Result<Sexp, E>> for &'a Procedure {
    type Error = ();

    fn try_from(value: &'a Result<Sexp, E>) -> Result<Self, Self::Error> {
        if let Ok(Sexp::Primitive(Primitive::Procedure(procedure))) = value {
            Ok(procedure)
        } else {
            Err(())
        }
    }
}
