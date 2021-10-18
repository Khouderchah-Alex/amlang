use std::convert::TryFrom;

use super::sexp::{HeapSexp, Sexp};


#[derive(Clone, Debug, Default, PartialEq)]
pub struct Cons {
    car: Option<HeapSexp>,
    cdr: Option<HeapSexp>,
}

impl Cons {
    pub fn new(car: Option<HeapSexp>, cdr: Option<HeapSexp>) -> Cons {
        Cons { car, cdr }
    }

    pub fn car(&self) -> Option<&Sexp> {
        match &self.car {
            Some(val) => Some(val.as_ref()),
            None => None,
        }
    }

    pub fn cdr(&self) -> Option<&Sexp> {
        match &self.cdr {
            Some(val) => Some(val.as_ref()),
            None => None,
        }
    }

    pub fn consume(self) -> (Option<HeapSexp>, Option<HeapSexp>) {
        (self.car, self.cdr)
    }

    pub fn set_car(&mut self, new: Option<HeapSexp>) {
        self.car = new;
    }
    pub fn set_cdr(&mut self, new: Option<HeapSexp>) {
        self.cdr = new;
    }
}


// TryFrom<Sexp-like> impls.
impl TryFrom<Sexp> for Cons {
    type Error = Sexp;

    fn try_from(value: Sexp) -> Result<Self, Self::Error> {
        if let Sexp::Cons(cons) = value {
            Ok(cons)
        } else {
            Err(value)
        }
    }
}

impl TryFrom<HeapSexp> for Cons {
    type Error = HeapSexp;

    fn try_from(value: HeapSexp) -> Result<Self, Self::Error> {
        if let Sexp::Cons(cons) = *value {
            Ok(cons)
        } else {
            Err(value)
        }
    }
}

impl<'a> TryFrom<&'a Sexp> for &'a Cons {
    type Error = &'a Sexp;

    fn try_from(value: &'a Sexp) -> Result<Self, Self::Error> {
        if let Sexp::Cons(cons) = value {
            Ok(cons)
        } else {
            Err(value)
        }
    }
}

impl<'a> TryFrom<Option<&'a Sexp>> for &'a Cons {
    type Error = Option<&'a Sexp>;

    fn try_from(value: Option<&'a Sexp>) -> Result<Self, Self::Error> {
        if let Some(Sexp::Cons(cons)) = value {
            Ok(cons)
        } else {
            Err(value)
        }
    }
}

impl TryFrom<Option<HeapSexp>> for Cons {
    type Error = Option<HeapSexp>;

    fn try_from(value: Option<HeapSexp>) -> Result<Self, Self::Error> {
        if let Some(heap) = value {
            if let Sexp::Cons(cons) = *heap {
                return Ok(cons);
            }
            return Err(Some(heap));
        }
        Err(None)
    }
}

impl<E> TryFrom<Result<Sexp, E>> for Cons {
    type Error = Result<Sexp, E>;

    fn try_from(value: Result<Sexp, E>) -> Result<Self, Self::Error> {
        if let Ok(Sexp::Cons(cons)) = value {
            Ok(cons)
        } else {
            Err(value)
        }
    }
}

impl<'a, E> TryFrom<&'a Result<Sexp, E>> for &'a Cons {
    type Error = &'a Result<Sexp, E>;

    fn try_from(value: &'a Result<Sexp, E>) -> Result<Self, Self::Error> {
        if let Ok(Sexp::Cons(cons)) = value {
            Ok(cons)
        } else {
            Err(value)
        }
    }
}
