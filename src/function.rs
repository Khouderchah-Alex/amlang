//! Basic blocks for procedural representation.

use std::fmt;

use self::EvalErr::*;
pub use crate::builtin::BuiltIn;
use crate::sexp::Value;

pub type Args<'a> = &'a Vec<Value>;
pub type Ret = Result<Value, EvalErr>;

pub trait Func {
    fn call(self: &Self, args: Args) -> Ret;
}

#[derive(Debug)]
pub enum EvalErr {
    InvalidArgument,
    MissingArguments { given: usize, expected: usize },
    UnboundSymbol(String),
}

impl fmt::Display for EvalErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[Error] ")?;
        let res = match self {
            InvalidArgument => write!(f, "Invalid argument"),
            MissingArguments { given, expected } => write!(
                f,
                "Missing arguments; given {}, expected {}",
                given, expected
            ),
            UnboundSymbol(symbol) => write!(f, "Unbound symbol: \"{}\"", symbol),
        };
        res
    }
}
