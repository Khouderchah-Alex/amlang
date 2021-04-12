use crate::environment::environment::{EnvObject, Environment};
use crate::environment::mem_environment::MemEnvironment;
use crate::environment::meta_environment::{MetaEnvStructure, MetaEnvironment};
use crate::environment::NodeId;


pub struct EnvState {
    env: NodeId,
    pos: NodeId,

    // TODO(func) Move to central location.
    meta: MetaEnvironment,
}

impl EnvState {
    pub fn new() -> Self {
        let mut meta = MetaEnvironment::new();
        let meta_self = meta.self_node();
        Self {
            env: meta.insert_structure(MetaEnvStructure::Env(Box::new(MemEnvironment::new()))),
            pos: meta_self,

            meta,
        }
    }

    pub fn pos(&self) -> NodeId {
        self.pos
    }

    pub fn jump(&mut self, node: NodeId) {
        // TODO(sec) Verify.
        self.pos = node;
    }

    // TODO(func) impl
    // pub fn teleport(&mut self, portal: Portal)

    pub fn env(&mut self) -> &mut EnvObject {
        match self.meta.node_structure(self.env).unwrap() {
            MetaEnvStructure::Env(env) => env.as_mut(),
            _ => panic!(),
        }
    }
}
