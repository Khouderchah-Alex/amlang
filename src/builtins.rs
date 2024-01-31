use lazy_static::lazy_static;

use std::collections::HashMap;
use std::mem;

use crate::agent::lang_error::LangError;
use crate::agent::Agent;
use crate::env::LocalNode;
use crate::error::Error;
use crate::primitive::prelude::*;
use crate::sexp::{Cons, HeapSexp, Sexp};


// Used for bootstrapping and auxiliary purposes, not as an environment.
pub fn generate_builtin_map() -> HashMap<&'static str, BuiltIn> {
    macro_rules! builtins {
        [$($x:expr),*] => {
            {
                let mut m = HashMap::new();
                $(m.insert(stringify!($x), BuiltIn::new(stringify!($x), $x));)*
                m
            }
        };
        [$($n:tt : $x:expr),+ ,] => {
            builtins![$($n : $x),*]
        };
    }

    builtins![
        car, cdr, cons, list_len, println, eq, curr, jump, env_find, env_jump, add, sub, mul, div
    ]
}

// Auto-gen builtins from raw rust functions.
wrap_builtin!(car_(Cons) => car);
wrap_builtin!(cdr_(Cons) => cdr);
wrap_builtin!(cons_(HeapSexp, HeapSexp) => cons);
wrap_builtin!(list_len_(HeapSexp) => list_len);
wrap_builtin!(println_(Sexp) => println);
wrap_builtin!(eq_(Sexp, Sexp) => eq);
wrap_builtin!(curr_() => curr);
wrap_builtin!(jump_(Node) => jump);
wrap_builtin!(env_find_(LangString) => env_find);
wrap_builtin!(env_jump_(Node) => env_jump);


fn car_(cons: Cons, _agent: &mut Agent) -> Result<Sexp, Error> {
    if let Some(val) = cons.consume().0 {
        Ok(*val)
    } else {
        Ok(Sexp::default())
    }
}

fn cdr_(cons: Cons, _agent: &mut Agent) -> Result<Sexp, Error> {
    if let Some(val) = cons.consume().1 {
        Ok(*val)
    } else {
        Ok(Sexp::default())
    }
}

fn cons_(car: HeapSexp, cdr: HeapSexp, _agent: &mut Agent) -> Result<Cons, Error> {
    // Prefer to represent '() using None.
    let to_option = |s: HeapSexp| if s.is_none() { None } else { Some(s) };
    Ok(Cons::new(to_option(car), to_option(cdr)))
}

fn list_len_(sexp: HeapSexp, agent: &mut Agent) -> Result<Number, Error> {
    let mut count = 0;
    for (_elem, proper) in sexp.iter() {
        if !proper {
            return err!(
                agent,
                LangError::InvalidArgument {
                    given: sexp.into(),
                    expected: "Proper list".into()
                }
            );
        }
        count += 1;
    }
    Ok(Number::USize(count).into())
}

fn println_(arg: Sexp, agent: &mut Agent) -> Result<Sexp, Error> {
    agent.print_sexp(&arg);
    println!("");
    Ok(Sexp::default())
}

fn eq_(a: Sexp, b: Sexp, agent: &mut Agent) -> Result<Node, Error> {
    // TODO(perf) Would be better to cache the Node, not Sym, but
    // having trouble passing agent into lazy_static. Alternatively,
    // if we could access the Context from an Agent or copy context to
    // a builtin, that would work too.
    lazy_static! {
        static ref T: Symbol = "true".to_symbol_or_panic(policy_base);
        static ref F: Symbol = "false".to_symbol_or_panic(policy_base);
    }

    if a == b {
        agent.resolve_name(&T)
    } else {
        agent.resolve_name(&F)
    }
}

fn curr_(agent: &mut Agent) -> Result<Node, Error> {
    Ok(agent.pos().into())
}

fn jump_(node: Node, agent: &mut Agent) -> Result<Node, Error> {
    agent.jump(node);
    Ok(node)
}

fn env_jump_(node: Node, agent: &mut Agent) -> Result<Node, Error> {
    if node.env().id() != 0 {
        return err!(
            agent,
            LangError::InvalidArgument {
                given: node.into(),
                expected: "Env node".into()
            }
        );
    }

    agent.jump_env(node.local());
    Ok(node)
}

fn env_find_(path: LangString, agent: &mut Agent) -> Result<Sexp, Error> {
    let res = if let Some(lnode) = agent.find_env(path.as_str()) {
        Node::new(LocalNode::default(), lnode).into()
    } else {
        Sexp::default()
    };
    Ok(res)
}


fn add(args: Sexp, agent: &mut Agent) -> Result<Sexp, Error> {
    let (mut curr, mut tail) = break_sexp!(args => (Number; remainder), agent)?;
    let curr_d = mem::discriminant(&curr);
    while tail != None {
        let (num, new_tail) = break_sexp!(tail.unwrap() => (Number; remainder), agent)?;
        tail = new_tail;

        if mem::discriminant(&num) != curr_d {
            return err!(
                agent,
                LangError::InvalidArgument {
                    given: num.clone().into(),
                    expected: std::borrow::Cow::Owned(format!("discriminant {:?}", curr_d))
                }
            );
        }
        curr += num;
    }
    Ok(curr.into())
}

fn sub(args: Sexp, agent: &mut Agent) -> Result<Sexp, Error> {
    let (mut curr, mut tail) = break_sexp!(args => (Number; remainder), agent)?;
    let curr_d = mem::discriminant(&curr);
    while tail != None {
        let (num, new_tail) = break_sexp!(tail.unwrap() => (Number; remainder), agent)?;
        tail = new_tail;

        if mem::discriminant(&num) != curr_d {
            return err!(
                agent,
                LangError::InvalidArgument {
                    given: num.clone().into(),
                    expected: std::borrow::Cow::Owned(format!("discriminant {:?}", curr_d))
                }
            );
        }
        curr -= num;
    }
    Ok(curr.into())
}

fn mul(args: Sexp, agent: &mut Agent) -> Result<Sexp, Error> {
    let (mut curr, mut tail) = break_sexp!(args => (Number; remainder), agent)?;
    let curr_d = mem::discriminant(&curr);
    while tail != None {
        let (num, new_tail) = break_sexp!(tail.unwrap() => (Number; remainder), agent)?;
        tail = new_tail;

        if mem::discriminant(&num) != curr_d {
            return err!(
                agent,
                LangError::InvalidArgument {
                    given: num.clone().into(),
                    expected: std::borrow::Cow::Owned(format!("discriminant {:?}", curr_d))
                }
            );
        }
        curr *= num;
    }
    Ok(curr.into())
}

fn div(args: Sexp, agent: &mut Agent) -> Result<Sexp, Error> {
    let (mut curr, mut tail) = break_sexp!(args => (Number; remainder), agent)?;
    let curr_d = mem::discriminant(&curr);
    while tail != None {
        let (num, new_tail) = break_sexp!(tail.unwrap() => (Number; remainder), agent)?;
        tail = new_tail;

        if mem::discriminant(&num) != curr_d {
            return err!(
                agent,
                LangError::InvalidArgument {
                    given: num.clone().into(),
                    expected: std::borrow::Cow::Owned(format!("discriminant {:?}", curr_d))
                }
            );
        }
        curr /= num;
    }
    Ok(curr.into())
}


/// Autogen function taking args: Vec<Sexp> from one taking specific subtypes.
macro_rules! wrap_builtin {
    ($raw:ident() => $wrapped:ident) => {
        fn $wrapped(args: Sexp, agent: &mut Agent) -> Result<Sexp, Error> {
            break_sexp!(args => (), agent)?;
            Ok($raw(agent)?.into())
        }
    };
    ($raw:ident($ta:ident) => $wrapped:ident) => {
        fn $wrapped(args: Sexp, agent: &mut Agent) -> Result<Sexp, Error> {
            let (a,) = break_sexp!(args => ($ta), agent)?;
            Ok($raw(a, agent)?.into())
        }
    };
    ($raw:ident($ta:ident, $tb:ident) => $wrapped:ident) => {
        fn $wrapped(args: Sexp, agent: &mut Agent) -> Result<Sexp, Error> {
            let (a, b) = break_sexp!(args => ($ta, $tb), agent)?;
            Ok($raw(a, b, agent)?.into())
        }
    };
    ($raw:ident($($type:ident),+) => $wrapped:ident) => {
        fn $wrapped(args: Sexp, agent: &mut Agent) -> Result<Sexp, Error> {
            let tuple = break_sexp!(args => ($($type),+), agent)?;
            $raw(tuple, agent)
        }
    };
}
use wrap_builtin;
