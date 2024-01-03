//! Representation of primitives.

use std::fmt;
use std::mem;

use serde::{Deserialize, Serialize};

use crate::sexp::{HeapSexp, Sexp};

#[macro_use]
mod try_from_helper;

pub mod builtin;
pub mod node;
pub mod number;
pub mod path;
pub mod procedure;
pub mod string;
pub mod symbol;
pub mod symbol_policies;
pub mod table;
pub mod vector;

/// Some modules tend to interact with most primitive types rather
/// than just a few. Provide this for those clients to ::* use.
pub mod prelude {
    pub use super::Primitive;

    pub use super::builtin::BuiltIn;
    pub use super::node::Node;
    pub use super::number::Number;
    pub use super::path::LangPath;
    pub use super::procedure::Procedure;
    pub use super::string::{LangString, ToLangString};
    pub use super::symbol::{Symbol, ToSymbol};
    pub use super::symbol_policies::{
        policy_admin, policy_base, policy_uuid, AdminSymbolInfo, SymbolPolicy,
    };
    pub use super::table::{LocalNodeTable, SymNodeTable, SymSexpTable, Table};
    pub use super::vector::Vector;
}
/// All other clients can simply pick out what to use as normal.
pub use prelude::*;


#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Primitive {
    Number(Number),
    Symbol(Symbol),
    LangString(LangString),
    BuiltIn(BuiltIn),
    Node(Node),
    LangPath(LangPath),

    SymNodeTable(SymNodeTable),
    SymSexpTable(SymSexpTable),
    LocalNodeTable(LocalNodeTable),
    Vector(Vector),
    Procedure(Procedure),
}


impl fmt::Display for Primitive {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Primitive::Number(num) => write!(f, "{}", num),
            Primitive::Symbol(s) => write!(f, "{}", s),
            Primitive::LangString(s) => write!(f, "{}", s),
            Primitive::BuiltIn(b) => write!(f, "{}", b),
            Primitive::Node(node) => write!(f, "{}", node),
            Primitive::LangPath(path) => write!(f, "{}", path),

            Primitive::SymNodeTable(table) => write!(f, "{:?}", table),
            Primitive::SymSexpTable(table) => write!(f, "{:?}", table),
            Primitive::LocalNodeTable(table) => write!(f, "{:?}", table),
            Primitive::Procedure(proc) => write!(f, "{:?}", proc),
            Primitive::Vector(vector) => write!(f, "{:?}", vector),
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
                    (&Primitive::LangString(ref this), &Primitive::LangString(ref that)) => {
                        (*this) == (*that)
                    }
                    (&Primitive::BuiltIn(ref this), &Primitive::BuiltIn(ref that)) => {
                        (*this) == (*that)
                    }
                    (&Primitive::Node(ref this), &Primitive::Node(ref that)) => (*this) == (*that),
                    (&Primitive::LangPath(ref this), &Primitive::LangPath(ref that)) => {
                        (*this) == (*that)
                    }
                    (&Primitive::SymNodeTable(ref this), &Primitive::SymNodeTable(ref that)) => {
                        (*this) == (*that)
                    }
                    (&Primitive::SymSexpTable(ref this), &Primitive::SymSexpTable(ref that)) => {
                        (*this) == (*that)
                    }
                    (
                        &Primitive::LocalNodeTable(ref this),
                        &Primitive::LocalNodeTable(ref that),
                    ) => (*this) == (*that),
                    (&Primitive::Procedure(ref this), &Primitive::Procedure(ref that)) => {
                        (*this) == (*that)
                    }
                    (&Primitive::Vector(ref this), &Primitive::Vector(ref that)) => {
                        (*this) == (*that)
                    }
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


// Impl From<T> over Primitive subtypes.
macro_rules! primitive_from {
    ($from:ident, $($tail:tt)*) => {
        impl From<$from> for Primitive {
            fn from(elem: $from) -> Self {
                Primitive::$from(elem)
            }
        }
        impl From<$from> for Sexp {
            fn from(elem: $from) -> Self {
                Sexp::Primitive(Primitive::$from(elem))
            }
        }
        impl From<$from> for HeapSexp {
            fn from(elem: $from) -> Self {
                Self::new(Sexp::Primitive(Primitive::$from(elem)))
            }
        }
        impl From<$from> for Option<HeapSexp> {
            fn from(elem: $from) -> Self {
                Some(HeapSexp::new(Sexp::Primitive(Primitive::$from(elem))))
            }
        }
        primitive_from!($($tail)*);
    };
    () => {};
}

primitive_from!(
    Number,
    Symbol,
    LangString,
    BuiltIn,
    Node,
    LangPath,
    SymNodeTable,
    SymSexpTable,
    LocalNodeTable,
    Procedure,
    Vector,
);
