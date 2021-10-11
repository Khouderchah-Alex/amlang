//! Module for representing table primitives.

use std::borrow::Borrow;
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use std::convert::TryFrom;

use super::{Node, Primitive, Symbol};
use crate::environment::LocalNode;
use crate::sexp::Sexp;


pub type SymbolTable = AmlangTable<Symbol, Node>;
pub type LocalNodeTable = AmlangTable<LocalNode, LocalNode>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AmlangTable<K, V> {
    map: BTreeMap<K, V>,
}

// Using a trait rather than normal impl because LocalNode is a bit of an
// exception (not convertible to a Sexp through Into). Mostly boils down to:
//   https://github.com/rust-lang/rust/issues/20400
pub trait Table<K: Ord, V: Copy> {
    fn as_map(&self) -> &BTreeMap<K, V>;
    fn as_map_mut(&mut self) -> &mut BTreeMap<K, V>;

    fn lookup<Q>(&self, k: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Ord + Eq + ?Sized,
    {
        if let Some(v) = self.as_map().get(k) {
            Some(*v)
        } else {
            None
        }
    }

    fn contains_key<Q>(&self, k: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Ord + Eq + ?Sized,
    {
        self.as_map().contains_key(k)
    }

    fn insert(&mut self, k: K, v: V) -> Option<V> {
        self.as_map_mut().insert(k, v)
    }

    fn entry(&mut self, k: K) -> Entry<K, V> {
        self.as_map_mut().entry(k)
    }
}


impl<K: Ord, V: Copy> Table<K, V> for AmlangTable<K, V> {
    fn as_map(&self) -> &BTreeMap<K, V> {
        &self.map
    }
    fn as_map_mut(&mut self) -> &mut BTreeMap<K, V> {
        &mut self.map
    }
}

impl<K: Ord, V> Default for AmlangTable<K, V> {
    fn default() -> Self {
        Self {
            map: Default::default(),
        }
    }
}


impl_try_from!(Sexp                 ->  SymbolTable,          SymbolTable;
               ref Sexp             ->  ref SymbolTable,      SymbolTable;
               Option<Sexp>         ->  SymbolTable,          SymbolTable;
               Option<ref Sexp>     ->  ref SymbolTable,      SymbolTable;
               Option<ref mut Sexp> ->  ref mut SymbolTable,  SymbolTable;
               Result<Sexp>         ->  SymbolTable,          SymbolTable;
               Result<ref Sexp>     ->  ref SymbolTable,      SymbolTable;
);

impl_try_from!(Sexp                 ->  LocalNodeTable,          LocalNodeTable;
               ref Sexp             ->  ref LocalNodeTable,      LocalNodeTable;
               Option<Sexp>         ->  LocalNodeTable,          LocalNodeTable;
               Option<ref Sexp>     ->  ref LocalNodeTable,      LocalNodeTable;
               Option<ref mut Sexp> ->  ref mut LocalNodeTable,  LocalNodeTable;
               Result<Sexp>         ->  LocalNodeTable,          LocalNodeTable;
               Result<ref Sexp>     ->  ref LocalNodeTable,      LocalNodeTable;
);
