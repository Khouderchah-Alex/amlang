use std::collections::btree_map::Entry;
use std::convert::TryFrom;
use std::fmt;

use super::table::Table;
use super::{Node, Primitive};
use crate::sexp::Sexp;


// TODO(func) Allow for more than dynamic Node lookups (e.g. static tables).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Continuation {
    next: Option<Box<Continuation>>,

    context: Node,
    map: Table<Node, Node>,
}

impl Continuation {
    pub fn new_front(next: Option<Box<Continuation>>, context: Node) -> Box<Self> {
        Box::new(Self {
            next,

            context,
            map: Default::default(),
        })
    }

    pub fn set_next(&mut self, next: Option<Box<Continuation>>) {
        self.next = next;
    }

    pub fn pop_front(self) -> Option<Box<Continuation>> {
        self.next
    }

    pub fn depth(&self) -> usize {
        let mut count: usize = 0;
        let mut p = &self.next;
        while let Some(pp) = p {
            count += 1;
            p = &pp.next;
        }
        count
    }

    pub fn lookup(&self, node: &Node) -> Option<Node> {
        if let Some(n) = self.map.lookup(node) {
            return Some(n);
        }

        let mut p = &self.next;
        while let Some(pp) = p {
            if let Some(n) = pp.map.lookup(node) {
                return Some(n);
            }
            p = &pp.next;
        }
        None
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
}


impl fmt::Display for Continuation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[Cont @ {}, depth {}]", self.context, self.depth())
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
