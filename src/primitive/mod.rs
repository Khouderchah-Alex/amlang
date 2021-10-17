//! Representation of primitives.

use std::fmt;
use std::mem;

#[macro_use]
mod try_from_helper;

pub mod builtin;
pub mod error;
pub mod node;
pub mod number;
pub mod path;
pub mod procedure;
pub mod string;
pub mod symbol;
pub mod symbol_policies;
pub mod table;

/// Some modules tend to interact with most primitive types rather
/// than just a few. Provide this for those clients to ::* use.
pub mod prelude {
    pub use super::Primitive;

    pub use super::builtin::BuiltIn;
    pub use super::error::Error;
    pub use super::node::Node;
    pub use super::number::Number;
    pub use super::path::Path;
    pub use super::procedure::Procedure;
    pub use super::string::AmString;
    pub use super::symbol::{Symbol, ToSymbol};
    pub use super::table::{LocalNodeTable, SymbolTable};
    pub use crate::environment::environment::EnvObject;
}
/// All other clients can simply pick out what to use as normal.
pub use prelude::*;


#[derive(Clone, Debug)]
pub enum Primitive {
    Number(Number),
    Symbol(Symbol),
    AmString(AmString),
    BuiltIn(BuiltIn),
    Node(Node),
    Path(Path),

    SymbolTable(SymbolTable),
    LocalNodeTable(LocalNodeTable),
    Procedure(Procedure),

    // There is no plan to return errors as Sexp rather than Result<Sexp, Error>
    // *within* the core amlang implementation; Result is simply too useful WRT
    // compilation & error handling. However, providing Error as a Primitive
    // variant allows library clients to pass Errors back into their system.
    // Implementing Reflective will also enable clients to model errors in amlang.
    //
    // TODO(func) Impl Reflective.
    Error(Error),
    // Presumably only present in meta env Nodes, but this comes down
    // to how base Agents are implemented.
    //
    // TODO(flex) Use newtype.
    Env(Box<EnvObject>),
}


impl fmt::Display for Primitive {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Primitive::Number(num) => write!(f, "{}", num),
            Primitive::Symbol(s) => write!(f, "{}", s),
            Primitive::AmString(s) => write!(f, "{}", s),
            Primitive::BuiltIn(b) => write!(f, "{}", b),
            Primitive::Node(node) => write!(f, "{}", node),
            Primitive::Path(path) => write!(f, "{}", path),

            Primitive::SymbolTable(table) => write!(f, "{:?}", table),
            Primitive::LocalNodeTable(table) => write!(f, "{:?}", table),
            Primitive::Procedure(proc) => write!(f, "{:?}", proc),
            Primitive::Error(error) => write!(f, "{}", error),
            Primitive::Env(env) => write!(f, "{:?}", env),
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
                    (&Primitive::AmString(ref this), &Primitive::AmString(ref that)) => {
                        (*this) == (*that)
                    }
                    (&Primitive::BuiltIn(ref this), &Primitive::BuiltIn(ref that)) => {
                        (*this) == (*that)
                    }
                    (&Primitive::Node(ref this), &Primitive::Node(ref that)) => (*this) == (*that),
                    (&Primitive::Path(ref this), &Primitive::Path(ref that)) => (*this) == (*that),
                    (&Primitive::SymbolTable(ref this), &Primitive::SymbolTable(ref that)) => {
                        (*this) == (*that)
                    }
                    (
                        &Primitive::LocalNodeTable(ref this),
                        &Primitive::LocalNodeTable(ref that),
                    ) => (*this) == (*that),
                    (&Primitive::Procedure(ref this), &Primitive::Procedure(ref that)) => {
                        (*this) == (*that)
                    }
                    (&Primitive::Error(ref this), &Primitive::Error(ref that)) => {
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


// Impl From<T> over Primitive subtypes (except Env).
macro_rules! primitive_from {
    ($from:ident, $($tail:tt)*) => {
        impl From<$from> for Primitive {
            fn from(elem: $from) -> Self {
                Primitive::$from(elem)
            }
        }
        primitive_from!($($tail)*);
    };
    () => {};
}

primitive_from!(
    Number,
    Symbol,
    AmString,
    BuiltIn,
    Node,
    Path,
    SymbolTable,
    LocalNodeTable,
    Procedure,
    Error,
);
