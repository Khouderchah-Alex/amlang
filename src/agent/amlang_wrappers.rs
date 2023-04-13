use crate::agent::lang_error::{ExpectedCount, LangError};
use crate::agent::Agent;
use crate::error::Error;
use crate::primitive::{Node, Primitive, Symbol};
use crate::sexp::{ConsList, HeapSexp, Sexp, SexpIntoIter};


pub fn quote_wrapper(args: Option<HeapSexp>, agent: &Agent) -> Result<HeapSexp, Error> {
    let iter = args.map_or(SexpIntoIter::default(), |e| e.into_iter());
    let (val,) = break_sexp!(iter => (HeapSexp), agent)?;
    Ok(val)
}

pub fn make_lambda_wrapper(
    args: Option<HeapSexp>,
    agent: &Agent,
) -> Result<(Vec<Symbol>, HeapSexp), Error> {
    if args.is_none() {
        return err!(
            agent,
            LangError::WrongArgumentCount {
                given: 0,
                expected: ExpectedCount::AtLeast(2),
            }
        );
    }

    let (param_sexp, body) = break_sexp!(args.unwrap() => (HeapSexp; remainder), agent)?;
    // Pull params into a list of symbols.
    let mut params = Vec::<Symbol>::with_capacity(param_sexp.iter().count());
    for (param, proper) in param_sexp {
        if !proper {
            return err!(agent, LangError::InvalidSexp(*param));
        }
        let name = match *param {
            Sexp::Primitive(Primitive::Symbol(symbol)) => symbol,
            _ => {
                return err!(
                    agent,
                    LangError::InvalidArgument {
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
                agent,
                LangError::InvalidArgument {
                    given: primitive.into(),
                    expected: "procedure body".into(),
                }
            ),
        },
        None => err!(
            agent,
            LangError::WrongArgumentCount {
                given: 1,
                expected: ExpectedCount::AtLeast(2),
            }
        ),
    };
}

pub fn let_wrapper(
    args: Option<HeapSexp>,
    agent: &Agent,
) -> Result<(Vec<Symbol>, HeapSexp, HeapSexp), Error> {
    if args.is_none() {
        return err!(
            agent,
            LangError::WrongArgumentCount {
                given: 0,
                expected: ExpectedCount::AtLeast(2),
            }
        );
    }

    let (bindings, body) = break_sexp!(args.unwrap() => (HeapSexp; remainder), agent)?;
    let len = bindings.iter().count();
    let mut params = Vec::with_capacity(len);
    let mut exprs = ConsList::new();
    for (binding, proper) in bindings {
        if !proper {
            return err!(agent, LangError::InvalidSexp(*binding));
        }
        let (name, expr) = break_sexp!(binding => (Symbol, HeapSexp), agent)?;
        params.push(name);
        exprs.append(expr);
    }

    return match body {
        Some(hsexp) => match *hsexp {
            Sexp::Cons(_) => Ok((params, HeapSexp::new(exprs.release()), hsexp)),
            Sexp::Primitive(primitive) => err!(
                agent,
                LangError::InvalidArgument {
                    given: primitive.into(),
                    expected: "procedure body".into(),
                }
            ),
        },
        None => err!(
            agent,
            LangError::WrongArgumentCount {
                given: 1,
                expected: ExpectedCount::AtLeast(2),
            }
        ),
    };
}

pub fn tell_wrapper(args: &Vec<Node>, agent: &Agent) -> Result<(Node, Node, Node), Error> {
    if args.len() != 3 {
        return err!(
            agent,
            LangError::WrongArgumentCount {
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

pub fn def_wrapper(args: &Vec<Node>, agent: &Agent) -> Result<(Node, Option<Node>), Error> {
    if args.len() < 1 {
        return err!(
            agent,
            LangError::WrongArgumentCount {
                given: args.len(),
                expected: ExpectedCount::AtLeast(1),
            }
        );
    } else if args.len() > 2 {
        return err!(
            agent,
            LangError::WrongArgumentCount {
                given: args.len(),
                expected: ExpectedCount::AtMost(2),
            }
        );
    }

    let name = args[0];
    let structure = if args.len() == 2 { Some(args[1]) } else { None };
    Ok((name, structure))
}

pub fn defa_wrapper(args: &Vec<Node>, agent: &Agent) -> Result<Option<Node>, Error> {
    if args.len() > 1 {
        return err!(
            agent,
            LangError::WrongArgumentCount {
                given: args.len(),
                expected: ExpectedCount::AtMost(1),
            }
        );
    }

    Ok(args.iter().next().copied())
}

pub fn apply_wrapper(args: &Vec<Node>, agent: &Agent) -> Result<(Node, Node), Error> {
    if args.len() != 2 {
        return err!(
            agent,
            LangError::WrongArgumentCount {
                given: args.len(),
                expected: ExpectedCount::Exactly(2),
            }
        );
    }

    let proc_node = args[0];
    let args_node = args[1];
    Ok((proc_node, args_node))
}
