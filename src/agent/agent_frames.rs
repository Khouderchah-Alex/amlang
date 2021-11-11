//! Building blocks for Agent to create its state.

use dyn_clone::DynClone;
use std::collections::btree_map::Entry;
use std::fmt;

use super::Agent;
use crate::model::Interpreter;
use crate::primitive::table::{AmlangTable, Table};
use crate::primitive::Node;
use crate::sexp::Sexp;


/// State which Agent can use to create an Interpreter.
/// Can be stored in Continuation and facilitates reifying metacontinuations.
// TODO(func) Allow storage in Env.
pub trait InterpreterState: DynClone + fmt::Debug {
    fn borrow_agent<'a>(&'a mut self, agent: &'a mut Agent) -> Box<dyn Interpreter + 'a>;
}

// TODO(func) Allow for more than dynamic Node lookups (e.g. static tables).
#[derive(Clone, Debug, PartialEq)]
pub struct ExecFrame {
    context: Node,
    map: AmlangTable<Node, Sexp>,
}

#[derive(Clone, Debug)]
pub struct EnvFrame {
    pub(super) pos: Node,
}


impl ExecFrame {
    pub fn new(context: Node) -> Self {
        Self {
            context,
            map: Default::default(),
        }
    }

    pub fn insert(&mut self, from: Node, to: Sexp) -> bool {
        let entry = self.map.entry(from);
        if let Entry::Occupied(..) = entry {
            false
        } else {
            entry.or_insert(to);
            true
        }
    }

    pub fn lookup(&self, key: Node) -> Option<&Sexp> {
        self.map.as_map().get(&key)
    }

    pub fn context(&self) -> Node {
        self.context
    }
}


dyn_clone::clone_trait_object!(InterpreterState);
