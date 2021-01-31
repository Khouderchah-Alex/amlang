//! Module for breaking Amlang text into tokens.

use std::collections::VecDeque;

use super::token::{Token, TokenInfo};
use crate::atom;
use crate::number;

pub type TokenStore = VecDeque<TokenInfo>;

pub fn tokenize_line<S: AsRef<str>>(line: S, linum: usize, result: &mut TokenStore) {
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
        let mut token = match ptoken {
            "(" => Token::LeftParen,
            ")" => Token::RightParen,
            "'" => Token::Quote,
            _ => Token::Atom(atom::Atom::Symbol(ptoken.to_string())),
        };

        // Some additional post-processing for numbers.
        if let Token::Atom(atom::Atom::Symbol(ref s)) = token {
            if let Ok(num) = s.parse::<number::Number>() {
                token = Token::Atom(atom::Atom::Number(num));
            }
        }
        result.push_back(TokenInfo {
            token,
            line: linum + 1,
        });
    }

    if let Some(comment) = comment {
        result.push_back(comment);
    }
}

#[cfg(test)]
#[path = "./tokenize_test.rs"]
mod tokenize_test;
