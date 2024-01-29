use derive_getters::Getters;
use serde::{Deserialize, Serialize};

use crate::agent::Agent;
use crate::env::LocalNode;
use crate::error::Error;
use crate::primitive::{policy_base, Node, ToSymbol};
use crate::sexp::{Cons, Sexp};


pub trait Context<'de>: Deserialize<'de> + Sized {
    fn load(node: Node, agent: &'de mut Agent) -> Result<Self, Error> {
        let mut reified: Sexp = agent
            .access_env(node.env())
            .unwrap()
            .designation_pairs(node.local())
            .into_iter()
            .map(|(k, v)| Cons::new(k.clone(), Node::new(node.env(), *v)).into())
            .collect::<Vec<Sexp>>()
            .into();
        reified.push_front(Cons::new("node".to_symbol_or_panic(policy_base), node));
        reified.push_front(Self::name().to_symbol_or_panic(policy_base));
        println!("{}", reified);

        agent.reflect::<Self>(reified)
    }

    fn name() -> &'static str;
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

impl<'de> Context<'de> for MetaEnvContext {
    fn name() -> &'static str {
        "MetaEnvContext"
    }
}
