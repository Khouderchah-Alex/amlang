use log::warn;
use std::convert::{TryFrom, TryInto};

use crate::agent::lang_error::LangError;
use crate::agent::symbol_policies::policy_admin;
use crate::agent::Agent;
use crate::error::Error;
use crate::model::Reflective;
use crate::primitive::symbol::ToSymbol;
use crate::primitive::table::Table;
use crate::primitive::{EnvObject, Node, Number, Primitive, SymSexpTable, Symbol};
use crate::sexp::{Cons, ConsList, Sexp};


pub struct EnvHeader {
    file_version: usize,
    node_count: usize,
    triple_count: usize,
    unrecognized: SymSexpTable,
}

impl EnvHeader {
    pub fn from_env(env: &Box<EnvObject>) -> Self {
        let node_count = env.all_nodes().len();
        let triple_count = env.match_all().len();
        Self {
            file_version: 1,
            node_count,
            triple_count,
            unrecognized: SymSexpTable::default(),
        }
    }
}

impl Reflective for EnvHeader {
    fn reify(&self, agent: &Agent) -> Sexp {
        let mut list = ConsList::new();
        list.append("header".to_symbol_or_panic(policy_admin));
        list.append(Cons::new(
            "version".to_symbol_or_panic(policy_admin),
            Number::USize(self.file_version),
        ));
        list.append(Cons::new(
            "node-count".to_symbol_or_panic(policy_admin),
            Number::USize(self.node_count),
        ));
        list.append(Cons::new(
            "triple-count".to_symbol_or_panic(policy_admin),
            Number::USize(self.triple_count),
        ));
        list.release_with_tail(
            Cons::try_from(self.unrecognized.reify(agent))
                .unwrap()
                .consume()
                .1,
        )
    }

    fn reflect<F>(structure: Sexp, agent: &Agent, resolve: F) -> Result<Self, Error>
    where
        F: Fn(&Agent, &Primitive) -> Result<Node, Error>,
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
        // Leverage SymSexpTable reflection.
        let mut table = SymSexpTable::reflect(
            Cons::new(amlang_node!(sym_sexp_table, agent.context()), cdr).into(),
            agent,
            resolve,
        )?;

        let map = table.as_map_mut();
        let mut extract = |key| match map.remove(key) {
            Some(Sexp::Primitive(Primitive::Number(Number::I64(i)))) => i,
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
            unrecognized: table,
        })
    }

    fn valid_discriminator(_node: Node, _agent: &Agent) -> bool {
        todo!("Need to represent header in Env before this is useful");
    }
}
