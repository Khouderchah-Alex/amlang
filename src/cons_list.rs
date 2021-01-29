//! Module for constructing lists as S-exps.

use crate::sexp::{Cons, Value};

#[derive(Debug)]
pub struct ConsList {
    head: Box<Value>,
    end: *mut Cons,
}

impl ConsList {
    pub fn new() -> ConsList {
        ConsList {
            head: Box::new(Value::Cons(Cons::default())),
            end: std::ptr::null_mut(),
        }
    }

    pub fn release(self) -> Box<Value> {
        self.head
    }

    pub fn append(&mut self, val: Box<Value>) {
        let mut tail = Box::new(Value::Cons(Cons::cons(Some(val), None)));
        let new_end;
        if let Value::Cons(c) = tail.as_mut() {
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
