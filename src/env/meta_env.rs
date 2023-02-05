use std::collections::BTreeMap;

use super::entry::{Entry, EntryMut};
use super::local_node::{LocalNode, LocalTriple};
use super::EnvObject;
use super::{Environment, NodeSet, TripleSet};
use crate::sexp::Sexp;


#[derive(Clone, Debug)]
pub struct MetaEnv {
    base: Box<EnvObject>,
    envs: BTreeMap<LocalNode, Box<EnvObject>>,
}

impl MetaEnv {
    pub fn new(base: Box<EnvObject>) -> Self {
        Self {
            base: base,
            envs: Default::default(),
        }
    }

    pub fn insert_env(&mut self, node: LocalNode, env: Box<EnvObject>) {
        self.envs.insert(node, env);
    }

    pub fn env(&self, node: LocalNode) -> Option<&Box<EnvObject>> {
        self.envs.get(&node)
    }

    pub fn env_mut(&mut self, node: LocalNode) -> Option<&mut Box<EnvObject>> {
        self.envs.get_mut(&node)
    }

    pub fn base(&self) -> &Box<EnvObject> {
        &self.base
    }

    pub fn base_mut(&mut self) -> &mut Box<EnvObject> {
        &mut self.base
    }
}

impl Environment for MetaEnv {
    fn type_name(&self) -> &'static str {
        "MetaEnv"
    }

    fn all_nodes(&self) -> NodeSet {
        self.base().all_nodes()
    }
    fn insert_atom(&mut self) -> LocalNode {
        self.base_mut().insert_atom()
    }
    fn insert_structure(&mut self, structure: Sexp) -> LocalNode {
        self.base_mut().insert_structure(structure)
    }
    fn insert_triple(
        &mut self,
        subject: LocalNode,
        predicate: LocalNode,
        object: LocalNode,
    ) -> LocalTriple {
        self.base_mut().insert_triple(subject, predicate, object)
    }


    fn match_subject(&self, subject: LocalNode) -> TripleSet {
        self.base().match_subject(subject)
    }
    fn match_predicate(&self, predicate: LocalNode) -> TripleSet {
        self.base().match_predicate(predicate)
    }
    fn match_object(&self, object: LocalNode) -> TripleSet {
        self.base().match_object(object)
    }
    fn match_but_subject(&self, predicate: LocalNode, object: LocalNode) -> TripleSet {
        self.base().match_but_subject(predicate, object)
    }
    fn match_but_predicate(&self, subject: LocalNode, object: LocalNode) -> TripleSet {
        self.base().match_but_predicate(subject, object)
    }
    fn match_but_object(&self, subject: LocalNode, predicate: LocalNode) -> TripleSet {
        self.base().match_but_object(subject, predicate)
    }
    fn match_triple(
        &self,
        subject: LocalNode,
        predicate: LocalNode,
        object: LocalNode,
    ) -> TripleSet {
        self.base().match_triple(subject, predicate, object)
    }
    fn match_all(&self) -> TripleSet {
        self.base().match_all()
    }

    fn entry(&self, node: LocalNode) -> Entry {
        self.base().entry(node)
    }
    fn entry_mut(&mut self, node: LocalNode) -> EntryMut {
        self.base_mut().entry_mut(node)
    }
    fn entry_update(&mut self, entry: EntryMut) -> LocalNode {
        self.base_mut().entry_update(entry)
    }
    fn node_as_triple(&self, node: LocalNode) -> Option<LocalTriple> {
        self.base().node_as_triple(node)
    }

    fn triple_subject(&self, triple: LocalTriple) -> LocalNode {
        self.base().triple_subject(triple)
    }
    fn triple_predicate(&self, triple: LocalTriple) -> LocalNode {
        self.base().triple_predicate(triple)
    }
    fn triple_object(&self, triple: LocalTriple) -> LocalNode {
        self.base().triple_object(triple)
    }
    fn triple_index(&self, triple: LocalTriple) -> usize {
        self.base().triple_index(triple)
    }
    fn triple_from_index(&self, index: usize) -> LocalTriple {
        self.base().triple_from_index(index)
    }
}
