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
