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
    fn to_symbol<Info, P>(&self, policy: P) -> Result<Symbol, SymbolError>
    where
        P: Fn(&str) -> Result<Info, SymbolError>;

    fn to_symbol_or_panic<Info, P>(&self, policy: P) -> Symbol
    where
        P: Fn(&str) -> Result<Info, SymbolError>,
    {
        self.to_symbol(policy).unwrap()
    }
}

#[derive(Debug)]
pub enum SymbolError {
    NonAlphabetic(String),
    DunderPrefix(String),
    InvalidNodeSpec(String),
    EmptyString,
}

impl Symbol {
    pub fn try_policy<S, Info, P>(sym: S, policy: P) -> Result<(Symbol, Info), SymbolError>
    where
        S: AsRef<str>,
        P: Fn(&str) -> Result<Info, SymbolError>,
    {
        let s = sym.as_ref();
        if s.len() == 0 {
            return Err(SymbolError::EmptyString);
        }

        let info = policy(s)?;
        Ok((Symbol(s.to_string()), info))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}


impl<S: AsRef<str>> ToSymbol for S {
    fn to_symbol<Info, P>(&self, policy: P) -> Result<Symbol, SymbolError>
    where
        P: Fn(&str) -> Result<Info, SymbolError>,
    {
        let (sym, _info) = Symbol::try_policy(self, policy)?;
        Ok(sym)
    }
}

impl ToSymbol for Symbol {
    fn to_symbol<Info, P>(&self, policy: P) -> Result<Symbol, SymbolError>
    where
        P: Fn(&str) -> Result<Info, SymbolError>,
    {
        let (sym, _info) = Symbol::try_policy(self.as_str(), policy)?;
        Ok(sym)
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


impl_try_from!(Symbol;
               Sexp              ->  Symbol,
               HeapSexp          ->  Symbol,
               ref Sexp          ->  ref Symbol,
               Option<Sexp>      ->  Symbol,
               Option<ref Sexp>  ->  ref Symbol,
               Result<Sexp>      ->  Symbol,
               Result<ref Sexp>  ->  ref Symbol,
);
