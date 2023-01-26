use super::{ErrorHandler, Read, Transform};
use crate::error::Error;


pub struct Strategy<Input, Output> {
    kind: StrategyKind,

    read: Box<dyn Read<Input>>,
    transform: Box<dyn Transform<Input, Output>>,
    error_handler: Box<ErrorHandler>,
}

#[derive(PartialEq)]
pub enum StrategyKind {
    Lazy,
    Eager,
}


impl<Input, Output> Strategy<Input, Output> {
    pub fn new(
        kind: StrategyKind,
        read: Box<dyn Read<Input>>,
        transform: Box<dyn Transform<Input, Output>>,
        error_handler: Box<ErrorHandler>,
    ) -> Result<Self, Error> {
        let mut res = Self {
            kind,
            read,
            transform,
            error_handler,
        };

        match res.kind {
            StrategyKind::Lazy => {}
            StrategyKind::Eager => res.load()?,
        }
        Ok(res)
    }

    fn load(&mut self) -> Result<(), Error> {
        while let Some(input) = self.read.read() {
            if self.transform.input(input)? && self.kind == StrategyKind::Lazy {
                break;
            }
        }
        Ok(())
    }
}

impl<Input, Output> Iterator for Strategy<Input, Output> {
    type Item = Output;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(output) = self.transform.output() {
            return Some(output);
        }

        if let Err(error) = self.load() {
            (self.error_handler)(error);
        }

        self.transform.output()
    }
}
