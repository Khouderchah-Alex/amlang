use super::*;

use crate::number::Number;
use crate::symbol::ToSymbol;
use crate::token::string_stream::StringStream;
use Token::*;

fn nest(mut v: Vec<Token>) -> Vec<Token> {
    v.insert(0, LeftParen);
    v.push(RightParen);
    v
}

#[test]
fn nested() {
    let input = "(testing (this (out)))";
    let mut expected = nest(vec![Primitive(Symbol("out".to_symbol_or_panic()))]);
    expected.insert(0, Primitive(Symbol("this".to_symbol_or_panic())));
    expected = nest(expected);
    expected.insert(0, Primitive(Symbol("testing".to_symbol_or_panic())));
    expected = nest(expected);

    let tokens = StringStream::new(input).unwrap();
    for (i, elem) in tokens.enumerate() {
        assert_eq!(elem.token, expected[i]);
    }
}

#[test]
fn newlines() {
    let input = "\n(testing\n\n (\nthis (out))\n)";
    let mut expected = nest(vec![Primitive(Symbol("out".to_symbol_or_panic()))]);
    expected.insert(0, Primitive(Symbol("this".to_symbol_or_panic())));
    expected = nest(expected);
    expected.insert(0, Primitive(Symbol("testing".to_symbol_or_panic())));
    expected = nest(expected);

    let tokens = StringStream::new(input).unwrap();
    for (i, elem) in tokens.enumerate() {
        assert_eq!(elem.token, expected[i]);
    }
}

#[test]
fn ints() {
    let input = "(1 2 -4 33 128)";
    let mut expected: Vec<Token> = vec![1, 2, -4, 33, 128]
        .iter_mut()
        .map(|elem| Primitive(Number(Number::Integer(*elem))))
        .collect();
    expected = nest(expected);

    let tokens = StringStream::new(input).unwrap();
    for (i, elem) in tokens.enumerate() {
        assert_eq!(elem.token, expected[i]);
    }
}

#[test]
fn floats() {
    let input = "(1. 2.2 -4.5 33. 128.128)";
    let mut expected: Vec<Token> = vec![1., 2.2, -4.5, 33., 128.128]
        .iter_mut()
        .map(|elem| Primitive(Number(Number::Float(*elem))))
        .collect();
    expected = nest(expected);

    let tokens = StringStream::new(input).unwrap();
    for (i, elem) in tokens.enumerate() {
        assert_eq!(elem.token, expected[i]);
    }
}
