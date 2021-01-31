//! Module for parsing Amlang tokens into an AST.

use std::fmt;
use std::iter::Peekable;

use crate::atom::Atom;
use crate::cons_list::ConsList;
use crate::sexp::Sexp;
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

pub fn parse<I: Iterator<Item = TokenInfo>>(tokens: I) -> Result<Vec<Box<Sexp>>, ParseError> {
    let mut sexps = Vec::<Box<Sexp>>::new();
    let mut peekable = tokens.peekable();
    while let Some(sexp) = parse_sexp(&mut peekable, 0)? {
        sexps.push(sexp);
    }

    Ok(sexps)
}

/// Returns None when finished parsing, otherwise returns Some(sexp).
pub fn parse_sexp<I: Iterator<Item = TokenInfo>>(
    tokens: &mut Peekable<I>,
    depth: usize,
) -> Result<Option<Box<Sexp>>, ParseError> {
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
                    list.append(val);
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
                list.append(Box::new(Sexp::Atom(Atom::Symbol("quote".to_string()))));
                list.append(val);
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
            return Ok(Some(Box::new(Sexp::Atom(atom))));
        }
        Token::Comment(_) => {
            unreachable!();
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[Parse Error] {:?}: {}", self.reason, self.token)
    }
}
