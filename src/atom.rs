//! Representation of atoms.

use std::fmt;

use crate::function::BuiltIn;
use crate::number::Number;

#[derive(Clone, Debug, PartialEq)]
pub enum Atom {
    Number(Number),
    Symbol(String),
    BuiltIn(&'static BuiltIn),
}

impl fmt::Display for Atom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Atom::Number(num) => write!(f, "{}", num),
            Atom::Symbol(s) => write!(f, "{}", s),
            Atom::BuiltIn(b) => write!(f, "{}", b),
        }
    }
}
