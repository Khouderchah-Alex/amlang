use std::cell::UnsafeCell;
use std::ops::Index;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Vector which will not reallocate or mutate existing elements; either new
/// elements can be pushed to the end of the buffer, or a new AppendVec will be
/// created with larger capacity. Thus the vector can be accessed and appended
/// at the same time. Clients must manage lifetime of different AppendVec
/// instances and must ensure that concurrent pushes are synchronized.
pub struct AppendVec<T: Clone + Default> {
    vec: Vec<UnsafeCell<T>>,
    len: AtomicUsize,
}

pub struct AppendVecIter<'a, T: Clone + Default> {
    iter: std::slice::Iter<'a, UnsafeCell<T>>,
    len: usize,
}


impl<'a, T: Clone + Default + 'a> AppendVec<T> {
    // Amount to multiplicatively increase capacity size when space runs out.
    const CAPACITY_MULTIPLIER: usize = 2;

    pub fn new(capacity: usize) -> Self {
        let mut vec = Vec::with_capacity(capacity);
        vec.resize_with(capacity, Default::default);
        AppendVec {
            vec,
            len: AtomicUsize::new(0),
        }
    }

    pub fn iter(&self) -> AppendVecIter<T> {
        let len = self.len();
        AppendVecIter {
            iter: self.vec[0..len].iter(),
            len,
        }
    }

    pub fn len(&self) -> usize {
        self.len.load(Ordering::SeqCst)
    }

    // Either pushes this element into spare capacity and returns None, or
    // creates a larger AppendVec, appends the existing data, pushes this
    // element, and returns Some(new_vec).
    //
    // SAFETY: Clients must ensure that this is not called concurrently. While
    // we enforce there is no immutable access to elements in the spare
    // capacity, calling this concurrently could allow multiple concurrent
    // mutable accesses to the same element in spare capacity.
    pub unsafe fn push(&self, value: T) -> Option<Self> {
        if self.vec.capacity() > self.len() {
            self.push_unchecked(value);
            return None;
        }

        let new = Self::new(
            self.vec
                .capacity()
                .saturating_mul(Self::CAPACITY_MULTIPLIER),
        );
        new.append(self.iter());
        new.push_unchecked(value);
        Some(new)
    }

    // SAFETY: Clients must ensure that this is not called concurrently. While
    // we enforce there is no immutable access to elements in the spare
    // capacity, calling this concurrently could allow multiple concurrent
    // mutable accesses to the same element in spare capacity.
    pub unsafe fn append<I>(&self, iter: I) -> Option<Self>
    where
        I: ExactSizeIterator<Item = &'a T>,
    {
        if (self.vec.capacity() - self.len()) > iter.len() {
            for elem in iter {
                // TODO(perf) Don't modify len until all elements are pushed.
                self.push_unchecked(elem.clone());
            }
            return None;
        }

        let new = Self::new(
            self.vec
                .capacity()
                .saturating_mul(Self::CAPACITY_MULTIPLIER),
        );
        new.append(self.iter());
        new.append(iter);
        Some(new)
    }

    // Like push, but assumes we already have the capacity for this element.
    unsafe fn push_unchecked(&self, value: T) {
        let len = self.len();
        (*self.vec[len].get()) = value;
        self.len.fetch_add(1, Ordering::SeqCst);
    }
}


impl<T: Clone + Default> Index<usize> for AppendVec<T> {
    type Output = T;

    fn index(&self, i: usize) -> &Self::Output {
        assert!(i < self.len());
        unsafe { &*self.vec[i].get() }
    }
}

impl<'a, T: Clone + Default> Iterator for AppendVecIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(cell) = self.iter.next() {
            unsafe { return Some(&*cell.get()) }
        }

        None
    }
}

impl<'a, T: Clone + Default> ExactSizeIterator for AppendVecIter<'a, T> {
    fn len(&self) -> usize {
        self.len
    }
}


#[cfg(test)]
#[path = "./append_vec_test.rs"]
mod append_vec_test;
