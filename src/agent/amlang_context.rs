use std::cell::UnsafeCell;

use crate::environment::meta_environment::MetaEnvironment;
use crate::environment::NodeId;


pub struct AmlangContext {
    meta: UnsafeCell<MetaEnvironment>,

    base_env: NodeId,
    designation: NodeId,

    pub quote: NodeId,
    pub lambda: NodeId,
    pub def: NodeId,
    pub tell: NodeId,
    pub curr: NodeId,
    pub jump: NodeId,
    pub ask: NodeId,
    pub placeholder: NodeId,
}


impl AmlangContext {
    pub(super) fn new(meta: MetaEnvironment, base_env: NodeId, designation: NodeId) -> Self {
        Self {
            meta: UnsafeCell::new(meta),
            base_env,
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
        }
    }

    pub fn meta(&self) -> &mut MetaEnvironment {
        // TODO(func) Need to develop SharedEnv to do this safely long-term.
        unsafe { &mut *self.meta.get() }
    }

    pub fn base_env(&self) -> NodeId {
        self.base_env
    }

    pub fn designation(&self) -> NodeId {
        self.designation
    }
}
