use std::io::Cursor;

use super::*;
use Token::*;

fn nest(mut v: Vec<Token>) -> Vec<Token> {
    v.insert(0, LeftParen);
    v.push(RightParen);
    v
}

#[test]
fn nested() -> Result<(), TokenError> {
    let input = "(testing (this (out)))";
    let mut expected = nest(vec![Atom(sexp::Atom::Symbol("out".to_string()))]);
    expected.insert(0, Atom(sexp::Atom::Symbol("this".to_string())));
    expected = nest(expected);
    expected.insert(0, Atom(sexp::Atom::Symbol("testing".to_string())));
    expected = nest(expected);

    let tokens = tokenize(Cursor::new(input))?;
    for (i, elem) in tokens.iter().enumerate() {
        assert_eq!(elem.token, expected[i]);
    }
    Ok(())
}

#[test]
fn ints() -> Result<(), TokenError> {
    let input = "(1 2 -4 33 128)";
    let mut expected: Vec<Token> = vec![1, 2, -4, 33, 128]
        .iter_mut()
        .map(|elem| Atom(sexp::Atom::Integer(*elem)))
        .collect();
    expected = nest(expected);

    let tokens = tokenize(Cursor::new(input))?;
    for (i, elem) in tokens.iter().enumerate() {
        assert_eq!(elem.token, expected[i]);
    }
    Ok(())
}

#[test]
fn floats() -> Result<(), TokenError> {
    let input = "(1. 2.2 -4.5 33. 128.128)";
    let mut expected: Vec<Token> = vec![1., 2.2, -4.5, 33., 128.128]
        .iter_mut()
        .map(|elem| Atom(sexp::Atom::Float(*elem)))
        .collect();
    expected = nest(expected);

    let tokens = tokenize(Cursor::new(input))?;
    for (i, elem) in tokens.iter().enumerate() {
        assert_eq!(elem.token, expected[i]);
    }
    Ok(())
}
