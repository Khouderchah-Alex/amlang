use std::env;
use std::path::{Path, PathBuf};

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


pub fn init(start_dir: PathBuf) -> Result<(), String> {
    assert!(start_dir.is_absolute());
    env::set_current_dir(start_dir).unwrap();

    if let Err(err) = env_logger::try_init() {
        // Integration tests will call this method multiple times; ignore the error.
        if !cfg!(not(test)) {
            panic!("{}", err);
        }
    }

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
