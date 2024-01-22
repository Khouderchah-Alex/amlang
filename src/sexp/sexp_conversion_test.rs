use crate::primitive::{LangString, Node, Number, Symbol, ToLangString};
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
    let original: Vec<Sexp> = vec![1.into(), 2.into(), 3.into()];
    let (a, b, c) =
        break_sexp!(original.into_iter().map(|e| (e, true)) => (Number, Number, Number)).unwrap();
    assert_eq!(a, Number::from(1));
    assert_eq!(b, Number::from(2));
    assert_eq!(c, Number::from(3));
}

#[test]
fn wrong_type() {
    let original: Sexp = "(lambda (a b) ing)".parse().unwrap();
    if let Err(err) = break_sexp!(original => (Node, Sexp, Symbol)) {
        let (_, kind, _) =
            break_sexp!(err.kind().reify() => (LangString, LangString; remainder)).unwrap();
        assert_eq!(kind, "InvalidArgument".to_lang_string());
    } else {
        panic!();
    }
}

#[test]
fn extra_arguments() {
    let original: Sexp = "(test ing 1 2)".parse().unwrap();
    if let Err(err) = break_sexp!(original => (Symbol, Symbol)) {
        let (_, kind, _) =
            break_sexp!(err.kind().reify() => (LangString, LangString; remainder)).unwrap();
        assert_eq!(kind, "WrongArgumentCount".to_lang_string());
    } else {
        panic!();
    }
}

#[test]
fn missing_arguments() {
    let original: Sexp = "(test)".parse().unwrap();
    if let Err(err) = break_sexp!(original => (Symbol, Symbol, Symbol)) {
        let (_, kind, _) =
            break_sexp!(err.kind().reify() => (LangString, LangString; remainder)).unwrap();
        assert_eq!(kind, "WrongArgumentCount".to_lang_string());
    } else {
        panic!();
    }
}


#[test]
fn simple_list() {
    let original: Sexp = "(1 2)".parse().unwrap();
    let l = list!(Number::generic(1), Number::generic(2));
    let (a, b) = break_sexp!(original => (Number, Number)).unwrap();
    let (aa, bb) = break_sexp!(l => (Number, Number)).unwrap();
    assert_eq!(a, aa);
    assert_eq!(b, bb);
}

#[test]
fn multi_type_list() {
    let original: Sexp = "(1 \"test\")".parse().unwrap();
    let l = list!(Number::generic(1), "test".to_lang_string());
    let (a, b) = break_sexp!(original => (Number, LangString)).unwrap();
    let (aa, bb) = break_sexp!(l => (Number, LangString)).unwrap();
    assert_eq!(a, aa);
    assert_eq!(b, bb);
}

#[test]
fn simple_list_vars() {
    let a = Number::generic(1);
    let b = Number::generic(2);
    let l = list!(a, b);
    let (aa, bb) = break_sexp!(l => (Number, Number)).unwrap();
    assert_eq!(a, aa);
    assert_eq!(b, bb);
}

#[test]
fn nested_list_vars() {
    let a = Number::generic(1);
    let b = Number::generic(2);
    let c = Number::generic(3);
    let l = list!(a, (b, c));
    let (aa, sub) = break_sexp!(l => (Number, HeapSexp)).unwrap();
    let (bb, cc) = break_sexp!(sub => (Number, Number)).unwrap();
    assert_eq!(a, aa);
    assert_eq!(b, bb);
    assert_eq!(c, cc);
}

#[test]
fn empty_list() {
    assert_eq!(list!(), "()".parse().unwrap());
    println!("{}", list!(()));
    assert_eq!(list!(()), "(())".parse().unwrap());
    assert_eq!(list!((), ()), "(() ())".parse().unwrap());
}

#[test]
fn multiple_nested_list() {
    assert_eq!(list!((())), "((()))".parse().unwrap());
    assert_eq!(list!((list!(()))), "(((())))".parse().unwrap());
    assert_eq!(list!((list!((list!())))), "((((()))))".parse().unwrap());
}
