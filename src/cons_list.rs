//! Module for constructing lists as S-exps.

use crate::sexp::{Cons, Sexp};

#[derive(Debug)]
pub struct ConsList {
    head: Box<Sexp>,
    end: *mut Cons,
}

impl ConsList {
    pub fn new() -> ConsList {
        ConsList {
            head: Box::new(Sexp::Cons(Cons::default())),
            end: std::ptr::null_mut(),
        }
    }

    pub fn release(self) -> Box<Sexp> {
        self.head
    }

    pub fn append(&mut self, val: Box<Sexp>) {
        let mut tail = Box::new(Sexp::Cons(Cons::new(Some(val), None)));
        let new_end;
        if let Sexp::Cons(c) = tail.as_mut() {
            new_end = c as *mut Cons;
        } else {
            panic!();
        }

        unsafe {
            if self.end.is_null() {
                self.head = tail;
            } else {
                (*self.end).set_cdr(Some(tail));
            }
            self.end = new_end;
        }
    }
}
