use std::convert::TryFrom;

use serde::{Deserialize, Serialize};

use super::{Node, Primitive};
use crate::sexp::{HeapSexp, Sexp};


#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Procedure {
    #[serde(rename = "apply")]
    Application(Node, Vec<Node>),

    #[serde(rename = "lambda")]
    UserAbstraction(Vec<Node>, Node),

    #[serde(rename = "fexpr")]
    InterpreterAbstraction(Vec<Node>, Node),

    #[serde(rename = "progn")]
    Sequence(Vec<Node>),

    #[serde(rename = "if")]
    Branch(Box<(Node, Node, Node)>), // Pred, A, B.
}


impl_try_from!(Procedure;
               Primitive         ->  Procedure,
               Sexp              ->  Procedure,
               HeapSexp          ->  Procedure,
               ref Sexp          ->  ref Procedure,
               Option<Sexp>      ->  Procedure,
               Option<ref Sexp>  ->  ref Procedure,
               Result<Sexp>      ->  Procedure,
               Result<ref Sexp>  ->  ref Procedure,
);
