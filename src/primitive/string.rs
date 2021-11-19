use std::convert::TryFrom;
use std::fmt;

use super::Primitive;
use crate::sexp::{HeapSexp, Sexp};


#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LangString(String);

pub trait ToLangString {
    fn to_lang_string(&self) -> LangString;
}

impl LangString {
    pub fn new<S: ToString>(s: S) -> Self {
        Self(s.to_string())
    }

    pub fn unescape_char(c: char) -> char {
        match c {
            't' => '\t',
            'r' => '\r',
            'n' => '\n',
            // TODO(func) decode escape_unicodes for symmetry with escape_char.
            _ => c,
        }
    }

    pub fn escape_char(c: char) -> String {
        c.escape_default().to_string()
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn to_escaped(self) -> String {
        self.as_str().chars().map(LangString::escape_char).collect()
    }
}


impl<S: ToString + std::fmt::Display> ToLangString for S {
    fn to_lang_string(&self) -> LangString {
        LangString::new(self)
    }
}

impl fmt::Display for LangString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\"{}\"", self.as_str())
    }
}


impl_try_from!(LangString;
               Primitive         ->  LangString,
               Sexp              ->  LangString,
               HeapSexp          ->  LangString,
               ref Sexp          ->  ref LangString,
               Option<Sexp>      ->  LangString,
               Option<ref Sexp>  ->  ref LangString,
               Result<Sexp>      ->  LangString,
               Result<ref Sexp>  ->  ref LangString,
);
