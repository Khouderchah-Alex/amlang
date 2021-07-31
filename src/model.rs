use crate::agent::env_state::EnvState;
use crate::lang_err::LangErr;
use crate::sexp::{HeapSexp, Sexp};


pub type Ret = Result<Sexp, LangErr>;

/// Meaning of Structures.
pub trait Eval {
    fn eval(&mut self, structure: HeapSexp) -> Ret;
}

pub trait Model {
    /// Model structure according to (possibly implicit) metamodel.
    fn generate_structure(&self, env_state: &mut EnvState) -> HeapSexp;
}
