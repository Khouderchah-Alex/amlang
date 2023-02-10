use core::marker::PhantomData;
use std::collections::VecDeque;

use crate::error::Error;


pub trait Transform<Input, Output> {
    fn input(&mut self, input: Result<Input, Error>) -> Result<bool, Error>; // output_available
    fn output(&mut self) -> Option<Result<Output, Error>>;
}


/// Simple Transform where every invocation of f will produce one output.
pub struct PlainTransform<Input, Output, F: FnMut(Result<Input, Error>) -> Result<Output, Error>> {
    f: F,
    cache: VecDeque<Output>,
    phantom_input: PhantomData<Input>,
    phantom_output: PhantomData<Output>,
}

impl<Input, Output, F: FnMut(Result<Input, Error>) -> Result<Output, Error>>
    PlainTransform<Input, Output, F>
{
    pub fn new(f: F) -> Self {
        Self {
            f,
            cache: Default::default(),
            phantom_input: Default::default(),
            phantom_output: Default::default(),
        }
    }
}

impl<Input, Output, F: FnMut(Result<Input, Error>) -> Result<Output, Error>>
    Transform<Input, Output> for PlainTransform<Input, Output, F>
{
    fn input(&mut self, input: Result<Input, Error>) -> Result<bool, Error> {
        self.cache.push_back((self.f)(input)?);
        return Ok(true);
    }

    fn output(&mut self) -> Option<Result<Output, Error>> {
        Some(Ok(self.cache.pop_front()?))
    }
}
