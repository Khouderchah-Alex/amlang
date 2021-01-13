use crate::builtin;
use crate::function::{
    EvalErr::{self, *},
    Func, Ret,
};
use crate::sexp::{Atom, Value};

pub fn eval(form: &Value) -> Ret {
    match form {
        Value::Atom(atom) => {
            if let Atom::Symbol(symbol) = atom {
                let value = builtin::BUILTINS.lookup(symbol);
                return match value {
                    Some(builtin) => Ok(Value::Atom(Atom::BuiltIn(builtin))),
                    None => Err(UnboundSymbol(symbol.clone())),
                };
            }
            return Ok(Value::Atom(atom.clone()));
        }

        Value::Cons(cons) => {
            let first = match cons.car() {
                Some(car) => eval(car)?,
                None => return Err(InvalidSexp(Value::Cons(cons.clone()))),
            };

            if let Value::Atom(Atom::BuiltIn(builtin)) = first {
                let args = evlis(cons.cdr())?;
                return builtin.call(&args);
            }
            panic!(
                "TODO we need to handle more cases here;
                 functional application is the catchall"
            );
        }
    }
}

fn evlis(args: Option<&Value>) -> Result<Vec<Value>, EvalErr> {
    let mut res = Vec::<Value>::new();
    if args.is_none() {
        return Ok(res);
    }

    match args.unwrap() {
        Value::Atom(atom) => {
            return Err(InvalidSexp(Value::Atom(atom.clone())));
        }

        Value::Cons(cons) => {
            for arg in cons {
                let val = eval(&arg)?;
                res.push(val);
            }
        }
    }
    Ok(res)
}
