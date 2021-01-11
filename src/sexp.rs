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

impl Cons {
    pub fn cons(car: Option<Value>, cdr: Option<Value>) -> Cons {
        Cons {
            car: car.map(Box::new),
            cdr: cdr.map(Box::new),
        }
    }

    pub fn iter(&self) -> SexpIter {
        SexpIter {
            current: Some(&self),
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

    pub fn consume(self) -> (Option<Box<Value>>, Option<Box<Value>>) {
        (self.car, self.cdr)
    }

    pub fn set_cdr(&mut self, new: Option<Box<Value>>) {
        self.cdr = new;
    }

    fn list_fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Any list longer than this will simply be suffixed with "..." after these
        // many elements.
        const MAX_DISPLAY_LENGTH: usize = 64;

        let mut pos: usize = 0;
        write!(f, "(")?;
        for val in self.iter() {
            if pos >= MAX_DISPLAY_LENGTH {
                write!(f, "...")?;
                break;
            }

            if pos > 0 {
                write!(f, " ")?;
            }
            write!(f, "{:#}", val)?;

            pos += 1;
        }
        write!(f, ")")
    }
}

pub struct SexpIter<'a> {
    current: Option<&'a Cons>,
}

impl<'a> Iterator for SexpIter<'a> {
    type Item = &'a Value;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(cons) = self.current {
            if let Some(Value::Cons(next)) = cons.cdr() {
                self.current = Some(next);
            } else {
                self.current = None;
            }

            return cons.car();
        }

        None
    }
}

pub struct SexpIntoIter {
    current: Option<Cons>,
}

impl Iterator for SexpIntoIter {
    type Item = Box<Value>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current.is_none() {
            return None;
        }

        let (car, cdr) = self.current.take().unwrap().consume();
        if let Some(next) = cdr {
            if let Value::Cons(c) = *next {
                self.current = Some(c);
            }
        }

        car
    }
}

impl IntoIterator for Cons {
    type Item = Box<Value>;
    type IntoIter = SexpIntoIter;

    fn into_iter(self) -> Self::IntoIter {
        SexpIntoIter {
            current: Some(self),
        }
    }
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
