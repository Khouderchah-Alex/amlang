//! Module for representing S-exps.

use std::convert::TryFrom;
use std::fmt;
use std::str::FromStr;

use super::cons_list::ConsList;
use crate::environment::Environment;
use crate::lang_err::{ExpectedCount, LangErr};
use crate::parser::{parse_sexp, ParseError};
use crate::primitive::symbol_policies::policy_base;
use crate::primitive::{
    AmString, BuiltIn, LocalNodeTable, Node, Number, Path, Primitive, Procedure, Symbol,
    SymbolTable,
};
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

    pub fn cons(&self) -> &Cons {
        if let Sexp::Cons(c) = self {
            return c;
        }
        panic!("Expected {:?} to be Cons", self);
    }

    // TODO This needs to be merged with list_fmt. Struggling to make generic
    // over io:: and fmt::Write led to this duplication.
    pub fn write_list<W, F>(
        &self,
        w: &mut W,
        depth: usize,
        write_primitive: &mut F,
    ) -> std::io::Result<()>
    where
        W: std::io::Write,
        F: FnMut(&mut W, &Primitive, usize) -> std::io::Result<()>,
    {
        // Any list longer than this will simply be suffixed with "..." after these
        // many elements.
        const MAX_DISPLAY_LENGTH: usize = 64;
        const MAX_DISPLAY_DEPTH: usize = 32;

        if let Sexp::Primitive(primitive) = self {
            return write_primitive(w, primitive, depth);
        };

        if depth >= MAX_DISPLAY_DEPTH {
            return write!(w, "(..)");
        }

        let mut pos: usize = 0;
        let mut outer_quote = false;
        for val in self.cons().iter() {
            if pos == 0 {
                if let Ok(symbol) = <&Symbol>::try_from(val) {
                    if symbol.as_str() == "quote" {
                        outer_quote = true;
                        write!(w, "'")?;
                        pos += 1;
                        continue;
                    }
                }
                write!(w, "(")?;
            }

            if pos >= MAX_DISPLAY_LENGTH {
                write!(w, "...")?;
                break;
            }

            if pos > 0 && !outer_quote {
                write!(w, " ")?;
            }
            val.write_list(w, depth + 1, write_primitive)?;

            pos += 1;
        }

        if pos == 0 {
            write!(w, "(")?;
        }
        if !outer_quote {
            write!(w, ")")?;
        }
        Ok(())
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

    fn list_fmt(&self, f: &mut fmt::Formatter<'_>, depth: usize) -> fmt::Result {
        // Any list longer than this will simply be suffixed with "..." after these
        // many elements.
        const MAX_DISPLAY_LENGTH: usize = 64;
        const MAX_DISPLAY_DEPTH: usize = 32;

        if depth >= MAX_DISPLAY_DEPTH {
            return write!(f, "(..)");
        }

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
            match val {
                Sexp::Primitive(primitive) => write!(f, "{}", primitive)?,
                Sexp::Cons(cons) => cons.list_fmt(f, depth + 1)?,
            }

            pos += 1;
        }

        if pos == 0 {
            write!(f, "(")?;
        }
        if !outer_quote { write!(f, ")") } else { Ok(()) }
    }
}

impl SexpIntoIter {
    pub fn consume(self) -> Option<HeapSexp> {
        self.current.map(|c| HeapSexp::new(Sexp::Cons(c)))
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
            return self.list_fmt(f, 0);
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

impl TryFrom<Sexp> for Primitive {
    type Error = ();

    fn try_from(value: Sexp) -> Result<Self, Self::Error> {
        if let Sexp::Primitive(primitive) = value {
            Ok(primitive)
        } else {
            Err(())
        }
    }
}

impl<'a> TryFrom<&'a Sexp> for &'a Primitive {
    type Error = ();

    fn try_from(value: &'a Sexp) -> Result<Self, Self::Error> {
        if let Sexp::Primitive(primitive) = value {
            Ok(primitive)
        } else {
            Err(())
        }
    }
}

impl TryFrom<Sexp> for Cons {
    type Error = ();

    fn try_from(value: Sexp) -> Result<Self, Self::Error> {
        if let Sexp::Cons(cons) = value {
            Ok(cons)
        } else {
            Err(())
        }
    }
}

impl<'a> TryFrom<&'a Sexp> for &'a Cons {
    type Error = ();

    fn try_from(value: &'a Sexp) -> Result<Self, Self::Error> {
        if let Sexp::Cons(cons) = value {
            Ok(cons)
        } else {
            Err(())
        }
    }
}

impl<'a> TryFrom<Option<&'a Sexp>> for &'a Cons {
    type Error = ();

    fn try_from(value: Option<&'a Sexp>) -> Result<Self, Self::Error> {
        if let Some(Sexp::Cons(cons)) = value {
            Ok(cons)
        } else {
            Err(())
        }
    }
}

impl TryFrom<Option<HeapSexp>> for Cons {
    type Error = ();

    fn try_from(value: Option<HeapSexp>) -> Result<Self, Self::Error> {
        if let Some(heap) = value {
            if let Sexp::Cons(cons) = *heap {
                return Ok(cons);
            }
        }
        Err(())
    }
}

impl<E> TryFrom<Result<Sexp, E>> for Cons {
    type Error = ();

    fn try_from(value: Result<Sexp, E>) -> Result<Self, Self::Error> {
        if let Ok(Sexp::Cons(cons)) = value {
            Ok(cons)
        } else {
            Err(())
        }
    }
}

impl<'a, E> TryFrom<&'a Result<Sexp, E>> for &'a Cons {
    type Error = ();

    fn try_from(value: &'a Result<Sexp, E>) -> Result<Self, Self::Error> {
        if let Ok(Sexp::Cons(cons)) = value {
            Ok(cons)
        } else {
            Err(())
        }
    }
}

impl TryFrom<Sexp> for SexpIntoIter {
    type Error = LangErr;

    fn try_from(value: Sexp) -> Result<Self, Self::Error> {
        match value {
            Sexp::Primitive(primitive) => err_nost!(InvalidSexp(primitive.clone().into())),
            Sexp::Cons(cons) => Ok(cons.into_iter()),
        }
    }
}

impl TryFrom<Option<HeapSexp>> for SexpIntoIter {
    type Error = LangErr;

    fn try_from(value: Option<HeapSexp>) -> Result<Self, Self::Error> {
        match value {
            Some(sexp) => match *sexp {
                Sexp::Primitive(primitive) => err_nost!(InvalidSexp(primitive.clone().into())),
                Sexp::Cons(cons) => Ok(cons.into_iter()),
            },
            None => err_nost!(WrongArgumentCount {
                given: 0,
                expected: ExpectedCount::AtLeast(1),
            }),
        }
    }
}

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

impl From<AmString> for Sexp {
    fn from(string: AmString) -> Self {
        Sexp::Primitive(Primitive::AmString(string))
    }
}

impl From<Path> for Sexp {
    fn from(path: Path) -> Self {
        Sexp::Primitive(Primitive::Path(path))
    }
}

impl From<SymbolTable> for Sexp {
    fn from(table: SymbolTable) -> Self {
        Sexp::Primitive(Primitive::SymbolTable(table))
    }
}

impl From<LocalNodeTable> for Sexp {
    fn from(table: LocalNodeTable) -> Self {
        Sexp::Primitive(Primitive::LocalNodeTable(table))
    }
}

impl From<BuiltIn> for Sexp {
    fn from(builtin: BuiltIn) -> Self {
        Sexp::Primitive(Primitive::BuiltIn(builtin))
    }
}

impl From<Procedure> for Sexp {
    fn from(procedure: Procedure) -> Self {
        Sexp::Primitive(Primitive::Procedure(procedure))
    }
}

impl From<Node> for Sexp {
    fn from(node: Node) -> Self {
        Sexp::Primitive(Primitive::Node(node))
    }
}

impl<T: 'static + Environment> From<Box<T>> for Sexp {
    fn from(env: Box<T>) -> Self {
        Sexp::Primitive(Primitive::Env(env))
    }
}


#[cfg(test)]
#[path = "./sexp_test.rs"]
mod sexp_test;
