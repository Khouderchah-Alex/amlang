//! Basic REPL in Amlang; no special impls, single-threaded w/SimplePolicy.
//!
//! Run with saved state as:       `RUST_LOG=info cargo run --example repl`.
//! Reset saved state and run as:  `RUST_LOG=info cargo run --example repl -- -r`.
//!
//!
//! Uses envs in the .gitignore'd examples/envs/ directory and {de,}serializes,
//! so state is maintained between executions. Lacking a meta.env, the meta.env
//! used for integration tests will copied over.
//!
//! The lang env is actually the one in the top-level envs/ directory, so this
//! can be used to make changes to the env as part of a commit.

use clap::{App, Arg};
use log::{info, LevelFilter};
use std::convert::TryFrom;
use std::fs;
use std::io;
use std::path::Path;
use std::process::Command;

use amlang::agent::env_policy::SimplePolicy;
use amlang::agent::{Agent, EnvManager};
use amlang::error::Error;
use amlang::parser::ParseIter;
use amlang::primitive::{Node, Primitive};
use amlang::sexp::Sexp;
use amlang::token::cli_stream::CliStream;


fn main() -> Result<(), String> {
    const RELATIVE_META_PATH: &str = "envs/meta.env";
    let base_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .canonicalize()
        .unwrap();

    // Setup logging.
    env_logger::Builder::from_default_env()
        .filter_module("rustyline", LevelFilter::Warn)
        .init();

    // Parse args.
    let matches = App::new("Cli Amlang REPL")
        .version("0.1")
        .about("Bare-bones single-threaded Amlang REPL with persistence")
        .arg(
            Arg::with_name("reset")
                .short("r")
                .long("reset")
                .help("Reset all serialized state"),
        )
        .get_matches();

    // Start amlang in this file's dir.
    let file_dir = Path::new(file!()).parent().unwrap();
    let start_dir = base_dir.join(file_dir);
    amlang::init(start_dir).unwrap();

    // Perform any needed state preparation.
    if matches.is_present("reset") {
        if let Err(err) = reset_state(&base_dir) {
            return Err(format!("Resetting state failed: {}", err));
        }
    }
    if !Path::new(RELATIVE_META_PATH).exists() {
        if let Err(err) = copy_meta(&base_dir) {
            return Err(format!("Copying meta.env failed: {}", err));
        }
    }

    // Bootstrap/deserialize.
    let mut manager = match EnvManager::<SimplePolicy>::bootstrap(RELATIVE_META_PATH) {
        Ok(val) => val,
        Err(err) => return Err(format!("{}", err)),
    };

    // Prep agent.
    let mut agent = manager.agent().clone();
    let working_env = agent.find_env("working.env").unwrap();
    agent.jump_env(working_env);
    agent.designation_chain_mut().push_back(working_env);

    // TODO(func) Rm once we sort out the deal with CliHelper holding a
    // potentially-stale Agent copy.
    let lang_env = agent.context().lang_env();
    agent.designation_chain_mut().push_front(lang_env);

    // Run agent.
    let tokens = CliStream::new(agent.clone());
    let sexps = ParseIter::from_tokens(tokens);
    for _result in agent.run(sexps, print_result) {}

    // Serialize.
    if let Err(err) = manager.serialize_full(RELATIVE_META_PATH) {
        return Err(err.to_string());
    }

    Ok(())
}

fn print_result(agent: &mut Agent, result: &Result<Sexp, Error>) {
    match result {
        Ok(val) => {
            print!("-> ");
            if let Ok(node) = <Node>::try_from(val) {
                print!("{}->", node);
                let designated = agent.designate(Primitive::Node(node)).unwrap();
                agent.print_sexp(&designated);
            } else {
                agent.print_sexp(&val);
            }
            println!("");
        }
        Err(err) => {
            agent.print_sexp(&err.kind().reify());
            println!("");
            agent.trace_error(err);
        }
    };
    println!("");
}


fn reset_state(base_dir: &Path) -> io::Result<()> {
    let envs_dir = base_dir.join("examples/envs/");
    for entry in fs::read_dir(envs_dir)? {
        let entry = entry?;
        let path = entry.path();
        // For now, let's just leave subdirs as safe havens.
        if path.is_dir() || path.file_name() == Some(".gitignore".as_ref()) {
            continue;
        }

        fs::remove_file(path)?;
    }
    Ok(())
}

fn copy_meta(base_dir: &Path) -> io::Result<()> {
    info!("No meta env; copying from tests/.");
    let test_meta = base_dir.join("tests/common/meta.env");
    let example_meta = base_dir.join("examples/envs/meta.env");
    fs::copy(test_meta, example_meta.clone())?;

    let a = Command::new("sh")
        .arg("-c")
        .arg(r"sed -i 's|../../envs/lang.env|../envs/lang.env|g' envs/meta.env")
        .status()
        .expect("failed to monkeypatch lang.env path");
    let b = Command::new("sh")
        .arg("-c")
        .arg(r"sed -E -i 's/(working|history)/envs\/\1/g' envs/meta.env")
        .status()
        .expect("failed to monkeypatch {working,history}.env path");
    if !a.success() || !b.success() {
        fs::remove_file(example_meta)?;
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Failed to monkeypatch paths",
        ));
    }
    Ok(())
}
