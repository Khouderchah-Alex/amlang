//! Module for representing symbol tables.

use std::borrow::Borrow;
use std::collections::BTreeMap;
use std::convert::TryFrom;

use super::{Node, Primitive, Symbol, ToSymbol};
use crate::function::EvalErr;
use crate::sexp::Sexp;


#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SymbolTable {
    map: BTreeMap<Symbol, Node>,
}

impl SymbolTable {
    pub fn new(map: BTreeMap<Symbol, Node>) -> SymbolTable {
        SymbolTable { map }
    }

    pub fn lookup<Q>(&self, k: &Q) -> Result<Node, EvalErr>
    where
        Symbol: Borrow<Q>,
        Q: Ord + Eq + ToSymbol + ?Sized,
    {
        if let Some(node) = self.map.get(k) {
            Ok(*node)
        } else {
            Err(EvalErr::UnboundSymbol(k.to_symbol_or_panic()))
        }
    }

    pub fn contains_key<Q>(&self, k: &Q) -> bool
    where
        Symbol: Borrow<Q>,
        Q: Ord + Eq + ?Sized,
    {
        self.map.contains_key(k)
    }

    pub fn insert(&mut self, k: Symbol, v: Node) -> Option<Node> {
        self.map.insert(k, v)
    }

    pub fn as_map(&self) -> &BTreeMap<Symbol, Node> {
        &self.map
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
