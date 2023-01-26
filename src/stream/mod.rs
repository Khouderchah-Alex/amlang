use crate::error::{Error, ErrorKind};
use crate::primitive::prelude::*;
use crate::sexp::Sexp;

pub mod input;
pub mod strategy;

pub mod prelude {
    pub use super::input::{FifoReader, FileReader, StringReader};
    pub use super::strategy::{Strategy, StrategyKind};
    pub use super::{ErrorHandler, Read, Stream, StreamError, Transform};
}

pub struct Stream<Output> {
    strategy: Box<dyn Iterator<Item = Output>>,
}

impl<Output> Stream<Output> {
    pub fn new(strategy: Box<dyn Iterator<Item = Output>>) -> Self {
        Self { strategy }
    }
}

impl<Output> Iterator for Stream<Output> {
    type Item = Output;
    fn next(&mut self) -> Option<Self::Item> {
        self.strategy.next()
    }
}


pub trait Read<Input> {
    fn read(&mut self) -> Option<Input>;
}

pub trait Transform<Input, Output> {
    fn input(&mut self, input: Input) -> Result<bool, Error>; // output_available
    fn output(&mut self) -> Option<Output>;
}

pub type ErrorHandler = dyn FnMut(Error);


#[derive(Debug)]
pub enum StreamError {
    IoError(std::io::Error),
    Error(Error),
}

impl ErrorKind for StreamError {
    // TODO(func) Model within env rather than fall back on strings.
    fn reify(&self) -> Sexp {
        match self {
            StreamError::IoError(err) => list!(
                "ParseError".to_lang_string(),
                format!("{:?}", err).to_lang_string(),
            ),
            StreamError::Error(err) => err.kind().reify(),
        }
    }
}
