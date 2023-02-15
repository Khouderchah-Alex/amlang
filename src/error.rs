//! Representation of errors which can be reified in Amlang.
//!
//! General error mechanism that can use any ErrorKind. Reification allows for
//! Errors to be mapped to semantic content within Environments, even if not
//! part of the base implementation.

use std::fmt;

use serde::{de, ser};

use crate::agent::agent_frames::ExecFrame;
use crate::continuation::Continuation;
use crate::sexp::Sexp;


/// Creates a stateful Error wrapped in Err.
#[macro_export]
macro_rules! err {
    ($agent:expr, $($kind:tt)+) => {
        Err($crate::error::Error::with_cont(
            $agent.exec_state().clone(),
            Box::new($($kind)+),
        ))
    };
}


pub type ErrorCont = Continuation<ExecFrame>;

pub struct Error {
    cont: Option<ErrorCont>,
    kind: Box<dyn ErrorKind>,
}

pub trait ErrorKind: fmt::Debug /* fmt::Display auto-impled below */ {
    // Cannot use Reflective since we use ErrorKind as a trait object.
    fn reify(&self) -> Sexp;
}


impl Error {
    /// Prefer using err! for convenience.
    pub fn with_cont(cont: ErrorCont, kind: Box<dyn ErrorKind>) -> Self {
        Self {
            cont: Some(cont),
            kind,
        }
    }

    /// Prefer using stateful Error when possible.
    pub fn no_cont(kind: Box<dyn ErrorKind>) -> Self {
        Self { cont: None, kind }
    }

    pub fn kind(&self) -> &dyn ErrorKind {
        &*self.kind
    }

    pub fn cont(&self) -> Option<&ErrorCont> {
        self.cont.as_ref()
    }

    pub fn consume(self) -> Box<dyn ErrorKind> {
        self.kind
    }

    pub fn set_cont(&mut self, cont: ErrorCont) {
        self.cont = Some(cont)
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

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: ", self.kind())?;
        for frame in &self.cont {
            write!(f, "{:?}; ", frame)?;
        }
        Ok(())
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
