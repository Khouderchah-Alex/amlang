use std::env;
use std::path::PathBuf;

#[macro_use]
pub mod lang_err;
#[macro_use]
pub mod sexp;

pub mod agent;
pub mod builtins;
pub mod environment;
pub mod model;
pub mod parser;
pub mod primitive;
pub mod token;


pub fn init(start_dir: PathBuf) -> Result<(), String> {
    // Need to set dir to properly read relative paths in envs.
    assert!(start_dir.is_absolute());
    env::set_current_dir(start_dir).unwrap();

    if let Err(err) = env_logger::try_init() {
        // Integration tests will call this method multiple times; ignore the error.
        if !cfg!(not(test)) {
            panic!("{}", err);
        }
    }

    Ok(())
}
