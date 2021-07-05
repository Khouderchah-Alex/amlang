use super::token::TokenInfo;
use super::tokenize::{tokenize_line, TokenStore, TokenizeError};
use crate::primitive::symbol::SymbolError;


pub struct StringStream {
    tokens: TokenStore,
}

impl StringStream {
    pub fn new<S: AsRef<str>, SymbolInfo, SymbolPolicy>(
        line: S,
        symbol_policy: SymbolPolicy,
    ) -> Result<StringStream, TokenizeError>
    where
        SymbolPolicy: Fn(&str) -> Result<SymbolInfo, SymbolError>,
    {
        let mut tokens = TokenStore::default();
        if let Err(err) = tokenize_line(&line, 0, &symbol_policy, &mut tokens) {
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
