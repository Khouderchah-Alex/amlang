//! Environment abstraction.

use dyn_clone::DynClone;
use std::collections::BTreeSet;
use std::fmt;

use super::entry::{Entry, EntryMut};
use super::local_node::{LocalNode, LocalTriple};
use super::triple_set::TripleSet;
use crate::primitive::{Node, Symbol};
use crate::sexp::Sexp;


pub type EnvObject = dyn Environment;

pub type NodeSet = BTreeSet<LocalNode>;

/// Triple store of Nodes, which can be atoms, structures, or triples.
/// Always contains at least one node, which represents itself.
pub trait Environment: DynClone {
    fn type_name(&self) -> &'static str;
    fn all_nodes(&self) -> NodeSet;

    fn insert_node(&mut self, structure: Option<Sexp>) -> LocalNode;
    fn insert_triple(
        &mut self,
        subject: LocalNode,
        predicate: LocalNode,
        object: LocalNode,
    ) -> LocalTriple;

    fn insert_designation(&mut self, node: Node, designation: Symbol, context: LocalNode);
    fn match_designation(&self, designation: &Symbol, context: LocalNode) -> Option<Node>;
    fn find_designation(&self, node: Node, context: LocalNode) -> Option<&Symbol>;
    // TODO(perf) Abstract concrete Iter type for coroutine possibilities.
    fn designation_pairs(&self, context: LocalNode) -> Vec<(&Symbol, &Node)>;

    fn match_subject(&self, subject: LocalNode) -> TripleSet;
    fn match_predicate(&self, predicate: LocalNode) -> TripleSet;
    fn match_object(&self, object: LocalNode) -> TripleSet;

    fn match_but_subject(&self, predicate: LocalNode, object: LocalNode) -> TripleSet;
    fn match_but_predicate(&self, subject: LocalNode, object: LocalNode) -> TripleSet;
    fn match_but_object(&self, subject: LocalNode, predicate: LocalNode) -> TripleSet;

    fn match_triple(
        &self,
        subject: LocalNode,
        predicate: LocalNode,
        object: LocalNode,
    ) -> TripleSet;
    fn match_all(&self) -> TripleSet;

    fn match_any(&self, node: LocalNode) -> TripleSet {
        let a = self.match_subject(node);
        let b = a.union(&self.match_predicate(node));
        b.union(&self.match_object(node))
    }

    fn entry(&self, node: LocalNode) -> Entry;
    fn entry_mut(&mut self, node: LocalNode) -> EntryMut;
    /// Can't be used directly. Use EntryMut::update or implicit drop.
    fn entry_update(&mut self, entry: EntryMut) -> LocalNode;
    fn node_as_triple(&self, node: LocalNode) -> Option<LocalTriple>;

    fn triple_subject(&self, triple: LocalTriple) -> LocalNode;
    fn triple_predicate(&self, triple: LocalTriple) -> LocalNode;
    fn triple_object(&self, triple: LocalTriple) -> LocalNode;
    fn triple_index(&self, triple: LocalTriple) -> usize;
    fn triple_from_index(&self, index: usize) -> LocalTriple;
}


dyn_clone::clone_trait_object!(Environment);


impl fmt::Debug for Box<EnvObject> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{} @ {:p}]", self.type_name(), self)
    }
}
