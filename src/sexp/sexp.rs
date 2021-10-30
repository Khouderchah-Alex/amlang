//! Representation of S-expressions, as either a Primitive or pair of HeapSexps.
//!
//! Unlike S-expressions in traditional Lisps, Sexp cannot directly represent
//! cycles, and Cons cells have unique ownership over their car & cdr. Cycles
//! can still be created through the use of Nodes (for example, a list can be
//! made circular by inserting it into an Environment and replacing the final
//! cdr with its corresponding Node).
//!
//! This has the benefit of giving clients control over lifetimes of entire
//! Sexps without precluding representational capability. An interesting
//! downstream result is that cycle detection can be performed by checking
//! Nodes/looking at Environment traversals rather than memory accesses, and
//! that Sexps not containing Nodes are inherently cycle-free.
//!
//!
//! Ownership:
//!   Heuristically, Sexp is preferred for passing ownership when "building up"
//!   S-expressions or using them in some local context, while HeapSexp is
//!   preferred when "breaking down" S-expressions.
//!
//!   In general, we simply want to defer moving a Sexp to the heap until we
//!   need to (usually because we're passing ownership to a Cons). On the other
//!   hand, if we're already consuming HeapSexps from Cons cells, we'd rather
//!   leave them on the heap in case the ownership ultimately ends back in a
//!   Cons cell. Realistically, the cost of copying a Sexp b/w stack and heap is
//!   not a huge deal outside of hot spots; rather, this convention serves to
//!   prevent scenarios in which a chain of function calls involves
//!   unnecessarily copying Sexps b/w stack and heap many times.

use std::convert::TryFrom;
use std::fmt;
use std::io::Write;
use std::str::FromStr;

use super::cons::Cons;
use super::cons_list::ConsList;
use super::fmt_io_adapter::FmtIoAdapter;
use crate::environment::Environment;
use crate::parser::{parse_sexp, ParseError};
use crate::primitive::error::Error;
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

pub struct SexpIter<'a> {
    current: Option<&'a Sexp>,
}

pub enum SexpIntoIter {
    None,
    Stack(Sexp),
    Heap(HeapSexp),
}

#[derive(Debug)]
pub enum FromStrError {
    TokenizeError(TokenizeError),
    ParseError(ParseError),
}

impl Sexp {
    pub fn is_none(&self) -> bool {
        if let Sexp::Cons(c) = self {
            c.car() == None && c.cdr() == None
        } else {
            false
        }
    }

    pub fn iter(&self) -> SexpIter {
        SexpIter {
            current: Some(&self),
        }
    }

    pub fn write<W, F, P>(
        &self,
        w: &mut W,
        depth: usize,
        write_primitive: &mut F,
        write_paren: &mut P,
        max_length: Option<usize>,
        max_depth: Option<usize>,
    ) -> std::io::Result<()>
    where
        W: std::io::Write,
        F: FnMut(&mut W, &Primitive, usize) -> std::io::Result<()>,
        P: FnMut(&mut W, &str, usize) -> std::io::Result<()>,
    {
        let mut pos: usize = 0;
        let mut outer_quote = false;
        for (val, proper) in self.iter() {
            if pos == 0 {
                if !proper {
                    if let Sexp::Primitive(primitive) = self {
                        return write_primitive(w, primitive, depth);
                    };
                }

                if let Some(max) = max_depth {
                    if depth >= max {
                        return write!(w, "(..)");
                    }
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

            if let Some(max) = max_length {
                if pos >= max {
                    write!(w, " ...")?;
                    break;
                }
            }

            if pos > 0 && !outer_quote {
                if proper {
                    write!(w, " ")?;
                } else {
                    write!(w, " . ")?;
                }
            }
            val.write(
                w,
                depth + 1,
                write_primitive,
                write_paren,
                max_length,
                max_depth,
            )?;

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


impl SexpIntoIter {
    pub fn consume(self) -> Option<HeapSexp> {
        match self {
            Self::None => None,
            Self::Stack(sexp) => sexp.into(),
            Self::Heap(hsexp) => Some(hsexp),
        }
    }
}

impl<'a> SexpIter<'a> {
    pub fn consume(self) -> Option<&'a Sexp> {
        self.current
    }
}

impl<'a> Iterator for SexpIter<'a> {
    // (&Sexp, proper).
    //
    // If proper is false, it means the HeapSexp is a top-level Primitive
    // rather than the car of a Cons. If proper is false, this is necessarily
    // the last element (since there is no Cons to get a cdr from).
    type Item = (&'a Sexp, bool);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(sexp) = self.current {
            match sexp {
                Sexp::Cons(cons) => {
                    self.current = cons.cdr();
                    cons.car().map(|s| (s, true))
                }
                _ => {
                    self.current = None;
                    Some((sexp, false))
                }
            }
        } else {
            None
        }
    }
}

impl<'a> IntoIterator for &'a Sexp {
    // (&Sexp, proper). See impl Iterator blocks above for more info.
    type Item = (&'a Sexp, bool);
    type IntoIter = SexpIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl Default for SexpIntoIter {
    fn default() -> Self {
        Self::None
    }
}

impl<'a> Default for SexpIter<'a> {
    fn default() -> Self {
        Self { current: None }
    }
}

impl Iterator for SexpIntoIter {
    // (HeapSexp, proper).
    //
    // If proper is false, it means the HeapSexp is a top-level Primitive
    // rather than the car of a Cons. If proper is false, this is necessarily
    // the last element (since there is no Cons to get a cdr from).
    type Item = (HeapSexp, bool);

    fn next(&mut self) -> Option<Self::Item> {
        // Self set to None unless some special-case keeps the iteration going.
        let head = std::mem::replace(self, Self::None);
        match head {
            Self::None => None,
            Self::Stack(sexp) => match sexp {
                Sexp::Cons(cons) => {
                    let (car, cdr) = cons.consume();
                    if let Some(hsexp) = cdr {
                        *self = Self::Heap(hsexp);
                    }
                    car.map(|s| (s, true))
                }
                // We only need to do this if we call into_iter on a Primitive.
                _ => Some((HeapSexp::new(sexp), false)),
            },
            Self::Heap(hsexp) => match *hsexp {
                Sexp::Cons(cons) => {
                    let (car, cdr) = cons.consume();
                    if let Some(hsexp) = cdr {
                        *self = Self::Heap(hsexp);
                    }
                    car.map(|s| (s, true))
                }
                _ => Some((hsexp, false)),
            },
        }
    }
}

impl IntoIterator for Sexp {
    // (HeapSexp, proper). See impl Iterator blocks above for more info.
    type Item = (HeapSexp, bool);
    type IntoIter = SexpIntoIter;

    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter::Stack(self)
    }
}

impl IntoIterator for HeapSexp {
    // (HeapSexp, proper). See impl Iterator blocks above for more info.
    type Item = (HeapSexp, bool);
    type IntoIter = SexpIntoIter;

    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter::Heap(self)
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
        // Any list longer than this will simply be suffixed with "..." after these
        // many elements.
        const MAX_LENGTH: usize = 64;
        const MAX_DEPTH: usize = 16;

        match self.write(
            &mut FmtIoAdapter::new(f),
            0,
            &mut |writer, primitive, _depth| write!(writer, "{}", primitive),
            &mut |writer, paren, _depth| write!(writer, "{}", paren),
            Some(MAX_LENGTH),
            Some(MAX_DEPTH),
        ) {
            Ok(()) => Ok(()),
            Err(_) => Err(fmt::Error),
        }
    }
}


// TryFrom<Sexp-like> impls.
impl TryFrom<Sexp> for Primitive {
    type Error = Sexp;

    fn try_from(value: Sexp) -> Result<Self, <Self as TryFrom<Sexp>>::Error> {
        if let Sexp::Primitive(primitive) = value {
            Ok(primitive)
        } else {
            Err(value)
        }
    }
}

impl TryFrom<HeapSexp> for Primitive {
    type Error = HeapSexp;

    fn try_from(value: HeapSexp) -> Result<Self, <Self as TryFrom<HeapSexp>>::Error> {
        if let Sexp::Primitive(primitive) = *value {
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


// From<T> impls.
impl FromStr for Sexp {
    type Err = FromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let stream = match StringStream::new(s, policy_base) {
            Ok(stream) => stream,
            Err(err) => return Err(FromStrError::TokenizeError(err)),
        };

        return match parse_sexp(&mut stream.peekable(), 0) {
            Ok(Some(sexp)) => Ok(sexp),
            Ok(None) => Ok(Sexp::default()),
            Err(err) => Err(FromStrError::ParseError(err)),
        };
    }
}

impl<T: Into<Sexp>> From<Vec<T>> for Sexp {
    fn from(vec: Vec<T>) -> Self {
        let mut list = ConsList::new();
        for value in vec {
            list.append(HeapSexp::new(value.into()));
        }
        list.release()
    }
}

impl<'a, T: Into<Sexp> + Clone> From<&'a Vec<T>> for Sexp {
    fn from(vec: &'a Vec<T>) -> Self {
        let mut list = ConsList::new();
        for value in vec {
            list.append(HeapSexp::new(value.clone().into()));
        }
        list.release()
    }
}

impl From<Option<HeapSexp>> for SexpIntoIter {
    fn from(value: Option<HeapSexp>) -> Self {
        match value {
            Some(sexp) => sexp.into_iter(),
            None => Self::default(),
        }
    }
}

// Used by break_sexp when taking a Sexp.
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

impl From<Cons> for HeapSexp {
    fn from(cons: Cons) -> Self {
        HeapSexp::new(Sexp::Cons(cons))
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
        impl From<$from> for HeapSexp {
            fn from(elem: $from) -> Self {
                Self::new(Sexp::Primitive(Primitive::$from(elem)))
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
    Error,
);

impl<T: 'static + Environment> From<Box<T>> for Sexp {
    fn from(env: Box<T>) -> Self {
        Sexp::Primitive(Primitive::Env(env))
    }
}


#[cfg(test)]
#[path = "./sexp_test.rs"]
mod sexp_test;
