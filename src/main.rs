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
    let mut serializer = agent::env_serializer::EnvSerializer::new();
    if let Err(err) = serializer.deserialize("./test.env") {
        return Err(format!("{:?}", err));
    }
    let mut user_agent = agent::amlang_agent::AmlangAgent::from_env(serializer.extract_env());
    user_agent.run()?;

    let mut serializer = agent::env_serializer::EnvSerializer::from_env(user_agent.extract_env());
    if let Err(err) = serializer.serialize("./testt.env") {
        return Err(err.to_string());
    }

    Ok(())
}
