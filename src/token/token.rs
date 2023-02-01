use std::fmt;

use crate::primitive::Primitive;

#[derive(Clone, Debug, PartialEq)]
pub enum TokenKind {
    LeftParen,
    RightParen,
    Quote,
    Period,
    Primitive(Primitive),
    Comment(String),
}

#[derive(Clone, Debug)]
pub struct Token {
    pub token: TokenKind,
    pub line: usize,
    pub col: usize,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.token {
            TokenKind::Primitive(p) => write!(f, "{} @ ({}, {})", p, self.line, self.col),
            _ => write!(f, "{:?} @ ({}, {})", self.token, self.line, self.col),
        }
    }
}
