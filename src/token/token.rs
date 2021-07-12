use std::fmt;

use crate::primitive::Primitive;

#[derive(Debug, PartialEq)]
pub enum Token {
    LeftParen,
    RightParen,
    Quote,
    Primitive(Primitive),
    Comment(String),
}

#[derive(Debug)]
pub struct TokenInfo {
    pub token: Token,
    pub line: usize,
}

impl fmt::Display for TokenInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.token {
            Token::Primitive(p) => write!(f, "{} @ line {}", p, self.line),
            _ => write!(f, "{:?} @ line {}", self.token, self.line),
        }
    }
}
