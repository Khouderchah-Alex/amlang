use std::borrow::Cow;
use std::convert::TryFrom;

use crate::agent::agent_state::AgentState;
use crate::lang_err::{ExpectedCount, LangErr};
use crate::model::Ret;
use crate::primitive::{Node, Primitive, Symbol};
use crate::sexp::{HeapSexp, Sexp};


pub fn quote_wrapper(args: Option<HeapSexp>, state: &AgentState) -> Ret {
    if args.is_none() {
        return err!(
            state,
            WrongArgumentCount {
                given: 0,
                expected: ExpectedCount::Exactly(1),
            }
        );
    }

    let (val,) = break_hsexp!(args.unwrap() => (Sexp), state)?;
    Ok(val)
}

pub fn make_lambda_wrapper(
    args: Option<HeapSexp>,
    state: &AgentState,
) -> Result<(Vec<Symbol>, HeapSexp), LangErr> {
    if args.is_none() {
        return err!(
            state,
            WrongArgumentCount {
                given: 0,
                expected: ExpectedCount::AtLeast(2),
            }
        );
    }

    let (param_sexp, body) = break_hsexp!(args.unwrap() => (HeapSexp; remainder), state)?;
    // Pull params into a list of symbols.
    let mut params = Vec::<Symbol>::with_capacity(param_sexp.iter().count());
    for (param, from_cons) in param_sexp {
        if !from_cons {
            return err!(state, InvalidSexp(*param));
        }
        let name = match *param {
            Sexp::Primitive(Primitive::Symbol(symbol)) => symbol,
            _ => {
                return err!(
                    state,
                    InvalidArgument {
                        given: param.clone().into(),
                        expected: Cow::Borrowed("symbol"),
                    }
                );
            }
        };
        params.push(name);
    }

    return match body {
        Some(hsexp) => match *hsexp {
            Sexp::Cons(_) => Ok((params, hsexp)),
            Sexp::Primitive(primitive) => err!(
                state,
                InvalidArgument {
                    given: primitive.into(),
                    expected: Cow::Borrowed("procedure body"),
                }
            ),
        },
        None => err!(
            state,
            WrongArgumentCount {
                given: 1,
                expected: ExpectedCount::AtLeast(2),
            }
        ),
    };
}

pub fn tell_wrapper(args: &Vec<Node>, state: &AgentState) -> Result<(Node, Node, Node), LangErr> {
    if args.len() != 3 {
        return err!(
            state,
            WrongArgumentCount {
                given: args.len(),
                expected: ExpectedCount::Exactly(3),
            }
        );
    }

    let subject = args[0];
    let predicate = args[1];
    let object = args[2];
    Ok((subject, predicate, object))
}

pub fn def_wrapper(args: &Vec<Node>, state: &AgentState) -> Result<(Node, Option<Node>), LangErr> {
    if args.len() < 1 {
        return err!(
            state,
            WrongArgumentCount {
                given: args.len(),
                expected: ExpectedCount::AtLeast(1),
            }
        );
    } else if args.len() > 2 {
        return err!(
            state,
            WrongArgumentCount {
                given: args.len(),
                expected: ExpectedCount::AtMost(2),
            }
        );
    }

    let name = args[0];
    let structure = if args.len() == 2 { Some(args[1]) } else { None };
    Ok((name, structure))
}

pub fn apply_wrapper(args: &Vec<Node>, state: &AgentState) -> Result<(Node, Node), LangErr> {
    if args.len() != 2 {
        return err!(
            state,
            WrongArgumentCount {
                given: args.len(),
                expected: ExpectedCount::Exactly(2),
            }
        );
    }

    let proc_node = args[0];
    let args_node = args[1];
    Ok((proc_node, args_node))
}
