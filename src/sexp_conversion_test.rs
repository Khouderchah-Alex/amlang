use std::convert::TryFrom;

use crate::function::EvalErr;
use crate::primitive::{NodeId, Symbol};
use crate::sexp::{Cons, Sexp};


#[test]
fn symbol_sexp() {
    let original = "(test ing)".parse::<Sexp>().unwrap();
    let (a, b) = break_by_types!(original, Symbol, Symbol).unwrap();
    assert_eq!(a.as_str(), "test");
    assert_eq!(b.as_str(), "ing");
}

#[test]
fn lambda_sexp() {
    let original = "(lambda (a b) ing)".parse::<Sexp>().unwrap();
    let (command, _, body) = break_by_types!(original, Symbol, Cons, Symbol).unwrap();
    assert_eq!(command.as_str(), "lambda");
    assert_eq!(body.as_str(), "ing");
}

#[test]
fn wrong_type() {
    let original = "(lambda (a b) ing)".parse::<Sexp>().unwrap();
    if let Err(EvalErr::InvalidArgument { .. }) = break_by_types!(original, NodeId, Sexp, Symbol) {
    } else {
        panic!();
    }
}

#[test]
fn extra_arguments() {
    let original = "(test ing 1 2)".parse::<Sexp>().unwrap();
    if let Err(EvalErr::WrongArgumentCount { given: 4, .. }) =
        break_by_types!(original, Symbol, Symbol)
    {
    } else {
        panic!();
    }
}

#[test]
fn missing_arguments() {
    let original = "(test)".parse::<Sexp>().unwrap();
    if let Err(EvalErr::WrongArgumentCount { given: 1, .. }) =
        break_by_types!(original, Symbol, Symbol, Symbol)
    {
    } else {
        panic!();
    }
}
