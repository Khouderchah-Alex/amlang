//! Module for breaking Amlang text into tokens.

use std::collections::VecDeque;

use super::token::{Token, TokenInfo};
use crate::primitive::symbol::{SymbolError, ToSymbol};
use crate::primitive::Number as Num;
use crate::primitive::Primitive::*;


pub struct Tokenizer {
    tokens: VecDeque<TokenInfo>,
    line_count: usize,
    depth: usize,
}

#[derive(Debug)]
pub enum TokenizeError {
    InvalidSymbol(SymbolError),
}

impl Tokenizer {
    pub fn new() -> Self {
        Self {
            tokens: Default::default(),
            line_count: 0,
            depth: 0,
        }
    }

    pub fn clear(&mut self) {
        self.tokens.clear();
        self.depth = 0;
    }

    pub fn depth(&self) -> usize {
        self.depth
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
        let mut sexp_slice = line.as_ref();

        let mut comment: Option<TokenInfo> = None;
        if let Some(j) = sexp_slice.find(';') {
            comment = Some(TokenInfo {
                token: Token::Comment(sexp_slice[j + 1..].to_string()),
                line: self.line_count,
            });
            sexp_slice = &sexp_slice[..j];
        }

        let expanded = sexp_slice
            .replace("(", " ( ")
            .replace(")", " ) ")
            .replace("'", " ' ");

        for ptoken in expanded.split_whitespace() {
            let token = match ptoken {
                "(" => {
                    self.depth += 1;
                    Token::LeftParen
                }
                ")" => {
                    self.depth = self.depth.saturating_sub(1);
                    Token::RightParen
                }
                "'" => Token::Quote,
                _ => {
                    // Try to parse as number before imposing Symbol constraints.
                    if let Ok(num) = ptoken.parse::<Num>() {
                        Token::Primitive(Number(num))
                    } else {
                        match ptoken.to_symbol(symbol_policy) {
                            Ok(symbol) => Token::Primitive(Symbol(symbol)),
                            Err(err) => return Err(TokenizeError::InvalidSymbol(err)),
                        }
                    }
                }
            };

            self.tokens.push_back(TokenInfo {
                token,
                line: self.line_count + 1,
            });
        }

        if let Some(comment) = comment {
            self.tokens.push_back(comment);
        }

        self.line_count += 1;
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
