use std::fmt;

use crate::primitive::Primitive;

#[derive(Debug, PartialEq)]
pub enum Token {
    LeftParen,
    RightParen,
    Quote,
    Period,
    Primitive(Primitive),
    Comment(String),
}

#[derive(Debug)]
pub struct TokenInfo {
    pub token: Token,
    pub line: usize,
    pub col: usize,
}

impl fmt::Display for TokenInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.token {
            Token::Primitive(p) => write!(f, "{} @ ({}, {})", p, self.line, self.col),
            _ => write!(f, "{:?} @ ({}, {})", self.token, self.line, self.col),
        }
    }
}
