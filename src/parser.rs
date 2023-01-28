//! Module for parsing Amlang tokens into an AST.

use std::iter::Peekable;

use crate::agent::symbol_policies::policy_base;
use crate::error::{Error, ErrorKind};
use crate::primitive::{ToLangString, ToSymbol};
use crate::sexp::{ConsList, Sexp};
use crate::token::{Token, TokenKind};

use self::ParseErrorReason::*;


/// Converts stream of Token into stream of Result<Sexp, Error>.
pub struct ParseIter<'a, S: Iterator<Item = Token>> {
    stream: &'a mut Peekable<S>,
}

impl<'a, S: Iterator<Item = Token>> ParseIter<'a, S> {
    pub fn from_peekable(peekable: &'a mut Peekable<S>) -> Self {
        Self { stream: peekable }
    }
}

const MAX_LIST_DEPTH: usize = 128;

#[derive(Debug)]
pub enum ParseErrorReason {
    DepthOverflow,
    TrailingQuote,
    UnmatchedOpen,
    UnmatchedClose,
    IsolatedPeriod,
    NotPenultimatePeriod,
}

#[derive(Debug)]
pub struct ParseError {
    reason: ParseErrorReason,
    token: Token,
}

/// Returns None when finished parsing, otherwise returns Some(sexp).
///
/// This returns a ParseError rather than Error so that clients can
/// determine what state, if any, to include; this module is too
/// low-level to make such decisions.
pub fn parse_sexp<I: Iterator<Item = Token>>(
    tokens: &mut Peekable<I>,
) -> Result<Option<Sexp>, ParseError> {
    parse_sexp_internal(tokens, 0)
}

fn parse_sexp_internal<I: Iterator<Item = Token>>(
    tokens: &mut Peekable<I>,
    depth: usize,
) -> Result<Option<Sexp>, ParseError> {
    // Let's just ignore comments for now.
    let mut current = tokens.next();
    while let Some(Token {
        token: TokenKind::Comment(_),
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
        TokenKind::LeftParen => {
            if depth >= MAX_LIST_DEPTH {
                return Err(ParseError {
                    reason: DepthOverflow,
                    token,
                });
            }

            let mut list = ConsList::new();
            loop {
                if let Some(Token {
                    token: TokenKind::Period,
                    ..
                }) = tokens.peek()
                {
                    tokens.next();
                    let cdr = if let Some(val) = parse_sexp_internal(tokens, depth + 1)? {
                        val
                    } else {
                        return Err(ParseError {
                            reason: UnmatchedOpen,
                            token,
                        });
                    };
                    if let Some(Token {
                        token: TokenKind::RightParen,
                        ..
                    }) = tokens.next()
                    {
                    } else {
                        return Err(ParseError {
                            reason: NotPenultimatePeriod,
                            token,
                        });
                    }
                    return Ok(Some(list.release_with_tail(Some(cdr.into()))));
                }

                if let Some(Token {
                    token: TokenKind::RightParen,
                    ..
                }) = tokens.peek()
                {
                    tokens.next();
                    return Ok(Some(list.release()));
                }

                let sexp = parse_sexp_internal(tokens, depth + 1)?;
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
        TokenKind::Quote => {
            let sexp = parse_sexp_internal(tokens, depth + 1)?;
            if let Some(val) = sexp {
                let mut list = ConsList::new();

                list.append("quote".to_symbol_or_panic(policy_base));
                list.append(val);
                return Ok(Some(list.release()));
            } else {
                return Err(ParseError {
                    reason: TrailingQuote,
                    token,
                });
            }
        }
        TokenKind::Period => {
            return Err(ParseError {
                reason: IsolatedPeriod,
                token,
            });
        }
        TokenKind::RightParen => {
            return Err(ParseError {
                reason: UnmatchedClose,
                token,
            });
        }
        TokenKind::Primitive(primitive) => {
            return Ok(Some(primitive.into()));
        }
        TokenKind::Comment(_) => {
            unreachable!();
        }
    }
}

impl<'a, S: Iterator<Item = Token>> Iterator for ParseIter<'a, S> {
    type Item = Result<Sexp, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        match parse_sexp(&mut self.stream) {
            Ok(Some(parsed)) => Some(Ok(parsed)),
            Ok(None) => None,
            Err(err) => Some(Err(Error::no_agent(Box::new(err)))),
        }
    }
}

impl ErrorKind for ParseError {
    // TODO(func) Model within env rather than fall back on strings.
    fn reify(&self) -> Sexp {
        list!(
            "ParseError".to_lang_string(),
            format!("{:?}", self.reason).to_lang_string(),
            self.token.to_lang_string(),
        )
    }
}
