//! Module for parsing Amlang tokens into an AST.
use std::collections::VecDeque;

use crate::agent::symbol_policies::policy_base;
use crate::continuation::Continuation;
use crate::error::{Error, ErrorKind};
use crate::primitive::{ToLangString, ToSymbol};
use crate::sexp::{ConsList, Sexp};
use crate::stream::Transform;
use crate::token::{Token, TokenKind};

use self::ParseErrorReason::*;
use self::ParserState::*;

const MAX_DEPTH: usize = 128;


pub struct Parser {
    // Control state of ' (as currently the only special instruction
    // for the parser).
    state: Continuation<(ParserState, usize)>, // All but root are in quotes.
    current: Vec<ConsList>,
    max_current_len: usize,

    sexps: VecDeque<Sexp>,
}

#[derive(Debug)]
enum ParserState {
    Base, // Only state possible when depth is 0.
    ImproperTail,
    ImproperClose(Sexp),
}

impl Parser {
    pub fn new() -> Self {
        Self {
            state: Continuation::new((Base, 0)),
            current: Default::default(),
            max_current_len: MAX_DEPTH,
            sexps: Default::default(),
        }
    }

    fn parse_token(&mut self, token: Token) -> Result<(), ParseError> {
        match token.token {
            TokenKind::LeftParen => match self.curr_state() {
                Base => {
                    if self.current.len() >= self.max_current_len {
                        return Err(ParseError {
                            reason: DepthOverflow,
                            token,
                        });
                    }
                    self.current.push(ConsList::new());
                }
                _ => {
                    return Err(ParseError {
                        reason: UnmatchedOpen,
                        token,
                    });
                }
            },
            TokenKind::RightParen => match self.curr_state() {
                Base => {
                    if self.current.len() == 0 {
                        return Err(ParseError {
                            reason: UnmatchedClose,
                            token,
                        });
                    }
                    self.close(None)?;
                }
                ImproperClose(_) => {
                    let mut state = Base;
                    std::mem::swap(&mut self.state.top_mut().0, &mut state);
                    if let ImproperClose(sexp) = state {
                        self.close(Some(sexp))?;
                    }
                }
                _ => {
                    return Err(ParseError {
                        reason: UnmatchedClose,
                        token,
                    });
                }
            },
            TokenKind::Primitive(primitive) => match self.curr_state() {
                Base => {
                    self.append(primitive.into())?;
                }
                ImproperTail => {
                    self.state.top_mut().0 = ImproperClose(primitive.into());
                }
                ImproperClose(_) => {
                    return Err(ParseError {
                        reason: NotPenultimatePeriod,
                        token: Token {
                            token: TokenKind::Primitive(primitive),
                            line: token.line,
                            col: token.col,
                        },
                    });
                }
            },
            TokenKind::Quote => {
                self.current.push(ConsList::new());
                let len = self.current.len();

                self.current[len - 1].append("quote".to_symbol_or_panic(policy_base));
                self.state.push((Base, len));
            }
            TokenKind::Period => match self.curr_state() {
                Base => {
                    if self.current.len() == 0 {
                        return Err(ParseError {
                            reason: IsolatedPeriod,
                            token,
                        });
                    }
                    self.state.top_mut().0 = ImproperTail;
                }
                _ => {
                    return Err(ParseError {
                        reason: NotPenultimatePeriod,
                        token,
                    });
                }
            },
            TokenKind::Comment(_) => {
                // Let's just ignore comments for now.
            }
        }

        Ok(())
    }

    fn append(&mut self, sexp: Sexp) -> Result<(), ParseError> {
        let len = self.current.len();
        if len == 0 {
            self.sexps.push_back(sexp);
            return Ok(());
        } else {
            self.current[len - 1].append(sexp);
        }

        if self.state.top().1 == self.current.len() {
            match self.state.pop() {
                Some((Base, _)) => self.close(None),
                Some((ImproperTail, _)) => {
                    return Err(ParseError {
                        reason: NotPenultimatePeriod,
                        token: Token {
                            token: TokenKind::Period,
                            line: 0,
                            col: 0,
                        },
                    });
                }
                Some((ImproperClose(sexp), _)) => self.close(Some(sexp)),
                state @ _ => panic!("{:?}", state),
            }?
        }
        Ok(())
    }

    fn close(&mut self, tail: Option<Sexp>) -> Result<(), ParseError> {
        let last = self.current.pop().unwrap();
        let sexp = match tail {
            Some(tail) => last.release_with_tail(Some(tail.into())),
            None => last.release(),
        };

        self.append(sexp)
    }

    fn curr_state(&self) -> &ParserState {
        &self.state.top().0
    }
}


impl Transform<Token, Sexp> for Parser {
    fn input(&mut self, input: Result<Token, Error>) -> Result<bool, Error> {
        if let Err(err) = self.parse_token(input?) {
            return Err(Error::no_agent(Box::new(err)));
        }
        Ok(self.sexps.len() > 0)
    }

    fn output(&mut self) -> Option<Result<Sexp, Error>> {
        Some(Ok(self.sexps.pop_front()?))
    }
}


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
