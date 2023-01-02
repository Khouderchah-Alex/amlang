//! Abstraction of the ownership semantics of Environment accesses.
//!
//! As an example, while a single-threaded deployment may expose references to
//! Node entries, a multi-threaded or multi-process deployment may have entries
//! which pass ownership. In a sense, Entry* classes have a mix of Option and
//! Cow semantics, with ownership/borrow semantics that allow for a wide range
//! of Environment impls.

use crate::environment::environment::EnvObject;
use crate::environment::LocalNode;
use crate::primitive::Primitive;
use crate::sexp::Sexp;


pub struct Entry<'a> {
    kind: EntryKind<'a>,
}

#[derive(Debug, PartialEq)]
pub enum EntryKind<'a> {
    Atomic,
    Borrowed(&'a Sexp),
    Owned(Sexp),
}

/// Mutable entry that updates the Environment implicitly on drop or explicitly
/// on update().
pub struct EntryMut<'a> {
    node: LocalNode,
    kind: EntryMutKind<'a>,
    env: Option<*mut EnvObject>,
}

#[derive(Debug, PartialEq)]
pub enum EntryMutKind<'a> {
    Atomic,
    Borrowed(&'a mut Sexp),
    Owned(Sexp),
}


impl<'a> Entry<'a> {
    pub(super) fn new(kind: EntryKind<'a>) -> Self {
        Self { kind }
    }

    pub fn kind(&self) -> &EntryKind<'a> {
        &self.kind
    }

    /// Similar to Option::unwrap, but ties reference ownership back to &self,
    /// since Entries may own the structures in some impls.
    ///
    /// For this reason, it's preferable to call Entry::owned() rather than
    /// clone()ing this reference.
    pub fn structure(&self) -> &Sexp {
        match &self.kind {
            EntryKind::Borrowed(sexp) => sexp,
            EntryKind::Owned(sexp) => sexp,
            _ => panic!(),
        }
    }

    /// Presents an Option, but ties reference ownership back to &self,
    /// since Entries may own the structures in some impls.
    ///
    /// For this reason, it's preferable to call Entry::owned() rather than
    /// calling cloned() on this.
    pub fn as_option(&self) -> Option<&Sexp> {
        match &self.kind {
            EntryKind::Atomic => None,
            EntryKind::Borrowed(sexp) => Some(sexp),
            EntryKind::Owned(sexp) => Some(sexp),
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

    /// Return as a &mut Env if posssible. Ownership of the reference ties back
    /// to that of the Environment which created this Entry.
    // TODO(func) Modify interface to support Owned.
    pub fn env(self) -> Option<&'a Box<EnvObject>> {
        match self.kind() {
            EntryKind::Borrowed(Sexp::Primitive(Primitive::Env(env))) => Some(env),
            _ => None,
        }
    }
}

impl<'a> EntryMut<'a> {
    pub(super) fn new(node: LocalNode, kind: EntryMutKind<'a>, env: *mut EnvObject) -> Self {
        Self {
            node,
            kind,
            env: Some(env),
        }
    }

    pub(super) fn consume(mut self) -> (LocalNode, EntryMutKind<'a>, Option<*mut EnvObject>) {
        let kind = std::mem::replace(&mut self.kind, EntryMutKind::Atomic);
        // Skip update-on-drop.
        let env = std::mem::replace(&mut self.env, None);
        (self.node, kind, env)
    }

    pub fn kind(&self) -> &EntryMutKind<'a> {
        &self.kind
    }
    pub fn kind_mut(&mut self) -> &mut EntryMutKind<'a> {
        &mut self.kind
    }

    pub fn update(self) -> LocalNode {
        self.node
    }

    /// Similar to Option::unwrap, but ties reference ownership back to
    /// &mut self, since Entries may own the structures in some impls.
    pub fn structure(&mut self) -> &mut Sexp {
        match &mut self.kind {
            EntryMutKind::Borrowed(sexp) => sexp,
            EntryMutKind::Owned(sexp) => sexp,
            _ => panic!(),
        }
    }

    /// Presents an Option, but ties reference ownership back to &mut self,
    /// since Entries may own the structures in some impls.
    pub fn as_option(&mut self) -> Option<&mut Sexp> {
        match &mut self.kind {
            EntryMutKind::Atomic => None,
            EntryMutKind::Borrowed(sexp) => Some(sexp),
            EntryMutKind::Owned(sexp) => Some(sexp),
        }
    }

    /// Return as a &mut Env if posssible. Ownership of the reference ties back
    /// to that of the Environment which created this Entry.
    // TODO(func) Modify interface to support Owned.
    pub fn env(self) -> Option<&'a mut Box<EnvObject>> {
        // Skip update-on-drop.
        let (_, kind, _env) = self.consume();
        match kind {
            EntryMutKind::Borrowed(Sexp::Primitive(Primitive::Env(env))) => Some(env),
            _ => None,
        }
    }
}

impl<'a> Drop for EntryMut<'a> {
    fn drop(&mut self) {
        if let Some(env) = self.env {
            // Replace self with placeholder.
            let original = std::mem::replace(
                self,
                Self {
                    node: LocalNode::default(),
                    kind: EntryMutKind::Atomic,
                    env: None,
                },
            );
            // EntryMutKind already ensures self is the only one with env access.
            unsafe {
                (*env).entry_update(original);
            }
        }
    }
}
