//! Module for representing table primitives.

use std::borrow::Borrow;
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use std::convert::TryFrom;

use super::{Node, Primitive, Symbol};
use crate::agent::AgentState;
use crate::environment::LocalNode;
use crate::error::Error;
use crate::model::Reflective;
use crate::sexp::{Cons, HeapSexp, Sexp, SexpIntoIter};


pub type SymbolTable = AmlangTable<Symbol, Node>;
pub type LocalNodeTable = AmlangTable<LocalNode, LocalNode>;

#[derive(Clone, Debug, PartialEq)]
pub struct AmlangTable<K, V> {
    map: BTreeMap<K, V>,
}

// Using a trait rather than normal impl because LocalNode is a bit of an
// exception (not convertible to a Sexp through Into). Mostly boils down to:
//   https://github.com/rust-lang/rust/issues/20400
pub trait Table<K: Ord, V: Clone> {
    fn as_map(&self) -> &BTreeMap<K, V>;
    fn as_map_mut(&mut self) -> &mut BTreeMap<K, V>;

    fn lookup<Q>(&self, k: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Ord + Eq + ?Sized,
    {
        self.as_map().get(k).cloned()
    }

    fn contains_key<Q>(&self, k: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Ord + Eq + ?Sized,
    {
        self.as_map().contains_key(k)
    }

    fn insert(&mut self, k: K, v: V) -> Option<V> {
        self.as_map_mut().insert(k, v)
    }

    fn entry(&mut self, k: K) -> Entry<K, V> {
        self.as_map_mut().entry(k)
    }
}


impl<K: Ord, V: Clone> Table<K, V> for AmlangTable<K, V> {
    fn as_map(&self) -> &BTreeMap<K, V> {
        &self.map
    }
    fn as_map_mut(&mut self) -> &mut BTreeMap<K, V> {
        &mut self.map
    }
}

impl<K: Ord, V> Default for AmlangTable<K, V> {
    fn default() -> Self {
        Self {
            map: Default::default(),
        }
    }
}


// TODO (flex) Would rather impl Reflective once. Maybe use macro.
impl Reflective for AmlangTable<Symbol, Node> {
    fn reify(&self, state: &mut AgentState) -> Sexp {
        let mut alist = None;
        for (k, v) in self.as_map() {
            alist = Some(
                Cons::new(
                    Some(Cons::new(Some(k.clone().into()), Some((*v).into())).into()),
                    alist,
                )
                .into(),
            );
        }
        let node = amlang_node!(state.context(), symbol_table);
        Cons::new(Some(node.into()), alist).into()
    }

    fn reflect<F>(
        structure: Sexp,
        state: &mut AgentState,
        mut process_primitive: F,
    ) -> Result<Self, Error>
    where
        Self: Sized,
        F: FnMut(&mut AgentState, &Primitive) -> Result<Node, Error>,
    {
        let (command, cdr) = break_sexp!(structure => (Primitive; remainder), state)?;
        let node = process_primitive(state, &command)?;
        if !Self::valid_discriminator(node, state) {
            return err!(
                state,
                InvalidArgument {
                    given: command.into(),
                    expected: "Symbol table node".into()
                }
            );
        }

        let mut table = Self::default();
        for (assoc, _proper) in SexpIntoIter::from(cdr) {
            let cons = match Cons::try_from(assoc) {
                Ok(cons) => cons,
                Err(err) => {
                    return err!(
                        state,
                        InvalidArgument {
                            given: *err,
                            expected: "Association Cons".into()
                        }
                    );
                }
            };
            match cons.consume() {
                (Some(k), Some(v)) => {
                    if let Ok(kk) = <&Symbol>::try_from(&*k) {
                        if let Ok(vp) = <&Primitive>::try_from(&*v) {
                            table.insert(kk.clone(), process_primitive(state, &vp)?);
                            continue;
                        }
                    }
                    return err!(
                        state,
                        InvalidArgument {
                            given: Cons::new(Some(k), Some(v)).into(),
                            expected: "(Symbol . Node) association".into()
                        }
                    );
                }
                (k, v) => {
                    return err!(
                        state,
                        InvalidArgument {
                            given: Cons::new(k, v).into(),
                            expected: "Association cons".into()
                        }
                    );
                }
            }
        }
        Ok(table)
    }

    fn valid_discriminator(node: Node, state: &AgentState) -> bool {
        let context = state.context();
        if node.env() != context.lang_env() {
            return false;
        }

        node.local() == context.symbol_table
    }
}

impl Reflective for AmlangTable<LocalNode, LocalNode> {
    fn reify(&self, state: &mut AgentState) -> Sexp {
        let mut alist = None;
        for (k, v) in self.as_map() {
            alist = Some(
                Cons::new(
                    Some(
                        Cons::new(
                            Some(k.globalize(state).into()),
                            Some(v.globalize(state).into()),
                        )
                        .into(),
                    ),
                    alist,
                )
                .into(),
            );
        }
        let node = amlang_node!(state.context(), local_node_table);
        Cons::new(Some(node.into()), alist).into()
    }

    fn reflect<F>(
        structure: Sexp,
        state: &mut AgentState,
        mut process_primitive: F,
    ) -> Result<Self, Error>
    where
        Self: Sized,
        F: FnMut(&mut AgentState, &Primitive) -> Result<Node, Error>,
    {
        let (command, cdr) = break_sexp!(structure => (Primitive; remainder), state)?;
        let node = process_primitive(state, &command)?;
        if !Self::valid_discriminator(node, state) {
            return err!(
                state,
                InvalidArgument {
                    given: command.into(),
                    expected: "Lnode table node".into()
                }
            );
        }

        let mut table = Self::default();
        for (assoc, _proper) in SexpIntoIter::from(cdr) {
            let cons = match Cons::try_from(assoc) {
                Ok(cons) => cons,
                Err(err) => {
                    return err!(
                        state,
                        InvalidArgument {
                            given: *err,
                            expected: "Association Cons".into()
                        }
                    );
                }
            };
            match cons.consume() {
                (Some(k), Some(v)) => match (*k, *v) {
                    (Sexp::Primitive(kp), Sexp::Primitive(vp)) => {
                        let key = process_primitive(state, &kp)?;
                        let val = process_primitive(state, &vp)?;
                        if let Ok(kk) = Node::try_from(key) {
                            if let Ok(vv) = Node::try_from(val) {
                                table.insert(kk.local(), vv.local());
                                continue;
                            }
                        }
                    }
                    (k, v) => {
                        return err!(
                            state,
                            InvalidArgument {
                                given: Cons::new(Some(k.into()), Some(v.into())).into(),
                                expected: "(Node . Node) association".into()
                            }
                        );
                    }
                },
                (k, v) => {
                    return err!(
                        state,
                        InvalidArgument {
                            given: Cons::new(k, v).into(),
                            expected: "Association cons".into()
                        }
                    );
                }
            }
        }
        Ok(table)
    }

    fn valid_discriminator(node: Node, state: &AgentState) -> bool {
        let context = state.context();
        if node.env() != context.lang_env() {
            return false;
        }

        node.local() == context.local_node_table
    }
}


impl_try_from!(Sexp                 ->  SymbolTable,          SymbolTable;
               HeapSexp             ->  SymbolTable,          SymbolTable;
               ref Sexp             ->  ref SymbolTable,      SymbolTable;
               Option<Sexp>         ->  SymbolTable,          SymbolTable;
               Option<ref Sexp>     ->  ref SymbolTable,      SymbolTable;
               Option<ref mut Sexp> ->  ref mut SymbolTable,  SymbolTable;
               Result<Sexp>         ->  SymbolTable,          SymbolTable;
               Result<ref Sexp>     ->  ref SymbolTable,      SymbolTable;
);

impl_try_from!(Sexp                 ->  LocalNodeTable,          LocalNodeTable;
               HeapSexp             ->  LocalNodeTable,          LocalNodeTable;
               ref Sexp             ->  ref LocalNodeTable,      LocalNodeTable;
               Option<Sexp>         ->  LocalNodeTable,          LocalNodeTable;
               Option<ref Sexp>     ->  ref LocalNodeTable,      LocalNodeTable;
               Option<ref mut Sexp> ->  ref mut LocalNodeTable,  LocalNodeTable;
               Result<Sexp>         ->  LocalNodeTable,          LocalNodeTable;
               Result<ref Sexp>     ->  ref LocalNodeTable,      LocalNodeTable;
);
