use std::convert::TryFrom;

use super::env_prelude::EnvPrelude;
use crate::agent::Agent;
use crate::env::meta_env::MetaEnv;
use crate::env::LocalNode;
use crate::error::Error;
use crate::model::Reflective;
use crate::primitive::{Node, Primitive};
use crate::sexp::{HeapSexp, Sexp, SexpIntoIter};


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
        label,
        vector,
        set
    )
);

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
            meta: MetaEnv,

            // Relative to meta env.
            $($meta_node: LocalNode,)+
            // Relative to lang_env.
            $($lang_node: LocalNode,)+
        }


        impl AmlangContext {
            pub(super) fn new(meta: MetaEnv) -> Self {
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
            fn reify(&self, _agent: &Agent) -> Sexp {
                let meta: Sexp = vec![
                    $(Node::new(LocalNode::default(), self.$meta_node),)+
                ].into();
                let lang: Sexp = vec![
                    $(Node::new(self.lang_env, self.$lang_node),)+
                ].into();
                list!(meta, lang,)
            }

            fn reflect<F>(structure: Sexp, agent: &Agent, resolve: F) -> Result<Self, Error>
            where
                Self: Sized,
                F: Fn(&Agent, &Primitive) -> Result<Node, Error> {
                // Clone passed-in agent's context to hack around the
                // fact that we can't create a meta env here.
                let mut context = agent.context().clone();
                let (meta, lang) = break_sexp!(structure => (HeapSexp, HeapSexp), agent)?;

                let resolve_node = |iter: &mut SexpIntoIter| -> Node {
                    let primitive = Primitive::try_from(iter.next().unwrap().0).unwrap();
                    resolve(agent, &primitive).unwrap()
                };

                let mut miter = meta.into_iter();
                $(context.$meta_node = resolve_node(&mut miter).local();)+
                let mut liter = lang.into_iter();
                $(context.$lang_node = resolve_node(&mut liter).local();)+

                Ok(context)
            }

            fn valid_discriminator(_node: Node, _agent: &Agent) -> bool {
                unimplemented!();
            }
        }
    };
}

impl AmlangContext {
    pub fn meta(&self) -> &MetaEnv {
        &self.meta
    }
    pub fn meta_mut(&mut self) -> &mut MetaEnv {
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

use generate_context;
