use log::warn;
use std::collections::BTreeMap;
use std::convert::{TryFrom, TryInto};

use crate::agent::lang_error::LangError;
use crate::agent::Agent;
use crate::error::Error;
use crate::model::Reflective;
use crate::primitive::symbol::ToSymbol;
use crate::primitive::symbol_policies::policy_admin;
use crate::primitive::{EnvObject, Node, Number, Primitive, Symbol};
use crate::sexp::{Cons, Sexp, SexpIntoIter};


// TODO(func) Leverage a Reflective AmlangTable<Symbol, Sexp>, both for the
// Reflective impl and for storage of unrecognized associations.
pub struct EnvHeader {
    file_version: usize,
    node_count: usize,
    triple_count: usize,
}

impl EnvHeader {
    pub fn from_env(env: &mut Box<EnvObject>) -> Self {
        let node_count = env.all_nodes().into_iter().count();
        let triple_count = env.match_all().into_iter().count();
        Self {
            file_version: 1,
            node_count,
            triple_count,
        }
    }
}

impl Reflective for EnvHeader {
    fn reify(&self, _agent: &mut Agent) -> Sexp {
        list!(
            "header".to_symbol_or_panic(policy_admin),
            Cons::new(
                Some("version".to_symbol_or_panic(policy_admin).into()),
                Some(Number::Integer(self.file_version.try_into().unwrap()).into())
            ),
            Cons::new(
                Some("node-count".to_symbol_or_panic(policy_admin).into()),
                Some(Number::Integer(self.node_count.try_into().unwrap()).into())
            ),
            Cons::new(
                Some("triple-count".to_symbol_or_panic(policy_admin).into()),
                Some(Number::Integer(self.triple_count.try_into().unwrap()).into())
            ),
        )
    }

    fn reflect<F>(structure: Sexp, agent: &mut Agent, mut _resolve: F) -> Result<Self, Error>
    where
        F: FnMut(&mut Agent, &Primitive) -> Result<Node, Error>,
    {
        let (command, cdr) = break_sexp!(structure => (Symbol; remainder), agent)?;
        if command.as_str() != "header" {
            return err!(
                agent,
                LangError::InvalidArgument {
                    given: command.into(),
                    expected: "\"header\"".into()
                }
            );
        }

        let mut map = BTreeMap::<Symbol, Sexp>::new();
        let iter = cdr.map_or(SexpIntoIter::default(), |e| e.into_iter());
        for (assoc, proper) in iter {
            assert!(proper);
            let (k, v) = Cons::try_from(assoc).unwrap().consume();
            let key = Symbol::try_from(k.unwrap()).unwrap();
            if let Some(_existing) = map.get(&key) {
                return err!(
                    agent,
                    LangError::InvalidArgument {
                        given: Cons::new(Some(key.into()), v.into()).into(),
                        expected: "Unique keys only".into()
                    }
                );
            }
            map.insert(key, *v.unwrap());
        }

        let mut extract = |key| match map.remove(key) {
            Some(Sexp::Primitive(Primitive::Number(Number::Integer(i)))) => i,
            _ => panic!(),
        };
        let file_version = extract("version").try_into().unwrap();
        let node_count = extract("node-count").try_into().unwrap();
        let triple_count = extract("triple-count").try_into().unwrap();

        for (k, v) in map {
            warn!("Unrecognized EnvHeader key-val pair: ({} . {})", k, v);
        }
        Ok(Self {
            file_version,
            node_count,
            triple_count,
        })
    }

    fn valid_discriminator(_node: Node, _agent: &Agent) -> bool {
        todo!("Need to represent header in Env before this is useful");
    }
}
