use std::convert::TryFrom;

use super::{Node, Primitive};
use crate::agent::agent_state::AgentState;
use crate::model::Model;
use crate::sexp::{HeapSexp, Sexp};


#[derive(Clone, Debug, PartialEq)]
pub enum Procedure {
    Application(Node, Vec<Node>),
    Abstraction(Vec<Node>, Node, bool),
    Sequence(Vec<Node>),
    Branch(Node, Node, Node), // Pred, A, B.
}


impl Model for Procedure {
    fn generate_structure(&self, state: &mut AgentState) -> HeapSexp {
        let context = state.context();
        match self {
            Procedure::Application(func, args) => {
                let apply_node = Node::new(context.lang_env(), context.apply);
                list!(apply_node, *func, args,)
            }
            Procedure::Abstraction(params, body, reflect) => {
                let special_node = if *reflect {
                    Node::new(context.lang_env(), context.fexpr)
                } else {
                    Node::new(context.lang_env(), context.lambda)
                };
                list!(special_node, params, *body,)
            }
            Procedure::Branch(pred, a, b) => {
                let branch_node = Node::new(context.lang_env(), context.branch);
                list!(branch_node, *pred, *a, *b,)
            }
            _ => panic!(),
        }
    }
}


impl_try_from!(Sexp              ->  Procedure,      Procedure;
               ref Sexp          ->  ref Procedure,  Procedure;
               Option<Sexp>      ->  Procedure,      Procedure;
               Option<ref Sexp>  ->  ref Procedure,  Procedure;
               Result<Sexp>      ->  Procedure,      Procedure;
               Result<ref Sexp>  ->  ref Procedure,  Procedure;
);
