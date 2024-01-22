//! Module for representing table primitives.

use std::borrow::Borrow;
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use std::convert::TryFrom;

use serde::{Deserialize, Serialize};

use super::{Node, Primitive, Symbol};
use crate::env::LocalNode;
use crate::sexp::{HeapSexp, Sexp};


pub type SymNodeTable = AmlangTable<Symbol, Node>;
pub type SymSexpTable = AmlangTable<Symbol, Sexp>;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AmlangTable<K: Ord, V> {
    map: BTreeMap<K, V>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LocalNodeTable {
    map: BTreeMap<LocalNode, LocalNode>,
    env: LocalNode,
}

// Using a trait rather than normal impl because LocalNode is a bit of an
// exception (not convertible to a Sexp through Into). Mostly boils down to:
//   https://github.com/rust-lang/rust/issues/20400
pub trait Table<K: Ord, V: Clone> {
    fn as_map(&self) -> &BTreeMap<K, V>;
    fn as_map_mut(&mut self) -> &mut BTreeMap<K, V>;

    fn lookup<Q>(&self, k: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Ord + Eq + ?Sized,
    {
        self.as_map().get(k).cloned()
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


impl<K: Ord, V: Clone> Table<K, V> for AmlangTable<K, V> {
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


impl LocalNodeTable {
    pub fn in_env(env: LocalNode) -> Self {
        Self {
            map: Default::default(),
            env,
        }
    }
}

impl Table<LocalNode, LocalNode> for LocalNodeTable {
    fn as_map(&self) -> &BTreeMap<LocalNode, LocalNode> {
        &self.map
    }
    fn as_map_mut(&mut self) -> &mut BTreeMap<LocalNode, LocalNode> {
        &mut self.map
    }
}

impl_amlang_table!(SymNodeTable, Symbol, Node, sym_node_table);
impl_amlang_table!(SymSexpTable, Symbol, Sexp, sym_sexp_table);

macro_rules! impl_amlang_table {
    ($alias:ident, $key:ident, $val:ident, $discriminator:ident) => {
        impl_try_from!($alias;
                       Primitive            ->  $alias,
                       Sexp                 ->  $alias,
                       HeapSexp             ->  $alias,
                       ref Sexp             ->  ref $alias,
                       Option<Sexp>         ->  $alias,
                       Option<ref Sexp>     ->  ref $alias,
                       Option<ref mut Sexp> ->  ref mut $alias,
                       Result<Sexp>         ->  $alias,
                       Result<ref Sexp>     ->  ref $alias,
        );
    };
    // For Node types, we want to try_from Primitive & call resolve.
    (@process $resolve:ident, $agent:ident, $val:ident, Node) => {
        $resolve($agent, &$val)
    };
    (@process $resolve:ident, $agent:ident, $val:ident, $($tail:tt)*) => {
        Ok($val)
    };
    (@try_from Node) => {
        Primitive
    };
    (@try_from $ty:ident) => {
        $ty
    };
}
use impl_amlang_table;


impl_try_from!(LocalNodeTable;
               Primitive            ->  LocalNodeTable,
               Sexp                 ->  LocalNodeTable,
               HeapSexp             ->  LocalNodeTable,
               ref Sexp             ->  ref LocalNodeTable,
               Option<Sexp>         ->  LocalNodeTable,
               Option<ref Sexp>     ->  ref LocalNodeTable,
               Option<ref mut Sexp> ->  ref mut LocalNodeTable,
               Result<Sexp>         ->  LocalNodeTable,
               Result<ref Sexp>     ->  ref LocalNodeTable,
);
