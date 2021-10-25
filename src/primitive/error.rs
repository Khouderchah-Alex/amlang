use dyn_clone::DynClone;
use std::convert::TryFrom;
use std::fmt;

use super::Primitive;
use crate::agent::agent_state::AgentState;
use crate::sexp::{HeapSexp, Sexp};


#[derive(Clone, Debug)]
pub struct Error {
    state: Option<Box<AgentState>>,
    kind: Box<dyn ErrorKind>,
}

pub trait ErrorKind: fmt::Display + fmt::Debug + DynClone {
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


impl_try_from!(Sexp              ->  Error,      Error;
               HeapSexp          ->  Error,      Error;
               ref Sexp          ->  ref Error,  Error;
               Option<Sexp>      ->  Error,      Error;
               Option<ref Sexp>  ->  ref Error,  Error;
               Result<Sexp>      ->  Error,      Error;
               Result<ref Sexp>  ->  ref Error,  Error;
);

dyn_clone::clone_trait_object!(ErrorKind);
