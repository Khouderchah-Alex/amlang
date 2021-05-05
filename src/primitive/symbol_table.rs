//! Module for representing symbol tables.

use std::borrow::Borrow;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::hash::Hash;

use super::Primitive;
use super::Symbol;
use crate::environment::NodeId;
use crate::sexp::Sexp;


#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SymbolTable {
    map: HashMap<Symbol, NodeId>,
}

impl SymbolTable {
    pub fn new(map: HashMap<Symbol, NodeId>) -> SymbolTable {
        SymbolTable { map }
    }

    pub fn lookup<Q>(&self, k: &Q) -> Option<&NodeId>
    where
        Symbol: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.map.get(k)
    }

    pub fn contains_key<Q>(&self, k: &Q) -> bool
    where
        Symbol: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.map.contains_key(k)
    }

    pub fn insert(&mut self, k: Symbol, v: NodeId) -> Option<NodeId> {
        self.map.insert(k, v)
    }
}


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
