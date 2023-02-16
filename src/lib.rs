use log::info;
use std::env::set_current_dir;
use std::fs::{copy, read_dir, remove_file};
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

#[macro_use]
pub mod amlang_helpers;
#[macro_use]
pub mod error;
#[macro_use]
pub mod sexp;
#[macro_use]
pub mod stream;

pub mod agent;
pub mod builtins;
pub mod continuation;
pub mod env;
pub mod model;
pub mod parser;
pub mod primitive;
pub mod token;

pub mod prelude {
    pub use crate::agent::{Agent, EnvManager};
    pub use crate::env::{NodeSet, TripleSet};
    pub use crate::error::{Error, ErrorKind};
    pub use crate::primitive::*;
    pub use crate::sexp::{Cons, ConsList, Sexp};
    pub use crate::InitOptions;
    pub use crate::{pull_transform, push_transform};
}


/// Method of starting up Amlang.
///
/// RootRun will directly use all Amlang envs, so modifications can be
/// applied to the project (e.g. for developing Amlang).
///
/// IsolatedRun will use a different, project-specific path for
/// reading/writing envs. Some Amlang envs will be copied over for
/// bootstrapping purposes. Note that the lang.env is currently still
/// the direct one from upstream Amlang.
pub enum InitOptions {
    IsolatedRun(PathBuf, bool), // env_path, reset_state
    RootRun,
}

/// Initialization function that must be called prior to using Amlang agents.
///
/// Note that this function does *not* setup logging, clients should
/// take care of that prior to calling this function. See:
///   https://github.com/rust-lang/log#in-executables.
pub fn init(options: InitOptions) -> Result<(), String> {
    let amlang_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .canonicalize()
        .unwrap();

    match options {
        InitOptions::IsolatedRun(env_path, should_reset_state) => {
            // Need to set dir to properly read relative paths in envs.
            if !env_path.is_absolute() {
                return Err(format!("env_path must be absolute"));
            }
            if let Err(err) = set_current_dir(env_path.clone()) {
                return Err(format!("Setting current dir failed: {}", err));
            }

            // Perform any needed state preparation.
            if should_reset_state {
                if let Err(err) = reset_state(&env_path) {
                    return Err(format!("Resetting state failed: {}", err));
                }
            }
            let meta_path = "meta.env";
            if !Path::new(meta_path).exists() {
                if let Err(err) = copy_meta(&amlang_root, &env_path) {
                    return Err(format!("Copying meta.env failed: {}", err));
                }
            }
            // Always copy context.bootstrap to easily handle new lang additions.
            if let Err(err) = copy_context(&amlang_root, &env_path) {
                return Err(format!("Copying context.bootstrap failed: {}", err));
            }
        }
        InitOptions::RootRun => {
            if let Err(err) = set_current_dir(amlang_root.join("envs")) {
                return Err(format!("Setting to amlang dir failed: {}", err));
            }
        }
    }

    Ok(())
}


fn reset_state(env_path: &Path) -> io::Result<()> {
    for entry in read_dir(env_path)? {
        let entry = entry?;
        let path = entry.path();
        // For now, let's just leave subdirs as safe havens.
        if path.is_dir() || path.file_name() == Some(".gitignore".as_ref()) {
            continue;
        }

        remove_file(path)?;
    }
    Ok(())
}

fn copy_meta(amlang_root: &Path, env_path: &Path) -> io::Result<()> {
    info!("No meta env; copying from envs/.");
    let amlang_meta = amlang_root.join("envs/meta.env");
    let target_meta = env_path.join("meta.env");
    copy(amlang_meta, target_meta.clone())?;

    // Monkeypatch amlang's lang.env.
    let a = Command::new("sh")
        .arg("-c")
        .arg(format!(
            "{}{}{}",
            r#"sed -i 's|"lang.env"|"#,
            format!(
                "\"{}\"",
                amlang_root.join("envs/lang.env").to_string_lossy()
            ),
            r#"|g' meta.env"#
        ))
        .status()
        .expect("failed to monkeypatch lang.env path");
    if !a.success() {
        remove_file(target_meta)?;
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Failed to monkeypatch paths",
        ));
    }
    Ok(())
}

fn copy_context(amlang_root: &Path, env_path: &Path) -> io::Result<u64> {
    info!("Copying context bootstrap from envs/.");
    let amlang_context = amlang_root.join("envs/context.bootstrap");
    let target_context = env_path.join("context.bootstrap");
    copy(amlang_context, target_context)
}
