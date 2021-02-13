//! Representation of primitives.

use std::fmt;

use crate::function::BuiltIn;
use crate::number::Number;

#[derive(Clone, Debug, PartialEq)]
pub enum Primitive {
    Number(Number),
    Symbol(String),
    BuiltIn(&'static BuiltIn),
}

impl fmt::Display for Primitive {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Primitive::Number(num) => write!(f, "{}", num),
            Primitive::Symbol(s) => write!(f, "{}", s),
            Primitive::BuiltIn(b) => write!(f, "{}", b),
        }
    }
}
