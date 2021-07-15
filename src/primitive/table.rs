//! Module for representing symbol tables.

use std::borrow::Borrow;
use std::collections::BTreeMap;
use std::convert::TryFrom;

use super::{Node, Primitive, Symbol};
use crate::environment::LocalNode;
use crate::sexp::Sexp;


pub type SymbolTable = Table<Symbol, Node>;
pub type LocalNodeTable = Table<LocalNode, LocalNode>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Table<K, V> {
    map: BTreeMap<K, V>,
}

impl<K: Ord, V: Copy> Table<K, V> {
    pub fn new(map: BTreeMap<K, V>) -> Self {
        Self { map }
    }

    pub fn lookup<Q>(&self, k: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Ord + Eq + ?Sized,
    {
        if let Some(v) = self.map.get(k) {
            Some(*v)
        } else {
            None
        }
    }

    pub fn contains_key<Q>(&self, k: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Ord + Eq + ?Sized,
    {
        self.map.contains_key(k)
    }

    pub fn insert(&mut self, k: K, v: V) -> Option<V> {
        self.map.insert(k, v)
    }

    pub fn as_map(&self) -> &BTreeMap<K, V> {
        &self.map
    }
}

impl<K: Ord, V> Default for Table<K, V> {
    fn default() -> Self {
        Self {
            map: Default::default(),
        }
    }
}


impl_try_from!(Sexp, SymbolTable, SymbolTable;
               ref Sexp, ref SymbolTable, SymbolTable;
               Option<Sexp>, SymbolTable, SymbolTable;
               Option<ref Sexp>, ref SymbolTable, SymbolTable;
               Option<ref mut Sexp>, ref mut SymbolTable, SymbolTable;
               Result<Sexp>, SymbolTable, SymbolTable;
               Result<ref Sexp>, ref SymbolTable, SymbolTable;);

impl_try_from!(Sexp, LocalNodeTable, LocalNodeTable;
               ref Sexp, ref LocalNodeTable, LocalNodeTable;
               Option<Sexp>, LocalNodeTable, LocalNodeTable;
               Option<ref Sexp>, ref LocalNodeTable, LocalNodeTable;
               Option<ref mut Sexp>, ref mut LocalNodeTable, LocalNodeTable;
               Result<Sexp>, LocalNodeTable, LocalNodeTable;
               Result<ref Sexp>, ref LocalNodeTable, LocalNodeTable;);
