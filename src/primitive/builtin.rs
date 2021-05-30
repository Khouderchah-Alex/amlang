//! Representation of builtin methods.

use std::convert::TryFrom;
use std::fmt;

use crate::function::{Args, Func, Ret};
use crate::primitive::Primitive;
use crate::sexp::Sexp;


#[derive(Clone, Copy)]
pub struct BuiltIn {
    name: &'static str,
    fun: fn(Args) -> Ret,
}

impl BuiltIn {
    pub fn new(name: &'static str, fun: fn(Args) -> Ret) -> BuiltIn {
        BuiltIn { name, fun }
    }

    pub fn name(&self) -> &'static str {
        self.name
    }
}


impl Func for BuiltIn {
    fn call(&self, args: Args) -> Ret {
        (self.fun)(args)
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

impl TryFrom<Sexp> for BuiltIn {
    type Error = ();

    fn try_from(value: Sexp) -> Result<Self, Self::Error> {
        if let Sexp::Primitive(Primitive::BuiltIn(builtin)) = value {
            Ok(builtin)
        } else {
            Err(())
        }
    }
}

impl<'a> TryFrom<&'a Sexp> for &'a BuiltIn {
    type Error = ();

    fn try_from(value: &'a Sexp) -> Result<Self, Self::Error> {
        if let Sexp::Primitive(Primitive::BuiltIn(builtin)) = value {
            Ok(builtin)
        } else {
            Err(())
        }
    }
}

impl<'a> TryFrom<Option<&'a Sexp>> for &'a BuiltIn {
    type Error = ();

    fn try_from(value: Option<&'a Sexp>) -> Result<Self, Self::Error> {
        if let Some(Sexp::Primitive(Primitive::BuiltIn(builtin))) = value {
            Ok(builtin)
        } else {
            Err(())
        }
    }
}

impl<E> TryFrom<Result<Sexp, E>> for BuiltIn {
    type Error = ();

    fn try_from(value: Result<Sexp, E>) -> Result<Self, Self::Error> {
        if let Ok(Sexp::Primitive(Primitive::BuiltIn(builtin))) = value {
            Ok(builtin)
        } else {
            Err(())
        }
    }
}

impl<'a, E> TryFrom<&'a Result<Sexp, E>> for &'a BuiltIn {
    type Error = ();

    fn try_from(value: &'a Result<Sexp, E>) -> Result<Self, Self::Error> {
        if let Ok(Sexp::Primitive(Primitive::BuiltIn(builtin))) = value {
            Ok(builtin)
        } else {
            Err(())
        }
    }
}
