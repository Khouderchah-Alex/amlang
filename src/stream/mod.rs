#[macro_use]
pub mod strategy;
pub mod input;

pub mod prelude {
    pub use super::input::{FifoReader, FileReader, StringReader};
    pub use super::strategy::{ErrorHandler, Strategy, StrategyKind, Transform};
    pub use super::Stream;
}

pub use strategy::{ErrorHandler, Strategy, StrategyKind, Transform};


// Benefits:
//  * "Generic barrier" over Iterators
//  * Allows for impling Transform rather than Iterator; replacing next() with input() & output() allows for policy to be abstracted out & reused
//  * Better model when the source is continually generating (think IPC-like interactions)
//  * Better model when fair mean diff b/w N:M (think tokens -> sexps)
//  * Think: use for event queue [TODO]
//
// Notes:
//  * Data can be stored along the pipeline, but so can Iterators with closures
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
