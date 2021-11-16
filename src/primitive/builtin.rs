//! Representation of builtin methods.

use std::convert::TryFrom;
use std::fmt;

use crate::agent::Agent;
use crate::error::Error;
use crate::primitive::Primitive;
use crate::sexp::{HeapSexp, Sexp};


pub type Args = Vec<Sexp>;

#[derive(Clone, Copy)]
pub struct BuiltIn {
    name: &'static str,
    fun: fn(Args, &mut Agent) -> Result<Sexp, Error>,
}

impl BuiltIn {
    pub fn new(name: &'static str, fun: fn(Args, &mut Agent) -> Result<Sexp, Error>) -> BuiltIn {
        BuiltIn { name, fun }
    }

    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn call(&self, args: Args, agent: &mut Agent) -> Result<Sexp, Error> {
        (self.fun)(args, agent)
    }
}

impl PartialEq for BuiltIn {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl fmt::Debug for BuiltIn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[BUILTIN_{} @ {:p}]", self.name, &self.fun)
    }
}

impl fmt::Display for BuiltIn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[BUILTIN_{}]", self.name)
    }
}


impl_try_from!(BuiltIn;
               Primitive         ->  BuiltIn,
               Sexp              ->  BuiltIn,
               HeapSexp          ->  BuiltIn,
               ref Sexp          ->  ref BuiltIn,
               Option<Sexp>      ->  BuiltIn,
               Option<ref Sexp>  ->  ref BuiltIn,
               Result<Sexp>      ->  BuiltIn,
               Result<ref Sexp>  ->  ref BuiltIn,
);
