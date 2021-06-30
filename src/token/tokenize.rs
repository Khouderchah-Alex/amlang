//! Module for breaking Amlang text into tokens.

use std::collections::VecDeque;

use super::token::{Token, TokenInfo};
use crate::primitive::symbol::{SymbolError, ToSymbol};
use crate::primitive::Number as Num;
use crate::primitive::Primitive::*;


pub type TokenStore = VecDeque<TokenInfo>;

#[derive(Debug)]
pub enum TokenizeError {
    InvalidSymbol(SymbolError),
}

/// Returns depth change on success (positive means deeper, negative
/// means shallower). Tokens returned through result out-param.
// TODO(func) Reflect depth from quoting as well.
pub fn tokenize_line<S: AsRef<str>>(
    line: S,
    linum: usize,
    result: &mut TokenStore,
) -> Result<i16, TokenizeError> {
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

    let mut depth: i16 = 0;
    for ptoken in expanded.split_whitespace() {
        let token = match ptoken {
            "(" => {
                depth += 1;
                Token::LeftParen
            }
            ")" => {
                depth -= 1;
                Token::RightParen
            }
            "'" => Token::Quote,
            _ => {
                // Try to parse as number before imposing Symbol constraints.
                if let Ok(num) = ptoken.parse::<Num>() {
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
    Ok(depth)
}


#[cfg(test)]
#[path = "./tokenize_test.rs"]
mod tokenize_test;
