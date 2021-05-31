use crate::function::Ret;
use crate::sexp::HeapSexp;


/// Meaning of Structures.
pub trait Eval {
    fn eval(&mut self, structure: HeapSexp) -> Ret;
}

pub trait Model {
    /// Model structure according to (possibly implicit) metamodel.
    fn generate_structure(&self) -> HeapSexp;
}
