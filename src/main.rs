use std::env;
use std::path::Path;

#[macro_use]
mod sexp_conversion;

mod agent;
#[cfg(test)]
mod append_vec;
mod builtins;
mod cons_list;
mod environment;
mod function;
mod model;
mod parser;
mod primitive;
mod sexp;
mod token;

use crate::agent::agent::Agent;


fn usage(args: &Vec<String>) {
    println!(
        "usage: {}",
        Path::new(&args[0]).file_name().unwrap().to_string_lossy()
    );
    println!();
}

fn main() -> Result<(), String> {
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        usage(&args);
        return Err(format!(
            "Wrong argument count: {}, expected 0",
            args.len() - 1
        ));
    }

    interactive_agent()
}

fn interactive_agent() -> Result<(), String> {
    const META_ENV_PATH: &str = "envs/meta.env";
    let mut manager = match agent::env_manager::EnvManager::bootstrap(META_ENV_PATH) {
        Ok(val) => val,
        Err(err) => return Err(format!("{:?}", err)),
    };

    let mut history_state = manager.env_state().clone();
    let history_env = history_state.find_env("history.env").unwrap();
    history_state.jump_env(history_env);

    let mut agent_state = manager.env_state().clone();
    let working_env = agent_state.find_env("working.env").unwrap();
    agent_state.jump_env(working_env);
    agent_state.designation_chain_mut().push_back(working_env);

    let mut user_agent = agent::amlang_agent::AmlangAgent::from_state(agent_state, history_state);
    user_agent.run()?;

    if let Err(err) = manager.serialize_full(META_ENV_PATH) {
        return Err(err.to_string());
    }

    Ok(())
}
