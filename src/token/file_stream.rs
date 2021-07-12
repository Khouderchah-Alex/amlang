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
        let file = match File::open(path) {
            Ok(file) => file,
            Err(err) => return Err(FileStreamError::IoError(err)),
        };
        let reader = BufReader::new(file);

        let mut tokenizer = Tokenizer::new();
        for line in reader.lines() {
            match line {
                Ok(l) => {
                    if let Err(err) = tokenizer.tokenize_line(&l, &symbol_policy) {
                        return Err(FileStreamError::TokenizeError(err));
                    }
                }
                Err(err) => return Err(FileStreamError::IoError(err)),
            }
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
