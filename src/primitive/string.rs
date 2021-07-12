use std::convert::TryFrom;
use std::fmt;

use super::Primitive;
use crate::sexp::{HeapSexp, Sexp};


#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct AmString(String);

impl AmString {
    pub fn new<S: AsRef<str>>(s: S) -> Self {
        Self(s.as_ref().to_string())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}


impl fmt::Display for AmString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\"{}\"", self.as_str())
    }
}

impl TryFrom<Sexp> for AmString {
    type Error = ();

    fn try_from(value: Sexp) -> Result<Self, Self::Error> {
        if let Sexp::Primitive(Primitive::AmString(string)) = value {
            Ok(string)
        } else {
            Err(())
        }
    }
}

impl<'a> TryFrom<&'a Sexp> for &'a AmString {
    type Error = ();

    fn try_from(value: &'a Sexp) -> Result<Self, Self::Error> {
        if let Sexp::Primitive(Primitive::AmString(string)) = value {
            Ok(string)
        } else {
            Err(())
        }
    }
}

impl<'a> TryFrom<Option<&'a Sexp>> for &'a AmString {
    type Error = ();

    fn try_from(value: Option<&'a Sexp>) -> Result<Self, Self::Error> {
        if let Some(Sexp::Primitive(Primitive::AmString(string))) = value {
            Ok(string)
        } else {
            Err(())
        }
    }
}

impl TryFrom<Option<Sexp>> for AmString {
    type Error = ();

    fn try_from(value: Option<Sexp>) -> Result<Self, Self::Error> {
        if let Some(Sexp::Primitive(Primitive::AmString(string))) = value {
            Ok(string)
        } else {
            Err(())
        }
    }
}

impl TryFrom<Option<HeapSexp>> for AmString {
    type Error = ();

    fn try_from(value: Option<HeapSexp>) -> Result<Self, Self::Error> {
        if let Some(heap) = value {
            if let Sexp::Primitive(Primitive::AmString(string)) = *heap {
                return Ok(string);
            }
        }
        Err(())
    }
}

impl<E> TryFrom<Result<Sexp, E>> for AmString {
    type Error = ();

    fn try_from(value: Result<Sexp, E>) -> Result<Self, Self::Error> {
        if let Ok(Sexp::Primitive(Primitive::AmString(string))) = value {
            Ok(string)
        } else {
            Err(())
        }
    }
}

impl<'a, E> TryFrom<&'a Result<Sexp, E>> for &'a AmString {
    type Error = ();

    fn try_from(value: &'a Result<Sexp, E>) -> Result<Self, Self::Error> {
        if let Ok(Sexp::Primitive(Primitive::AmString(string))) = value {
            Ok(string)
        } else {
            Err(())
        }
    }
}
