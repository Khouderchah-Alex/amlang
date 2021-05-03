//! Representation of primitives.

use std::fmt;

use crate::environment::NodeId;
use crate::function::BuiltIn;
use crate::number::Number;
use crate::symbol::Symbol;
use crate::symbol_table::SymbolTable;


#[derive(Clone, Debug, PartialEq)]
pub enum Primitive {
    Number(Number),
    Symbol(Symbol),
    BuiltIn(&'static BuiltIn),
    Node(NodeId),

    SymbolTable(SymbolTable),
}


impl fmt::Display for Primitive {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Primitive::Number(num) => write!(f, "{}", num),
            Primitive::Symbol(s) => write!(f, "{}", s),
            Primitive::BuiltIn(b) => write!(f, "{}", b),
            Primitive::Node(node) => write!(f, "{}", node),

            Primitive::SymbolTable(table) => write!(f, "{:?}", table),
        }
    }
}
