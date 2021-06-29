//! Representation of primitives.

use std::fmt;
use std::mem;

pub mod builtin;
pub mod node;
pub mod number;
pub mod procedure;
pub mod symbol;
pub mod symbol_table;

pub use self::builtin::BuiltIn;
pub use self::node::Node;
pub use self::number::Number;
pub use self::procedure::Procedure;
pub use self::symbol::{Symbol, ToSymbol};
pub use self::symbol_table::SymbolTable;
pub use crate::environment::environment::EnvObject;


#[derive(Clone, Debug)]
pub enum Primitive {
    Number(Number),
    Symbol(Symbol),
    BuiltIn(BuiltIn),
    Node(Node),

    SymbolTable(SymbolTable),
    Procedure(Procedure),
    // Presumably only present in meta env Nodes, but this comes down
    // to how base Agents are implemented.
    Env(Box<EnvObject>),
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
            Primitive::Env(_env) => write!(f, "[Env]"),
        }
    }
}

// Manually implementing this due to:
//   https://github.com/rust-lang/rust/issues/67369
impl PartialEq for Primitive {
    #[inline]
    fn eq(&self, other: &Primitive) -> bool {
        {
            let self_d = mem::discriminant(&*self);
            let other_d = mem::discriminant(&*other);
            if self_d == other_d {
                match (&*self, &*other) {
                    (&Primitive::Number(ref this), &Primitive::Number(ref that)) => {
                        (*this) == (*that)
                    }
                    (&Primitive::Symbol(ref this), &Primitive::Symbol(ref that)) => {
                        (*this) == (*that)
                    }
                    (&Primitive::BuiltIn(ref this), &Primitive::BuiltIn(ref that)) => {
                        (*this) == (*that)
                    }
                    (&Primitive::Node(ref this), &Primitive::Node(ref that)) => (*this) == (*that),
                    (&Primitive::SymbolTable(ref this), &Primitive::SymbolTable(ref that)) => {
                        (*this) == (*that)
                    }
                    (&Primitive::Procedure(ref this), &Primitive::Procedure(ref that)) => {
                        (*this) == (*that)
                    }
                    // Consider all envs to be different a priori.
                    (&Primitive::Env(_), &Primitive::Env(_)) => false,
                    _ => {
                        panic!();
                    }
                }
            } else {
                false
            }
        }
    }
}
