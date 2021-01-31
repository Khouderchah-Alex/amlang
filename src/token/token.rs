use std::fmt;

use crate::atom;

#[derive(Debug, PartialEq)]
pub enum Token {
    LeftParen,
    RightParen,
    Quote,
    Atom(atom::Atom),
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
