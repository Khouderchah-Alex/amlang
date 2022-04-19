use super::env_prelude::EnvPrelude;
use crate::environment::environment::EnvObject;
use crate::environment::LocalNode;


// TODO(perf) Make generic over Environment, but in a manner consistent with
// library usage and without pushing templating over most of the codebase
// (e.g. changing build flag to select Overlay class).
#[derive(Clone, Debug)]
pub struct AmlangContext {
    meta: Box<EnvObject>,

    // Relative to meta env.
    pub lang_env: LocalNode,
    pub imports: LocalNode,
    pub import_table: LocalNode,
    pub serialize_path: LocalNode,

    // All below are relative to lang_env.
    pub quote: LocalNode,
    pub lambda: LocalNode,
    pub fexpr: LocalNode,
    pub def: LocalNode,
    pub tell: LocalNode,
    pub curr: LocalNode,
    pub jump: LocalNode,
    pub ask: LocalNode,
    pub placeholder: LocalNode,
    pub apply: LocalNode,
    pub eval: LocalNode,
    pub exec: LocalNode,
    pub import: LocalNode,
    pub progn: LocalNode,
    pub branch: LocalNode,
    pub let_basic: LocalNode,
    pub let_rec: LocalNode,
    pub env_find: LocalNode,
    pub t: LocalNode,
    pub f: LocalNode,
    pub eq: LocalNode,
    pub sym_node_table: LocalNode,
    pub sym_sexp_table: LocalNode,
    pub local_node_table: LocalNode,
    pub label: LocalNode,
}


impl AmlangContext {
    pub(super) fn new(meta: Box<EnvObject>) -> Self {
        let placeholder = LocalNode::new(1);
        Self {
            meta,

            // This is delicate; putting placeholders here, which must be set
            // properly during bootstrapping.
            lang_env: placeholder.clone(),
            imports: placeholder.clone(),
            import_table: placeholder.clone(),
            serialize_path: placeholder.clone(),

            quote: placeholder.clone(),
            lambda: placeholder.clone(),
            fexpr: placeholder.clone(),
            def: placeholder.clone(),
            tell: placeholder.clone(),
            curr: placeholder.clone(),
            jump: placeholder.clone(),
            ask: placeholder.clone(),
            placeholder: placeholder.clone(),
            apply: placeholder.clone(),
            eval: placeholder.clone(),
            exec: placeholder.clone(),
            import: placeholder.clone(),
            progn: placeholder.clone(),
            branch: placeholder.clone(),
            let_basic: placeholder.clone(),
            let_rec: placeholder.clone(),
            env_find: placeholder.clone(),
            t: placeholder.clone(),
            f: placeholder.clone(),
            eq: placeholder.clone(),
            sym_node_table: placeholder.clone(),
            sym_sexp_table: placeholder.clone(),
            local_node_table: placeholder.clone(),
            label: placeholder.clone(),
        }
    }

    pub fn meta(&self) -> &Box<EnvObject> {
        &self.meta
    }
    pub fn meta_mut(&mut self) -> &mut Box<EnvObject> {
        &mut self.meta
    }

    pub fn lang_env(&self) -> LocalNode {
        self.lang_env
    }

    pub const fn self_node(&self) -> LocalNode {
        EnvPrelude::SelfEnv.local()
    }
    pub const fn designation(&self) -> LocalNode {
        EnvPrelude::Designation.local()
    }
    pub const fn tell_handler(&self) -> LocalNode {
        EnvPrelude::TellHandler.local()
    }
}
