use std::borrow::Cow;
use std::convert::TryFrom;

use crate::function::{
    EvalErr::{self, *},
    ExpectedCount, Ret,
};
use crate::primitive::{Primitive, Symbol};
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

/*
pub fn env_insert_triple_wrapper(
    args: Option<&Sexp>,
) -> Result<(&Symbol, &Symbol, &Symbol), EvalErr> {
    if args.is_none() {
        return Err(WrongArgumentCount {
            given: 0,
            expected: ExpectedCount::Exactly(3),
        });
    }

    let cons = match args.unwrap() {
        Sexp::Primitive(primitive) => {
            return Err(InvalidSexp(primitive.clone().into()));
        }
        Sexp::Cons(cons) => cons,
    };

    fn extract_symbol<'a, I: Iterator<Item = &'a Sexp>>(
        i: usize,
        iter: &mut I,
    ) -> Result<&'a Symbol, EvalErr> {
        if let Some(elem) = iter.next() {
            if let Ok(symbol) = <&Symbol>::try_from(elem) {
                Ok(symbol)
            } else {
                Err(InvalidArgument {
                    given: elem.clone().into(),
                    expected: Cow::Borrowed("symbol"),
                })
            }
        } else {
            Err(WrongArgumentCount {
                given: i,
                expected: ExpectedCount::Exactly(3),
            })
        }
    }

    let mut iter = cons.iter();
    let subject = extract_symbol(0, &mut iter)?;
    let predicate = extract_symbol(1, &mut iter)?;
    let object = extract_symbol(2, &mut iter)?;

    if let Some(_) = iter.next() {
        return Err(WrongArgumentCount {
            given: cons.iter().count(),
            expected: ExpectedCount::Exactly(3),
        });
    }

    Ok((subject, predicate, object))
}

pub fn env_insert_node_wrapper(args: Option<&Sexp>) -> Result<(&Symbol, Option<&Sexp>), EvalErr> {
    if args.is_none() {
        return Err(WrongArgumentCount {
            given: 0,
            expected: ExpectedCount::AtLeast(1),
        });
    }

    let cons = match args.unwrap() {
        Sexp::Primitive(primitive) => {
            return Err(InvalidSexp(primitive.clone().into()));
        }
        Sexp::Cons(cons) => cons,
    };

    let mut iter = cons.iter();
    let name = if let Ok(symbol) = <&Symbol>::try_from(iter.next()) {
        symbol
    } else {
        return Err(InvalidArgument {
            given: cons.clone().into(),
            expected: Cow::Borrowed("symbol"),
        });
    };

    let structure = iter.next();
    if structure.is_some() && iter.next().is_some() {
        return Err(WrongArgumentCount {
            given: iter.count() + 3,
            expected: ExpectedCount::AtMost(2),
        });
    }

    Ok((name, structure))
}
*/
