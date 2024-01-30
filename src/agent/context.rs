use derive_getters::Getters;
use log::info;
use serde::{Deserialize, Serialize};

use std::collections::BTreeSet;

use crate::agent::Agent;
use crate::env::LocalNode;
use crate::error::Error;
use crate::introspect::Introspection;
use crate::primitive::{policy_base, Node, ToSymbol};
use crate::sexp::{Cons, Sexp};


pub trait Context<'de>: Deserialize<'de> + Sized {
    fn load(node: Node, agent: &'de mut Agent) -> Result<Self, Error> {
        Self::_load(node, false, agent)
    }
    fn load_strict(node: Node, agent: &'de mut Agent) -> Result<Self, Error> {
        Self::_load(node, true, agent)
    }

    fn _load(node: Node, strict: bool, agent: &'de mut Agent) -> Result<Self, Error> {
        let introspection = Introspection::of::<Self>();
        let mut reified = Sexp::default();
        // Need to own str so we can release agent ownership.
        let mut provided = BTreeSet::<String>::new();
        provided.insert("node".to_string());
        for (name, lnode) in agent
            .access_env(node.env())
            .unwrap()
            .designation_pairs(node.local())
        {
            provided.insert(name.as_str().to_string());
            reified.push_front(Cons::new(name.clone(), Node::new(node.env(), *lnode)));
        }

        // During development/self-modification, create missing context nodes as needed.
        if !strict {
            let all_fields: BTreeSet<String> = introspection
                .fields()
                .iter()
                .map(|s| s.to_string())
                .collect();
            let remaining = all_fields.difference(&provided);
            for name in remaining {
                info!("{}: Bootstrapping field {}", introspection.name(), name);
                let sym = name.to_symbol_or_panic(policy_base);
                let val = agent.define_to(node.env(), None)?;
                agent
                    .access_env_mut(node.env())
                    .unwrap()
                    .insert_designation(val.local(), sym.clone(), node.local());

                reified.push_front(Cons::new(sym, node));
            }
        }

        reified.push_front(Cons::new("node".to_symbol_or_panic(policy_base), node));
        reified.push_front(introspection.name().to_symbol_or_panic(policy_base));

        agent.reflect::<Self>(reified)
    }
}

/// Creates a Node from a Context field.
#[macro_export]
macro_rules! context_node {
    ($local:ident, $context:expr) => {{
        let ctx = &$context;
        $crate::primitive::Node::new(ctx.node().env(), *ctx.$local())
    }};
}

#[derive(Clone, Debug, Deserialize, Getters, Serialize)]
pub struct MetaEnvContext {
    node: Node,

    #[serde(rename = "__imports")]
    imports: LocalNode,
    #[serde(rename = "__import_table")]
    import_table: LocalNode,
    #[serde(rename = "__serialize_path")]
    serialize_path: LocalNode,
}

impl MetaEnvContext {
    pub(super) fn placeholder() -> Self {
        let e = LocalNode::default();
        Self {
            node: Node::new(e, e),
            imports: e,
            import_table: e,
            serialize_path: e,
        }
    }
}

impl<'de> Context<'de> for MetaEnvContext {}
