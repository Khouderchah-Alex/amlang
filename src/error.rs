//! Representation of errors which can be reified in Amlang.
//!
//! General error mechanism that can use any ErrorKind. Reification allows for
//! Errors to be mapped to semantic content within Environments, even if not
//! part of the base implementation.

use std::fmt;

use serde::{de, ser};

use crate::agent::agent_frames::ExecFrame;
use crate::continuation::Continuation;
use crate::sexp::{Cons, Sexp};


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

impl Error {
    /// Prefer using err! for convenience.
    pub fn with_cont(cont: ErrorCont, kind: Box<dyn ErrorKind>) -> Self {
        Self {
            cont: Some(cont),
            kind,
        }
    }

    /// Prefer using stateful Error when possible.
    pub fn no_cont<K: ErrorKind + 'static>(kind: K) -> Self {
        Self {
            cont: None,
            kind: Box::new(kind),
        }
    }

    /// Take existing error and wrap it in parent ErrorKind.
    pub fn wrap<K: ErrorKind + 'static>(self, parent: K) -> Self {
        let child = self.kind;
        let kind = Box::new(GenericError::Nested(Box::new(parent), child));
        Self {
            cont: self.cont,
            kind,
        }
    }

    pub fn adhoc<S: Into<String>, B: Into<Sexp>>(name: S, body: B) -> Self {
        let kind = Box::new(GenericError::AdHoc(name.into(), body.into()));
        Self { cont: None, kind }
    }

    pub fn wrap_adhoc<S: Into<String>, B: Into<Sexp>>(self, name: S, body: B) -> Self {
        let child = self.kind;
        let kind = Box::new(GenericError::Nested(
            Box::new(GenericError::AdHoc(name.into(), body.into())),
            child,
        ));
        Self {
            cont: self.cont,
            kind,
        }
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


pub trait ErrorKind: fmt::Debug /* fmt::Display auto-impled below */ {
    // Cannot use Reflective since we use ErrorKind as a trait object.
    fn reify(&self) -> Sexp;
}

#[derive(Debug)]
enum GenericError {
    AdHoc(String, Sexp),                            // (Error "class" name, body)
    Nested(Box<dyn ErrorKind>, Box<dyn ErrorKind>), // (Parent, child)
}

impl ErrorKind for GenericError {
    // TODO(func) Model within env rather than fall back on strings.
    fn reify(&self) -> Sexp {
        match self {
            Self::AdHoc(name, body) => {
                Cons::new(Sexp::from(name.clone()), Cons::new(body.clone(), None)).into()
            }
            Self::Nested(parent, child) => {
                Cons::new(parent.reify(), Cons::new(child.reify(), None)).into()
            }
        }
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
