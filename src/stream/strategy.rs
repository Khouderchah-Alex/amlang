use crate::error::Error;


#[macro_export]
macro_rules! transform {
    (
        $input:expr
            => $transform:expr
            $(;$err_handler:expr)?
            $(=> ($tail:tt)*)*
    ) => {
        transform!(
            $crate::stream::Strategy::new(
                $crate::stream::StrategyKind::Lazy,
                Box::new($input),
                Box::new($transform),
                Box::new(transform!(@error $($err_handler)*)),
            )
                => $($tail)*)
    };
    (
        $input:expr
            =>> $transform:expr
            $(;$err_handler:expr)?
            $(=> ($tail:tt)*)*
    ) => {
        transform!(
            $crate::stream::Strategy::new(
                $crate::stream::StrategyKind::Eager,
                Box::new($input),
                Box::new($transform),
                Box::new(transform!(@error $($err_handler)*)),
            )
                => $($tail)*)
    };
    ($input:expr =>) => { $input };
    (@error) => {
        |_error: Error| panic!()
    };
    (@error $err_handler:expr) => {
        $err_handler
    };
}

pub struct Strategy<Input, Output> {
    kind: StrategyKind,
    input: Box<dyn Iterator<Item = Input>>,
    transform: Box<dyn Transform<Input, Output>>,
    error_handler: Box<ErrorHandler>,
}

#[derive(PartialEq)]
pub enum StrategyKind {
    Lazy,
    Eager,
}


pub trait Transform<Input, Output> {
    fn input(&mut self, input: Input) -> Result<bool, Error>; // output_available
    fn output(&mut self) -> Option<Output>;
}

pub type ErrorHandler = dyn FnMut(Error);


impl<Input, Output> Strategy<Input, Output> {
    pub fn new(
        kind: StrategyKind,
        input: Box<dyn Iterator<Item = Input>>,
        transform: Box<dyn Transform<Input, Output>>,
        error_handler: Box<ErrorHandler>,
    ) -> Result<Self, Error> {
        let mut res = Self {
            kind,
            input,
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
        while let Some(input) = self.input.next() {
            if self.transform.input(input)? && self.kind == StrategyKind::Lazy {
                break;
            }
        }
        Ok(())
    }

    fn output(&mut self) -> Option<Output> {
        if let Some(output) = self.transform.output() {
            return Some(output);
        }

        if let Err(error) = self.load() {
            (self.error_handler)(error);
        }

        self.transform.output()
    }
}

impl<Input, Output> Iterator for Strategy<Input, Output> {
    type Item = Output;
    fn next(&mut self) -> Option<Self::Item> {
        self.output()
    }
}
