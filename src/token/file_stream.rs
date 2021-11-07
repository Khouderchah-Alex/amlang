use std::fmt;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use super::token::TokenInfo;
use super::tokenizer::{TokenizeError, Tokenizer};
use crate::primitive::symbol::SymbolError;


pub struct FileStream {
    tokenizer: Tokenizer,
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
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        let mut tokenizer = Tokenizer::new();
        for line in reader.lines() {
            tokenizer.tokenize(line?, &symbol_policy)?;
        }

        Ok(FileStream { tokenizer })
    }
}


impl Iterator for FileStream {
    type Item = TokenInfo;

    fn next(&mut self) -> Option<TokenInfo> {
        self.tokenizer.next()
    }
}


impl From<std::io::Error> for FileStreamError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err)
    }
}

impl From<TokenizeError> for FileStreamError {
    fn from(err: TokenizeError) -> Self {
        Self::TokenizeError(err)
    }
}

impl fmt::Display for FileStreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[FileStreamError] ")?;
        match self {
            Self::IoError(err) => write!(f, "[IoError] {}", err),
            Self::TokenizeError(err) => write!(f, "[TokenizeError] {}", err),
        }
    }
}
