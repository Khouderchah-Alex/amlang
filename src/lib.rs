use std::env;
use std::io;
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


/// Initialization function that must be called prior to using Amlang agents.
///
/// Note that this function does *not* setup logging. Clients can choose from
/// the available set of loggers or implement their own. See:
///   https://github.com/rust-lang/log#in-executables.
pub fn init(start_dir: PathBuf) -> Result<(), io::Error> {
    // Need to set dir to properly read relative paths in envs.
    if !start_dir.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "start_dir must be absolute",
        ));
    }
    env::set_current_dir(start_dir)
}
