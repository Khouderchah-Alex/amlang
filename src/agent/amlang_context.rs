use std::cell::UnsafeCell;

use crate::environment::environment::EnvObject;
use crate::environment::LocalNode;


pub struct AmlangContext {
    meta: UnsafeCell<Box<EnvObject>>,

    // Available in all envs.
    self_node: LocalNode,
    designation: LocalNode,

    // Relative to meta env.
    pub lang_env: LocalNode,
    pub imports: LocalNode,
    pub import_table: LocalNode,
    pub serialize_path: LocalNode,

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
    pub import: LocalNode,
}


impl AmlangContext {
    pub(super) fn new(meta: Box<EnvObject>, self_node: LocalNode, designation: LocalNode) -> Self {
        Self {
            meta: UnsafeCell::new(meta),

            self_node,
            designation: designation.clone(),

            // This is delicate; putting placeholders here, which must be set
            // properly during bootstrapping.
            lang_env: designation.clone(),
            imports: designation.clone(),
            import_table: designation.clone(),
            serialize_path: designation.clone(),

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
            import: designation.clone(),
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
