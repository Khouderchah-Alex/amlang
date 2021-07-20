use std::convert::TryFrom;

use crate::function::EvalErr;
use crate::primitive::{Node, Number, Symbol};
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
fn list_sexp() {
    let original = "(a b c)".parse::<Sexp>().unwrap();
    let (a, r) = break_by_types!(original, Symbol; remainder).unwrap();
    assert_eq!(a.as_str(), "a");

    let (b, r2) = break_by_types!(*r.unwrap(), Symbol; remainder).unwrap();
    assert_eq!(b.as_str(), "b");

    let (c, r3) = break_by_types!(*r2.unwrap(), Symbol; remainder).unwrap();
    assert_eq!(c.as_str(), "c");

    assert_eq!(r3, None);
}

#[test]
fn wrong_type() {
    let original = "(lambda (a b) ing)".parse::<Sexp>().unwrap();
    if let Err(EvalErr::InvalidArgument { .. }) = break_by_types!(original, Node, Sexp, Symbol) {
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


#[test]
fn simple_list() {
    let original = "(1 2)".parse::<Sexp>().unwrap();
    let l = list!(Number::Integer(1), Number::Integer(2),);
    let (a, b) = break_by_types!(original, Number, Number).unwrap();
    let (aa, bb) = break_by_types!(*l, Number, Number).unwrap();
    assert_eq!(a, aa);
    assert_eq!(b, bb);
}

#[test]
fn simple_list_vars() {
    let a = Number::Integer(1);
    let b = Number::Integer(2);
    let l = list!(a, b,);
    let (aa, bb) = break_by_types!(*l, Number, Number).unwrap();
    assert_eq!(a, aa);
    assert_eq!(b, bb);
}

#[test]
fn nested_list_vars() {
    let a = Number::Integer(1);
    let b = Number::Integer(2);
    let c = Number::Integer(3);
    let l = list!(a, (b, c,),);
    let (aa, sub) = break_by_types!(*l, Number, Sexp).unwrap();
    let (bb, cc) = break_by_types!(sub, Number, Number).unwrap();
    assert_eq!(a, aa);
    assert_eq!(b, bb);
    assert_eq!(c, cc);
}