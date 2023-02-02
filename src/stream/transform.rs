use crate::error::Error;


#[macro_export]
macro_rules! pull_transform {
    (
        $(?$unwrap:ident)?
        $input:expr => $transform:expr
        $(=> $($tail:tt)*)*
    ) => {
        pull_transform!($(?$unwrap)*
            pull_transform!(@unwrap $($unwrap)*
                       $crate::stream::Strategy::new(
                           $crate::stream::StrategyKind::Lazy,
                           Box::new($input),
                           Box::new($transform),
                       ))
            => $($($tail)*)*)
    };
    (
        $(?$unwrap:ident)?
        $input:expr =>> $transform:expr
        $(=> $($tail:tt)*)*
    ) => {
        pull_transform!($(?$unwrap)*
            pull_transform!(@unwrap $($unwrap)*
                       $crate::stream::Strategy::new(
                           $crate::stream::StrategyKind::Eager,
                           Box::new($input),
                           Box::new($transform),
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

pub struct Strategy<Input, Output> {
    kind: StrategyKind,
    input: Box<dyn Iterator<Item = Result<Input, Error>>>,
    transform: Box<dyn Transform<Input, Output>>,
}

#[derive(PartialEq)]
pub enum StrategyKind {
    Lazy,
    Eager,
}

impl<Input, Output> Strategy<Input, Output> {
    pub fn new(
        kind: StrategyKind,
        input: Box<dyn Iterator<Item = Result<Input, Error>>>,
        transform: Box<dyn Transform<Input, Output>>,
    ) -> Result<Self, Error> {
        let mut res = Self {
            kind,
            input,
            transform,
        };

        match res.kind {
            StrategyKind::Lazy => {}
            StrategyKind::Eager => res.load()?,
        }
        Ok(res)
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

impl<Input, Output> Iterator for Strategy<Input, Output> {
    type Item = Result<Output, Error>;
    fn next(&mut self) -> Option<Self::Item> {
        self.output()
    }
}
