//! Module for representing S-exps.

use std::fmt;

use crate::function::BuiltIn;
use crate::number::Number;

#[derive(Clone, Debug)]
pub enum Sexp {
    Atom(Atom),
    Cons(Cons),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Atom {
    Number(Number),
    Symbol(String),
    BuiltIn(&'static BuiltIn),
}

#[derive(Clone, Debug, Default)]
pub struct Cons {
    car: Option<Box<Sexp>>,
    cdr: Option<Box<Sexp>>,
}

impl Sexp {
    pub fn cons(&self) -> &Cons {
        if let Sexp::Cons(c) = self {
            return c;
        }
        panic!("Expected {:?} to be Cons", self);
    }
}

impl Cons {
    pub fn cons(car: Option<Box<Sexp>>, cdr: Option<Box<Sexp>>) -> Cons {
        Cons { car, cdr }
    }

    pub fn iter(&self) -> SexpIter {
        SexpIter {
            current: Some(&self),
        }
    }

    pub fn car(&self) -> Option<&Sexp> {
        match &self.car {
            Some(val) => Some(val.as_ref()),
            None => None,
        }
    }

    pub fn cdr(&self) -> Option<&Sexp> {
        match &self.cdr {
            Some(val) => Some(val.as_ref()),
            None => None,
        }
    }

    pub fn consume(self) -> (Option<Box<Sexp>>, Option<Box<Sexp>>) {
        (self.car, self.cdr)
    }

    pub fn set_cdr(&mut self, new: Option<Box<Sexp>>) {
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
    type Item = &'a Sexp;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(cons) = self.current {
            if let Some(Sexp::Cons(next)) = cons.cdr() {
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

impl<'a> IntoIterator for &'a Cons {
    type Item = &'a Sexp;
    type IntoIter = SexpIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl Iterator for SexpIntoIter {
    type Item = Box<Sexp>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current.is_none() {
            return None;
        }

        let (car, cdr) = self.current.take().unwrap().consume();
        if let Some(next) = cdr {
            if let Sexp::Cons(c) = *next {
                self.current = Some(c);
            }
        }

        car
    }
}

impl IntoIterator for Cons {
    type Item = Box<Sexp>;
    type IntoIter = SexpIntoIter;

    fn into_iter(self) -> Self::IntoIter {
        SexpIntoIter {
            current: Some(self),
        }
    }
}

impl fmt::Display for Sexp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Sexp::Atom(atom) => write!(f, "{}", atom),
            Sexp::Cons(cons) => {
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
            Atom::Number(num) => write!(f, "{}", num),
            Atom::Symbol(s) => write!(f, "{}", s),
            Atom::BuiltIn(b) => write!(f, "{}", b),
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
