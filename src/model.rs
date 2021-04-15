use crate::function::Ret;
use crate::primitive::Primitive;
use crate::sexp::Sexp;


/// Meaning of Structures.
pub trait Eval {
    fn eval(&mut self, structure: &Sexp) -> Ret;
}

/// Meaning of Primitives; may be used in modeling Eval.
pub trait Designation {
    // TODO(perf) Can we return Sexp refs to avoid cloning?
    fn designate(&mut self, designator: &Primitive) -> Ret;
}
