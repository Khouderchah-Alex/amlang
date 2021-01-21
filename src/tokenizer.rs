//! Module for breaking Amlang text into tokens.

use std::collections::VecDeque;
use std::io::BufRead;

use crate::number;
use crate::sexp;

#[derive(Debug, PartialEq)]
pub enum Token {
    LeftParen,
    RightParen,
    Quote,
    Atom(sexp::Atom),
    Comment(String),
}

#[derive(Debug)]
pub struct TokenInfo {
    pub token: Token,
    pub line: usize,
}

pub type Tokens = VecDeque<TokenInfo>;

#[derive(Debug)]
pub struct TokenError {
    current_tokens: Tokens,
}

pub fn tokenize<T: BufRead>(source: T) -> Result<Tokens, TokenError> {
    let mut result = VecDeque::new();
    let mut iter = source.lines().enumerate();
    while let Some((i, line_result)) = iter.next() {
        if let Err(_err) = line_result {
            return Err(TokenError {
                current_tokens: result,
            });
        }

        let line = line_result.unwrap();
        tokenize_line(&line, i, &mut result);
    }

    Ok(result)
}

fn tokenize_line<S: AsRef<str>>(line: S, linum: usize, result: &mut Tokens) {
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
            _ => Token::Atom(sexp::Atom::Symbol(ptoken.to_string())),
        };

        // Some additional post-processing for numbers.
        if let Token::Atom(sexp::Atom::Symbol(ref s)) = token {
            if let Ok(num) = s.parse::<number::Number>() {
                token = Token::Atom(sexp::Atom::Number(num));
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
#[path = "./tokenizer_test.rs"]
mod tokenizer_test;
