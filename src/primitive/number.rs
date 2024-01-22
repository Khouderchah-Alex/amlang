//! Representation of Amlang numbers.

use std::convert::TryFrom;
use std::{fmt, mem, ops, str};

use serde::{Deserialize, Serialize};

use self::Number::*;
use super::Primitive;
use crate::sexp::{HeapSexp, Sexp};


generate_number!(
    (
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
    ),
    (
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
    ),
    (F32: f32, F64: f64,),
);

macro_rules! generate_number {
    (
        ($($variant:ident : $type:ident),+$(,)?),
        ($($ivariant:ident : $itype:ident),+$(,)?),
        ($($fvariant:ident : $ftype:ident),+$(,)?),
    ) => {
        #[derive(Clone, Copy, Serialize, Deserialize)]
        pub enum Number {
            $($variant($type),)+
            GenericInt(i128),
        }

        impl Number {
            /// Create a generic numeric type.
            ///
            /// Prefer over Number::from.
            ///
            /// Since unconstrained int literals will be coerced into an i32,
            /// Number::from(2) will be an I32, not GenericInt.
            pub fn generic<I: Into<Self>>(num: I) -> Self {
                match num.into() {
                    f @ Self::F32(_) | f @ Self::F64(_) => f,
                    $(Self::$ivariant(i) => GenericInt(i as i128),)+
                    g @ GenericInt(_) => g,
                }
            }

            fn pre_op(&mut self, other: Self) -> Self {
                if let Some((_, other)) = self.matching_types(other) {
                    return other;
                } else {
                    panic!("Mismatching number types");
                }
            }
        }

        impl ops::AddAssign for Number {
            fn add_assign(&mut self, other: Self) {
                let other = self.pre_op(other);
                match (self, &other) {
                    $(
                        ($variant(this), &$variant(ref that)) => *this += that,
                    )+
                    (GenericInt(this), &GenericInt(ref that)) => *this += that,
                    _ => panic!()
                }
            }
        }

        impl ops::SubAssign for Number {
            fn sub_assign(&mut self, other: Self) {
                let other = self.pre_op(other);
                match (self, &other) {
                    $(
                        ($variant(this), &$variant(ref that)) => *this -= that,
                    )+
                    (GenericInt(this), &GenericInt(ref that)) => *this -= that,
                    _ => panic!()
                }
            }
        }

        impl ops::MulAssign for Number {
            fn mul_assign(&mut self, other: Self) {
                let other = self.pre_op(other);
                match (self, &other) {
                    $(
                        ($variant(this), &$variant(ref that)) => *this *= that,
                    )+
                    (GenericInt(this), &GenericInt(ref that)) => *this *= that,
                    _ => panic!()
                }
            }
        }

        impl ops::DivAssign for Number {
            fn div_assign(&mut self, other: Self) {
                let other = self.pre_op(other);
                match (self, &other) {
                    $(
                        ($variant(this), &$variant(ref that)) => *this /= that,
                    )+
                    (GenericInt(this), &GenericInt(ref that)) => *this /= that,
                    _ => panic!()
                }
            }
        }

        $(
            impl TryFrom<Number> for $itype {
                type Error = Number;

                fn try_from(value: Number) -> Result<Self, Self::Error> {
                    if let Number::$ivariant(val) = value {
                        Ok(val)
                    } else if let Number::GenericInt(i) = value {
                        if let Ok(v) = $itype::try_from(i) {
                            Ok(v)
                        } else {
                            Err(value)
                        }
                    } else {
                        Err(value)
                    }
                }
            }
        )+
        $(
            impl TryFrom<Number> for $ftype {
                type Error = Number;

                fn try_from(value: Number) -> Result<Self, Self::Error> {
                    if let Number::$fvariant(val) = value {
                        Ok(val)
                    } else {
                        Err(value)
                    }
                }
            }
        )+
        $(
            impl TryFrom<Sexp> for $type {
                type Error = Sexp;

                fn try_from(value: Sexp) -> Result<Self, Self::Error> {
                    let num = Number::try_from(value)?;
                    if let Ok(val) = $type::try_from(num) {
                        Ok(val)
                    } else {
                        Err(num.into())
                    }
                }
            }

            impl From<$type> for Number {
                fn from(elem: $type) -> Self {
                    Number::$variant(elem)
                }
            }

            impl From<$type> for Sexp {
                fn from(elem: $type) -> Self {
                    Sexp::Primitive(Primitive::Number(elem.into()))
                }
            }

            impl From<$type> for HeapSexp {
                fn from(elem: $type) -> Self {
                    Sexp::from(elem).into()
                }
            }
        )+

        impl Number {
            fn matching_types(&mut self, other: Number) -> Option<(&mut Number, Number)> {
                let self_d = mem::discriminant(&*self);
                let other_d = mem::discriminant(&other);
                if self_d == other_d {
                    Some((self, other))
                } else {
                    match (&*self, &other) {
                        $((Number::GenericInt(ref this), Number::$ivariant(ref _that)) => {
                            if let Ok(i) = $itype::try_from(*this) {
                                *self = Number::$ivariant(i);
                                Some((self, other))
                            } else {
                                None
                            }
                        })+
                        $((Number::$ivariant(ref _this), Number::GenericInt(ref that)) => {
                            if let Ok(i) = $itype::try_from(*that) {
                                Some((self, Number::$ivariant(i)))
                            } else {
                                None
                            }
                         })+
                         _ => None
                    }
                }
            }
        }

        impl PartialEq for Number {
            #[inline]
            fn eq(&self, other: &Number) -> bool {
                let self_d = mem::discriminant(&*self);
                let other_d = mem::discriminant(&*other);
                if self_d == other_d {
                    match (&*self, &*other) {
                        $((&Number::$variant(ref this), &Number::$variant(ref that)) => {
                            (*this) == (*that)
                        })+
                        (Number::GenericInt(ref this), Number::GenericInt(ref that)) => {
                            (*this) == (*that)
                        }
                        _ => {
                            panic!();
                        }
                    }
                } else {
                    match (&*self, &*other) {
                        $((Number::GenericInt(ref this), Number::$ivariant(ref that)) => {
                            if let Ok(i) = $itype::try_from(*this) {
                                i == (*that)
                            } else {
                                false
                            }
                        })+
                        $((Number::$ivariant(ref this), Number::GenericInt(ref that)) => {
                            if let Ok(i) = $itype::try_from(*that) {
                                i == (*this)
                            } else {
                                false
                            }
                        })+
                        _ => false
                    }
                }
            }
        }
    };
}

#[derive(Debug)]
pub struct ParseNumberError(String);


impl str::FromStr for Number {
    type Err = ParseNumberError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let integer = s.parse::<i128>();
        if let Ok(int) = integer {
            return Ok(GenericInt(int));
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
            GenericInt(val) => write!(f, "{}", val),
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
            GenericInt(val) => write!(f, "{}_", val),
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
