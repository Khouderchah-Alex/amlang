use crate::builtin;
use crate::function::{EvalErr, Func, Ret};
use crate::sexp::{Atom, Value};

pub fn eval(form: Box<Value>) -> Ret {
    match *form {
        Value::Atom(atom) => {
            if let Atom::Symbol(symbol) = atom {
                let value = builtin::BUILTINS.lookup(&symbol);
                if value.is_none() {
                    return Err(EvalErr::UnboundSymbol(symbol));
                }
                return Ok(Value::Atom(Atom::BuiltIn(value.unwrap())));
            }
            return Ok(Value::Atom(atom));
        }
        Value::Cons(cons) => {
            if cons.car().is_none() {
                return Err(EvalErr::InvalidSexp(Value::Cons(cons)));
            }

            let (car, cdr) = cons.consume();
            let first = eval(car.unwrap())?;
            let args = evlis(cdr)?;
            if let Value::Atom(Atom::BuiltIn(f)) = first {
                return f.call(&args);
            }
            panic!(
                "TODO we need to handle more cases here;
                 functional application is the catchall"
            );
        }
    }
}

fn evlis(args: Option<Box<Value>>) -> Result<Vec<Value>, EvalErr> {
    let mut res = Vec::<Value>::new();
    if args.is_none() {
        return Ok(res);
    }

    match *args.unwrap() {
        Value::Cons(cons) => {
            for arg in cons {
                let val = eval(arg)?;
                res.push(val);
            }
        }
        Value::Atom(atom) => {
            return Err(EvalErr::InvalidSexp(Value::Atom(atom)));
        }
    }

    Ok(res)
}
