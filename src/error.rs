//! Representation of errors which can be reified in Amlang.
//!
//! General error mechanism that can use any ErrorKind. Reification allows for
//! Errors to be mapped to semantic content within Environments, even if not
//! part of the base implementation.

use std::fmt;

use crate::agent::agent_state::AgentState;
use crate::sexp::Sexp;


/// Creates a stateful Error (currently of kind LangError).
///
/// Called as:  err!(state, error).
/// Stateful errors should always be used when possible.
#[macro_export]
macro_rules! err {
    ($state:expr, $($inner:tt)+) => {
        Err($crate::error::Error::with_state(
            $state.clone(),
            Box::new($crate::agent::lang_error::LangError::$($inner)+),
        ))
    };
}
/// Creates a stateless Error (currently of kind LangError).
///
/// Called as:  err_nost!(error).
/// Stateful errors are always preferred when possible.
#[macro_export]
macro_rules! err_nost {
    ($($inner:tt)+) => {
        Err($crate::error::Error::empty_state(
            Box::new($crate::agent::lang_error::LangError::$($inner)+),
        ))
    };
}


#[derive(Debug)]
pub struct Error {
    state: Option<Box<AgentState>>,
    kind: Box<dyn ErrorKind>,
}

pub trait ErrorKind: fmt::Display + fmt::Debug {
    // Cannot use Reflective since we use ErrorKind as a trait object.
    fn reify(&self) -> Sexp;
}


impl Error {
    // Prefer using err_nost! for convenience.
    pub fn empty_state(kind: Box<dyn ErrorKind>) -> Self {
        Self { state: None, kind }
    }

    // Prefer using err! for convenience.
    pub fn with_state(state: AgentState, kind: Box<dyn ErrorKind>) -> Self {
        Self {
            state: Some(Box::new(state)),
            kind,
        }
    }

    pub fn kind(&self) -> &dyn ErrorKind {
        &*self.kind
    }

    pub fn state(&self) -> Option<&AgentState> {
        self.state.as_ref().map(|e| &**e)
    }
}

impl PartialEq for Error {
    /// Compare kind.
    fn eq(&self, other: &Self) -> bool {
        self.kind().reify() == other.kind().reify()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind())
    }
}
