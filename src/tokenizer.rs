//! Module for breaking Amlang text into tokens.

use std::collections::VecDeque;
use std::io::BufRead;

use crate::sexp;

#[derive(Debug)]
pub enum Token {
    LeftParen,
    RightParen,
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

        let line = line_result.unwrap().replace("(", " ( ").replace(")", " ) ");

        if let Some(j) = line.find(';') {
            line_to_tokens(&line[..j], i, &mut result);
            result.push_back(TokenInfo {
                token: Token::Comment(line[j + 1..].to_string()),
                line: i + 1,
            });
        } else {
            line_to_tokens(line, i, &mut result);
        }
    }

    Ok(result)
}

fn line_to_tokens<S: AsRef<str>>(line: S, linum: usize, result: &mut Tokens) {
    for ptoken in line.as_ref().split_whitespace() {
        let mut token = match ptoken {
            "(" => Token::LeftParen,
            ")" => Token::RightParen,
            _ => Token::Atom(sexp::Atom::Symbol(ptoken.to_string())),
        };

        // Some additional post-processing for numbers.
        if let Token::Atom(sexp::Atom::Symbol(ref s)) = token {
            if let Ok(i) = s.parse::<i64>() {
                token = Token::Atom(sexp::Atom::Integer(i));
            } else if let Ok(f) = s.parse::<f64>() {
                token = Token::Atom(sexp::Atom::Float(f));
            }
        }
        result.push_back(TokenInfo {
            token,
            line: linum + 1,
        });
    }
}
