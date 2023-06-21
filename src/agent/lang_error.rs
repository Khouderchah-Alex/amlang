use std::borrow::Cow;
use std::fmt;

use crate::error::ErrorKind;
use crate::primitive::{Number, Symbol, ToLangString};
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
                    "InvalidArgument".to_lang_string(),
                    ("given".to_lang_string(), given.clone(),),
                    ("expected".to_lang_string(), expected.to_lang_string(),),
                )
            }
            Self::InvalidState { actual, expected } => {
                list!(
                    "InvalidState".to_lang_string(),
                    ("actual".to_lang_string(), actual.to_lang_string(),),
                    ("expected".to_lang_string(), expected.to_lang_string(),),
                )
            }
            Self::InvalidSexp(val) => list!("InvalidSexp".to_lang_string(), val.clone()),
            Self::WrongArgumentCount { given, expected } => list!(
                "WrongArgumentCount".to_lang_string(),
                ("given".to_lang_string(), Number::USize(*given),),
                (
                    "expected".to_lang_string(),
                    expected.to_string().to_lang_string(),
                ),
            ),
            Self::UnboundSymbol(symbol) => {
                list!("UnboundSymbol".to_lang_string(), symbol.clone())
            }
            Self::AlreadyBoundSymbol(symbol) => {
                list!("AlreadyBoundSymbol".to_lang_string(), symbol.clone())
            }
            Self::DuplicateTriple(sexp) => {
                list!("DuplicateTriple".to_lang_string(), sexp.clone())
            }
            Self::RejectedTriple(triple, reason) => {
                list!(
                    "RejectedTriple".to_lang_string(),
                    triple.clone(),
                    reason.clone(),
                )
            }
            Self::Unsupported(msg) => list!("Unsupported".to_lang_string(), msg.to_lang_string()),
        };
        Cons::new("LangError".to_lang_string(), inner).into()
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
