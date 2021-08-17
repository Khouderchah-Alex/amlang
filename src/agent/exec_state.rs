use std::collections::btree_map::Entry;

use super::continuation::Continuation;
use crate::primitive::table::Table;
use crate::primitive::Node;


pub type ExecState = Continuation<ExecFrame>;

// TODO(func) Allow for more than dynamic Node lookups (e.g. static tables).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecFrame {
    context: Node,
    map: Table<Node, Node>,
}


impl ExecFrame {
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

    pub fn lookup(&self, key: Node) -> Option<Node> {
        self.map.lookup(&key)
    }

    pub fn context(&self) -> Node {
        self.context
    }
}
