use super::*;

use crate::agent::symbol_policies::policy_base;
use crate::primitive::{Number, ToSymbol};
use crate::token::string_stream;
use Token::*;

fn nest(mut v: Vec<Token>) -> Vec<Token> {
    v.insert(0, LeftParen);
    v.push(RightParen);
    v
}

#[test]
fn nested() {
    let input = "(testing (this (out)))";
    let mut expected = nest(vec![Primitive(Symbol(
        "out".to_symbol_or_panic(policy_base),
    ))]);
    expected.insert(0, Primitive(Symbol("this".to_symbol_or_panic(policy_base))));
    expected = nest(expected);
    expected.insert(
        0,
        Primitive(Symbol("testing".to_symbol_or_panic(policy_base))),
    );
    expected = nest(expected);

    let tokens = string_stream(input, policy_base).unwrap();
    for (i, elem) in tokens.enumerate() {
        assert_eq!(elem.token, expected[i]);
    }
}

#[test]
fn newlines() {
    let input = "\n(testing\n\n (\nthis (out))\n)";
    let mut expected = nest(vec![Primitive(Symbol(
        "out".to_symbol_or_panic(policy_base),
    ))]);
    expected.insert(0, Primitive(Symbol("this".to_symbol_or_panic(policy_base))));
    expected = nest(expected);
    expected.insert(
        0,
        Primitive(Symbol("testing".to_symbol_or_panic(policy_base))),
    );
    expected = nest(expected);

    let tokens = string_stream(input, policy_base).unwrap();
    for (i, elem) in tokens.enumerate() {
        assert_eq!(elem.token, expected[i]);
    }
}

#[test]
fn ints() {
    let input = "(1 2 -4 33 128)";
    let mut expected: Vec<Token> = vec![1, 2, -4, 33, 128]
        .iter_mut()
        .map(|elem| Primitive(Number(Number::I64(*elem))))
        .collect();
    expected = nest(expected);

    let tokens = string_stream(input, policy_base).unwrap();
    for (i, elem) in tokens.enumerate() {
        assert_eq!(elem.token, expected[i]);
    }
}

#[test]
fn floats() {
    let input = "(1. 2.2 -4.5 33. 128.128 .2)";
    let mut expected: Vec<Token> = vec![1., 2.2, -4.5, 33., 128.128, 0.2]
        .iter_mut()
        .map(|elem| Primitive(Number(Number::F64(*elem))))
        .collect();
    expected = nest(expected);

    let tokens = string_stream(input, policy_base).unwrap();
    for (i, elem) in tokens.enumerate() {
        assert_eq!(elem.token, expected[i]);
    }
}

#[test]
fn strings() {
    let input = "(\"test.'(est)\" \n\"hello\")";
    let mut expected: Vec<Token> = vec!["test.'(est)", "hello"]
        .iter_mut()
        .map(|elem| Primitive(LangString(LangString::new(*elem))))
        .collect();
    expected = nest(expected);

    let tokens = string_stream(input, policy_base).unwrap();
    for (i, elem) in tokens.enumerate() {
        assert_eq!(elem.token, expected[i]);
    }
}
