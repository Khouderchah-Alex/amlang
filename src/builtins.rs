use std::borrow::Cow;
use std::collections::HashMap;

use crate::agent::agent_state::AgentState;
use crate::primitive::builtin::Args;
use crate::primitive::error::ExpectedCount;
use crate::primitive::{BuiltIn, Error, Node, Number, Primitive};
use crate::sexp::{Cons, HeapSexp, Sexp};


// Used for bootstrapping and auxiliary purposes, not as an environment.
pub fn generate_builtin_map() -> HashMap<&'static str, BuiltIn> {
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

    builtins![car, cdr, cons, println, eq, add, sub, mul, div]
}

// Auto-gen builtins from raw rust functions.
wrap_builtin!(car_(Cons) => car);
wrap_builtin!(cdr_(Cons) => cdr);
wrap_builtin!(cons_(HeapSexp, HeapSexp) => cons);
wrap_builtin!(println_(Sexp) => println);
wrap_builtin!(eq_(Sexp, Sexp) => eq);


fn car_(cons: Cons, _state: &mut AgentState) -> Result<Sexp, Error> {
    if let Some(val) = cons.consume().0 {
        Ok(*val)
    } else {
        Ok(Sexp::default())
    }
}

fn cdr_(cons: Cons, _state: &mut AgentState) -> Result<Sexp, Error> {
    if let Some(val) = cons.consume().1 {
        Ok(*val)
    } else {
        Ok(Sexp::default())
    }
}

fn cons_(car: HeapSexp, cdr: HeapSexp, _state: &mut AgentState) -> Result<Sexp, Error> {
    // Prefer to represent '() using None.
    let to_option = |s: HeapSexp| if s.is_none() { None } else { Some(s) };
    Ok(Cons::new(to_option(car), to_option(cdr)).into())
}

fn println_(arg: Sexp, state: &mut AgentState) -> Result<Sexp, Error> {
    state.print_list(&arg);
    println!("");
    Ok(Sexp::default())
}

fn eq_(a: Sexp, b: Sexp, state: &mut AgentState) -> Result<Sexp, Error> {
    let local = if a == b {
        state.context().t
    } else {
        state.context().f
    };
    Ok(Node::new(state.context().lang_env(), local).into())
}


fn add(args: Args, state: &mut AgentState) -> Result<Sexp, Error> {
    let mut curr = Number::default();
    for arg in args {
        if let Sexp::Primitive(Primitive::Number(num)) = arg {
            curr += num;
        } else {
            return err!(
                state,
                InvalidArgument {
                    given: arg.clone(),
                    expected: Cow::Borrowed("a Number"),
                }
            );
        }
    }

    Ok(curr.into())
}

fn sub(args: Args, state: &mut AgentState) -> Result<Sexp, Error> {
    if args.len() < 1 {
        return err!(
            state,
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
            return err!(
                state,
                InvalidArgument {
                    given: arg.clone(),
                    expected: Cow::Borrowed("a Number"),
                }
            );
        }
    }

    Ok(curr.into())
}

fn mul(args: Args, state: &mut AgentState) -> Result<Sexp, Error> {
    let mut curr = Number::Integer(1);
    for arg in args {
        if let Sexp::Primitive(Primitive::Number(num)) = arg {
            curr *= num;
        } else {
            return err!(
                state,
                InvalidArgument {
                    given: arg.clone(),
                    expected: Cow::Borrowed("a Number"),
                }
            );
        }
    }

    Ok(curr.into())
}

fn div(args: Args, state: &mut AgentState) -> Result<Sexp, Error> {
    if args.len() < 1 {
        return err!(
            state,
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
            return err!(
                state,
                InvalidArgument {
                    given: arg.clone(),
                    expected: Cow::Borrowed("a Number"),
                }
            );
        }
    }

    Ok(curr.into())
}


/// Autogen function taking args: Vec<Sexp> from one taking specific subtypes.
macro_rules! wrap_builtin {
    ($raw:ident($ta:ident) => $wrapped:ident) => {
        fn $wrapped(args: Args, state: &mut AgentState) -> Result<Sexp, Error> {
            let (a,) = break_sexp!(args.into_iter().map(|e| (e, true)) => ($ta), state)?;
            $raw(a, state)
        }
    };
    ($raw:ident($ta:ident, $tb:ident) => $wrapped:ident) => {
        fn $wrapped(args: Args, state: &mut AgentState) -> Result<Sexp, Error> {
            let (a, b) = break_sexp!(args.into_iter().map(|e| (e, true)) => ($ta, $tb), state)?;
            $raw(a, b, state)
        }
    };
    ($raw:ident($($type:ident),+) => $wrapped:ident) => {
        fn $wrapped(args: Args, state: &mut AgentState) -> Result<Sexp, Error> {
            let tuple = break_sexp!(args.into_iter().map(|e| (e, true)) => ($($type),+), state)?;
            $raw(tuple, state)
        }
    };
}
use wrap_builtin;
