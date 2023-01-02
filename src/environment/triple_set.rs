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

    pub(super) fn from_option(env: &'a EnvObject, option: Option<LocalTriple>) -> Self {
        let mut elements = BTreeSet::new();
        if let Some(triple) = option {
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

    pub fn subjects(&self) -> SubjIter {
        SubjIter::new(self)
    }

    pub fn predicates(&self) -> PredIter {
        PredIter::new(self)
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

macro_rules! impl_iters {
    ($($name:ident -> $method:ident),+ $(,)? ) => {
        $(
            pub struct $name<'a> {
                iter: Iter<'a, LocalTriple>,
                env: &'a EnvObject,
            }

            impl<'a> $name<'a> {
                fn new(set: &'a TripleSet) -> Self {
                    Self {
                        iter: set.elements.iter(),
                        env: set.env,
                    }
                }
            }

            impl<'a> Iterator for $name<'a> {
                type Item = LocalNode;
                fn next(&mut self) -> Option<Self::Item> {
                    if let Some(triple) = self.iter.next() {
                        Some(self.env.$method(*triple))
                    } else {
                        None
                    }
                }
            }
        )+
    };
}

impl_iters!(SubjIter -> triple_subject,
            PredIter -> triple_predicate,
            ObjIter -> triple_object,
);
