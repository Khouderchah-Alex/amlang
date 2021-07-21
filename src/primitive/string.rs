use std::convert::TryFrom;
use std::fmt;

use super::Primitive;
use crate::sexp::Sexp;


#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct AmString(String);

impl AmString {
    pub fn new<S: AsRef<str>>(s: S) -> Self {
        Self(s.as_ref().to_string())
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
        self.as_str().chars().map(AmString::escape_char).collect()
    }
}


impl fmt::Display for AmString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\"{}\"", self.as_str())
    }
}


impl_try_from!(Sexp              ->  AmString,      AmString;
               ref Sexp          ->  ref AmString,  AmString;
               Option<Sexp>      ->  AmString,      AmString;
               Option<ref Sexp>  ->  ref AmString,  AmString;
               Result<Sexp>      ->  AmString,      AmString;
               Result<ref Sexp>  ->  ref AmString,  AmString;
);
