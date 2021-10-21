use std::borrow::Cow;
use std::convert::TryFrom;
use std::fmt;

use self::ErrKind::*;
use self::ExpectedCount::*;
use super::Primitive;
use crate::agent::agent_state::AgentState;
use crate::primitive::Symbol;
use crate::sexp::{HeapSexp, Sexp};


#[derive(Clone, Debug)]
pub struct Error {
    state: Option<Box<AgentState>>,
    kind: Box<ErrKind>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ErrKind {
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

impl Error {
    // Prefer using err_nost! for convenience.
    pub fn empty_state(kind: ErrKind) -> Self {
        Self {
            state: None,
            kind: Box::new(kind),
        }
    }

    // Prefer using err! for convenience.
    pub fn with_state(state: AgentState, kind: ErrKind) -> Self {
        Self {
            state: Some(Box::new(state)),
            kind: Box::new(kind),
        }
    }

    pub fn kind(&self) -> &ErrKind {
        &*self.kind
    }

    pub fn state(&self) -> Option<&AgentState> {
        self.state.as_ref().map(|e| &**e)
    }
}


impl PartialEq for Error {
    /// Compare ErrKinds.
    fn eq(&self, other: &Self) -> bool {
        *self.kind == *other.kind
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[Lang Error] ")?;
        match self.kind() {
            InvalidArgument { given, expected } => write!(
                f,
                "Invalid argument: given {}, expected {}",
                given, expected
            ),
            InvalidState { actual, expected } => {
                write!(f, "Invalid state: actual {}, expected {}", actual, expected)
            }
            InvalidSexp(val) => write!(f, "Invalid S-exp for evaluation: {:#}", val),
            WrongArgumentCount { given, expected } => write!(
                f,
                "Wrong argument count: given {}, expected {}",
                given, expected
            ),
            UnboundSymbol(symbol) => write!(f, "Unbound symbol: \"{}\"", symbol),
            AlreadyBoundSymbol(symbol) => write!(f, "Already bound symbol: \"{}\"", symbol),
            DuplicateTriple(sexp) => write!(f, "Duplicate triple: {}", sexp),
            Unsupported(msg) => write!(f, "Unsupported: {}", msg),
        }
    }
}

impl fmt::Display for ExpectedCount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        return match self {
            Exactly(exactly) => write!(f, "{}", exactly),
            AtLeast(minimum) => write!(f, "at least {}", minimum),
            AtMost(maximum) => write!(f, "at most {}", maximum),
        };
    }
}


impl_try_from!(Sexp              ->  Error,      Error;
               HeapSexp          ->  Error,      Error;
               ref Sexp          ->  ref Error,  Error;
               Option<Sexp>      ->  Error,      Error;
               Option<ref Sexp>  ->  ref Error,  Error;
               Result<Sexp>      ->  Error,      Error;
               Result<ref Sexp>  ->  ref Error,  Error;
);
