use std::convert::TryFrom;
use std::fmt;
use std::path::PathBuf;

use super::Primitive;
use crate::sexp::{HeapSexp, Sexp};


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


impl_try_from!(Path;
               Primitive         ->  Path,
               Sexp              ->  Path,
               HeapSexp          ->  Path,
               ref Sexp          ->  ref Path,
               Option<Sexp>      ->  Path,
               Option<ref Sexp>  ->  ref Path,
               Result<Sexp>      ->  Path,
               Result<ref Sexp>  ->  ref Path,
);
