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

// SymbolTable TryFrom impls.
impl TryFrom<Sexp> for SymbolTable {
    type Error = ();

    fn try_from(value: Sexp) -> Result<Self, Self::Error> {
        if let Sexp::Primitive(Primitive::SymbolTable(table)) = value {
            Ok(table)
        } else {
            Err(())
        }
    }
}

impl<'a> TryFrom<&'a Sexp> for &'a SymbolTable {
    type Error = ();

    fn try_from(value: &'a Sexp) -> Result<Self, Self::Error> {
        if let Sexp::Primitive(Primitive::SymbolTable(table)) = value {
            Ok(table)
        } else {
            Err(())
        }
    }
}

impl<'a> TryFrom<Option<&'a Sexp>> for &'a SymbolTable {
    type Error = ();

    fn try_from(value: Option<&'a Sexp>) -> Result<Self, Self::Error> {
        if let Some(Sexp::Primitive(Primitive::SymbolTable(table))) = value {
            Ok(table)
        } else {
            Err(())
        }
    }
}

impl<'a> TryFrom<Option<&'a mut Sexp>> for &'a mut SymbolTable {
    type Error = ();

    fn try_from(value: Option<&'a mut Sexp>) -> Result<Self, Self::Error> {
        if let Some(Sexp::Primitive(Primitive::SymbolTable(table))) = value {
            Ok(table)
        } else {
            Err(())
        }
    }
}

impl<E> TryFrom<Result<Sexp, E>> for SymbolTable {
    type Error = ();

    fn try_from(value: Result<Sexp, E>) -> Result<Self, Self::Error> {
        if let Ok(Sexp::Primitive(Primitive::SymbolTable(table))) = value {
            Ok(table)
        } else {
            Err(())
        }
    }
}

impl<'a, E> TryFrom<&'a Result<Sexp, E>> for &'a SymbolTable {
    type Error = ();

    fn try_from(value: &'a Result<Sexp, E>) -> Result<Self, Self::Error> {
        if let Ok(Sexp::Primitive(Primitive::SymbolTable(table))) = value {
            Ok(table)
        } else {
            Err(())
        }
    }
}


// LocalNodeTable TryFrom impls.
impl TryFrom<Sexp> for LocalNodeTable {
    type Error = ();

    fn try_from(value: Sexp) -> Result<Self, Self::Error> {
        if let Sexp::Primitive(Primitive::LocalNodeTable(table)) = value {
            Ok(table)
        } else {
            Err(())
        }
    }
}

impl<'a> TryFrom<&'a Sexp> for &'a LocalNodeTable {
    type Error = ();

    fn try_from(value: &'a Sexp) -> Result<Self, Self::Error> {
        if let Sexp::Primitive(Primitive::LocalNodeTable(table)) = value {
            Ok(table)
        } else {
            Err(())
        }
    }
}

impl<'a> TryFrom<Option<&'a Sexp>> for &'a LocalNodeTable {
    type Error = ();

    fn try_from(value: Option<&'a Sexp>) -> Result<Self, Self::Error> {
        if let Some(Sexp::Primitive(Primitive::LocalNodeTable(table))) = value {
            Ok(table)
        } else {
            Err(())
        }
    }
}

impl<'a> TryFrom<Option<&'a mut Sexp>> for &'a mut LocalNodeTable {
    type Error = ();

    fn try_from(value: Option<&'a mut Sexp>) -> Result<Self, Self::Error> {
        if let Some(Sexp::Primitive(Primitive::LocalNodeTable(table))) = value {
            Ok(table)
        } else {
            Err(())
        }
    }
}

impl<E> TryFrom<Result<Sexp, E>> for LocalNodeTable {
    type Error = ();

    fn try_from(value: Result<Sexp, E>) -> Result<Self, Self::Error> {
        if let Ok(Sexp::Primitive(Primitive::LocalNodeTable(table))) = value {
            Ok(table)
        } else {
            Err(())
        }
    }
}

impl<'a, E> TryFrom<&'a Result<Sexp, E>> for &'a LocalNodeTable {
    type Error = ();

    fn try_from(value: &'a Result<Sexp, E>) -> Result<Self, Self::Error> {
        if let Ok(Sexp::Primitive(Primitive::LocalNodeTable(table))) = value {
            Ok(table)
        } else {
            Err(())
        }
    }
}
