//! Module for breaking Amlang text into tokens.

use std::collections::VecDeque;
use std::fmt;

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
pub struct TokenizeError {
    line: usize,
    col: usize,
    kind: ErrorKind,
}

#[derive(Debug)]
enum ErrorKind {
    InvalidSymbol(SymbolError),
}

#[derive(Debug)]
enum TokenizerState {
    Base,
    InString(String, usize),
    InStringEscaped(String, usize),
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
        self.started_quote = false;
    }

    pub fn depth(&self) -> usize {
        let q = self.started_quote || matches!(self.state, TokenizerState::InString(..));
        // Don't return 0 if in quote.
        std::cmp::max(self.depth, q as usize)
    }

    pub fn tokenize<S: AsRef<str>, SymbolInfo, SymbolPolicy>(
        &mut self,
        input: S,
        symbol_policy: &SymbolPolicy,
    ) -> Result<(), TokenizeError>
    where
        SymbolPolicy: Fn(&str) -> Result<SymbolInfo, SymbolError>,
    {
        for line in input.as_ref().split('\n') {
            self.tokenize_line(line, symbol_policy)?;
        }
        Ok(())
    }

    fn tokenize_line<S: AsRef<str>, SymbolInfo, SymbolPolicy>(
        &mut self,
        line: S,
        symbol_policy: &SymbolPolicy,
    ) -> Result<(), TokenizeError>
    where
        SymbolPolicy: Fn(&str) -> Result<SymbolInfo, SymbolError>,
    {
        let mut start: usize = 0;
        let mut empty = true;
        let l = line.as_ref();
        for (i, c) in l.char_indices() {
            match &mut self.state {
                Base => {
                    if c.is_whitespace() {
                        if !empty {
                            self.push_token(&l[start..i], start, symbol_policy)?;
                            empty = true;
                        }
                        continue;
                    } else if c == ';' {
                        if !empty {
                            self.push_token(&l[start..i], start, symbol_policy)?;
                        }
                        self.tokens.push_back(TokenInfo {
                            token: Token::Comment(l[i..].to_string()),
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
                                self.push_token(&l[start..i], start, symbol_policy)?;
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
                                col: i,
                            });
                        }
                        '"' => {
                            if !empty {
                                self.push_token(&l[start..i], start, symbol_policy)?;
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
                            self.tokens.push_back(TokenInfo {
                                token: Token::Primitive(AmString(AmString::new(s))),
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
                    s.push(AmString::unescape_char(c));

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
                    self.push_token(&l[start..], start, symbol_policy)?;
                }
            }
        }

        self.line_count += 1;
        Ok(())
    }


    fn push_token<SymbolInfo, SymbolPolicy>(
        &mut self,
        ptoken: &str,
        start: usize,
        symbol_policy: &SymbolPolicy,
    ) -> Result<(), TokenizeError>
    where
        SymbolPolicy: Fn(&str) -> Result<SymbolInfo, SymbolError>,
    {
        if ptoken == "." {
            self.tokens.push_back(TokenInfo {
                token: Token::Period,
                line: self.line_count,
                col: start,
            });
            return Ok(());
        }

        // Try to parse as number before imposing Symbol constraints.
        let token = if let Ok(num) = ptoken.parse::<Num>() {
            Token::Primitive(Number(num))
        } else {
            match ptoken.to_symbol(symbol_policy) {
                Ok(symbol) => Token::Primitive(Symbol(symbol)),
                Err(err) => {
                    return Err(TokenizeError {
                        line: self.line_count,
                        col: start,
                        kind: ErrorKind::InvalidSymbol(err),
                    });
                }
            }
        };

        self.tokens.push_back(TokenInfo {
            token,
            line: self.line_count,
            col: start,
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


impl fmt::Display for TokenizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[Tokenize Error]: {:?} @ ({}, {})",
            self.kind, self.line, self.col
        )
    }
}


#[cfg(test)]
#[path = "./tokenizer_test.rs"]
mod tokenizer_test;
