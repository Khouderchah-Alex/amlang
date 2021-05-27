use crate::function::Ret;
use crate::sexp::HeapSexp;


/// Meaning of Structures.
pub trait Eval {
    fn eval(&mut self, structure: HeapSexp) -> Ret;
}
