//! Representation of primitives.

use std::fmt;

pub mod builtin;
pub mod number;
pub mod procedure;
pub mod symbol;
pub mod symbol_table;

pub use self::builtin::BuiltIn;
pub use self::number::Number;
pub use self::procedure::Procedure;
pub use self::symbol::{Symbol, ToSymbol};
pub use self::symbol_table::SymbolTable;
pub use crate::environment::NodeId;


#[derive(Clone, Debug, PartialEq)]
pub enum Primitive {
    Number(Number),
    Symbol(Symbol),
    BuiltIn(BuiltIn),
    Node(NodeId),

    SymbolTable(SymbolTable),
    Procedure(Procedure),
}


impl fmt::Display for Primitive {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Primitive::Number(num) => write!(f, "{}", num),
            Primitive::Symbol(s) => write!(f, "{}", s),
            Primitive::BuiltIn(b) => write!(f, "{}", b),
            Primitive::Node(node) => write!(f, "{}", node),

            Primitive::SymbolTable(table) => write!(f, "{:?}", table),
            Primitive::Procedure(proc) => write!(f, "{:?}", proc),
        }
    }
}
