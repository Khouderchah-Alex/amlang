use super::token::TokenInfo;
use super::tokenizer::{TokenizeError, Tokenizer};
use crate::primitive::symbol::SymbolError;


pub struct StringStream {
    tokenizer: Tokenizer,
}

impl StringStream {
    pub fn new<S: AsRef<str>, SymbolInfo, SymbolPolicy>(
        line: S,
        symbol_policy: SymbolPolicy,
    ) -> Result<StringStream, TokenizeError>
    where
        SymbolPolicy: Fn(&str) -> Result<SymbolInfo, SymbolError>,
    {
        let mut tokenizer = Tokenizer::new();
        if let Err(err) = tokenizer.tokenize(&line, &symbol_policy) {
            return Err(err);
        }

        Ok(StringStream { tokenizer })
    }
}


impl Iterator for StringStream {
    type Item = TokenInfo;

    fn next(&mut self) -> Option<TokenInfo> {
        self.tokenizer.next()
    }
}
