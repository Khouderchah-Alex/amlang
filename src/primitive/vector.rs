use std::convert::TryFrom;
use std::fmt;

use serde::{Deserialize, Serialize};

use super::Primitive;
use crate::sexp::{HeapSexp, Sexp};


#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Vector(Vec<Sexp>);


impl fmt::Display for Vector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;
        let mut is_first = true;
        for elem in &self.0 {
            if is_first {
                is_first = false;
            } else {
                write!(f, " ")?;
            }
            write!(f, "{}", elem)?;
        }
        write!(f, "]")
    }
}


impl_try_from!(Vector;
               Primitive         ->  Vector,
               Sexp              ->  Vector,
               HeapSexp          ->  Vector,
               ref Sexp          ->  ref Vector,
               Option<Sexp>      ->  Vector,
               Option<ref Sexp>  ->  ref Vector,
               Result<Sexp>      ->  Vector,
               Result<ref Sexp>  ->  ref Vector,
);
