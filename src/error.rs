//! Representation of errors which can be reified in Amlang.
//!
//! General error mechanism that can use any ErrorKind. Reification allows for
//! Errors to be mapped to semantic content within Environments, even if not
//! part of the base implementation.

use std::fmt;

use crate::agent::agent_state::AgentState;
use crate::sexp::Sexp;


/// Creates a stateful Error wrapped in Err.
#[macro_export]
macro_rules! err {
    ($state:expr, $($kind:tt)+) => {
        Err($crate::error::Error::with_state(
            $state.clone(),
            Box::new($($kind)+),
        ))
    };
}

#[derive(Debug)]
pub struct Error {
    state: Option<Box<AgentState>>,
    kind: Box<dyn ErrorKind>,
}

pub trait ErrorKind: fmt::Debug /* fmt::Display auto-impled below */ {
    // Cannot use Reflective since we use ErrorKind as a trait object.
    fn reify(&self) -> Sexp;
}


impl Error {
    /// Prefer using err! for convenience.
    pub fn with_state(state: AgentState, kind: Box<dyn ErrorKind>) -> Self {
        Self {
            state: Some(Box::new(state)),
            kind,
        }
    }

    /// Prefer using stateful Error when possible.
    pub fn empty_state(kind: Box<dyn ErrorKind>) -> Self {
        Self { state: None, kind }
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

impl fmt::Display for dyn ErrorKind + '_ {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.reify())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind())
    }
}
