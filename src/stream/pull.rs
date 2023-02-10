use core::marker::PhantomData;

use super::Transform;
use crate::error::Error;

#[macro_export]
macro_rules! pull_transform {
    (
        $(?$unwrap:ident)?
        $input:expr
        => $strategy:tt
        $transform:expr
        $(=> $($tail:tt)*)?
    ) => {
        pull_transform!(
            $(?$unwrap)*
            pull_transform!(
                @unwrap $($unwrap)*
                pull_transform!(@strategy $strategy $input, $transform)
            )
            => $($($tail)*)*)
    };
    ($(?$unwrap:ident)? $input:expr =>) => { $input };
    (@unwrap unwrap $e:expr) => { $e.unwrap() };
    (@unwrap $e:expr) => { $e? };
    (@strategy . $input:expr, $transform:expr) => {
        $crate::stream::pull::PullStrategy::lazy_transform($input, $transform)
    };
    (@strategy > $input:expr, $transform:expr) => {
        $crate::stream::pull::PullStrategy::eager_transform($input, $transform)
    };
    (@strategy fn $input:expr, $transform:expr) => {
        $crate::stream::pull::PullStrategy::lazy_transform(
            $input,
            $crate::stream::PlainTransform::new($transform)
        )
    };
    (@strategy $strategy:ident $input:expr, $transform:expr) => {
        $strategy::new($input, $transform)
    };
}


pub struct PullStrategy<
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
    PullStrategy<Input, Output, I, T>
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
    for PullStrategy<Input, Output, I, T>
{
    type Item = Result<Output, Error>;
    fn next(&mut self) -> Option<Self::Item> {
        self.output()
    }
}
