use std::borrow::Cow;
use std::convert::TryFrom;

use crate::lang_err::{ExpectedCount, LangErr};
use crate::model::Ret;
use crate::primitive::{Node, Primitive, Symbol};
use crate::sexp::{Cons, HeapSexp, Sexp};


pub fn quote_wrapper(args: Option<HeapSexp>) -> Ret {
    if args.is_none() {
        return err!(WrongArgumentCount {
            given: 0,
            expected: ExpectedCount::Exactly(1),
        });
    }

    let (val,) = break_by_types!(*args.unwrap(), Sexp)?;
    Ok(val)
}

pub fn make_lambda_wrapper(args: Option<HeapSexp>) -> Result<(Vec<Symbol>, Sexp), LangErr> {
    if args.is_none() {
        return err!(WrongArgumentCount {
            given: 0,
            expected: ExpectedCount::AtLeast(2),
        });
    }

    let (param_sexp, body) = break_by_types!(*args.unwrap(), Cons; remainder)?;
    // Pull params into a list of symbols.
    let mut params = Vec::<Symbol>::with_capacity(param_sexp.iter().count());
    for param in param_sexp {
        let name = match *param {
            Sexp::Primitive(Primitive::Symbol(symbol)) => symbol,
            _ => {
                return err!(InvalidArgument {
                    given: param.clone().into(),
                    expected: Cow::Borrowed("symbol"),
                });
            }
        };
        params.push(name);
    }

    return match body {
        Some(hsexp) => match *hsexp {
            Sexp::Cons(cons) => Ok((params, cons.into())),
            Sexp::Primitive(primitive) => err!(InvalidArgument {
                given: primitive.into(),
                expected: Cow::Borrowed("procedure body"),
            }),
        },
        None => err!(WrongArgumentCount {
            given: 1,
            expected: ExpectedCount::AtLeast(2),
        }),
    };
}

pub fn tell_wrapper(args: &Vec<Node>) -> Result<(Node, Node, Node), LangErr> {
    if args.len() != 3 {
        return err!(WrongArgumentCount {
            given: args.len(),
            expected: ExpectedCount::Exactly(3),
        });
    }

    let subject = args[0];
    let predicate = args[1];
    let object = args[2];
    Ok((subject, predicate, object))
}

pub fn def_wrapper(args: &Vec<Node>) -> Result<(Node, Option<Node>), LangErr> {
    if args.len() < 1 {
        return err!(WrongArgumentCount {
            given: args.len(),
            expected: ExpectedCount::AtLeast(1),
        });
    } else if args.len() > 2 {
        return err!(WrongArgumentCount {
            given: args.len(),
            expected: ExpectedCount::AtMost(2),
        });
    }

    let name = args[0];
    let structure = if args.len() == 2 { Some(args[1]) } else { None };
    Ok((name, structure))
}

pub fn apply_wrapper(args: &Vec<Node>) -> Result<(Node, Node), LangErr> {
    if args.len() != 2 {
        return err!(WrongArgumentCount {
            given: args.len(),
            expected: ExpectedCount::Exactly(2),
        });
    }

    let proc_node = args[0];
    let args_node = args[1];
    Ok((proc_node, args_node))
}
