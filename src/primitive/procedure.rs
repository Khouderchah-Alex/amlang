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


impl_try_from!(Sexp, Procedure, Procedure;
               ref Sexp, ref Procedure, Procedure;
               Option<Sexp>, Procedure, Procedure;
               Option<ref Sexp>, ref Procedure, Procedure;
               Result<Sexp>, Procedure, Procedure;
               Result<ref Sexp>, ref Procedure, Procedure;);
