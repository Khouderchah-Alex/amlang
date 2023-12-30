use std::collections::BTreeMap;

use super::local_node::LocalNode;
use super::EnvObject;


#[derive(Clone, Debug)]
pub struct MetaEnv {
    base: Box<EnvObject>,
    envs: BTreeMap<LocalNode, Box<EnvObject>>,
}

impl MetaEnv {
    pub fn new(base: Box<EnvObject>) -> Self {
        Self {
            base: base,
            envs: Default::default(),
        }
    }

    pub fn insert_env(&mut self, node: LocalNode, env: Box<EnvObject>) {
        self.envs.insert(node, env);
    }

    // For EnvManager usage in e.g. migrations & defrags.
    // Clients are responsible for ensuring semantic safety of this operation.
    pub unsafe fn replace_env(&mut self, node: LocalNode, env: Box<EnvObject>) {
        if let Some(entry) = self.envs.get_mut(&node) {
            *entry = env;
        }
    }

    pub fn env(&self, node: LocalNode) -> Option<&Box<EnvObject>> {
        self.envs.get(&node)
    }

    pub fn env_mut(&mut self, node: LocalNode) -> Option<&mut Box<EnvObject>> {
        self.envs.get_mut(&node)
    }

    pub fn base(&self) -> &Box<EnvObject> {
        &self.base
    }

    pub fn base_mut(&mut self) -> &mut Box<EnvObject> {
        &mut self.base
    }
}
