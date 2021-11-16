//! Representation of Amlang numbers.

use std::convert::TryFrom;
use std::fmt;
use std::ops;
use std::str;

use self::Number::*;
use super::Primitive;
use crate::sexp::{HeapSexp, Sexp};


#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Number {
    Integer(i64),
    Float(f64),
    // TODO fraction repr?
}

#[derive(Debug)]
pub struct ParseNumberError(String);


impl fmt::Display for Number {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Integer(i) => write!(f, "{}", i),
            Float(ff) => write!(f, "{}", ff),
        }
    }
}

impl str::FromStr for Number {
    type Err = ParseNumberError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let integer = s.parse::<i64>();
        if let Ok(int) = integer {
            return Ok(Integer(int));
        }

        let float = s.parse::<f64>();
        if let Ok(f) = float {
            return Ok(Float(f));
        }

        Err(ParseNumberError(s.to_string()))
    }
}

impl Default for Number {
    fn default() -> Self {
        Number::Integer(0)
    }
}

impl Into<f64> for Number {
    fn into(self) -> f64 {
        match self {
            Integer(i) => i as f64,
            Float(f) => f,
        }
    }
}

impl ops::AddAssign for Number {
    fn add_assign(&mut self, other: Self) {
        if let Integer(a) = self {
            if let Integer(b) = other {
                *a += b;
                return;
            }
        }

        if let Float(a) = self {
            let b: f64 = other.into();
            *a += b;
        } else if let Integer(a) = self {
            let a = *a as f64;
            let b: f64 = other.into();
            *self = Number::Float(a + b);
        }
    }
}

impl ops::SubAssign for Number {
    fn sub_assign(&mut self, other: Self) {
        if let Integer(a) = self {
            if let Integer(b) = other {
                *a -= b;
                return;
            }
        }

        if let Float(a) = self {
            let b: f64 = other.into();
            *a -= b;
        } else if let Integer(a) = self {
            let a = *a as f64;
            let b: f64 = other.into();
            *self = Number::Float(a - b);
        }
    }
}

impl ops::MulAssign for Number {
    fn mul_assign(&mut self, other: Self) {
        if let Integer(a) = self {
            if let Integer(b) = other {
                *a *= b;
                return;
            }
        }

        if let Float(a) = self {
            let b: f64 = other.into();
            *a *= b;
        } else if let Integer(a) = self {
            let a = *a as f64;
            let b: f64 = other.into();
            *self = Number::Float(a * b);
        }
    }
}

impl ops::DivAssign for Number {
    fn div_assign(&mut self, other: Self) {
        if let Float(a) = self {
            let b: f64 = other.into();
            *a /= b;
        } else if let Integer(a) = self {
            let a = *a as f64;
            let b: f64 = other.into();
            *self = Number::Float(a / b);
        }
    }
}


impl_try_from!(Number;
               Primitive         ->  Number,
               Sexp              ->  Number,
               HeapSexp          ->  Number,
               ref Sexp          ->  ref Number,
               Option<Sexp>      ->  Number,
               Option<ref Sexp>  ->  ref Number,
               Result<Sexp>      ->  Number,
               Result<ref Sexp>  ->  ref Number,
);
