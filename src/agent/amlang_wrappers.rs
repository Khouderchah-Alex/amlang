use std::borrow::Cow;
use std::convert::TryFrom;

use crate::function::{
    EvalErr::{self, *},
    ExpectedCount, Ret,
};
use crate::primitive::Symbol;
use crate::sexp::{Cons, HeapSexp, Sexp};


pub fn quote_wrapper(args: Option<HeapSexp>) -> Ret {
    if args.is_none() {
        return Err(WrongArgumentCount {
            given: 0,
            expected: ExpectedCount::Exactly(1),
        });
    }

    match *args.unwrap() {
        Sexp::Primitive(primitive) => {
            return Err(InvalidSexp(primitive.clone().into()));
        }

        Sexp::Cons(cons) => {
            let length = cons.iter().count();
            if length != 1 {
                return Err(WrongArgumentCount {
                    given: length,
                    expected: ExpectedCount::Exactly(1),
                });
            }

            let ret = cons.car();
            return match ret {
                None => Ok(Cons::default().into()),
                Some(val) => Ok(val.clone()),
            };
        }
    }
}

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
