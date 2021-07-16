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

    pub fn as_str(&self) -> &str {
        self.0.as_str()
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
