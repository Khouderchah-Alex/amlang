use std::cell::UnsafeCell;

use crate::environment::environment::EnvObject;
use crate::environment::NodeId;


pub struct AmlangContext {
    meta: UnsafeCell<Box<EnvObject>>,

    lang_env: NodeId,
    designation: NodeId,

    pub quote: NodeId,
    pub lambda: NodeId,
    pub def: NodeId,
    pub tell: NodeId,
    pub curr: NodeId,
    pub jump: NodeId,
    pub ask: NodeId,
    pub placeholder: NodeId,
    pub apply: NodeId,
    pub eval: NodeId,
    pub exec: NodeId,
}


impl AmlangContext {
    pub(super) fn new(meta: Box<EnvObject>, lang_env: NodeId, designation: NodeId) -> Self {
        Self {
            meta: UnsafeCell::new(meta),
            lang_env,
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

    pub fn lang_env(&self) -> NodeId {
        self.lang_env
    }

    pub fn designation(&self) -> NodeId {
        self.designation
    }
}
