use std::borrow::Borrow;
use std::convert::TryFrom;
use std::fmt;

use super::Primitive;
use crate::sexp::Sexp;


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
}

impl Symbol {
    pub fn new<S: AsRef<str>>(sym: S) -> SymbolResult {
        match sym.as_ref() {
            "+" | "-" | "*" | "/" => {}
            _ => {
                if !sym.as_ref().chars().all(|c| c.is_alphabetic() || c == '_') {
                    return Err(SymbolError::NonAlphabetic(sym.as_ref().to_string()));
                }
            }
        }

        Ok(Symbol(sym.as_ref().to_string()))
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

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
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
