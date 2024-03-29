//! Module for constructing lists as S-exps without building in reverse or
//! tolerating O(n) insertion => O(n^2) total construction.
//!
//! Not concurrency-safe; meant to be used serially.

use std::convert::TryFrom;

use crate::sexp::{Cons, HeapSexp, Sexp};

#[derive(Debug)]
pub struct ConsList {
    head: Box<Cons>,
    end: *mut Cons,
    len: usize,
}

impl Default for ConsList {
    fn default() -> Self {
        ConsList::new()
    }
}

impl ConsList {
    pub fn new() -> ConsList {
        ConsList {
            head: Box::new(Cons::default()),
            end: std::ptr::null_mut(),
            len: 0,
        }
    }

    pub fn release(self) -> Sexp {
        (*self.head).into()
    }

    pub fn release_with_tail(self, tail: Option<HeapSexp>) -> Sexp {
        match self.len {
            0 => {
                if let Some(hsexp) = tail {
                    *hsexp
                } else {
                    Sexp::default()
                }
            }
            _ => {
                unsafe {
                    (*self.end).set_cdr(tail);
                }
                (*self.head).into()
            }
        }
    }

    pub fn append<T: Into<HeapSexp>>(&mut self, val: T) {
        let l = if self.end.is_null() {
            self.head.set_car(Some(val.into()));
            &self.head
        } else {
            let tail = Cons::new(val.into(), None);
            unsafe {
                (*self.end).set_cdr(Some(tail.into()));
                <&Cons>::try_from((*self.end).cdr()).unwrap()
            }
        };
        self.end = l as *const Cons as *mut Cons;
        self.len += 1;
    }
}
