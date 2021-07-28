//! Module for parsing Amlang tokens into an AST.

use std::fmt;
use std::iter::Peekable;

use crate::primitive::symbol::ToSymbol;
use crate::primitive::symbol_policies::policy_base;
use crate::sexp::cons_list::ConsList;
use crate::sexp::HeapSexp;
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

/// Returns None when finished parsing, otherwise returns Some(sexp).
pub fn parse_sexp<I: Iterator<Item = TokenInfo>>(
    tokens: &mut Peekable<I>,
    depth: usize,
) -> Result<Option<HeapSexp>, ParseError> {
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

                list.append(HeapSexp::new(
                    "quote".to_symbol_or_panic(policy_base).into(),
                ));
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
        Token::Primitive(primitive) => {
            return Ok(Some(HeapSexp::new(primitive.into())));
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
