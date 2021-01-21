use super::token::TokenInfo;
use super::tokenize::{tokenize_line, TokenStore};

pub struct StringStream {
    tokens: TokenStore,
}

impl StringStream {
    pub fn new<S: AsRef<str>>(line: S) -> StringStream {
        let mut tokens = TokenStore::default();
        tokenize_line(&line, 0, &mut tokens);
        StringStream { tokens }
    }
}

impl Iterator for StringStream {
    type Item = TokenInfo;

    fn next(&mut self) -> Option<TokenInfo> {
        return self.tokens.pop_front();
    }
}
