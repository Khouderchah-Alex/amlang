//! Module for parsing Amlang tokens into an AST.

use std::iter::Peekable;

use crate::error::{Error, ErrorKind};
use crate::primitive::symbol::ToSymbol;
use crate::primitive::symbol_policies::policy_base;
use crate::primitive::AmString;
use crate::sexp::cons_list::ConsList;
use crate::sexp::Sexp;
use crate::token::{Token, TokenInfo};

use self::ParseErrorReason::*;


/// Converts stream of TokenInfo into stream of Result<Sexp, Error>.
pub struct ParseIter<S: Iterator<Item = TokenInfo>> {
    stream: Peekable<S>,
}

impl<S: Iterator<Item = TokenInfo>> ParseIter<S> {
    pub fn from_tokens(stream: S) -> Self {
        Self {
            stream: stream.peekable(),
        }
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
    token: TokenInfo,
}

/// Returns None when finished parsing, otherwise returns Some(sexp).
///
/// This returns a ParseError rather than Error so that clients can
/// determine what state, if any, to include; this module is too
/// low-level to make such decisions.
pub fn parse_sexp<I: Iterator<Item = TokenInfo>>(
    tokens: &mut Peekable<I>,
    depth: usize,
) -> Result<Option<Sexp>, ParseError> {
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
                    token: Token::Period,
                    ..
                }) = tokens.peek()
                {
                    tokens.next();
                    let cdr = if let Some(val) = parse_sexp(tokens, depth + 1)? {
                        val
                    } else {
                        return Err(ParseError {
                            reason: UnmatchedOpen,
                            token,
                        });
                    };
                    if let Some(TokenInfo {
                        token: Token::RightParen,
                        ..
                    }) = tokens.next()
                    {
                    } else {
                        return Err(ParseError {
                            reason: NotPenultimatePeriod,
                            token,
                        });
                    }
                    return Ok(Some(list.release_with_tail(cdr)));
                }

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
        Token::Period => {
            return Err(ParseError {
                reason: IsolatedPeriod,
                token,
            });
        }
        Token::RightParen => {
            return Err(ParseError {
                reason: UnmatchedClose,
                token,
            });
        }
        Token::Primitive(primitive) => {
            return Ok(Some(primitive.into()));
        }
        Token::Comment(_) => {
            unreachable!();
        }
    }
}

impl<S: Iterator<Item = TokenInfo>> Iterator for ParseIter<S> {
    type Item = Result<Sexp, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        match parse_sexp(&mut self.stream, 0) {
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
            AmString::new("ParseError"),
            AmString::new(format!("{:?}", self.reason)),
            AmString::new(self.token.to_string()),
        )
    }
}
