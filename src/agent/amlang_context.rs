use std::cell::UnsafeCell;

use crate::environment::meta_environment::MetaEnvironment;
use crate::environment::NodeId;


pub struct AmlangContext {
    meta: UnsafeCell<MetaEnvironment>,

    base_env: NodeId,
    designation: NodeId,
}


impl AmlangContext {
    pub(super) fn new(meta: MetaEnvironment, base_env: NodeId, designation: NodeId) -> Self {
        Self {
            meta: UnsafeCell::new(meta),
            base_env,
            designation,
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
