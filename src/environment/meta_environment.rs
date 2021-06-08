//! Really ugly impl just to get started.

use std::fmt;

use super::environment::EnvObject;
use super::mem_environment::MemEnvironment;
use crate::environment::environment::Environment;
use crate::primitive::NodeId;
use crate::sexp::Sexp;


pub enum MetaEnvStructure {
    Sexp(Sexp),
    Env(Box<EnvObject>),
}

pub type MetaEnvironment = MemEnvironment<MetaEnvStructure>;


impl MetaEnvironment {
    pub fn access_env(&mut self, node: NodeId) -> &mut EnvObject {
        match self.node_structure(node).unwrap() {
            MetaEnvStructure::Env(env) => env.as_mut(),
            _ => panic!(),
        }
    }
}


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
