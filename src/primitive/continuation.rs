use std::collections::btree_map::Entry;
use std::convert::TryFrom;
use std::fmt;

use super::table::Table;
use super::{Node, Primitive};
use crate::sexp::Sexp;


#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Continuation(Vec<ContinuationFrame>);

// TODO(func) Allow for more than dynamic Node lookups (e.g. static tables).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContinuationFrame {
    context: Node,
    map: Table<Node, Node>,
}

impl Continuation {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, frame: ContinuationFrame) {
        self.0.push(frame);
    }

    pub fn pop(&mut self) -> Option<ContinuationFrame> {
        self.0.pop()
    }

    pub fn depth(&self) -> usize {
        self.0.len()
    }

    pub fn lookup(&self, node: &Node) -> Option<Node> {
        for frame in &self.0 {
            if let Some(n) = frame.map.lookup(node) {
                return Some(n);
            }
        }
        None
    }

    /// Iterator from most-recent to least-recent frame.
    pub fn iter(&self) -> impl Iterator<Item = &ContinuationFrame> {
        self.0.iter().rev()
    }
}

impl ContinuationFrame {
    pub fn new(context: Node) -> Self {
        Self {
            context,
            map: Default::default(),
        }
    }

    pub fn insert(&mut self, from: Node, to: Node) -> bool {
        let entry = self.map.entry(from);
        if let Entry::Occupied(..) = entry {
            false
        } else {
            entry.or_insert(to);
            true
        }
    }

    pub fn context(&self) -> Node {
        self.context
    }
}


impl fmt::Display for Continuation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[Cont depth {}]", self.depth())
    }
}


impl_try_from!(Sexp                 ->  Continuation,          Continuation;
               ref Sexp             ->  ref Continuation,      Continuation;
               Option<Sexp>         ->  Continuation,          Continuation;
               Option<ref Sexp>     ->  ref Continuation,      Continuation;
               Option<ref mut Sexp> ->  ref mut Continuation,  Continuation;
               Result<Sexp>         ->  Continuation,          Continuation;
               Result<ref Sexp>     ->  ref Continuation,      Continuation;
);
