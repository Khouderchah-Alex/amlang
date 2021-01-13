//! Module for representing environments.

use std::borrow::Borrow;
use std::collections::HashMap;
use std::hash::Hash;

pub struct Environment<T> {
    pub map: HashMap<String, T>,
}

impl<T> Environment<T> {
    pub fn new(map: HashMap<String, T>) -> Environment<T> {
        Environment { map }
    }

    // Setting this up now to account for future complexities.
    pub fn lookup<K>(&self, k: &K) -> Option<&T>
    where
        String: Borrow<K>,
        K: Hash + Eq + ?Sized,
    {
        self.map.get(k)
    }
}
