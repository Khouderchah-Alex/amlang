use super::*;

use crate::number::Number;
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
    let mut expected = nest(vec![Primitive(Symbol("out".to_string()))]);
    expected.insert(0, Primitive(Symbol("this".to_string())));
    expected = nest(expected);
    expected.insert(0, Primitive(Symbol("testing".to_string())));
    expected = nest(expected);

    let tokens = StringStream::new(input);
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

    let tokens = StringStream::new(input);
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

    let tokens = StringStream::new(input);
    for (i, elem) in tokens.enumerate() {
        assert_eq!(elem.token, expected[i]);
    }
}
