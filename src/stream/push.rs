use core::marker::PhantomData;

use super::{Sink, Transform};
use crate::error::Error;

#[macro_export]
macro_rules! push_transform {
    (
        $(?$unwrap:ident)?
        => $strategy:tt
        $transform:expr
        $(=> $($tail:tt)*)?
    ) => {
        push_transform!(
            @unwrap $($unwrap)*
            push_transform!(@strategy
                            $strategy
                            $transform,
                            push_transform!(
                                $(?$unwrap)*
                                => $($($tail)*)*)))
    };
    ($(?$unwrap:ident)? $transform:expr =>) => { $transform };
    ($(?$unwrap:ident)? =>) => { $crate::stream::push::NullEnd::new() };
    (@unwrap unwrap $e:expr) => { $e.unwrap() };
    (@unwrap $e:expr) => { $e? };
    (@strategy > $transform:expr, $sink:expr) => {
        $crate::stream::push::PushStrategy::eager_transform($transform, $sink)
    };
    (@strategy fn $transform:expr, $sink:expr) => {
        $crate::stream::push::PushStrategy::eager_transform(
            $crate::stream::PlainTransform::new($transform),
            $sink
        )
    };
    (@strategy $strategy:ident $transform:expr, $sink:expr) => {
        $strategy::new($transform, $sink)
    };
}


pub struct PushStrategy<Input, Output, T: Transform<Input, Output>, S: Sink<Output>> {
    kind: StrategyKind,
    transform: T,
    sink: S,
    phantom_input: PhantomData<Input>,
    phantom_output: PhantomData<Output>,
}

#[derive(PartialEq)]
enum StrategyKind {
    Eager,
}

impl<Input, Output, T: Transform<Input, Output>, S: Sink<Output>>
    PushStrategy<Input, Output, T, S>
{
    pub fn eager_transform(transform: T, sink: S) -> Result<Self, Error> {
        Ok(Self {
            kind: StrategyKind::Eager,
            transform,
            sink,
            phantom_input: Default::default(),
            phantom_output: Default::default(),
        })
    }

    fn forward(&mut self) -> Result<(), Error> {
        match self.kind {
            StrategyKind::Eager => {
                while let Some(output) = self.transform.output() {
                    self.sink.input(output).unwrap();
                }
            }
        }
        Ok(())
    }
}

impl<Input, Output, T: Transform<Input, Output>, S: Sink<Output>> Sink<Input>
    for PushStrategy<Input, Output, T, S>
{
    fn input(&mut self, input: Result<Input, Error>) -> Result<(), Error> {
        self.transform.input(input).unwrap();
        self.forward()
    }
}


// Just a hack rn. Might be better to stick an error handler here, but
// need to nail down error handling semantics on push streams.
pub struct NullEnd {}
impl NullEnd {
    pub fn new() -> Self {
        Self {}
    }
}
impl<T> Sink<T> for NullEnd {
    fn input(&mut self, input: Result<T, Error>) -> Result<(), Error> {
        input?;
        Ok(())
    }
}
