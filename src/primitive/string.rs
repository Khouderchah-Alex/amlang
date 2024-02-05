use std::convert::TryFrom;
use std::fmt;

use serde::{Deserialize, Serialize};

use super::Primitive;
use crate::sexp::{HeapSexp, Sexp};


#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct LangString(String);

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


impl fmt::Display for LangString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\"{}\"", self.as_str())
    }
}

impl AsRef<str> for LangString {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl From<LangString> for String {
    fn from(s: LangString) -> Self {
        s.0
    }
}

impl From<&str> for LangString {
    fn from(s: &str) -> Self {
        LangString::new(s.to_string())
    }
}

impl From<&str> for Sexp {
    fn from(s: &str) -> Self {
        LangString::from(s).into()
    }
}

impl From<&str> for HeapSexp {
    fn from(s: &str) -> Self {
        Sexp::from(s).into()
    }
}

impl From<String> for LangString {
    fn from(s: String) -> Self {
        LangString::new(s)
    }
}

impl From<String> for Sexp {
    fn from(s: String) -> Self {
        LangString::from(s).into()
    }
}

impl From<String> for HeapSexp {
    fn from(s: String) -> Self {
        Sexp::from(s).into()
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
