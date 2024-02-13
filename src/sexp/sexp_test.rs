use super::*;

use crate::primitive::symbol_policies::policy_base;
use crate::primitive::ToSymbol;


#[test]
fn push_front() {
    // Append on primitive.
    let mut s = Sexp::from(Number::generic(1));

    s.push_front(Number::generic(2));
    assert_eq!(s, "(2 1)".parse().unwrap());

    s.push_front(Number::generic(3));
    assert_eq!(s, "(3 2 1)".parse().unwrap());

    s = Sexp::default();
    assert_eq!(s, "()".parse().unwrap());

    // Append on empty cons.
    s.push_front(Number::generic(1));
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
    assert_eq!(iter.next().unwrap(), (Number::generic(1).into(), true));
    assert_eq!(iter.next().unwrap(), (Number::generic(2).into(), true));
    assert_eq!(iter.next().unwrap(), (Number::generic(3).into(), true));
    assert_eq!(iter.next().unwrap(), (Number::generic(4).into(), false));
}

#[test]
fn reverse() {
    let s: Sexp = "(1 2 (3 4) 5)".parse().unwrap();
    let reversed = "(5 (4 3) 2 1)".parse().unwrap();
    println!("{}", s.clone().reverse());
    assert_eq!(s.reverse(), reversed);
}
