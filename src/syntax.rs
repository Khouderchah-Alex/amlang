use crate::function::{EvalErr::*, ExpectedCount, Ret};
use crate::sexp::{Cons, HeapSexp, Sexp};


pub fn quote(args: Option<HeapSexp>) -> Ret {
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
