use std::collections::btree_set::{BTreeSet, IntoIter, Iter};

use super::environment::EnvObject;
use super::local_node::{LocalNode, LocalTriple};


#[derive(Clone)]
pub struct TripleSet<'a> {
    elements: BTreeSet<LocalTriple>,
    env: &'a EnvObject,
}

impl<'a> TripleSet<'a> {
    pub(super) fn new(env: &'a EnvObject, elements: BTreeSet<LocalTriple>) -> Self {
        Self { elements, env }
    }

    pub fn match_triple(env: &'a EnvObject, s: LocalNode, p: LocalNode, o: LocalNode) -> Self {
        let mut elements = BTreeSet::new();
        if let Some(triple) = env.match_triple(s, p, o) {
            elements.insert(triple);
        }
        Self { elements, env }
    }

    pub fn len(&self) -> usize {
        self.elements.len()
    }

    pub fn triples(self) -> TripleIter {
        self.elements.into_iter()
    }

    pub fn objects(&self) -> ObjIter {
        ObjIter::new(self)
    }

    pub fn union(&self, other: &TripleSet) -> TripleSet<'a> {
        assert_eq!(
            self.env.all_nodes().iter().next(),
            other.env.all_nodes().iter().next()
        );
        Self {
            // TODO(perf) Can we clone + collect lazily?
            elements: self.elements.union(&other.elements).cloned().collect(),
            env: self.env,
        }
    }
}


pub type TripleIter = IntoIter<LocalTriple>;

pub struct ObjIter<'a> {
    iter: Iter<'a, LocalTriple>,
    env: &'a EnvObject,
}

impl<'a> ObjIter<'a> {
    fn new(set: &'a TripleSet) -> Self {
        Self {
            iter: set.elements.iter(),
            env: set.env,
        }
    }
}

impl<'a> Iterator for ObjIter<'a> {
    type Item = LocalNode;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(triple) = self.iter.next() {
            Some(self.env.triple_object(*triple))
        } else {
            None
        }
    }
}
