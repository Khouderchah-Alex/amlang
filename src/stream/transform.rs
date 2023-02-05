use core::marker::PhantomData;
use std::collections::VecDeque;

use crate::error::Error;


#[macro_export]
macro_rules! pull_transform {
    (
        $(?$unwrap:ident)?
        $input:expr => $transform:expr
        $(=> $($tail:tt)*)?
    ) => {
        pull_transform!($(?$unwrap)*
            pull_transform!(@unwrap $($unwrap)*
                       $crate::stream::Strategy::lazy_transform(
                           $input,
                           $transform,
                       ))
            => $($($tail)*)*)
    };
    (
        $(?$unwrap:ident)?
        $input:expr =>> $transform:expr
        $(=> $($tail:tt)*)?
    ) => {
        pull_transform!($(?$unwrap)*
            pull_transform!(@unwrap $($unwrap)*
                       $crate::stream::Strategy::eager_transform(
                           $input,
                           $transform,
                       ))
            => $($($tail)*)*)
    };
    ($(?$unwrap:ident)? $input:expr =>) => { $input };
    (@unwrap unwrap $e:expr) => { $e.unwrap() };
    (@unwrap $e:expr) => { $e? };
}


pub trait Transform<Input, Output> {
    fn input(&mut self, input: Result<Input, Error>) -> Result<bool, Error>; // output_available
    fn output(&mut self) -> Option<Result<Output, Error>>;
}

// Simple Transform when every invocation of f will produce one output.
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


pub struct Strategy<
    Input,
    Output,
    I: Iterator<Item = Result<Input, Error>>,
    T: Transform<Input, Output>,
> {
    kind: StrategyKind,
    input: I,
    transform: T,
    phantom_output: PhantomData<Output>,
}

#[derive(PartialEq)]
enum StrategyKind {
    Lazy,
    Eager,
}

impl<Input, Output, I: Iterator<Item = Result<Input, Error>>, T: Transform<Input, Output>>
    Strategy<Input, Output, I, T>
{
    pub fn eager_transform(input: I, transform: T) -> Result<Self, Error> {
        let mut res = Self {
            kind: StrategyKind::Eager,
            input,
            transform,
            phantom_output: Default::default(),
        };

        res.load()?;
        Ok(res)
    }

    pub fn lazy_transform(input: I, transform: T) -> Result<Self, Error> {
        Ok(Self {
            kind: StrategyKind::Lazy,
            input,
            transform,
            phantom_output: Default::default(),
        })
    }

    fn load(&mut self) -> Result<(), Error> {
        while let Some(input) = self.input.next() {
            if self.transform.input(input)? && self.kind == StrategyKind::Lazy {
                break;
            }
        }
        Ok(())
    }

    fn output(&mut self) -> Option<Result<Output, Error>> {
        if let Some(output) = self.transform.output() {
            return Some(output);
        }

        match self.load() {
            Ok(_) => self.transform.output(),
            Err(err) => Some(Err(err)),
        }
    }
}

impl<Input, Output, I: Iterator<Item = Result<Input, Error>>, T: Transform<Input, Output>> Iterator
    for Strategy<Input, Output, I, T>
{
    type Item = Result<Output, Error>;
    fn next(&mut self) -> Option<Self::Item> {
        self.output()
    }
}
