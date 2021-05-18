use crate::function::Ret;
use crate::primitive::Primitive;
use crate::sexp::HeapSexp;


/// Meaning of Structures.
pub trait Eval {
    fn eval(&mut self, structure: HeapSexp) -> Ret;
}

/// Meaning of Primitives; may be used in modeling Eval.
pub trait Designation {
    fn designate(&mut self, designator: Primitive) -> Ret;
}
