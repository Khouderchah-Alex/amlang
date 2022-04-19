use super::env_prelude::EnvPrelude;
use crate::environment::environment::EnvObject;
use crate::environment::LocalNode;

macro_rules! generate_context {
    (
        ($($meta_node:ident),+),
        ($($lang_node:ident),+)
        $(,)?
    ) => {
        // TODO(perf) Make generic over Environment, but in a manner consistent with
        // library usage and without pushing templating over most of the codebase
        // (e.g. changing build flag to select Overlay class).
        #[derive(Clone, Debug)]
        pub struct AmlangContext {
            meta: Box<EnvObject>,

            // TODO(func) Don't make these pub once we deserialize internally.
            // Relative to meta env.
            $(pub(super) $meta_node: LocalNode,)+
            // Relative to lang_env.
            $(pub(super) $lang_node: LocalNode,)+
        }


        impl AmlangContext {
            pub(super) fn new(meta: Box<EnvObject>) -> Self {
                let placeholder = LocalNode::new(1);
                Self {
                    meta,

                    // This is delicate; putting placeholders here, which must be set
                    // properly during bootstrapping.
                    $($meta_node: placeholder.clone(),)+

                    $($lang_node: placeholder.clone(),)+
                }
            }

            $(pub fn $meta_node(&self) -> LocalNode { self.$meta_node })+
            $(pub fn $lang_node(&self) -> LocalNode { self.$lang_node })+
        }
    };
}

generate_context!(
    (lang_env, imports, import_table, serialize_path),
    (
        quote,
        lambda,
        fexpr,
        def,
        tell,
        curr,
        jump,
        ask,
        placeholder,
        apply,
        eval,
        exec,
        import,
        progn,
        branch,
        let_basic,
        let_rec,
        env_find,
        t,
        f,
        eq,
        sym_node_table,
        sym_sexp_table,
        local_node_table,
        label
    )
);

impl AmlangContext {
    pub fn meta(&self) -> &Box<EnvObject> {
        &self.meta
    }
    pub fn meta_mut(&mut self) -> &mut Box<EnvObject> {
        &mut self.meta
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
