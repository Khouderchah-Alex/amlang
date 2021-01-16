//! Creation of built-in environment.

use lazy_static::lazy_static;

use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt;

use crate::environment::Environment;
use crate::function::{Args, EvalErr, Func, Ret};
use crate::sexp::{Atom, Value};

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
}

lazy_static! {
    pub static ref BUILTINS: Environment<BuiltIn> = builtins!["+": add, "-": sub, "*": mul];
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
    let mut curr: i64 = 0;
    for arg in args {
        if let Value::Atom(Atom::Integer(i)) = arg {
            curr += i;
        } else {
            return Err(EvalErr::InvalidArgument {
                given: (*arg).clone(),
                expected: Cow::Borrowed("an integer"),
            });
        }
    }

    Ok(Value::Atom(Atom::Integer(curr)))
}

fn sub(args: Args) -> Ret {
    if args.len() < 1 {
        return Err(EvalErr::MissingArguments {
            given: 0,
            expected: 1,
        });
    }

    let mut curr: i64 = 0;
    let mut first = true;
    for arg in args {
        if let Value::Atom(Atom::Integer(i)) = arg {
            if first {
                curr = *i;
                first = false;
            } else {
                curr -= i;
            }
        } else {
            return Err(EvalErr::InvalidArgument {
                given: (*arg).clone(),
                expected: Cow::Borrowed("an integer"),
            });
        }
    }

    Ok(Value::Atom(Atom::Integer(curr)))
}

fn mul(args: Args) -> Ret {
    let mut curr: i64 = 1;
    for arg in args {
        if let Value::Atom(Atom::Integer(i)) = arg {
            curr *= i;
        } else {
            return Err(EvalErr::InvalidArgument {
                given: (*arg).clone(),
                expected: Cow::Borrowed("an integer"),
            });
        }
    }

    Ok(Value::Atom(Atom::Integer(curr)))
}
