//! Module for representing S-exps.

use std::convert::TryFrom;
use std::fmt;
use std::str::FromStr;

use crate::parser::{parse_sexp, ParseError};
use crate::primitive::{BuiltIn, NodeId, Number, Primitive, Symbol, SymbolTable};
use crate::token::string_stream::StringStream;
use crate::token::TokenizeError;


pub type HeapSexp = Box<Sexp>;

#[derive(Clone, PartialEq)]
pub enum Sexp {
    Primitive(Primitive),
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

#[derive(Debug)]
pub enum FromStrError {
    TokenizeError(TokenizeError),
    ParseError(ParseError),
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
        let mut outer_quote = false;
        for val in self.iter() {
            if pos == 0 {
                if let Ok(symbol) = <&Symbol>::try_from(val) {
                    if symbol.as_str() == "quote" {
                        outer_quote = true;
                        write!(f, "'")?;
                        pos += 1;
                        continue;
                    }
                }
                write!(f, "(")?;
            }

            if pos >= MAX_DISPLAY_LENGTH {
                write!(f, "...")?;
                break;
            }

            if pos > 0 && !outer_quote {
                write!(f, " ")?;
            }
            write!(f, "{}", val)?;

            pos += 1;
        }

        if pos == 0 {
            write!(f, "(")?;
        }
        if !outer_quote { write!(f, ")") } else { Ok(()) }
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
    type Err = FromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let stream = match StringStream::new(s) {
            Ok(stream) => stream,
            Err(err) => return Err(FromStrError::TokenizeError(err)),
        };

        return match parse_sexp(&mut stream.peekable(), 0) {
            Ok(Some(sexp)) => Ok(*sexp),
            Ok(None) => Ok(Sexp::Cons(Cons::default())),
            Err(err) => Err(FromStrError::ParseError(err)),
        };
    }
}


impl From<HeapSexp> for Sexp {
    fn from(sexp: HeapSexp) -> Self {
        match *sexp {
            Sexp::Primitive(primitive) => Sexp::Primitive(primitive),
            Sexp::Cons(cons) => Sexp::Cons(cons),
        }
    }
}

impl From<Primitive> for Sexp {
    fn from(primitive: Primitive) -> Self {
        Sexp::Primitive(primitive)
    }
}

impl From<Cons> for Sexp {
    fn from(cons: Cons) -> Self {
        Sexp::Cons(cons)
    }
}

impl From<Number> for Sexp {
    fn from(number: Number) -> Self {
        Sexp::Primitive(Primitive::Number(number))
    }
}

impl From<Symbol> for Sexp {
    fn from(symbol: Symbol) -> Self {
        Sexp::Primitive(Primitive::Symbol(symbol))
    }
}

impl From<SymbolTable> for Sexp {
    fn from(table: SymbolTable) -> Self {
        Sexp::Primitive(Primitive::SymbolTable(table))
    }
}

impl From<BuiltIn> for Sexp {
    fn from(builtin: BuiltIn) -> Self {
        Sexp::Primitive(Primitive::BuiltIn(builtin))
    }
}

impl From<NodeId> for Sexp {
    fn from(node: NodeId) -> Self {
        Sexp::Primitive(Primitive::Node(node))
    }
}
