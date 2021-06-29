use std::convert::TryFrom;

use super::{Node, Primitive};
use crate::agent::env_state::EnvState;
use crate::model::Model;
use crate::sexp::{HeapSexp, Sexp};


#[derive(Clone, Debug, PartialEq)]
pub enum Procedure {
    Application(Node, Vec<Node>),
    Abstraction(Vec<Node>, Node),
    Sequence(Vec<Node>),
    Branch(Box<Branch>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Branch {
    cond: Node,
    a: Node,
    b: Node,
}


impl Model for Procedure {
    fn generate_structure(&self, env_state: &mut EnvState) -> HeapSexp {
        let context = env_state.context();
        match self {
            Procedure::Application(func, args) => {
                let apply_node = Node::new(context.lang_env(), context.apply);
                list!(apply_node, *func, args,)
            }
            Procedure::Abstraction(params, body) => {
                let lambda_node = Node::new(context.lang_env(), context.lambda);
                list!(lambda_node, params, *body,)
            }
            _ => panic!(),
        }
    }
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
