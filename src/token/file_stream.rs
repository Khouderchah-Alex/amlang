use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use super::token::TokenInfo;
use super::tokenize::{tokenize_line, TokenStore};

pub struct FileStream {
    tokens: TokenStore,
}

impl FileStream {
    pub fn new<P: AsRef<Path>>(path: P) -> std::io::Result<FileStream> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        let mut tokens = TokenStore::default();
        for line in reader.lines().enumerate() {
            if let (_, Err(err)) = line {
                return Err(err);
            }

            tokenize_line(&line.1.unwrap(), line.0, &mut tokens);
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
