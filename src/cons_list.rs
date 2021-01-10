//! Module for representing lists as S-exps.

use super::sexp::{Cons, Value};

#[derive(Debug)]
pub struct ConsList {
    head: Box<Cons>,
    end: *mut Cons,
}

impl ConsList {
    pub fn new() -> ConsList {
        ConsList {
            head: Box::default(),
            end: std::ptr::null_mut(),
        }
    }

    pub fn release(self) -> Box<Cons> {
        self.head
    }

    // TODO: Revisit when this isn't one of your first Rust functions and see if
    // this can be made safely.
    pub unsafe fn append(&mut self, val: Value) {
        if self.end.is_null() {
            let tail = Box::new(Cons {
                car: Some(Box::new(val)),
                cdr: None,
            });
            self.head = tail;
            self.end = self.head.as_mut() as *mut Cons;
        } else {
            let mut tail = Box::new(Value::Cons(Cons {
                car: Some(Box::new(val)),
                cdr: None,
            }));
            let old_end = self.end;
            if let Value::Cons(c) = tail.as_mut() {
                self.end = c as *mut Cons;
            }
            (*old_end).cdr = Some(tail);
        }
    }
}
