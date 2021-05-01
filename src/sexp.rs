//! Module for representing S-exps.

use std::fmt;
use std::str::FromStr;

use crate::parser::{parse_sexp, ParseError};
use crate::primitive;
use crate::token::string_stream::StringStream;


pub type HeapSexp = Box<Sexp>;

#[derive(Clone, PartialEq)]
pub enum Sexp {
    Primitive(primitive::Primitive),
    Cons(Cons),
}

#[derive(Clone, Default, PartialEq)]
pub struct Cons {
    car: Option<HeapSexp>,
    cdr: Option<HeapSexp>,
}

pub struct SexpIter<'a> {
    current: Option<&'a Cons>,
}

pub struct SexpIntoIter {
    current: Option<Cons>,
}

pub fn cons(car: Option<HeapSexp>, cdr: Option<HeapSexp>) -> Option<HeapSexp> {
    Some(HeapSexp::new(Sexp::Cons(Cons::new(car, cdr))))
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
    pub fn new(car: Option<HeapSexp>, cdr: Option<HeapSexp>) -> Cons {
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

    pub fn consume(self) -> (Option<HeapSexp>, Option<HeapSexp>) {
        (self.car, self.cdr)
    }

    pub fn set_cdr(&mut self, new: Option<HeapSexp>) {
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
            write!(f, "{}", val)?;

            pos += 1;
        }
        write!(f, ")")
    }
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

impl<'a> IntoIterator for &'a Cons {
    type Item = &'a Sexp;
    type IntoIter = SexpIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl Iterator for SexpIntoIter {
    type Item = HeapSexp;

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
    type Item = HeapSexp;
    type IntoIter = SexpIntoIter;

    fn into_iter(self) -> Self::IntoIter {
        SexpIntoIter {
            current: Some(self),
        }
    }
}

impl fmt::Debug for Sexp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self, f)
    }
}

impl fmt::Debug for Cons {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self, f)
    }
}

impl Default for Sexp {
    fn default() -> Self {
        Sexp::Cons(Cons::default())
    }
}

impl fmt::Display for Sexp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Sexp::Primitive(primitive) => write!(f, "{}", primitive),
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

impl fmt::Display for Cons {
    /// Note: alternate does not check for loops and doesn't have a max depth.
    /// Do NOT use the alternate formatting for untrusted S-exps.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Normal print this with list shorthand.
        if !f.alternate() {
            return self.list_fmt(f);
        }

        // Alternate print as sets of Cons.
        write!(f, "(")?;
        match self.car() {
            Some(val) => write!(f, "{:#}", val)?,
            None => write!(f, "NIL")?,
        };
        write!(f, " . ")?;
        match self.cdr() {
            Some(val) => write!(f, "{:#}", val)?,
            None => write!(f, "NIL")?,
        };
        write!(f, ")")
    }
}

impl FromStr for Sexp {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let stream = StringStream::new(s);

        return match parse_sexp(&mut stream.peekable(), 0) {
            Ok(Some(sexp)) => Ok(*sexp),
            Ok(None) => Ok(Sexp::Cons(Cons::default())),
            Err(err) => Err(err),
        };
    }
}
