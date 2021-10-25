use std::borrow::Cow;
use std::fmt;

use crate::primitive::error::ErrorKind;
use crate::primitive::{AmString, Number, Symbol};
use crate::sexp::{Cons, Sexp};


#[derive(Clone, Debug, PartialEq)]
pub enum LangError {
    InvalidArgument {
        given: Sexp,
        expected: Cow<'static, str>,
    },
    InvalidState {
        actual: Cow<'static, str>,
        expected: Cow<'static, str>,
    },
    InvalidSexp(Sexp),
    WrongArgumentCount {
        given: usize,
        expected: ExpectedCount,
    },
    UnboundSymbol(Symbol),
    AlreadyBoundSymbol(Symbol),
    DuplicateTriple(Sexp),
    Unsupported(Cow<'static, str>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum ExpectedCount {
    Exactly(usize),
    AtLeast(usize),
    AtMost(usize),
}


impl ErrorKind for LangError {
    // TODO(func) Model within env rather than fall back on strings.
    fn reify(&self) -> Sexp {
        let inner = match self {
            Self::InvalidArgument { given, expected } => {
                list!(
                    AmString::new("Invalid argument"),
                    (AmString::new("given"), given.clone(),),
                    (AmString::new("expected"), AmString::new(expected),),
                )
            }
            Self::InvalidState { actual, expected } => {
                list!(
                    AmString::new("Invalid state"),
                    (AmString::new("actual"), AmString::new(actual),),
                    (AmString::new("expected"), AmString::new(expected),),
                )
            }
            Self::InvalidSexp(val) => list!(AmString::new("Invalid sexp"), val.clone(),),
            Self::WrongArgumentCount { given, expected } => list!(
                AmString::new("Wrong argument count"),
                (AmString::new("given"), Number::Integer(*given as i64),),
                (
                    AmString::new("expected"),
                    AmString::new(expected.to_string()),
                ),
            ),
            Self::UnboundSymbol(symbol) => list!(AmString::new("Unbound symbol"), symbol.clone(),),
            Self::AlreadyBoundSymbol(symbol) => {
                list!(AmString::new("Already bound symbol"), symbol.clone(),)
            }
            Self::DuplicateTriple(sexp) => list!(AmString::new("Duplicate triple"), sexp.clone(),),
            Self::Unsupported(msg) => list!(AmString::new("Unsupported"), AmString::new(msg),),
        };
        Cons::new(Some(AmString::new("LangError").into()), inner.into()).into()
    }
}

impl fmt::Display for LangError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.reify())
    }
}

impl fmt::Display for ExpectedCount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        return match self {
            Self::Exactly(exactly) => write!(f, "{}", exactly),
            Self::AtLeast(minimum) => write!(f, "at least {}", minimum),
            Self::AtMost(maximum) => write!(f, "at most {}", maximum),
        };
    }
}
