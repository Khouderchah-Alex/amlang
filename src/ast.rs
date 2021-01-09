
#[derive(Debug)]
pub enum Value {
    Atom(Atom),
    Cons(Cons),
}

#[derive(Debug)]
pub enum Atom {
    Integer(i64),
    Float(f64),
    Symbol(String),
}

#[derive(Debug, Default)]
pub struct Cons {
    car: Option<Box<Value>>,
    cdr: Option<Box<Value>>,
}
