//! Module for representing table primitives.

use std::borrow::Borrow;
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use std::convert::TryFrom;

use super::{Node, Primitive, Symbol};
use crate::agent::lang_error::LangError;
use crate::agent::Agent;
use crate::environment::LocalNode;
use crate::error::Error;
use crate::model::Reflective;
use crate::sexp::{Cons, HeapSexp, Sexp, SexpIntoIter};


pub type SymNodeTable = AmlangTable<Symbol, Node>;
pub type SymSexpTable = AmlangTable<Symbol, Sexp>;

#[derive(Clone, Debug, PartialEq)]
pub struct AmlangTable<K, V> {
    map: BTreeMap<K, V>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LocalNodeTable {
    map: BTreeMap<LocalNode, LocalNode>,
    env: LocalNode,
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

impl LocalNodeTable {
    pub fn in_env(env: LocalNode) -> Self {
        Self {
            map: Default::default(),
            env,
        }
    }
}

impl Table<LocalNode, LocalNode> for LocalNodeTable {
    fn as_map(&self) -> &BTreeMap<LocalNode, LocalNode> {
        &self.map
    }
    fn as_map_mut(&mut self) -> &mut BTreeMap<LocalNode, LocalNode> {
        &mut self.map
    }
}


fn reflect_map<K, V, FK, FV>(
    structure: Option<HeapSexp>,
    agent: &mut Agent,
    resolve_key: FK,
    resolve_val: FV,
) -> Result<BTreeMap<K, V>, Error>
where
    K: Ord + Clone,
    V: Clone,
    FK: Fn(&mut Agent, Sexp) -> Result<K, Error>,
    FV: Fn(&mut Agent, Sexp) -> Result<V, Error>,
{
    let mut table = BTreeMap::<K, V>::default();
    for (assoc, _proper) in SexpIntoIter::from(structure) {
        let cons = match Cons::try_from(assoc) {
            Ok(cons) => cons,
            Err(err) => {
                return err!(
                    agent,
                    LangError::InvalidArgument {
                        given: *err,
                        expected: "Association Cons".into()
                    }
                );
            }
        };
        match cons.consume() {
            (Some(k), Some(v)) => table.insert(resolve_key(agent, *k)?, resolve_val(agent, *v)?),
            (k, v) => {
                return err!(
                    agent,
                    LangError::InvalidArgument {
                        given: Cons::new(k, v).into(),
                        expected: "Association cons".into()
                    }
                );
            }
        };
    }
    Ok(table)
}

impl_amlang_table!(SymNodeTable, Symbol, Node, sym_node_table);
impl_amlang_table!(SymSexpTable, Symbol, Sexp, sym_sexp_table);

macro_rules! impl_amlang_table {
    ($alias:ident, $key:ident, $val:ident, $discriminator:ident) => {
        impl Reflective for AmlangTable<$key, $val> {
            fn reify(&self, agent: &mut Agent) -> Sexp {
                let mut alist = None;
                for (k, v) in self.as_map() {
                    alist = Some(
                        Cons::new(
                            Some(Cons::new(Some(k.clone().into()), Some(v.clone().into())).into()),
                            alist,
                        )
                            .into(),
                    );
                }
                let node = amlang_node!(agent.context(), $discriminator);
                Cons::new(Some(node.into()), alist).into()
            }

            fn reflect<F>(structure: Sexp, agent: &mut Agent, resolve: F) -> Result<Self, Error>
            where
                Self: Sized,
                F: Fn(&mut Agent, &Primitive) -> Result<Node, Error>,
            {
                let (command, cdr) = break_sexp!(structure => (Primitive; remainder), agent)?;
                let cmd = resolve(agent, &command)?;
                if !Self::valid_discriminator(cmd, agent) {
                    return err!(
                        agent,
                        LangError::InvalidArgument {
                            given: command.into(),
                            expected: format!("{} node", stringify!($discriminator)).into()
                        }
                    );
                }

                let map = reflect_map(
                    cdr,
                    agent,
                    |agent, sexp| match <impl_amlang_table!(@try_from $key)>::try_from(sexp) {
                        Ok(key) => impl_amlang_table!(@process resolve, agent, key, $key),
                        Err(sexp) => err!(
                            agent,
                            LangError::InvalidArgument {
                                given: sexp.into(),
                                expected: format!("Key as a {}", stringify!($key)).into()
                            }
                        ),
                    },
                    |agent, sexp| match <impl_amlang_table!(@try_from $val)>::try_from(sexp) {
                        Ok(val) => impl_amlang_table!(@process resolve, agent, val, $val),
                        Err(sexp) => err!(
                            agent,
                            LangError::InvalidArgument {
                                given: sexp.into(),
                                expected: format!("Val as a {}", stringify!($val)).into()
                            }
                        ),
                    },
                )?;
                Ok(Self { map })
            }

            fn valid_discriminator(node: Node, agent: &Agent) -> bool {
                let context = agent.context();
                if node.env() != context.lang_env() {
                    return false;
                }

                node.local() == context.$discriminator
            }
        }

        impl_try_from!($alias;
                       Primitive            ->  $alias,
                       Sexp                 ->  $alias,
                       HeapSexp             ->  $alias,
                       ref Sexp             ->  ref $alias,
                       Option<Sexp>         ->  $alias,
                       Option<ref Sexp>     ->  ref $alias,
                       Option<ref mut Sexp> ->  ref mut $alias,
                       Result<Sexp>         ->  $alias,
                       Result<ref Sexp>     ->  ref $alias,
        );
    };
    // For Node types, we want to try_from Primitive & call resolve.
    (@process $resolve:ident, $agent:ident, $val:ident, Node) => {
        $resolve($agent, &$val)
    };
    (@process $resolve:ident, $agent:ident, $val:ident, $($tail:tt)*) => {
        Ok($val)
    };
    (@try_from Node) => {
        Primitive
    };
    (@try_from $ty:ident) => {
        $ty
    };
}
use impl_amlang_table;


/// Special impl for LocalNodeTable.
impl Reflective for LocalNodeTable {
    fn reify(&self, agent: &mut Agent) -> Sexp {
        let mut alist = None;
        for (k, v) in self.as_map() {
            alist = Some(
                Cons::new(
                    Some(
                        Cons::new(
                            Some(Node::new(self.env, *k).into()),
                            Some(Node::new(self.env, *v).into()),
                        )
                        .into(),
                    ),
                    alist,
                )
                .into(),
            );
        }
        let cmd = amlang_node!(agent.context(), local_node_table).into();
        Cons::new(
            Some(cmd),
            Some(
                Cons::new(
                    Some(Node::new(LocalNode::default(), self.env).into()),
                    alist,
                )
                .into(),
            ),
        )
        .into()
    }

    fn reflect<F>(structure: Sexp, agent: &mut Agent, resolve: F) -> Result<Self, Error>
    where
        Self: Sized,
        F: Fn(&mut Agent, &Primitive) -> Result<Node, Error>,
    {
        let (command, env, cdr) =
            break_sexp!(structure => (Primitive, Primitive; remainder), agent)?;
        let cmd = resolve(agent, &command)?;
        if !Self::valid_discriminator(cmd, agent) {
            return err!(
                agent,
                LangError::InvalidArgument {
                    given: command.into(),
                    expected: "Lnode table node".into()
                }
            );
        }

        let env = resolve(agent, &env)?;
        let map = reflect_map(
            cdr,
            agent,
            |agent, sexp| match Primitive::try_from(sexp) {
                Ok(key) => Ok(resolve(agent, &key)?.local()),
                Err(sexp) => err!(
                    agent,
                    LangError::InvalidArgument {
                        given: sexp,
                        expected: "Key as a Node".into()
                    }
                ),
            },
            |agent, sexp| match Primitive::try_from(sexp) {
                Ok(val) => Ok(resolve(agent, &val)?.local()),
                Err(sexp) => err!(
                    agent,
                    LangError::InvalidArgument {
                        given: sexp,
                        expected: "Val as a Node".into()
                    }
                ),
            },
        )?;
        Ok(Self {
            map,
            env: env.local(),
        })
    }

    fn valid_discriminator(node: Node, agent: &Agent) -> bool {
        let context = agent.context();
        if node.env() != context.lang_env() {
            return false;
        }

        node.local() == context.local_node_table
    }
}


impl_try_from!(LocalNodeTable;
               Primitive            ->  LocalNodeTable,
               Sexp                 ->  LocalNodeTable,
               HeapSexp             ->  LocalNodeTable,
               ref Sexp             ->  ref LocalNodeTable,
               Option<Sexp>         ->  LocalNodeTable,
               Option<ref Sexp>     ->  ref LocalNodeTable,
               Option<ref mut Sexp> ->  ref mut LocalNodeTable,
               Result<Sexp>         ->  LocalNodeTable,
               Result<ref Sexp>     ->  ref LocalNodeTable,
);
