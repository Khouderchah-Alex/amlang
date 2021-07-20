//! Module for breaking Amlang text into tokens.

use std::collections::VecDeque;

use super::token::{Token, TokenInfo};
use crate::primitive::symbol::{SymbolError, ToSymbol};
use crate::primitive::AmString;
use crate::primitive::Number as Num;
use crate::primitive::Primitive::*;

use self::TokenizerState::*;


#[derive(Debug)]
pub struct Tokenizer {
    tokens: VecDeque<TokenInfo>,
    line_count: usize,
    state: TokenizerState,

    depth: usize,
    started_quote: bool,
}

#[derive(Debug)]
pub enum TokenizeError {
    InvalidSymbol(SymbolError),
}

#[derive(Debug, PartialEq)]
enum TokenizerState {
    Base,
    InString,
}


impl Tokenizer {
    pub fn new() -> Self {
        Self {
            tokens: Default::default(),
            line_count: 0,
            state: TokenizerState::Base,

            depth: 0,
            started_quote: false,
        }
    }

    pub fn clear(&mut self) {
        self.tokens.clear();
        self.depth = 0;
    }

    pub fn depth(&self) -> usize {
        let q = self.started_quote || self.state == TokenizerState::InString;
        // Don't return 0 if in quote.
        std::cmp::max(self.depth, q as usize)
    }

    // TODO(func) Reflect depth from quoting as well.
    pub fn tokenize_line<S: AsRef<str>, SymbolInfo, SymbolPolicy>(
        &mut self,
        line: S,
        symbol_policy: &SymbolPolicy,
    ) -> Result<(), TokenizeError>
    where
        SymbolPolicy: Fn(&str) -> Result<SymbolInfo, SymbolError>,
    {
        // TODO(func) Allow for multi-line strings.
        let mut start: usize = 0;
        let mut empty = true;
        let l = line.as_ref();
        for (i, c) in l.char_indices() {
            match self.state {
                Base => {
                    if c.is_whitespace() {
                        if !empty {
                            self.push_token(&l[start..i], symbol_policy)?;
                            empty = true;
                        }
                        continue;
                    } else if c == ';' {
                        if !empty {
                            self.push_token(&l[start..i], symbol_policy)?;
                        }
                        self.tokens.push_back(TokenInfo {
                            token: Token::Comment(l[i..].to_string()),
                            line: self.line_count,
                        });
                        break;
                    }

                    // Once a quote has been started, any non-whitespace/comment
                    // token will suffice as far as depth calculation goes.
                    self.started_quote = false;

                    match c {
                        '(' | ')' | '\'' => {
                            if !empty {
                                self.push_token(&l[start..i], symbol_policy)?;
                                empty = true;
                            }

                            let token = match c {
                                '(' => {
                                    self.depth += 1;
                                    Token::LeftParen
                                }
                                ')' => {
                                    self.depth = self.depth.saturating_sub(1);
                                    Token::RightParen
                                }
                                '\'' => {
                                    self.started_quote = true;
                                    Token::Quote
                                }
                                _ => panic!(),
                            };
                            self.tokens.push_back(TokenInfo {
                                token,
                                line: self.line_count,
                            });
                        }
                        '"' => {
                            if !empty {
                                self.push_token(&l[start..i], symbol_policy)?;
                            }
                            start = i + 1;
                            self.state = InString;
                        }
                        _ => {
                            if empty {
                                empty = false;
                                start = i;
                            }
                        }
                    }
                }
                InString => {
                    // TODO(func) Allow for escaping.
                    if c == '"' {
                        self.tokens.push_back(TokenInfo {
                            token: Token::Primitive(AmString(AmString::new(
                                line.as_ref()[start..i].to_string(),
                            ))),
                            line: self.line_count,
                        });

                        self.state = Base;
                        empty = true;
                    }
                }
            }
        }

        if !empty {
            self.push_token(&l[start..], symbol_policy)?;
        }
        self.line_count += 1;

        Ok(())
    }


    fn push_token<SymbolInfo, SymbolPolicy>(
        &mut self,
        ptoken: &str,
        symbol_policy: &SymbolPolicy,
    ) -> Result<(), TokenizeError>
    where
        SymbolPolicy: Fn(&str) -> Result<SymbolInfo, SymbolError>,
    {
        // Try to parse as number before imposing Symbol constraints.
        let token = if let Ok(num) = ptoken.parse::<Num>() {
            Token::Primitive(Number(num))
        } else {
            match ptoken.to_symbol(symbol_policy) {
                Ok(symbol) => Token::Primitive(Symbol(symbol)),
                Err(err) => return Err(TokenizeError::InvalidSymbol(err)),
            }
        };

        self.tokens.push_back(TokenInfo {
            token,
            line: self.line_count,
        });
        Ok(())
    }
}

impl Iterator for Tokenizer {
    type Item = TokenInfo;

    fn next(&mut self) -> Option<TokenInfo> {
        self.tokens.pop_front()
    }
}


#[cfg(test)]
#[path = "./tokenizer_test.rs"]
mod tokenizer_test;
