use super::*;

use crate::environment::Environment;


#[test]
fn create_environment() {
    let mut meta = MetaEnvironment::new();
    let env_node = meta.insert_structure(MetaEnvStructure::Env(Box::new(MemEnvironment::new())));
    if let MetaEnvStructure::Env(env) = meta.node_structure(env_node).unwrap() {
        let a = env.insert_atom();
        let b = env.insert_atom();
        env.insert_triple(env.self_node(), a, b);
    }
}
