//! Module for breaking Amlang text into tokens.

use std::collections::VecDeque;
use std::io::BufRead;

#[derive(Debug)]
pub enum Token {
    LeftParen,
    RightParen,
    Symbol(String),
    Integer(i64),
    Float(f64),
    Comment(String),
}

pub type Tokens = VecDeque<Token>;

#[derive(Debug)]
pub struct TokenError {
    current_tokens: Tokens,
}

pub fn tokenize<T: BufRead>(source: T) -> Result<Tokens, TokenError> {
    let mut result = VecDeque::new();
    for line_result in source.lines() {
        if let Err(_err) = line_result {
            return Err(TokenError{current_tokens: result});
        }

        let line = line_result.unwrap()
            .replace("(", " ( ")
            .replace(")",  " ) ");
        if let Some(i) = line.find(';') {
            line_to_tokens(&line[..i], &mut result);
            result.push_back(Token::Comment(line[i+1..].to_string()));
        } else {
            line_to_tokens(line, &mut result);
        }
    }

    Ok(result)
}

fn line_to_tokens<S: AsRef<str>>(line: S, result: &mut Tokens) {
    for ptoken in line.as_ref().split_whitespace() {
        let mut token = match ptoken {
            "(" => Token::LeftParen,
            ")" => Token::RightParen,
            _ => Token::Symbol(ptoken.to_string()),
        };

        // Some additional post-processing for numbers.
        if let Token::Symbol(ref s) = token {
            if let Ok(i) = s.parse::<i64>() {
                token = Token::Integer(i);
            } else if let Ok(f) = s.parse::<f64>() {
                token = Token::Float(f);
            }
        }
        result.push_back(token);
    }
}
