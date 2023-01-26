//! Representation of errors which can be reified in Amlang.
//!
//! General error mechanism that can use any ErrorKind. Reification allows for
//! Errors to be mapped to semantic content within Environments, even if not
//! part of the base implementation.

use std::fmt;

use serde::{de, ser};

use crate::agent::Agent;
use crate::sexp::Sexp;


/// Creates a stateful Error wrapped in Err.
#[macro_export]
macro_rules! err {
    ($agent:expr, $($kind:tt)+) => {
        Err($crate::error::Error::with_agent(
            $agent.clone(),
            Box::new($($kind)+),
        ))
    };
}

#[derive(Debug)]
pub struct Error {
    agent: Option<Box<Agent>>,
    kind: Box<dyn ErrorKind>,
}

pub trait ErrorKind: fmt::Debug /* fmt::Display auto-impled below */ {
    // Cannot use Reflective since we use ErrorKind as a trait object.
    fn reify(&self) -> Sexp;
}


impl Error {
    /// Prefer using err! for convenience.
    pub fn with_agent(agent: Agent, kind: Box<dyn ErrorKind>) -> Self {
        Self {
            agent: Some(Box::new(agent)),
            kind,
        }
    }

    /// Prefer using stateful Error when possible.
    pub fn no_agent(kind: Box<dyn ErrorKind>) -> Self {
        Self { agent: None, kind }
    }

    pub fn kind(&self) -> &dyn ErrorKind {
        &*self.kind
    }

    pub fn agent(&self) -> Option<&Agent> {
        self.agent.as_ref().map(|a| &**a)
    }

    pub fn consume(self) -> Box<dyn ErrorKind> {
        self.kind
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


impl std::error::Error for Error {}

impl ser::Error for Error {
    fn custom<T: fmt::Display>(_msg: T) -> Self {
        panic!()
    }
}

impl de::Error for Error {
    fn custom<T: fmt::Display>(_msg: T) -> Self {
        panic!()
    }
}
