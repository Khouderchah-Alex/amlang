use std::borrow::Cow;
use std::convert::TryFrom;

use crate::function::{
    EvalErr::{self, *},
    ExpectedCount, Ret,
};
use crate::primitive::{NodeId, Primitive, Symbol};
use crate::sexp::{Cons, HeapSexp, Sexp};


pub fn quote_wrapper(args: Option<HeapSexp>) -> Ret {
    if args.is_none() {
        return Err(WrongArgumentCount {
            given: 0,
            expected: ExpectedCount::Exactly(1),
        });
    }

    let (val,) = break_by_types!(*args.unwrap(), Sexp)?;
    Ok(val)
}

pub fn make_procedure_wrapper(args: Option<HeapSexp>) -> Result<(Vec<Symbol>, Sexp), EvalErr> {
    if args.is_none() {
        return Err(WrongArgumentCount {
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
                return Err(InvalidArgument {
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
            Sexp::Primitive(primitive) => Err(InvalidArgument {
                given: primitive.into(),
                expected: Cow::Borrowed("procedure body"),
            }),
        },
        None => Err(WrongArgumentCount {
            given: 1,
            expected: ExpectedCount::AtLeast(2),
        }),
    };
}

pub fn env_insert_triple_wrapper(args: &Vec<NodeId>) -> Result<(NodeId, NodeId, NodeId), EvalErr> {
    if args.len() != 3 {
        return Err(WrongArgumentCount {
            given: args.len(),
            expected: ExpectedCount::Exactly(3),
        });
    }

    let subject = args[0];
    let predicate = args[1];
    let object = args[2];
    Ok((subject, predicate, object))
}

pub fn env_insert_node_wrapper(args: &Vec<NodeId>) -> Result<(NodeId, Option<NodeId>), EvalErr> {
    if args.len() < 1 {
        return Err(WrongArgumentCount {
            given: args.len(),
            expected: ExpectedCount::AtLeast(1),
        });
    } else if args.len() > 2 {
        return Err(WrongArgumentCount {
            given: args.len(),
            expected: ExpectedCount::AtMost(2),
        });
    }

    let name = args[0];
    let structure = if args.len() == 2 { Some(args[1]) } else { None };
    Ok((name, structure))
}
