use std::convert::TryFrom;
use std::fmt;
use std::path::PathBuf;

use super::Primitive;
use crate::sexp::{HeapSexp, Sexp};


/// Path which can be used as an identifier (amlang designator).
///
/// Currently this means only alphabetic characters and underscore.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Path(PathBuf);

impl Path {
    pub fn new(path: PathBuf) -> Self {
        Self(path)
    }

    pub fn as_std_path(&self) -> &std::path::Path {
        self.0.as_path()
    }
}


impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[Path_{}]", self.as_std_path().to_string_lossy())
    }
}

impl TryFrom<Sexp> for Path {
    type Error = ();

    fn try_from(value: Sexp) -> Result<Self, Self::Error> {
        if let Sexp::Primitive(Primitive::Path(symbol)) = value {
            Ok(symbol)
        } else {
            Err(())
        }
    }
}

impl<'a> TryFrom<&'a Sexp> for &'a Path {
    type Error = ();

    fn try_from(value: &'a Sexp) -> Result<Self, Self::Error> {
        if let Sexp::Primitive(Primitive::Path(symbol)) = value {
            Ok(symbol)
        } else {
            Err(())
        }
    }
}

impl<'a> TryFrom<Option<&'a Sexp>> for &'a Path {
    type Error = ();

    fn try_from(value: Option<&'a Sexp>) -> Result<Self, Self::Error> {
        if let Some(Sexp::Primitive(Primitive::Path(symbol))) = value {
            Ok(symbol)
        } else {
            Err(())
        }
    }
}

impl TryFrom<Option<Sexp>> for Path {
    type Error = ();

    fn try_from(value: Option<Sexp>) -> Result<Self, Self::Error> {
        if let Some(Sexp::Primitive(Primitive::Path(symbol))) = value {
            Ok(symbol)
        } else {
            Err(())
        }
    }
}

impl TryFrom<Option<HeapSexp>> for Path {
    type Error = ();

    fn try_from(value: Option<HeapSexp>) -> Result<Self, Self::Error> {
        if let Some(heap) = value {
            if let Sexp::Primitive(Primitive::Path(symbol)) = *heap {
                return Ok(symbol);
            }
        }
        Err(())
    }
}

impl<E> TryFrom<Result<Sexp, E>> for Path {
    type Error = ();

    fn try_from(value: Result<Sexp, E>) -> Result<Self, Self::Error> {
        if let Ok(Sexp::Primitive(Primitive::Path(symbol))) = value {
            Ok(symbol)
        } else {
            Err(())
        }
    }
}

impl<'a, E> TryFrom<&'a Result<Sexp, E>> for &'a Path {
    type Error = ();

    fn try_from(value: &'a Result<Sexp, E>) -> Result<Self, Self::Error> {
        if let Ok(Sexp::Primitive(Primitive::Path(symbol))) = value {
            Ok(symbol)
        } else {
            Err(())
        }
    }
}
