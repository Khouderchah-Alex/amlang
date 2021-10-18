use crate::primitive::error::ErrKind::*;
use crate::primitive::{AmString, Node, Number, Symbol};
use crate::sexp::{Cons, HeapSexp, Sexp};


#[test]
fn symbol_sexp() {
    let original: Sexp = "(test ing)".parse().unwrap();
    let (a, b) = break_sexp!(original => (Symbol, Symbol)).unwrap();
    assert_eq!(a.as_str(), "test");
    assert_eq!(b.as_str(), "ing");
}

#[test]
fn lambda_sexp() {
    let original: Sexp = "(lambda (a b) ing)".parse().unwrap();
    let (command, _, body) = break_sexp!(original => (Symbol, Cons, Symbol)).unwrap();
    assert_eq!(command.as_str(), "lambda");
    assert_eq!(body.as_str(), "ing");
}

#[test]
fn list_sexp() {
    let original: Sexp = "(a b c)".parse().unwrap();
    let (a, r) = break_sexp!(original => (Symbol; remainder)).unwrap();
    assert_eq!(a.as_str(), "a");

    let (b, r2) = break_sexp!(r.unwrap() => (Symbol; remainder)).unwrap();
    assert_eq!(b.as_str(), "b");

    let (c, r3) = break_sexp!(r2.unwrap() => (Symbol; remainder)).unwrap();
    assert_eq!(c.as_str(), "c");

    assert_eq!(r3, None);
}

#[test]
fn ref_elements() {
    let original: Sexp = "(a b c)".parse().unwrap();
    let (a, r) = break_sexp!(original.iter() => (&Symbol; remainder)).unwrap();
    assert_eq!(a.as_str(), "a");

    let (b, r2) = break_sexp!(r.unwrap() => (&Symbol; remainder)).unwrap();
    assert_eq!(b.as_str(), "b");

    let (c, r3) = break_sexp!(r2.unwrap() => (&Symbol; remainder)).unwrap();
    assert_eq!(c.as_str(), "c");
    assert_eq!(r3, None);

    let (a, b, c) = break_sexp!(original.iter() => (&Symbol, &Symbol, &Symbol)).unwrap();
    assert_eq!(a.as_str(), "a");
    assert_eq!(b.as_str(), "b");
    assert_eq!(c.as_str(), "c");

    // Verify original is unmodified.
    assert_eq!(original, "(a b c)".parse().unwrap());
}

#[test]
fn vec_break_no_remainder() {
    let original: Vec<Sexp> = vec![
        Number::Integer(1).into(),
        Number::Integer(2).into(),
        Number::Integer(3).into(),
    ];
    let (a, b, c) =
        break_sexp!(original.into_iter().map(|e| (e, true)) => (Number, Number, Number)).unwrap();
    assert_eq!(a, Number::Integer(1));
    assert_eq!(b, Number::Integer(2));
    assert_eq!(c, Number::Integer(3));
}

#[test]
fn wrong_type() {
    let original: Sexp = "(lambda (a b) ing)".parse().unwrap();
    if let Err(err) = break_sexp!(original => (Node, Sexp, Symbol)) {
        assert!(matches!(err.kind(), InvalidArgument { .. }));
    } else {
        panic!();
    }
}

#[test]
fn extra_arguments() {
    let original: Sexp = "(test ing 1 2)".parse().unwrap();
    if let Err(err) = break_sexp!(original => (Symbol, Symbol)) {
        assert!(matches!(err.kind(), WrongArgumentCount { given: 4, .. }));
    } else {
        panic!();
    }
}

#[test]
fn missing_arguments() {
    let original: Sexp = "(test)".parse().unwrap();
    if let Err(err) = break_sexp!(original => (Symbol, Symbol, Symbol)) {
        assert!(matches!(err.kind(), WrongArgumentCount { given: 1, .. }));
    } else {
        panic!();
    }
}


#[test]
fn simple_list() {
    let original: Sexp = "(1 2)".parse().unwrap();
    let l = list!(Number::Integer(1), Number::Integer(2),);
    let (a, b) = break_sexp!(original => (Number, Number)).unwrap();
    let (aa, bb) = break_sexp!(l => (Number, Number)).unwrap();
    assert_eq!(a, aa);
    assert_eq!(b, bb);
}

#[test]
fn multi_type_list() {
    let original: Sexp = "(1 \"test\")".parse().unwrap();
    let l = list!(Number::Integer(1), AmString::new("test"),);
    let (a, b) = break_sexp!(original => (Number, AmString)).unwrap();
    let (aa, bb) = break_sexp!(l => (Number, AmString)).unwrap();
    assert_eq!(a, aa);
    assert_eq!(b, bb);
}

#[test]
fn simple_list_vars() {
    let a = Number::Integer(1);
    let b = Number::Integer(2);
    let l = list!(a, b,);
    let (aa, bb) = break_sexp!(l => (Number, Number)).unwrap();
    assert_eq!(a, aa);
    assert_eq!(b, bb);
}

#[test]
fn nested_list_vars() {
    let a = Number::Integer(1);
    let b = Number::Integer(2);
    let c = Number::Integer(3);
    let l = list!(a, (b, c,),);
    let (aa, sub) = break_sexp!(l => (Number, HeapSexp)).unwrap();
    let (bb, cc) = break_sexp!(sub => (Number, Number)).unwrap();
    assert_eq!(a, aa);
    assert_eq!(b, bb);
    assert_eq!(c, cc);
}
