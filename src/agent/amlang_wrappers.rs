use crate::agent::agent_state::AgentState;
use crate::agent::lang_error::ExpectedCount;
use crate::error::Error;
use crate::primitive::{Node, Primitive, Symbol};
use crate::sexp::cons_list::ConsList;
use crate::sexp::{HeapSexp, Sexp, SexpIntoIter};


pub fn quote_wrapper(args: Option<HeapSexp>, state: &AgentState) -> Result<HeapSexp, Error> {
    let iter = args.map_or(SexpIntoIter::default(), |e| e.into_iter());
    let (val,) = break_sexp!(iter => (HeapSexp), state)?;
    Ok(val)
}

pub fn make_lambda_wrapper(
    args: Option<HeapSexp>,
    state: &AgentState,
) -> Result<(Vec<Symbol>, HeapSexp), Error> {
    if args.is_none() {
        return err!(
            state,
            WrongArgumentCount {
                given: 0,
                expected: ExpectedCount::AtLeast(2),
            }
        );
    }

    let (param_sexp, body) = break_sexp!(args.unwrap() => (HeapSexp; remainder), state)?;
    // Pull params into a list of symbols.
    let mut params = Vec::<Symbol>::with_capacity(param_sexp.iter().count());
    for (param, proper) in param_sexp {
        if !proper {
            return err!(state, InvalidSexp(*param));
        }
        let name = match *param {
            Sexp::Primitive(Primitive::Symbol(symbol)) => symbol,
            _ => {
                return err!(
                    state,
                    InvalidArgument {
                        given: param.clone().into(),
                        expected: "symbol".into(),
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
                    expected: "procedure body".into(),
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

pub fn let_wrapper(
    args: Option<HeapSexp>,
    state: &AgentState,
) -> Result<(Vec<Symbol>, HeapSexp, HeapSexp), Error> {
    if args.is_none() {
        return err!(
            state,
            WrongArgumentCount {
                given: 0,
                expected: ExpectedCount::AtLeast(2),
            }
        );
    }

    let (bindings, body) = break_sexp!(args.unwrap() => (HeapSexp; remainder), state)?;
    let len = bindings.iter().count();
    let mut params = Vec::with_capacity(len);
    let mut exprs = ConsList::new();
    for (binding, proper) in bindings {
        if !proper {
            return err!(state, InvalidSexp(*binding));
        }
        let (name, expr) = break_sexp!(binding => (Symbol, HeapSexp), state)?;
        params.push(name);
        exprs.append(expr);
    }

    return match body {
        Some(hsexp) => match *hsexp {
            Sexp::Cons(_) => Ok((params, HeapSexp::new(exprs.release()), hsexp)),
            Sexp::Primitive(primitive) => err!(
                state,
                InvalidArgument {
                    given: primitive.into(),
                    expected: "procedure body".into(),
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

pub fn tell_wrapper(args: &Vec<Node>, state: &AgentState) -> Result<(Node, Node, Node), Error> {
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

pub fn def_wrapper(args: &Vec<Node>, state: &AgentState) -> Result<(Node, Option<Node>), Error> {
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

pub fn apply_wrapper(args: &Vec<Node>, state: &AgentState) -> Result<(Node, Node), Error> {
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
