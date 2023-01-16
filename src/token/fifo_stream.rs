use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

use super::file_stream::FileStreamError;
use super::token::TokenInfo;
use super::tokenizer::Tokenizer;
use crate::agent::symbol_policies::policy_base;


/// Token stream that will perform non-blocking reads from a fifo when
/// in-mem cache empties. Iterator will return None if both cache &
/// fifo are empty, but future calls to next() may return Some(_) if
/// the fifo has new data.
pub struct FifoStream {
    tokenizer: Tokenizer,
    fifo: BufReader<File>,
}

impl FifoStream {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<FifoStream, FileStreamError> {
        let mut options = OpenOptions::new();
        options.read(true);
        if cfg!(unix) {
            options.custom_flags(libc::O_NONBLOCK);
        }
        let file = options.open(path)?;
        let reader = BufReader::new(file);

        Ok(FifoStream {
            tokenizer: Tokenizer::new(),
            fifo: reader,
        })
    }
}

impl Iterator for FifoStream {
    type Item = TokenInfo;

    fn next(&mut self) -> Option<TokenInfo> {
        match self.tokenizer.next() {
            Some(token) => Some(token),
            None => {
                let mut line = String::new();
                while self.fifo.read_line(&mut line).unwrap() != 0 {
                    if let Err(err) = self.tokenizer.tokenize(line, policy_base) {
                        println!("{}", err);
                        println!("");
                        self.tokenizer.clear();
                    }
                    line = String::new();
                }
                self.tokenizer.next()
            }
        }
    }
}
