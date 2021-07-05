use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use super::token::TokenInfo;
use super::tokenize::{tokenize_line, TokenStore, TokenizeError};
use crate::primitive::symbol::SymbolError;


pub struct FileStream {
    tokens: TokenStore,
}

#[derive(Debug)]
pub enum FileStreamError {
    IoError(std::io::Error),
    TokenizeError(TokenizeError),
}

impl FileStream {
    pub fn new<P: AsRef<Path>, SymbolInfo, SymbolPolicy>(
        path: P,
        symbol_policy: SymbolPolicy,
    ) -> Result<FileStream, FileStreamError>
    where
        SymbolPolicy: Fn(&str) -> Result<SymbolInfo, SymbolError>,
    {
        let file = match File::open(path) {
            Ok(file) => file,
            Err(err) => return Err(FileStreamError::IoError(err)),
        };
        let reader = BufReader::new(file);

        let mut tokens = TokenStore::default();
        for line in reader.lines().enumerate() {
            if let (_, Err(err)) = line {
                return Err(FileStreamError::IoError(err));
            }

            if let Err(err) = tokenize_line(&line.1.unwrap(), line.0, &symbol_policy, &mut tokens) {
                return Err(FileStreamError::TokenizeError(err));
            }
        }

        Ok(FileStream { tokens })
    }
}


impl Iterator for FileStream {
    type Item = TokenInfo;

    fn next(&mut self) -> Option<TokenInfo> {
        return self.tokens.pop_front();
    }
}
