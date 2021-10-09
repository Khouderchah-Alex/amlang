//! Module for representing S-exps.

use std::convert::TryFrom;
use std::fmt;
use std::io::Write;
use std::str::FromStr;

use super::cons_list::ConsList;
use super::fmt_io_bridge::FmtIoBridge;
use crate::environment::Environment;
use crate::lang_err::{ExpectedCount, LangErr};
use crate::parser::{parse_sexp, ParseError};
use crate::primitive::prelude::*;
use crate::primitive::symbol_policies::policy_base;
use crate::token::string_stream::StringStream;
use crate::token::TokenizeError;


pub type HeapSexp = Box<Sexp>;

#[derive(Clone, PartialEq)]
pub enum Sexp {
    Primitive(Primitive),
    Cons(Cons),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Cons {
    car: Option<HeapSexp>,
    cdr: Option<HeapSexp>,
}

pub struct SexpIter<'a> {
    current: Option<&'a Sexp>,
}

pub struct SexpIntoIter {
    current: Option<HeapSexp>,
}

// Consider using convenience macros in sexp_conversion rather than directly
// using this.
pub fn cons(car: Option<HeapSexp>, cdr: Option<HeapSexp>) -> Option<HeapSexp> {
    Some(HeapSexp::new(Sexp::Cons(Cons::new(car, cdr))))
}

#[derive(Debug)]
pub enum FromStrError {
    TokenizeError(TokenizeError),
    ParseError(ParseError),
}


impl Sexp {
    pub fn is_none(&self) -> bool {
        if let Sexp::Cons(c) = self {
            c.car == None && c.cdr == None
        } else {
            false
        }
    }

    pub fn iter(&self) -> SexpIter {
        SexpIter {
            current: Some(&self),
        }
    }

    pub fn cons(&self) -> &Cons {
        if let Sexp::Cons(c) = self {
            return c;
        }
        panic!("Expected {:?} to be Cons", self);
    }

    pub fn write_list<W, F, P>(
        &self,
        w: &mut W,
        depth: usize,
        write_primitive: &mut F,
        write_paren: &mut P,
    ) -> std::io::Result<()>
    where
        W: std::io::Write,
        F: FnMut(&mut W, &Primitive, usize) -> std::io::Result<()>,
        P: FnMut(&mut W, &str, usize) -> std::io::Result<()>,
    {
        // Any list longer than this will simply be suffixed with "..." after these
        // many elements.
        const MAX_DISPLAY_LENGTH: usize = 64;
        const MAX_DISPLAY_DEPTH: usize = 32;

        let mut pos: usize = 0;
        let mut outer_quote = false;
        for (val, from_cons) in self.iter() {
            if pos == 0 {
                if !from_cons {
                    if let Sexp::Primitive(primitive) = self {
                        return write_primitive(w, primitive, depth);
                    };
                }

                if depth >= MAX_DISPLAY_DEPTH {
                    return write!(w, "(..)");
                }
                if let Ok(symbol) = <&Symbol>::try_from(val) {
                    if symbol.as_str() == "quote" {
                        outer_quote = true;
                        write!(w, "'")?;
                        pos += 1;
                        continue;
                    }
                }
                write_paren(w, "(", depth)?;
            }

            if pos >= MAX_DISPLAY_LENGTH {
                write!(w, "...")?;
                break;
            }

            if pos > 0 && !outer_quote {
                write!(w, " ")?;
            }
            val.write_list(w, depth + 1, write_primitive, write_paren)?;

            pos += 1;
        }

        if pos == 0 {
            write_paren(w, "(", depth)?;
        }
        if !outer_quote {
            write_paren(w, ")", depth)?;
        }
        Ok(())
    }
}

impl Cons {
    pub fn new(car: Option<HeapSexp>, cdr: Option<HeapSexp>) -> Cons {
        Cons { car, cdr }
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
}

impl SexpIntoIter {
    pub fn consume(self) -> Option<HeapSexp> {
        self.current
    }
}


impl<'a> Iterator for SexpIter<'a> {
    // (Sexp, from_cons).
    //
    // If from_cons is false, it means the HeapSexp is a top-level Primitive
    // rather than the car of a Cons. If from_cons is false, this is necessarily
    // the last element (since there is no Cons to get a cdr from).
    type Item = (&'a Sexp, bool);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(sexp) = self.current {
            match sexp {
                Sexp::Cons(cons) => {
                    self.current = cons.cdr();
                    cons.car().map(|s| (s, true))
                }
                _ => Some((sexp, false)),
            }
        } else {
            None
        }
    }
}

impl<'a> IntoIterator for &'a Sexp {
    // (Sexp, from_cons). See impl Iterator blocks above for more info.
    type Item = (&'a Sexp, bool);
    type IntoIter = SexpIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl Iterator for SexpIntoIter {
    // (Sexp, from_cons).
    //
    // If from_cons is false, it means the HeapSexp is a top-level Primitive
    // rather than the car of a Cons. If from_cons is false, this is necessarily
    // the last element (since there is no Cons to get a cdr from).
    type Item = (HeapSexp, bool);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(sexp) = self.current.take() {
            match *sexp {
                Sexp::Cons(cons) => {
                    let (car, cdr) = cons.consume();
                    self.current = cdr;
                    car.map(|s| (s, true))
                }
                _ => Some((sexp, false)),
            }
        } else {
            None
        }
    }
}

// TODO(perf) Reduce the need of this by passing around HeapSexps.
impl IntoIterator for Sexp {
    // (Sexp, from_cons). See impl Iterator blocks above for more info.
    type Item = (HeapSexp, bool);
    type IntoIter = SexpIntoIter;

    fn into_iter(self) -> Self::IntoIter {
        SexpIntoIter {
            current: Some(Box::new(self)),
        }
    }
}

impl IntoIterator for HeapSexp {
    // (Sexp, from_cons). See impl Iterator blocks above for more info.
    type Item = (HeapSexp, bool);
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

impl Default for Sexp {
    fn default() -> Self {
        Sexp::Cons(Cons::default())
    }
}

impl fmt::Display for Sexp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.write_list(
            &mut FmtIoBridge::new(f),
            0,
            &mut |writer, primitive, _depth| write!(writer, "{}", primitive),
            &mut |writer, paren, _depth| write!(writer, "{}", paren),
        ) {
            Ok(()) => Ok(()),
            Err(_) => Err(fmt::Error),
        }
    }
}


// TryFrom<Sexp-like> impls.
impl TryFrom<Sexp> for Primitive {
    type Error = Sexp;

    fn try_from(value: Sexp) -> Result<Self, Self::Error> {
        if let Sexp::Primitive(primitive) = value {
            Ok(primitive)
        } else {
            Err(value)
        }
    }
}

impl<'a> TryFrom<&'a Sexp> for &'a Primitive {
    type Error = &'a Sexp;

    fn try_from(value: &'a Sexp) -> Result<Self, Self::Error> {
        if let Sexp::Primitive(primitive) = value {
            Ok(primitive)
        } else {
            Err(value)
        }
    }
}

impl TryFrom<Sexp> for Cons {
    type Error = Sexp;

    fn try_from(value: Sexp) -> Result<Self, Self::Error> {
        if let Sexp::Cons(cons) = value {
            Ok(cons)
        } else {
            Err(value)
        }
    }
}

impl<'a> TryFrom<&'a Sexp> for &'a Cons {
    type Error = &'a Sexp;

    fn try_from(value: &'a Sexp) -> Result<Self, Self::Error> {
        if let Sexp::Cons(cons) = value {
            Ok(cons)
        } else {
            Err(value)
        }
    }
}

impl<'a> TryFrom<Option<&'a Sexp>> for &'a Cons {
    type Error = Option<&'a Sexp>;

    fn try_from(value: Option<&'a Sexp>) -> Result<Self, Self::Error> {
        if let Some(Sexp::Cons(cons)) = value {
            Ok(cons)
        } else {
            Err(value)
        }
    }
}

impl TryFrom<Option<HeapSexp>> for Cons {
    type Error = Option<HeapSexp>;

    fn try_from(value: Option<HeapSexp>) -> Result<Self, Self::Error> {
        if let Some(heap) = value {
            if let Sexp::Cons(cons) = *heap {
                return Ok(cons);
            }
            return Err(Some(heap));
        }
        Err(None)
    }
}

impl<E> TryFrom<Result<Sexp, E>> for Cons {
    type Error = Result<Sexp, E>;

    fn try_from(value: Result<Sexp, E>) -> Result<Self, Self::Error> {
        if let Ok(Sexp::Cons(cons)) = value {
            Ok(cons)
        } else {
            Err(value)
        }
    }
}

impl<'a, E> TryFrom<&'a Result<Sexp, E>> for &'a Cons {
    type Error = &'a Result<Sexp, E>;

    fn try_from(value: &'a Result<Sexp, E>) -> Result<Self, Self::Error> {
        if let Ok(Sexp::Cons(cons)) = value {
            Ok(cons)
        } else {
            Err(value)
        }
    }
}

impl TryFrom<Option<HeapSexp>> for SexpIntoIter {
    type Error = LangErr;

    fn try_from(value: Option<HeapSexp>) -> Result<Self, Self::Error> {
        match value {
            Some(sexp) => Ok(sexp.into_iter()),
            None => err_nost!(WrongArgumentCount {
                given: 0,
                expected: ExpectedCount::AtLeast(1),
            }),
        }
    }
}


// From<T> impls.
impl FromStr for Sexp {
    type Err = FromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let stream = match StringStream::new(s, policy_base) {
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

impl<T: Into<Sexp>> From<Vec<T>> for Sexp {
    fn from(vec: Vec<T>) -> Self {
        let mut list = ConsList::new();
        for value in vec {
            list.append(Box::new(value.into()));
        }
        *list.release()
    }
}

impl<'a, T: Into<Sexp> + Clone> From<&'a Vec<T>> for Sexp {
    fn from(vec: &'a Vec<T>) -> Self {
        let mut list = ConsList::new();
        for value in vec {
            list.append(Box::new(value.clone().into()));
        }
        *list.release()
    }
}

// Used by break_by_types when taking a Sexp.
impl From<std::convert::Infallible> for Sexp {
    fn from(_: std::convert::Infallible) -> Self {
        Self::default()
    }
}

impl From<Sexp> for Option<HeapSexp> {
    fn from(sexp: Sexp) -> Self {
        if sexp.is_none() {
            None
        } else {
            Some(HeapSexp::new(sexp))
        }
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

// Impl From<T> over Primitive subtypes.
macro_rules! sexp_from {
    ($from:ident, $($tail:tt)*) => {
        impl From<$from> for Sexp {
            fn from(elem: $from) -> Self {
                Sexp::Primitive(Primitive::$from(elem))
            }
        }
        sexp_from!($($tail)*);
    };
    () => {};
}

sexp_from!(
    Number,
    Symbol,
    AmString,
    BuiltIn,
    Node,
    Path,
    SymbolTable,
    LocalNodeTable,
    Procedure,
);

impl<T: 'static + Environment> From<Box<T>> for Sexp {
    fn from(env: Box<T>) -> Self {
        Sexp::Primitive(Primitive::Env(env))
    }
}


#[cfg(test)]
#[path = "./sexp_test.rs"]
mod sexp_test;
