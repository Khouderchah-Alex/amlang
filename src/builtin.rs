//! Creation of built-in environment.

use lazy_static::lazy_static;

use std::borrow::Cow;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::fmt;

use crate::function::{Args, EvalErr, ExpectedCount, Func, Ret};
use crate::number::Number;
use crate::old_environment::Environment;
use crate::primitive::Primitive;
use crate::sexp::Sexp;

macro_rules! builtins {
    [$($n:tt : $x:expr),*] => {
        {
            let mut m = HashMap::new();
            $(
                m.insert(
                    $n.to_string(),
                    BuiltIn {
                        name: stringify!($x),
                        fun: $x,
                    },
                );
            )*
                Environment::new(m)
        }
    };
    [$($n:tt : $x:expr),+ ,] => {
        builtins![$($n : $x),*]
    };
}

lazy_static! {
    pub static ref BUILTINS: Environment<BuiltIn> =
        builtins!["+": add, "-": sub, "*": mul, "/": div,];
}

pub struct BuiltIn {
    name: &'static str,
    fun: fn(Args) -> Ret,
}


fn add(args: Args) -> Ret {
    let mut curr = Number::default();
    for arg in args {
        if let Ok(num) = <&Number>::try_from(arg) {
            curr += *num;
        } else {
            return Err(EvalErr::InvalidArgument {
                given: (*arg).clone(),
                expected: Cow::Borrowed("a Number"),
            });
        }
    }

    Ok(curr.into())
}

fn sub(args: Args) -> Ret {
    if args.len() < 1 {
        return Err(EvalErr::WrongArgumentCount {
            given: 0,
            expected: ExpectedCount::AtLeast(1),
        });
    }

    let mut curr = Number::default();
    let mut first = true;
    for arg in args {
        if let Ok(num) = <&Number>::try_from(arg) {
            if first {
                curr = *num;
                first = false;
            } else {
                curr -= *num;
            }
        } else {
            return Err(EvalErr::InvalidArgument {
                given: (*arg).clone(),
                expected: Cow::Borrowed("a Number"),
            });
        }
    }

    Ok(curr.into())
}

fn mul(args: Args) -> Ret {
    let mut curr = Number::Integer(1);
    for arg in args {
        if let Ok(num) = <&Number>::try_from(arg) {
            curr *= *num;
        } else {
            return Err(EvalErr::InvalidArgument {
                given: (*arg).clone(),
                expected: Cow::Borrowed("a Number"),
            });
        }
    }

    Ok(curr.into())
}

fn div(args: Args) -> Ret {
    if args.len() < 1 {
        return Err(EvalErr::WrongArgumentCount {
            given: 0,
            expected: ExpectedCount::AtLeast(1),
        });
    }

    let mut curr = Number::default();
    let mut first = true;
    for arg in args {
        if let Ok(num) = <&Number>::try_from(arg) {
            if first {
                curr = *num;
                first = false;
            } else {
                curr /= *num;
            }
        } else {
            return Err(EvalErr::InvalidArgument {
                given: (*arg).clone(),
                expected: Cow::Borrowed("a Number"),
            });
        }
    }

    Ok(curr.into())
}


impl Func for BuiltIn {
    fn call(&self, args: Args) -> Ret {
        (self.fun)(args)
    }
}

impl PartialEq for BuiltIn {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl fmt::Debug for BuiltIn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[BUILTIN_{} @ {:p}]", self.name, &self.fun)
    }
}

impl fmt::Display for BuiltIn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[BUILTIN_{}]", self.name)
    }
}

impl<'a> TryFrom<Sexp> for &'a BuiltIn {
    type Error = ();

    fn try_from(value: Sexp) -> Result<Self, Self::Error> {
        if let Sexp::Primitive(Primitive::BuiltIn(builtin)) = value {
            Ok(builtin)
        } else {
            Err(())
        }
    }
}

impl<'a> TryFrom<&'a Sexp> for &'a BuiltIn {
    type Error = ();

    fn try_from(value: &'a Sexp) -> Result<Self, Self::Error> {
        if let Sexp::Primitive(Primitive::BuiltIn(builtin)) = value {
            Ok(builtin)
        } else {
            Err(())
        }
    }
}

impl<'a> TryFrom<Option<&'a Sexp>> for &'a BuiltIn {
    type Error = ();

    fn try_from(value: Option<&'a Sexp>) -> Result<Self, Self::Error> {
        if let Some(Sexp::Primitive(Primitive::BuiltIn(builtin))) = value {
            Ok(builtin)
        } else {
            Err(())
        }
    }
}
