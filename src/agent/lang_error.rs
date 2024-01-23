use std::borrow::Cow;
use std::fmt;

use crate::error::ErrorKind;
use crate::primitive::{Number, Symbol};
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
    RejectedTriple(Sexp, Sexp),
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
                    "InvalidArgument",
                    ("given", given.clone()),
                    ("expected", expected.clone().into_owned()),
                )
            }
            Self::InvalidState { actual, expected } => {
                list!(
                    "InvalidState",
                    ("actual", actual.clone().into_owned()),
                    ("expected", expected.clone().into_owned()),
                )
            }
            Self::InvalidSexp(val) => list!("InvalidSexp", val.clone()),
            Self::WrongArgumentCount { given, expected } => list!(
                "WrongArgumentCount",
                ("given", Number::USize(*given),),
                ("expected", expected.to_string(),),
            ),
            Self::UnboundSymbol(symbol) => {
                list!("UnboundSymbol", symbol.clone())
            }
            Self::AlreadyBoundSymbol(symbol) => {
                list!("AlreadyBoundSymbol", symbol.clone())
            }
            Self::DuplicateTriple(sexp) => {
                list!("DuplicateTriple", sexp.clone())
            }
            Self::RejectedTriple(triple, reason) => {
                list!("RejectedTriple", triple.clone(), reason.clone(),)
            }
            Self::Unsupported(msg) => list!("Unsupported", msg.clone().into_owned()),
        };
        Cons::new(Sexp::from("LangError"), inner).into()
    }
}

impl fmt::Display for ExpectedCount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        return match self {
            Self::Exactly(exactly) => write!(f, "{}", exactly),
            Self::AtLeast(minimum) => write!(f, "AtLeast {}", minimum),
            Self::AtMost(maximum) => write!(f, "AtMost {}", maximum),
        };
    }
}
