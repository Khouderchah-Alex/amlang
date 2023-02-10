#[macro_use]
pub mod pull;
#[macro_use]
pub mod push;
pub mod input;
pub mod output;
pub mod transform;

pub mod prelude {
    pub use super::input::{FifoReader, FileReader, StringReader};
    pub use super::output::Writer;
    pub use super::transform::Transform;
    pub use super::PullStream;
}

pub use output::Writer;
pub use transform::{PlainTransform, Transform};

use crate::error::Error;


/// Benefits:
///  * "Generic barrier" over Iterators
///  * Allows for impling Transform rather than Iterator; replacing next() with input() & output() allows for policy to be abstracted out & reused
///  * Better model when the source is continually generating (think IPC-like interactions)
///  * Better model when fair mean diff b/w N:M (think tokens -> sexps)
///
/// Notes:
///  * Data can be stored along the pipeline, but so can Iterators with closures
pub struct PullStream<Output> {
    strategy: Box<dyn Iterator<Item = Result<Output, Error>> + Send + Sync>,
}

impl<Output> PullStream<Output> {
    pub fn new(strategy: Box<dyn Iterator<Item = Result<Output, Error>> + Send + Sync>) -> Self {
        Self { strategy }
    }
}

impl<Output> Iterator for PullStream<Output> {
    type Item = Result<Output, Error>;
    fn next(&mut self) -> Option<Self::Item> {
        self.strategy.next()
    }
}


pub trait Sink<Input> {
    fn input(&mut self, input: Result<Input, Error>) -> Result<(), Error>;
}

/// Uses Transform to perform computations asynchronously. Can be
/// integrated with an event loop for a full async experience.
pub struct PushStream<Input> {
    strategy: Box<dyn Sink<Input> + Send + Sync>,
}

impl<Input> PushStream<Input> {
    pub fn new(strategy: Box<dyn Sink<Input> + Send + Sync>) -> Self {
        Self { strategy }
    }
}

impl<Input> Sink<Input> for PushStream<Input> {
    fn input(&mut self, input: Result<Input, Error>) -> Result<(), Error> {
        self.strategy.input(input)
    }
}
