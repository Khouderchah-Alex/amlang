//! Module for parsing Amlang tokens into an AST.

use std::iter::Peekable;

use crate::cons_list::ConsList;
use crate::sexp::{Atom, Value};
use crate::token::{Token, TokenInfo};

use self::ParseErrorReason::*;

const MAX_LIST_DEPTH: usize = 128;

#[derive(Debug)]
pub enum ParseErrorReason {
    DepthOverflow,
    TrailingQuote,
    UnmatchedOpen,
    UnmatchedClose,
}

#[derive(Debug)]
pub struct ParseError {
    reason: ParseErrorReason,
    token: TokenInfo,
}

pub fn parse<I: Iterator<Item = TokenInfo>>(tokens: I) -> Result<Vec<Box<Value>>, ParseError> {
    let mut sexps = Vec::<Box<Value>>::new();
    let mut peekable = tokens.peekable();
    while let Some(sexp) = parse_sexp(&mut peekable, 0)? {
        sexps.push(sexp);
    }

    Ok(sexps)
}

/// Returns None when finished parsing, otherwise returns Some(sexp).
fn parse_sexp<I: Iterator<Item = TokenInfo>>(
    tokens: &mut Peekable<I>,
    depth: usize,
) -> Result<Option<Box<Value>>, ParseError> {
    // Let's just ignore comments for now.
    let mut current = tokens.next();
    while let Some(TokenInfo {
        token: Token::Comment(_),
        ..
    }) = current
    {
        current = tokens.next();
    }

    if current.is_none() {
        return Ok(None);
    }

    let token = current.unwrap();
    match token.token {
        Token::LeftParen => {
            if depth >= MAX_LIST_DEPTH {
                return Err(ParseError {
                    reason: DepthOverflow,
                    token,
                });
            }

            let mut list = ConsList::new();
            loop {
                if let Some(TokenInfo {
                    token: Token::RightParen,
                    ..
                }) = tokens.peek()
                {
                    tokens.next();
                    return Ok(Some(list.release()));
                }

                let sexp = parse_sexp(tokens, depth + 1)?;
                if let Some(val) = sexp {
                    unsafe {
                        list.append(val);
                    }
                } else {
                    return Err(ParseError {
                        reason: UnmatchedOpen,
                        token,
                    });
                }
            }
        }
        Token::Quote => {
            let sexp = parse_sexp(tokens, depth + 1)?;
            if let Some(val) = sexp {
                let mut list = ConsList::new();
                unsafe {
                    list.append(Box::new(Value::Atom(Atom::Symbol("quote".to_string()))));
                    list.append(val);
                }
                return Ok(Some(list.release()));
            } else {
                return Err(ParseError {
                    reason: TrailingQuote,
                    token,
                });
            }
        }
        Token::RightParen => {
            return Err(ParseError {
                reason: UnmatchedClose,
                token,
            });
        }
        Token::Atom(atom) => {
            return Ok(Some(Box::new(Value::Atom(atom))));
        }
        Token::Comment(_) => {
            unreachable!();
        }
    }
}
