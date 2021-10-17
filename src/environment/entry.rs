//! Abstraction of the ownership semantics of Environment accesses.
//!
//! As an example, while a single-threaded deployment may expose references to
//! Node entries, a multi-threaded or multi-process deployment may have entries
//! which pass ownership. In a sense, Entry* classes have a mix of Option and
//! Cow semantics, with ownership/borrow semantics that allow for a wide range
//! of Entry impls.
//!
//! This module provides an abstraction surface for both Environment impls and
//! clients to use, which allows for operationally-dependent policy to be
//! decoupled from the library and much client code. Currently, the plan is to
//! use a build script to be able to replace this module without forcing
//! essentially the entire codebase to be generic over Entry* types.
// TODO(flex) Implement build script solution.

use crate::environment::environment::EnvObject;
use crate::environment::LocalNode;
use crate::primitive::Primitive;
use crate::sexp::Sexp;


pub enum Entry<'a> {
    Atomic,
    Structured(&'a Sexp),
}

pub struct EntryMut<'a> {
    node: LocalNode,
    kind: EntryMutKind<'a>,
}

enum EntryMutKind<'a> {
    Atomic,
    Structured(&'a mut Sexp),
}


impl<'a> Entry<'a> {
    /// Similar to Option::unwrap, but ties reference ownership back to &self,
    /// since Entries may own the structures in some impls.
    ///
    /// For this reason, it's preferable to call Entry::owned() rather than
    /// clone()ing this reference.
    pub fn structure(&self) -> &Sexp {
        match self {
            Self::Structured(sexp) => sexp,
            _ => panic!(),
        }
    }

    /// Presents an Option, but ties reference ownership back to &self,
    /// since Entries may own the structures in some impls.
    ///
    /// For this reason, it's preferable to call Entry::owned() rather than
    /// calling cloned() on this.
    pub fn as_option(&self) -> Option<&Sexp> {
        match self {
            Self::Atomic => None,
            Self::Structured(sexp) => Some(sexp),
        }
    }

    /// Presents an Option with ownership. Since Entries may own the
    /// structures in some impls, this may simply pass ownership without
    /// additional copies.
    ///
    /// Note that EntryMut does not provide this method, since EntryMut impls
    /// may need to update the underlying Environment on drop.
    pub fn owned(self) -> Option<Sexp> {
        self.as_option().cloned()
    }
}

impl<'a> EntryMut<'a> {
    pub(super) fn new(node: LocalNode, data: Option<&'a mut Sexp>) -> Self {
        let kind = match data {
            None => EntryMutKind::Atomic,
            Some(sexp) => EntryMutKind::Structured(sexp),
        };
        Self { node, kind }
    }

    /// Similar to Option::unwrap, but ties reference ownership back to
    /// &mut self, since Entries may own the structures in some impls.
    pub fn structure(&mut self) -> &mut Sexp {
        match &mut self.kind {
            EntryMutKind::Structured(sexp) => sexp,
            _ => panic!(),
        }
    }

    /// Presents an Option, but ties reference ownership back to &mut self,
    /// since Entries may own the structures in some impls.
    pub fn as_option(&mut self) -> Option<&mut Sexp> {
        match &mut self.kind {
            EntryMutKind::Atomic => None,
            EntryMutKind::Structured(sexp) => Some(sexp),
        }
    }

    /// Return as a &mut Env if posssible. Ownership of the reference ties back
    /// to that of the Environment which created this Entry.
    pub fn env(self) -> Option<&'a mut Box<EnvObject>> {
        match self.kind {
            EntryMutKind::Structured(Sexp::Primitive(Primitive::Env(env))) => Some(env),
            _ => None,
        }
    }
}


impl<'a> From<Option<&'a Sexp>> for Entry<'a> {
    fn from(option: Option<&'a Sexp>) -> Self {
        match option {
            None => Self::Atomic,
            Some(sexp) => Self::Structured(sexp),
        }
    }
}
