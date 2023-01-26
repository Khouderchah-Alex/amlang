//! Module for breaking Amlang text into tokens.

use std::collections::VecDeque;

use super::token::{Token, TokenKind};
use crate::agent::symbol_policies::SymbolPolicy;
use crate::error::{Error, ErrorKind};
use crate::primitive::symbol::{SymbolError, ToSymbol};
use crate::primitive::Number as Num;
use crate::primitive::Primitive::*;
use crate::primitive::{LangString, ToLangString};
use crate::sexp::Sexp;
use crate::stream::Transform;

use self::TokenizerState::*;


/// Essentially a Mealy machine that outputs and accumulates Tokens
/// given string-like input.
pub struct Tokenizer<SymbolInfo> {
    // Mealy machine state.
    state: TokenizerState,
    depth: usize,
    started_quote: bool,

    // Non-control state.
    symbol_policy: SymbolPolicy<SymbolInfo>,

    line_count: usize,
    tokens: VecDeque<Token>,
}

#[derive(Debug)]
enum TokenizerState {
    Base,
    // (String accumulated from prev lines, col of first line).
    InString(String, usize),
    // (String accumulated from prev lines, col of first line).
    InStringEscaped(String, usize),
}


#[derive(Debug)]
pub struct TokenizeError {
    line: usize,
    col: usize,
    kind: TokenizeErrorKind,
}

#[derive(Debug)]
enum TokenizeErrorKind {
    InvalidSymbol(SymbolError),
}


impl<SymbolInfo> Tokenizer<SymbolInfo> {
    pub fn new(symbol_policy: SymbolPolicy<SymbolInfo>) -> Self {
        Self {
            state: TokenizerState::Base,
            depth: 0,
            started_quote: false,

            symbol_policy,
            line_count: 0,
            tokens: Default::default(),
        }
    }

    pub fn clear(&mut self) {
        self.depth = 0;
        self.started_quote = false;

        self.tokens.clear();
    }

    pub fn depth(&self) -> usize {
        let q = self.started_quote || matches!(self.state, TokenizerState::InString(..));
        // Don't return 0 if in quote.
        std::cmp::max(self.depth, q as usize)
    }

    pub fn tokenize<S: AsRef<str>>(&mut self, input: S) -> Result<(), TokenizeError> {
        for line in input.as_ref().split('\n') {
            self.tokenize_line(line)?;
        }
        Ok(())
    }

    fn tokenize_line<S: AsRef<str>>(&mut self, line: S) -> Result<(), TokenizeError> {
        let mut start: usize = 0;
        let mut empty = true;
        let l = line.as_ref();
        for (i, c) in l.char_indices() {
            match &mut self.state {
                Base => {
                    if c.is_whitespace() {
                        if !empty {
                            self.push_token(&l[start..i], start)?;
                            empty = true;
                        }
                        continue;
                    } else if c == ';' {
                        if !empty {
                            self.push_token(&l[start..i], start)?;
                        }
                        self.tokens.push_back(Token {
                            token: TokenKind::Comment(l[i..].to_string()),
                            line: self.line_count,
                            col: i,
                        });
                        break;
                    }

                    // Once a quote has been started, any non-whitespace/comment
                    // token will suffice as far as depth calculation goes.
                    self.started_quote = false;

                    match c {
                        '(' | ')' | '\'' => {
                            if !empty {
                                self.push_token(&l[start..i], start)?;
                                empty = true;
                            }

                            let token = match c {
                                '(' => {
                                    self.depth += 1;
                                    TokenKind::LeftParen
                                }
                                ')' => {
                                    self.depth = self.depth.saturating_sub(1);
                                    TokenKind::RightParen
                                }
                                '\'' => {
                                    self.started_quote = true;
                                    TokenKind::Quote
                                }
                                _ => panic!(),
                            };
                            self.tokens.push_back(Token {
                                token,
                                line: self.line_count,
                                col: i,
                            });
                        }
                        '"' => {
                            if !empty {
                                self.push_token(&l[start..i], start)?;
                            }
                            start = i + 1;
                            self.state = InString(String::default(), start);
                        }
                        _ => {
                            if empty {
                                empty = false;
                                start = i;
                            }
                        }
                    }
                }
                InString(s, col) => {
                    if empty {
                        empty = false;
                        start = i;
                    }
                    match c {
                        '\\' => {
                            s.push_str(&line.as_ref()[start..i]);
                            let curr_str = std::mem::replace(s, String::default());
                            self.state = InStringEscaped(curr_str, *col);
                        }
                        '"' => {
                            s.push_str(&line.as_ref()[start..i]);
                            self.tokens.push_back(Token {
                                token: TokenKind::Primitive(LangString(LangString::new(s))),
                                line: self.line_count,
                                col: *col,
                            });

                            self.state = Base;
                            empty = true;
                        }
                        _ => {}
                    }
                }
                InStringEscaped(s, col) => {
                    // TODO(func) allow for decoding of unicode.
                    s.push(LangString::unescape_char(c));

                    empty = true;
                    let curr_str = std::mem::replace(s, String::default());
                    self.state = InString(curr_str, *col);
                }
            }
        }

        // EOL handling.
        match &mut self.state {
            InString(s, ..) => {
                s.push_str(&line.as_ref()[start..]);
                s.push('\n');
            }
            InStringEscaped(s, col) => {
                // \ followed by EOL simply means ignore the newline.
                let curr_str = std::mem::replace(s, String::default());
                self.state = InString(curr_str, *col);
            }
            _ => {
                if !empty {
                    self.push_token(&l[start..], start)?;
                }
            }
        }

        self.line_count += 1;
        Ok(())
    }


    fn push_token(&mut self, ptoken: &str, start: usize) -> Result<(), TokenizeError> {
        if ptoken == "." {
            self.tokens.push_back(Token {
                token: TokenKind::Period,
                line: self.line_count,
                col: start,
            });
            return Ok(());
        }

        // Try to parse as number before imposing Symbol constraints.
        let token = if let Ok(num) = ptoken.parse::<Num>() {
            TokenKind::Primitive(Number(num))
        } else {
            match ptoken.to_symbol(self.symbol_policy) {
                Ok(symbol) => TokenKind::Primitive(Symbol(symbol)),
                Err(err) => {
                    return Err(TokenizeError {
                        line: self.line_count,
                        col: start,
                        kind: TokenizeErrorKind::InvalidSymbol(err),
                    });
                }
            }
        };

        self.tokens.push_back(Token {
            token,
            line: self.line_count,
            col: start,
        });
        Ok(())
    }
}


impl<S: AsRef<str>, SymbolInfo> Transform<S, Token> for Tokenizer<SymbolInfo> {
    fn input(&mut self, input: S) -> Result<bool, Error> {
        if let Err(error) = self.tokenize(input) {
            return Err(Error::no_agent(Box::new(error)));
        }
        Ok(self.tokens.len() > 0)
    }

    fn output(&mut self) -> Option<Token> {
        self.tokens.pop_front()
    }
}

impl ErrorKind for TokenizeError {
    // TODO(func) Model within env rather than fall back on strings.
    fn reify(&self) -> Sexp {
        list!(
            "TokenizeError".to_lang_string(),
            format!(
                "[Tokenize Error]: {:?} @ ({}, {})",
                self.kind, self.line, self.col
            )
            .to_lang_string(),
        )
    }
}


#[cfg(test)]
#[path = "./tokenizer_test.rs"]
mod tokenizer_test;
