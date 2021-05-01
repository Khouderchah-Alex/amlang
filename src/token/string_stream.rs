use super::token::TokenInfo;
use super::tokenize::{tokenize_line, TokenStore, TokenizeError};


pub struct StringStream {
    tokens: TokenStore,
}

impl StringStream {
    pub fn new<S: AsRef<str>>(line: S) -> Result<StringStream, TokenizeError> {
        let mut tokens = TokenStore::default();
        if let Err(err) = tokenize_line(&line, 0, &mut tokens) {
            return Err(err);
        }

        Ok(StringStream { tokens })
    }
}


impl Iterator for StringStream {
    type Item = TokenInfo;

    fn next(&mut self) -> Option<TokenInfo> {
        return self.tokens.pop_front();
    }
}
