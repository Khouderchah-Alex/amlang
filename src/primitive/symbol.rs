use std::borrow::Borrow;
use std::convert::TryFrom;
use std::fmt;

use super::Primitive;
use crate::sexp::{HeapSexp, Sexp};


/// String which can be used as an identifier (amlang designator).
///
/// Currently this means only alphabetic characters and underscore.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Symbol(String);

pub trait ToSymbol {
    fn to_symbol(&self) -> SymbolResult;

    fn to_symbol_or_panic(&self) -> Symbol {
        self.to_symbol().unwrap()
    }
}

pub type SymbolResult = Result<Symbol, SymbolError>;

#[derive(Debug)]
pub enum SymbolError {
    NonAlphabetic(String),
    EmptyString,
}

impl Symbol {
    pub fn new<S: AsRef<str>>(sym: S) -> SymbolResult {
        let s = sym.as_ref();
        if s.len() == 0 {
            return Err(SymbolError::EmptyString);
        }

        match s {
            "+" | "-" | "*" | "/" => {}
            _ => {
                if !s.chars().all(|c| c.is_alphabetic() || c == '_' || c == '-')
                    && !s
                        .chars()
                        .all(|c| c.is_ascii_digit() || c == '^' || c == 't')
                {
                    return Err(SymbolError::NonAlphabetic(s.to_string()));
                }
            }
        }

        Ok(Symbol(s.to_string()))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}


impl<S: AsRef<str>> ToSymbol for S {
    fn to_symbol(&self) -> SymbolResult {
        Symbol::new(self)
    }
}

impl ToSymbol for Symbol {
    fn to_symbol(&self) -> SymbolResult {
        Ok(self.clone())
    }
}

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[Symbol_{}]", self.0)
    }
}

impl Borrow<String> for Symbol {
    fn borrow(&self) -> &String {
        &self.0
    }
}

impl Borrow<str> for Symbol {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl TryFrom<Sexp> for Symbol {
    type Error = ();

    fn try_from(value: Sexp) -> Result<Self, Self::Error> {
        if let Sexp::Primitive(Primitive::Symbol(symbol)) = value {
            Ok(symbol)
        } else {
            Err(())
        }
    }
}

impl<'a> TryFrom<&'a Sexp> for &'a Symbol {
    type Error = ();

    fn try_from(value: &'a Sexp) -> Result<Self, Self::Error> {
        if let Sexp::Primitive(Primitive::Symbol(symbol)) = value {
            Ok(symbol)
        } else {
            Err(())
        }
    }
}

impl<'a> TryFrom<Option<&'a Sexp>> for &'a Symbol {
    type Error = ();

    fn try_from(value: Option<&'a Sexp>) -> Result<Self, Self::Error> {
        if let Some(Sexp::Primitive(Primitive::Symbol(symbol))) = value {
            Ok(symbol)
        } else {
            Err(())
        }
    }
}

impl TryFrom<Option<Sexp>> for Symbol {
    type Error = ();

    fn try_from(value: Option<Sexp>) -> Result<Self, Self::Error> {
        if let Some(Sexp::Primitive(Primitive::Symbol(symbol))) = value {
            Ok(symbol)
        } else {
            Err(())
        }
    }
}

impl TryFrom<Option<HeapSexp>> for Symbol {
    type Error = ();

    fn try_from(value: Option<HeapSexp>) -> Result<Self, Self::Error> {
        if let Some(heap) = value {
            if let Sexp::Primitive(Primitive::Symbol(symbol)) = *heap {
                return Ok(symbol);
            }
        }
        Err(())
    }
}

impl<E> TryFrom<Result<Sexp, E>> for Symbol {
    type Error = ();

    fn try_from(value: Result<Sexp, E>) -> Result<Self, Self::Error> {
        if let Ok(Sexp::Primitive(Primitive::Symbol(symbol))) = value {
            Ok(symbol)
        } else {
            Err(())
        }
    }
}

impl<'a, E> TryFrom<&'a Result<Sexp, E>> for &'a Symbol {
    type Error = ();

    fn try_from(value: &'a Result<Sexp, E>) -> Result<Self, Self::Error> {
        if let Ok(Sexp::Primitive(Primitive::Symbol(symbol))) = value {
            Ok(symbol)
        } else {
            Err(())
        }
    }
}
