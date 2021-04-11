use crate::agent::designation::Designation;
use crate::function::{
    EvalErr::{self, *},
    ExpectedCount, Func, Ret,
};
use crate::primitive::Primitive;
use crate::sexp::{self, Sexp};

pub fn eval(form: &Sexp, designation: &mut dyn Designation) -> Ret {
    match form {
        Sexp::Primitive(primitive) => {
            return designation.designate(primitive);
        }

        Sexp::Cons(cons) => {
            let car = match cons.car() {
                Some(car) => car,
                None => return Err(InvalidSexp(Sexp::Cons(cons.clone()))),
            };

            if let Sexp::Primitive(Primitive::Symbol(first)) = car {
                match first.as_str() {
                    "quote" => {
                        return quote(cons.cdr());
                    }
                    _ => { /* Fallthrough */ }
                }
            }

            if let Sexp::Primitive(Primitive::BuiltIn(builtin)) = eval(car, designation)? {
                let args = evlis(cons.cdr(), designation)?;
                return builtin.call(&args);
            }
            panic!(
                "TODO we need to handle more cases here;
                 functional application is the catchall"
            );
        }
    }
}

fn evlis(args: Option<&Sexp>, designation: &mut dyn Designation) -> Result<Vec<Sexp>, EvalErr> {
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
                let val = eval(&arg, designation)?;
                res.push(val);
            }
        }
    }
    Ok(res)
}

fn quote(args: Option<&Sexp>) -> Ret {
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
                None => Ok(Sexp::Cons(sexp::Cons::default())),
                Some(val) => Ok(val.clone()),
            };
        }
    }
}
