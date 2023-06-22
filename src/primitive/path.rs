use std::convert::TryFrom;
use std::fmt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::Primitive;
use crate::sexp::{HeapSexp, Sexp};


#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct LangPath(PathBuf);

impl LangPath {
    pub fn new(path: PathBuf) -> Self {
        Self(path)
    }

    pub fn as_std_path(&self) -> &std::path::Path {
        self.0.as_path()
    }
}


impl fmt::Display for LangPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_std_path().to_string_lossy())
    }
}


impl From<String> for LangPath {
    fn from(s: String) -> Self {
        Self::new(PathBuf::from(s.to_string()))
    }
}


impl From<PathBuf> for LangPath {
    fn from(p: PathBuf) -> Self {
        Self::new(p)
    }
}

impl From<&Path> for LangPath {
    fn from(p: &Path) -> Self {
        Self::new(PathBuf::from(p))
    }
}

impl From<PathBuf> for Sexp {
    fn from(p: PathBuf) -> Self {
        Sexp::Primitive(Primitive::LangPath(LangPath::from(p)))
    }
}

impl From<&Path> for Sexp {
    fn from(p: &Path) -> Self {
        Sexp::Primitive(Primitive::LangPath(LangPath::from(p)))
    }
}


impl_try_from!(LangPath;
               Primitive         ->  LangPath,
               Sexp              ->  LangPath,
               HeapSexp          ->  LangPath,
               ref Sexp          ->  ref LangPath,
               Option<Sexp>      ->  LangPath,
               Option<ref Sexp>  ->  ref LangPath,
               Result<Sexp>      ->  LangPath,
               Result<ref Sexp>  ->  ref LangPath,
);
