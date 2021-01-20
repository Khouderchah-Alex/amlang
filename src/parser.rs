//! Module for parsing Amlang tokens into an AST.

use crate::cons_list::ConsList;
use crate::sexp::{Atom, Value};
use crate::tokenizer::{Token, TokenInfo, Tokens};

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

pub fn parse(mut tokens: Tokens) -> Result<Vec<Box<Value>>, ParseError> {
    let mut sexps = Vec::<Box<Value>>::new();
    while let Some(sexp) = parse_sexp(&mut tokens, 0)? {
        sexps.push(sexp);
    }

    Ok(sexps)
}

/// Returns None when finished parsing, otherwise returns Some(sexp).
fn parse_sexp(tokens: &mut Tokens, depth: usize) -> Result<Option<Box<Value>>, ParseError> {
    // Let's just ignore comments for now.
    let mut current = tokens.pop_front();
    while let Some(TokenInfo {
        token: Token::Comment(_),
        ..
    }) = current
    {
        current = tokens.pop_front();
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
                }) = tokens.get(0)
                {
                    tokens.pop_front();
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
