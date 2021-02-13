use std::fmt;

use crate::primitive;

#[derive(Debug, PartialEq)]
pub enum Token {
    LeftParen,
    RightParen,
    Quote,
    Primitive(primitive::Primitive),
    Comment(String),
}

#[derive(Debug)]
pub struct TokenInfo {
    pub token: Token,
    pub line: usize,
}

impl fmt::Display for TokenInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} @ line {}", self.token, self.line)
    }
}
