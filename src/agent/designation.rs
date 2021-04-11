use crate::function::Ret;
use crate::primitive::Primitive;

/// Particular mapping between primitive and Structure.
pub trait Designation {
    // TODO(perf) Can we return Sexp refs to avoid cloning?
    fn designate(&mut self, designator: &Primitive) -> Ret;
}
