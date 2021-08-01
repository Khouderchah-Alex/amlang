use std::env;
use std::path::Path;

#[macro_use]
pub mod lang_err;
#[macro_use]
pub mod sexp;

pub mod agent;
#[cfg(test)]
pub mod append_vec;
pub mod builtins;
pub mod environment;
pub mod model;
pub mod parser;
pub mod primitive;
pub mod token;


pub fn init() -> Result<(), String> {
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        usage(&args);
        return Err(format!(
            "Wrong argument count: {}, expected 0",
            args.len() - 1
        ));
    }

    Ok(())
}

fn usage(args: &Vec<String>) {
    println!(
        "usage: {}",
        Path::new(&args[0]).file_name().unwrap().to_string_lossy()
    );
    println!();
}
