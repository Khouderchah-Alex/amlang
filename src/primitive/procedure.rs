use std::convert::TryFrom;

use super::{NodeId, Primitive};
use crate::sexp::Sexp;


#[derive(Clone, Debug, PartialEq)]
pub struct Procedure {
    surface: SProcedure,
    bindings: Bindings,
    body: BProcedure,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SProcedure {
    args: Vec<NodeId>,
    //ret: Sexp,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Bindings {
    structures: Vec<Sexp>,
    locations: Vec<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BProcedure {
    Application(NodeId),
    Sequence(Vec<Procedure>),
    Branch(Box<Branch>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Branch {
    cond: Procedure,
    a: Procedure,
    b: Procedure,
}


impl Procedure {
    pub fn new(surface: SProcedure, bindings: Bindings, body: BProcedure) -> Procedure {
        Procedure {
            surface,
            bindings,
            body,
        }
    }

    pub fn surface_args(&self) -> &Vec<NodeId> {
        &self.surface.args
    }

    pub fn body(&self) -> &BProcedure {
        &self.body
    }

    // TODO(func) Take cont obj.
    // TODO(perf) Return Cow.
    pub fn generate_args(&self, mut cont: Vec<Sexp>) -> Vec<Sexp> {
        match self.body {
            BProcedure::Application(_) => {
                let bmax = self.bindings.structures.len();
                let mut b: usize = 0;
                let mut ret = Vec::<Sexp>::with_capacity(bmax + cont.len());
                for i in 0..(bmax + cont.len()) {
                    if b < bmax && self.bindings.locations[b] == i {
                        ret.push(self.bindings.structures[b].clone());
                        b += 1;
                    } else {
                        ret.push(std::mem::replace(&mut cont[i - b], Sexp::default()));
                    }
                }
                ret
            }
            _ => panic!("Not yet supporting other procedure bodies"),
        }
    }
}

impl SProcedure {
    pub fn new() -> SProcedure {
        SProcedure::default()
    }

    pub fn push(&mut self, node: NodeId) {
        self.args.push(node);
    }

    pub fn is_empty(&self) -> bool {
        self.args.is_empty()
    }
}

impl Bindings {
    pub fn new() -> Bindings {
        Bindings::default()
    }

    pub fn insert(&mut self, i: usize, val: Sexp) {
        self.structures.push(val);
        self.locations.push(i);
    }
}


impl TryFrom<Sexp> for Procedure {
    type Error = ();

    fn try_from(value: Sexp) -> Result<Self, Self::Error> {
        if let Sexp::Primitive(Primitive::Procedure(procedure)) = value {
            Ok(procedure)
        } else {
            Err(())
        }
    }
}

impl<'a> TryFrom<&'a Sexp> for &'a Procedure {
    type Error = ();

    fn try_from(value: &'a Sexp) -> Result<Self, Self::Error> {
        if let Sexp::Primitive(Primitive::Procedure(procedure)) = value {
            Ok(procedure)
        } else {
            Err(())
        }
    }
}

impl<'a> TryFrom<Option<&'a Sexp>> for &'a Procedure {
    type Error = ();

    fn try_from(value: Option<&'a Sexp>) -> Result<Self, Self::Error> {
        if let Some(Sexp::Primitive(Primitive::Procedure(procedure))) = value {
            Ok(procedure)
        } else {
            Err(())
        }
    }
}

impl<E> TryFrom<Result<Sexp, E>> for Procedure {
    type Error = ();

    fn try_from(value: Result<Sexp, E>) -> Result<Self, Self::Error> {
        if let Ok(Sexp::Primitive(Primitive::Procedure(procedure))) = value {
            Ok(procedure)
        } else {
            Err(())
        }
    }
}

impl<'a, E> TryFrom<&'a Result<Sexp, E>> for &'a Procedure {
    type Error = ();

    fn try_from(value: &'a Result<Sexp, E>) -> Result<Self, Self::Error> {
        if let Ok(Sexp::Primitive(Primitive::Procedure(procedure))) = value {
            Ok(procedure)
        } else {
            Err(())
        }
    }
}
