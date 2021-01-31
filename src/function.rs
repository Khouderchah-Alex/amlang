//! Basic blocks for procedural representation.

use std::borrow::Cow;
use std::fmt;

use self::EvalErr::*;
use self::ExpectedCount::*;
pub use crate::builtin::BuiltIn;
use crate::sexp::Sexp;

pub type Args<'a> = &'a Vec<Sexp>;
pub type Ret = Result<Sexp, EvalErr>;

pub trait Func {
    fn call(self: &Self, args: Args) -> Ret;
}

#[derive(Debug)]
pub enum EvalErr {
    InvalidArgument {
        given: Sexp,
        expected: Cow<'static, str>,
    },
    InvalidSexp(Sexp),
    WrongArgumentCount {
        given: usize,
        expected: ExpectedCount,
    },
    UnboundSymbol(String),
}

#[derive(Debug)]
pub enum ExpectedCount {
    Exactly(usize),
    AtLeast(usize),
}

impl fmt::Display for EvalErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[Eval Error] ")?;
        let res = match self {
            InvalidArgument { given, expected } => write!(
                f,
                "Invalid argument: given {}, expected {}",
                given, expected
            ),
            InvalidSexp(val) => write!(f, "Invalid S-exp for evaluation: {:#}", val),
            WrongArgumentCount { given, expected } => write!(
                f,
                "Wrong argument count; given {}, expected {}",
                given, expected
            ),
            UnboundSymbol(symbol) => write!(f, "Unbound symbol: \"{}\"", symbol),
        };
        res
    }
}

impl fmt::Display for ExpectedCount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        return match self {
            Exactly(exactly) => write!(f, "{}", exactly),
            AtLeast(minimum) => write!(f, "at least {}", minimum),
        };
    }
}
