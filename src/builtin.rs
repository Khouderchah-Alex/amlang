//! Creation of built-in environment.

use lazy_static::lazy_static;

use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt;

use crate::environment::Environment;
use crate::function::{Args, EvalErr, ExpectedCount, Func, Ret};
use crate::number::Number;
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

fn add(args: Args) -> Ret {
    let mut curr = Number::default();
    for arg in args {
        if let Sexp::Primitive(Primitive::Number(num)) = arg {
            curr += *num;
        } else {
            return Err(EvalErr::InvalidArgument {
                given: (*arg).clone(),
                expected: Cow::Borrowed("a Number"),
            });
        }
    }

    Ok(Sexp::Primitive(Primitive::Number(curr)))
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
        if let Sexp::Primitive(Primitive::Number(num)) = arg {
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

    Ok(Sexp::Primitive(Primitive::Number(curr)))
}

fn mul(args: Args) -> Ret {
    let mut curr = Number::Integer(1);
    for arg in args {
        if let Sexp::Primitive(Primitive::Number(num)) = arg {
            curr *= *num;
        } else {
            return Err(EvalErr::InvalidArgument {
                given: (*arg).clone(),
                expected: Cow::Borrowed("a Number"),
            });
        }
    }

    Ok(Sexp::Primitive(Primitive::Number(curr)))
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
        if let Sexp::Primitive(Primitive::Number(num)) = arg {
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

    Ok(Sexp::Primitive(Primitive::Number(curr)))
}
