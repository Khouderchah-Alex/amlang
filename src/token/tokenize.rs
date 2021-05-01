//! Module for breaking Amlang text into tokens.

use std::collections::VecDeque;

use super::token::{Token, TokenInfo};
use crate::number;
use crate::primitive::Primitive::*;
use crate::symbol::{SymbolError, ToSymbol};


pub type TokenStore = VecDeque<TokenInfo>;

#[derive(Debug)]
pub enum TokenizeError {
    InvalidSymbol(SymbolError),
}

pub fn tokenize_line<S: AsRef<str>>(
    line: S,
    linum: usize,
    result: &mut TokenStore,
) -> Result<(), TokenizeError> {
    let mut sexp_slice = line.as_ref();

    let mut comment: Option<TokenInfo> = None;
    if let Some(j) = sexp_slice.find(';') {
        comment = Some(TokenInfo {
            token: Token::Comment(sexp_slice[j + 1..].to_string()),
            line: linum,
        });
        sexp_slice = &sexp_slice[..j];
    }

    let expanded = sexp_slice
        .replace("(", " ( ")
        .replace(")", " ) ")
        .replace("'", " ' ");

    for ptoken in expanded.split_whitespace() {
        let token = match ptoken {
            "(" => Token::LeftParen,
            ")" => Token::RightParen,
            "'" => Token::Quote,
            _ => {
                // Try to parse as number before imposing Symbol constraints.
                if let Ok(num) = ptoken.parse::<number::Number>() {
                    Token::Primitive(Number(num))
                } else {
                    match ptoken.to_symbol() {
                        Ok(symbol) => Token::Primitive(Symbol(symbol)),
                        Err(err) => return Err(TokenizeError::InvalidSymbol(err)),
                    }
                }
            }
        };

        result.push_back(TokenInfo {
            token,
            line: linum + 1,
        });
    }

    if let Some(comment) = comment {
        result.push_back(comment);
    }
    Ok(())
}


#[cfg(test)]
#[path = "./tokenize_test.rs"]
mod tokenize_test;
