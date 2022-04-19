use std::convert::TryFrom;

use super::env_prelude::EnvPrelude;
use crate::agent::Agent;
use crate::environment::environment::EnvObject;
use crate::environment::LocalNode;
use crate::error::Error;
use crate::model::Reflective;
use crate::primitive::{Node, Primitive};
use crate::sexp::{HeapSexp, Sexp};

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

        impl Reflective for AmlangContext {
            fn reify(&self, _agent: &mut Agent) -> Sexp {
                let meta: Sexp = vec![
                    $(Node::new(LocalNode::default(), self.$meta_node),)+
                ].into();
                let lang: Sexp = vec![
                    $(Node::new(self.lang_env, self.$lang_node),)+
                ].into();
                list!(meta, lang,)
            }

            fn reflect<F>(structure: Sexp, agent: &mut Agent, _resolve: F) -> Result<Self, Error>
            where
                Self: Sized,
                F: Fn(&mut Agent, &Primitive) -> Result<Node, Error> {
                // Clone passed-in agent's context to hack around the
                // fact that we can't create a meta env here.
                let mut context = agent.context().clone();

                let (meta, lang) = break_sexp!(structure => (HeapSexp, HeapSexp), agent)?;
                let mut miter = meta.into_iter();
                $(context.$meta_node = Node::try_from(miter.next().unwrap().0).unwrap().local();)+
                let mut liter = lang.into_iter();
                $(context.$lang_node = Node::try_from(liter.next().unwrap().0).unwrap().local();)+

                Ok(context)
            }

            fn valid_discriminator(_node: Node, _agent: &Agent) -> bool {
                unimplemented!();
            }
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
