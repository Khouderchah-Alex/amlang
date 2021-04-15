use crate::function::{
    EvalErr::{self, *},
    ExpectedCount, Ret,
};
use crate::model::Eval;
use crate::sexp::{Cons, Sexp};


/// Surface of syntax Model asserting that List elements can be usefully eval'd
/// independently.
///
/// TODO(feat) Make this an actual Model surface.
pub fn evlis<T: Eval>(args: Option<&Sexp>, eval: &mut T) -> Result<Vec<Sexp>, EvalErr> {
    let mut res = Vec::<Sexp>::new();
    if args.is_none() {
        return Ok(res);
    }

    match args.unwrap() {
        Sexp::Primitive(primitive) => {
            return Err(InvalidSexp(Sexp::Primitive(primitive.clone())));
        }

        Sexp::Cons(cons) => {
            for arg in cons {
                let val = eval.eval(&arg)?;
                res.push(val);
            }
        }
    }
    Ok(res)
}

pub fn quote(args: Option<&Sexp>) -> Ret {
    if args.is_none() {
        return Err(WrongArgumentCount {
            given: 0,
            expected: ExpectedCount::Exactly(1),
        });
    }

    match args.unwrap() {
        Sexp::Primitive(primitive) => {
            return Err(InvalidSexp(Sexp::Primitive(primitive.clone())));
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
                None => Ok(Sexp::Cons(Cons::default())),
                Some(val) => Ok(val.clone()),
            };
        }
    }
}
