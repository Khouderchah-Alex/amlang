use std::env;
use std::path::Path;

mod agent;
mod append_vec;
mod builtin;
mod cons_list;
mod environment;
mod function;
mod model;
mod number;
mod old_environment;
mod parser;
mod primitive;
mod sexp;
mod symbol;
mod syntax;
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
    let mut user_agent = agent::amlang_agent::AmlangAgent::new();
    user_agent.run()
}
