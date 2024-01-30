use derive_getters::Getters;
use serde::{Deserialize, Serialize};

use super::context::Context;
use crate::env::LocalNode;
use crate::primitive::Node;

#[derive(Clone, Debug, Deserialize, Getters, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct AmlangContext {
    node: Node,

    quote: LocalNode,
    lambda: LocalNode,
    fexpr: LocalNode,
    def: LocalNode,
    tell: LocalNode,
    curr: LocalNode,
    jump: LocalNode,
    ask: LocalNode,
    #[serde(rename = "_")]
    placeholder: LocalNode,
    apply: LocalNode,
    eval: LocalNode,
    exec: LocalNode,
    import: LocalNode,
    progn: LocalNode,
    #[serde(rename = "if")]
    branch: LocalNode,
    #[serde(rename = "let")]
    let_basic: LocalNode,
    #[serde(rename = "letrec")]
    let_rec: LocalNode,
    env_find: LocalNode,
    env_jump: LocalNode,
    #[serde(rename = "true")]
    t: LocalNode,
    #[serde(rename = "false")]
    f: LocalNode,
    eq: LocalNode,
    #[serde(rename = "table-sym-node")]
    sym_node_table: LocalNode,
    #[serde(rename = "table-sym-sexp")]
    sym_sexp_table: LocalNode,
    #[serde(rename = "table-lnode")]
    local_node_table: LocalNode,
    vector: LocalNode,
    #[serde(rename = "set!")]
    set: LocalNode,
    anon: LocalNode,
    #[serde(rename = "$")]
    self_ref: LocalNode,
}

impl<'de> Context<'de> for AmlangContext {}
