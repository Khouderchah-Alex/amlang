use bimap::BiMap;
use std::collections::BTreeSet;

use crate::env::local_node::{LocalNode, LocalTriple};
use crate::primitive::Symbol;
use crate::sexp::Sexp;


// TODO(perf, scale) Allow for Edges to be pushed on-disk?
#[derive(Debug, Default)]
pub struct Edges {
    pub as_subject: BTreeSet<LocalTriple>,
    pub as_predicate: BTreeSet<LocalTriple>,
    pub as_object: BTreeSet<LocalTriple>,
}

#[derive(Debug)]
pub enum Node {
    Atomic,
    Structured(Sexp),
}

#[derive(Debug)]
pub struct Triple {
    pub object: LocalNode,
    pub predicate: LocalNode,
    pub subject: LocalNode,
}

pub type Designator = BiMap<Symbol, LocalNode>;


// Not putting this functionality in local_node because this behavior is
// MemEnv-specific.
pub mod index_id_conv {
    use crate::env::local_node::{LocalId, LocalTriple};

    pub const fn is_triple_id(id: LocalId) -> bool {
        id.leading_ones() > 0
    }
    pub const fn index_to_triple_id(index: usize) -> LocalTriple {
        LocalTriple::new((index as LocalId) | !(LocalId::MAX >> 1))
    }

    // Note CANNOT be used for Nodes of Triples.
    pub const fn node_index_unchecked(id: LocalId) -> usize {
        id as usize
    }
    pub const fn triple_index_unchecked(id: LocalId) -> usize {
        (id & (LocalId::MAX >> 1)) as usize
    }
}
