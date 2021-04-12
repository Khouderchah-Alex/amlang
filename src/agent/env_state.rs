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
        let env = meta.insert_structure(MetaEnvStructure::Env(Box::new(MemEnvironment::new())));
        let pos = EnvState::access_env(&mut meta, env).self_node();

        Self { env, pos, meta }
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
        EnvState::access_env(&mut self.meta, self.env)
    }

    // TODO(func) Move to same central location as above.
    fn access_env(meta: &mut MetaEnvironment, node: NodeId) -> &mut EnvObject {
        match meta.node_structure(node).unwrap() {
            MetaEnvStructure::Env(env) => env.as_mut(),
            _ => panic!(),
        }
    }
}
