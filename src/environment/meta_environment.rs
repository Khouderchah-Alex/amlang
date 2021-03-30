//! Really ugly impl just to get started.

use std::fmt;

use super::environment::{BaseEnvObject, EnvObject};
use super::mem_environment::MemEnvironment;
use crate::sexp::Sexp;


pub type MetaEnvObject = BaseEnvObject<MetaEnvStructure>;

pub enum MetaEnvStructure {
    Sexp(Sexp),
    Env(Box<EnvObject>),
}

pub type MetaEnvironment = MemEnvironment<MetaEnvStructure>;


impl fmt::Debug for MetaEnvStructure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MetaEnvStructure::Sexp(sexp) => write!(f, "{}", sexp),
            MetaEnvStructure::Env(env) => write!(f, "[Env @ {:p}]", env),
        }
    }
}


#[cfg(test)]
#[path = "./meta_environment_test.rs"]
mod meta_environment_test;
