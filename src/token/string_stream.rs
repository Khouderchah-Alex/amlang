use super::token::TokenInfo;
use super::tokenizer::{TokenizeError, Tokenizer};
use crate::agent::symbol_policies::SymbolPolicy;


pub struct StringStream {
    tokenizer: Tokenizer,
}

impl StringStream {
    pub fn new<S: AsRef<str>, SymbolInfo>(
        line: S,
        symbol_policy: SymbolPolicy<SymbolInfo>,
    ) -> Result<StringStream, TokenizeError> {
        let mut tokenizer = Tokenizer::new();
        if let Err(err) = tokenizer.tokenize(&line, symbol_policy) {
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
