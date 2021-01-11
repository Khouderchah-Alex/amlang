//! Module for representing S-exps.

use std::fmt;

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

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Atom(atom) => write!(f, "{}", atom),
            Value::Cons(cons) => {
                if f.alternate() {
                    write!(f, "{:#}", cons)
                } else {
                    write!(f, "{}", cons)
                }
            }
        }
    }
}

impl fmt::Display for Atom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Atom::Integer(i) => write!(f, "{}", i),
            Atom::Float(ff) => write!(f, "{}f", ff),
            Atom::Symbol(s) => write!(f, "{:}", s),
        }
    }
}

impl fmt::Display for Cons {
    /// Note: this does not check for loops and doesn't have a max depth.
    /// Use the alternate formatting for untrusted S-exps.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Alternate print this with the list shorthand.
        if f.alternate() {
            return self.list_fmt(f);
        }

        let a = match self.car() {
            Some(val) => val.to_string(),
            None => "NIL".to_string(),
        };
        let b = match self.cdr() {
            Some(val) => val.to_string(),
            None => "NIL".to_string(),
        };

        write!(f, "({} . {})", a, b)
    }
}

impl Cons {
    pub fn cons(car: Option<Value>, cdr: Option<Value>) -> Cons {
        Cons {
            car: car.map(Box::new),
            cdr: cdr.map(Box::new),
        }
    }

    pub fn car(&self) -> Option<&Value> {
        match &self.car {
            Some(val) => Some(val.as_ref()),
            None => None,
        }
    }

    pub fn cdr(&self) -> Option<&Value> {
        match &self.cdr {
            Some(val) => Some(val.as_ref()),
            None => None,
        }
    }

    pub fn set_cdr(&mut self, new: Option<Box<Value>>) {
        self.cdr = new;
    }

    fn list_fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Any list longer than this will simply be suffixed with "..." after these
        // many elements.
        const MAX_DISPLAY_LENGTH: usize = 64;

        let mut pos: usize = 0;
        let mut curr: &Cons = &self;
        write!(f, "(")?;
        loop {
            if pos >= MAX_DISPLAY_LENGTH {
                write!(f, "...")?;
                break;
            }

            match &curr.car {
                Some(val) => {
                    write!(f, "{:#}", val)?;
                }
                None => {
                    write!(f, "NIL")?;
                }
            }

            match &curr.cdr {
                Some(val) => {
                    if let Value::Cons(next) = &*val.as_ref() {
                        curr = &next;
                        pos += 1;
                    }
                    write!(f, " ")?;
                }
                None => {
                    break;
                }
            };
        }
        write!(f, ")")
    }
}
