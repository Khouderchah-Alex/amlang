//! Representation of Amlang numbers.

use std::convert::TryFrom;
use std::{fmt, mem, ops, str};

use serde::{Deserialize, Serialize};

use self::Number::*;
use super::Primitive;
use crate::sexp::{HeapSexp, Sexp};


generate_number!(
    I8: i8,
    I16: i16,
    I32: i32,
    I64: i64,
    ISize: isize,
    U8: u8,
    U16: u16,
    U32: u32,
    U64: u64,
    USize: usize,
    F32: f32,
    F64: f64,
);

macro_rules! generate_number {
    (
        $($variant:ident : $type:ident),+$(,)?
    ) => {
        #[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
        pub enum Number {
            $($variant($type),)+
        }

        impl ops::AddAssign for Number {
            fn add_assign(&mut self, other: Self) {
                let self_d = mem::discriminant(&*self);
                let other_d = mem::discriminant(&other);
                if self_d != other_d {
                    panic!();
                }

                match (self, &other) {
                    $(
                        ($variant(this), &$variant(ref that)) => *this += that,
                    )+
                    _ => panic!()
                }
            }
        }

        impl ops::SubAssign for Number {
            fn sub_assign(&mut self, other: Self) {
                let self_d = mem::discriminant(&*self);
                let other_d = mem::discriminant(&other);
                if self_d != other_d {
                    panic!();
                }

                match (self, &other) {
                    $(
                        ($variant(this), &$variant(ref that)) => *this -= that,
                    )+
                    _ => panic!()
                }
            }
        }

        impl ops::MulAssign for Number {
            fn mul_assign(&mut self, other: Self) {
                let self_d = mem::discriminant(&*self);
                let other_d = mem::discriminant(&other);
                if self_d != other_d {
                    panic!();
                }

                match (self, &other) {
                    $(
                        ($variant(this), &$variant(ref that)) => *this *= that,
                    )+
                    _ => panic!()
                }
            }
        }

        impl ops::DivAssign for Number {
            fn div_assign(&mut self, other: Self) {
                let self_d = mem::discriminant(&*self);
                let other_d = mem::discriminant(&other);
                if self_d != other_d {
                    panic!();
                }

                match (self, &other) {
                    $(
                        ($variant(this), &$variant(ref that)) => *this /= that,
                    )+
                    _ => panic!()
                }
            }
        }

        $(
            impl TryFrom<Sexp> for $type {
                type Error = Sexp;

                fn try_from(value: Sexp) -> Result<Self, Self::Error> {
                    let num = Number::try_from(value)?;
                    if let Number::$variant(val) = num {
                        Ok(val)
                    } else {
                        Err(num.into())
                    }
                }
            }

            impl TryFrom<Number> for $type {
                type Error = Number;

                fn try_from(value: Number) -> Result<Self, Self::Error> {
                    if let Number::$variant(val) = value {
                        Ok(val)
                    } else {
                        Err(value)
                    }
                }
            }

            impl From<$type> for Sexp {
                fn from(elem: $type) -> Self {
                    Sexp::Primitive(Primitive::Number(Number::$variant(elem)))
                }
            }

            impl From<$type> for HeapSexp {
                fn from(elem: $type) -> Self {
                    Sexp::from(elem).into()
                }
            }
        )+

    };
}

#[derive(Debug)]
pub struct ParseNumberError(String);


impl str::FromStr for Number {
    type Err = ParseNumberError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let integer = s.parse::<i64>();
        if let Ok(int) = integer {
            return Ok(I64(int));
        }

        let float = s.parse::<f64>();
        if let Ok(f) = float {
            return Ok(F64(f));
        }

        Err(ParseNumberError(s.to_string()))
    }
}


impl fmt::Display for Number {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            I8(val) => write!(f, "{}", val),
            I16(val) => write!(f, "{}", val),
            I32(val) => write!(f, "{}", val),
            I64(val) => write!(f, "{}", val),
            ISize(val) => write!(f, "{}", val),
            U8(val) => write!(f, "{}", val),
            U16(val) => write!(f, "{}", val),
            U32(val) => write!(f, "{}", val),
            U64(val) => write!(f, "{}", val),
            USize(val) => write!(f, "{}", val),
            F32(val) => write!(f, "{}", val),
            F64(val) => write!(f, "{}", val),
        }
    }
}

impl fmt::Debug for Number {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            I8(val) => write!(f, "{}i8", val),
            I16(val) => write!(f, "{}i16", val),
            I32(val) => write!(f, "{}i32", val),
            I64(val) => write!(f, "{}i64", val),
            ISize(val) => write!(f, "{}isize", val),
            U8(val) => write!(f, "{}u8", val),
            U16(val) => write!(f, "{}u16", val),
            U32(val) => write!(f, "{}u32", val),
            U64(val) => write!(f, "{}u64", val),
            USize(val) => write!(f, "{}usize", val),
            F32(val) => write!(f, "{}f32", val),
            F64(val) => write!(f, "{}f64", val),
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

use generate_number;
