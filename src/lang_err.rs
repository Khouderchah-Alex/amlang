use std::borrow::Cow;
use std::fmt;

use self::ErrKind::*;
use self::ExpectedCount::*;
use crate::agent::agent_state::AgentState;
use crate::primitive::Symbol;
use crate::sexp::Sexp;


/// Creates a stateful LangErr.
///
/// Called as:  err!(state, error).
/// Stateful errors should always be used when possible.
macro_rules! err {
    ($state:expr, $($kind:tt)+) => {
        Err(crate::lang_err::LangErr::with_state(
            $state.clone(),
            crate::lang_err::ErrKind::$($kind)+,
        ))
    };
}
/// Creates a stateless LangErr.
///
/// Called as:  err_nost!(error).
/// Stateful errors are always preferred when possible.
macro_rules! err_nost {
    ($($kind:tt)+) => {
        Err(crate::lang_err::LangErr::empty_state(
            crate::lang_err::ErrKind::$($kind)+,
        ))
    };
}


#[derive(Debug)]
pub struct LangErr {
    state: Option<AgentState>,
    pub kind: ErrKind,
}

#[derive(Debug)]
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
}

#[derive(Debug)]
pub enum ExpectedCount {
    Exactly(usize),
    AtLeast(usize),
    AtMost(usize),
}

impl LangErr {
    // Prefer using err! for convenience.
    pub fn empty_state(kind: ErrKind) -> Self {
        Self { state: None, kind }
    }

    // Prefer using err! for convenience.
    pub fn with_state(state: AgentState, kind: ErrKind) -> Self {
        Self {
            state: Some(state),
            kind,
        }
    }

    pub fn state(&self) -> &Option<AgentState> {
        &self.state
    }
}


impl fmt::Display for LangErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[Lang Error] ")?;
        match &self.kind {
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
