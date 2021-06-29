use std::cell::UnsafeCell;

use crate::environment::environment::EnvObject;
use crate::environment::LocalNode;


pub struct AmlangContext {
    meta: UnsafeCell<Box<EnvObject>>,

    lang_env: LocalNode,
    self_node: LocalNode,
    designation: LocalNode,

    // All below are relative to lang_env.
    pub quote: LocalNode,
    pub lambda: LocalNode,
    pub def: LocalNode,
    pub tell: LocalNode,
    pub curr: LocalNode,
    pub jump: LocalNode,
    pub ask: LocalNode,
    pub placeholder: LocalNode,
    pub apply: LocalNode,
    pub eval: LocalNode,
    pub exec: LocalNode,
}


impl AmlangContext {
    pub(super) fn new(
        meta: Box<EnvObject>,
        lang_env: LocalNode,
        self_node: LocalNode,
        designation: LocalNode,
    ) -> Self {
        Self {
            meta: UnsafeCell::new(meta),
            lang_env,
            self_node,
            designation: designation.clone(),
            // This is delicate; putting placeholders here, but not used until
            // after EnvManager is bootstrapped.
            quote: designation.clone(),
            lambda: designation.clone(),
            def: designation.clone(),
            tell: designation.clone(),
            curr: designation.clone(),
            jump: designation.clone(),
            ask: designation.clone(),
            placeholder: designation.clone(),
            apply: designation.clone(),
            eval: designation.clone(),
            exec: designation.clone(),
        }
    }

    pub fn meta(&self) -> &mut EnvObject {
        // TODO(func) Need to develop SharedEnv to do this safely long-term.
        unsafe { &mut **self.meta.get() }
    }

    pub fn lang_env(&self) -> LocalNode {
        self.lang_env
    }

    pub fn self_node(&self) -> LocalNode {
        self.self_node
    }

    /// Returns designation node, which has the same id in every environment, as
    /// enforced by EnvManager.
    pub fn designation(&self) -> LocalNode {
        self.designation
    }
}
