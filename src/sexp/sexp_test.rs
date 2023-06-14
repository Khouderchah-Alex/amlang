use super::*;

use crate::primitive::symbol_policies::policy_base;
use crate::primitive::{Number, ToSymbol};


#[test]
fn push_front() {
    // Append on primitive.
    let mut s = Sexp::Primitive(Number::I64(1).into());

    s.push_front(Number::I64(2));
    assert_eq!(s, "(2 1)".parse().unwrap());

    s.push_front(Number::I64(3));
    assert_eq!(s, "(3 2 1)".parse().unwrap());

    s = Sexp::default();
    assert_eq!(s, "()".parse().unwrap());

    // Append on empty cons.
    s.push_front(Number::I64(1));
    assert_eq!(s, "(1)".parse().unwrap());
}

#[test]
fn vec_into_sexp() {
    let expected = "(test ing)".parse().unwrap();
    let v = vec![
        "test".to_symbol_or_panic(policy_base),
        "ing".to_symbol_or_panic(policy_base),
    ];
    assert_eq!(<Sexp>::from(&v), expected);
    assert_eq!(<Sexp>::from(v), expected);
}

#[test]
fn non_cons() {
    let s = "(1 2 3 . 4)".parse().unwrap();
    let mut iter = HeapSexp::new(s).into_iter();
    assert_eq!(
        iter.next().unwrap(),
        (HeapSexp::new(Number::I64(1).into()), true)
    );
    assert_eq!(
        iter.next().unwrap(),
        (HeapSexp::new(Number::I64(2).into()), true)
    );
    assert_eq!(
        iter.next().unwrap(),
        (HeapSexp::new(Number::I64(3).into()), true)
    );
    assert_eq!(
        iter.next().unwrap(),
        (HeapSexp::new(Number::I64(4).into()), false)
    );
}
