use std::borrow::Cow;
use std::collections::HashMap;

use crate::lang_err::ExpectedCount;
use crate::model::Ret;
use crate::primitive::builtin::Args;
use crate::primitive::{BuiltIn, Continuation, Number, Primitive};
use crate::sexp::{self, Sexp};


macro_rules! builtins {
    [$($x:expr),*] => {
        {
            let mut m = HashMap::new();
            $(
                m.insert(
                    stringify!($x),
                    BuiltIn::new(stringify!($x), $x),
                );
            )*
            m
        }
    };
    [$($n:tt : $x:expr),+ ,] => {
        builtins![$($n : $x),*]
    };
}

// Used for bootstrapping and auxiliary purposes, not as an environment.
pub fn generate_builtin_map() -> HashMap<&'static str, BuiltIn> {
    builtins![add, sub, mul, div, car, cdr, cons]
}


pub fn add(args: Args, cont: &Continuation) -> Ret {
    let mut curr = Number::default();
    for arg in args {
        if let Sexp::Primitive(Primitive::Number(num)) = arg {
            curr += num;
        } else {
            return err_ctx!(
                cont,
                InvalidArgument {
                    given: arg.clone(),
                    expected: Cow::Borrowed("a Number"),
                }
            );
        }
    }

    Ok(curr.into())
}

pub fn sub(args: Args, cont: &Continuation) -> Ret {
    if args.len() < 1 {
        return err_ctx!(
            cont,
            WrongArgumentCount {
                given: 0,
                expected: ExpectedCount::AtLeast(1),
            }
        );
    }

    let mut curr = Number::default();
    let mut first = true;
    for arg in args {
        if let Sexp::Primitive(Primitive::Number(num)) = arg {
            if first {
                curr = num;
                first = false;
            } else {
                curr -= num;
            }
        } else {
            return err_ctx!(
                cont,
                InvalidArgument {
                    given: arg.clone(),
                    expected: Cow::Borrowed("a Number"),
                }
            );
        }
    }

    Ok(curr.into())
}

pub fn mul(args: Args, cont: &Continuation) -> Ret {
    let mut curr = Number::Integer(1);
    for arg in args {
        if let Sexp::Primitive(Primitive::Number(num)) = arg {
            curr *= num;
        } else {
            return err_ctx!(
                cont,
                InvalidArgument {
                    given: arg.clone(),
                    expected: Cow::Borrowed("a Number"),
                }
            );
        }
    }

    Ok(curr.into())
}

pub fn div(args: Args, cont: &Continuation) -> Ret {
    if args.len() < 1 {
        return err_ctx!(
            cont,
            WrongArgumentCount {
                given: 0,
                expected: ExpectedCount::AtLeast(1),
            }
        );
    }

    let mut curr = Number::default();
    let mut first = true;
    for arg in args {
        if let Sexp::Primitive(Primitive::Number(num)) = arg {
            if first {
                curr = num;
                first = false;
            } else {
                curr /= num;
            }
        } else {
            return err_ctx!(
                cont,
                InvalidArgument {
                    given: arg.clone(),
                    expected: Cow::Borrowed("a Number"),
                }
            );
        }
    }

    Ok(curr.into())
}

pub fn car(mut args: Args, cont: &Continuation) -> Ret {
    if args.len() != 1 {
        return err_ctx!(
            cont,
            WrongArgumentCount {
                given: args.len(),
                expected: ExpectedCount::Exactly(1),
            }
        );
    }

    let first = args.pop().unwrap();
    if let Sexp::Cons(cons) = first {
        if let Some(val) = cons.consume().0 {
            Ok(*val)
        } else {
            Ok(Sexp::default())
        }
    } else {
        err_ctx!(
            cont,
            InvalidArgument {
                given: first,
                expected: Cow::Borrowed("Cons"),
            }
        )
    }
}

pub fn cdr(mut args: Args, cont: &Continuation) -> Ret {
    if args.len() != 1 {
        return err_ctx!(
            cont,
            WrongArgumentCount {
                given: args.len(),
                expected: ExpectedCount::Exactly(1),
            }
        );
    }

    let first = args.pop().unwrap();
    if let Sexp::Cons(cons) = first {
        if let Some(val) = cons.consume().1 {
            Ok(*val)
        } else {
            Ok(Sexp::default())
        }
    } else {
        err_ctx!(
            cont,
            InvalidArgument {
                given: first,
                expected: Cow::Borrowed("Cons"),
            }
        )
    }
}

pub fn cons(mut args: Args, cont: &Continuation) -> Ret {
    if args.len() != 2 {
        return err_ctx!(
            cont,
            WrongArgumentCount {
                given: args.len(),
                expected: ExpectedCount::Exactly(2),
            }
        );
    }

    let cdr = args.pop().unwrap().into();
    let car = args.pop().unwrap().into();
    Ok(*sexp::cons(car, cdr).unwrap())
}
