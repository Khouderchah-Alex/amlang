//! Basic REPL in Amlang; no special impls, single-threaded w/SimplePolicy.
//!
//! Run as `RUST_LOG=info cargo run --example repl`.
//!
//!
//! Uses envs in the .gitignore'd examples/envs/ directory and {de,}serializes,
//! so state is maintained between executions. State can be reset simply with
//! `rm examples/envs/*`; if run without a meta.env, the meta.env used for
//! integration tests will copied over.
//!
//! The lang env is actually the one in the top-level envs/ directory, so this
//! can be used to make changes to the env as part of a commit.

use log::info;
use std::convert::TryFrom;
use std::path::Path;
use std::process::Command;

use amlang::agent::env_policy::SimplePolicy;
use amlang::agent::{Agent, EnvManager};
use amlang::error::Error;
use amlang::primitive::{Node, Primitive};
use amlang::sexp::Sexp;
use amlang::token::interactive_stream::InteractiveStream;


fn main() -> Result<(), String> {
    const META_ENV_PATH: &str = "envs/meta.env";

    // Setup logging.
    env_logger::init();

    // Start in this dir.
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let file_dir = Path::new(file!()).parent().unwrap();
    let start_dir = crate_dir.join(file_dir).canonicalize().unwrap();
    amlang::init(start_dir).unwrap();

    // Copy meta.env from tests/ if needed.
    if !Path::new(META_ENV_PATH).exists() {
        info!("No meta env; copying from tests/.");
        let file_dir = "tests/common/meta.env";
        let test_meta_dir = crate_dir.join(file_dir).canonicalize().unwrap();

        if let Err(err) = std::fs::copy(test_meta_dir, META_ENV_PATH) {
            return Err(format!("Copying meta.env failed: {}", err));
        }

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
            std::fs::remove_file(META_ENV_PATH).unwrap();
            panic!("Failed to monkeypatch paths");
        }
    }

    // Bootstrap/deserialize.
    let mut manager = match EnvManager::<SimplePolicy>::bootstrap(META_ENV_PATH) {
        Ok(val) => val,
        Err(err) => return Err(format!("{}", err)),
    };

    // Prep agent.
    let mut agent = manager.agent().clone();
    let working_env = agent.find_env("working.env").unwrap();
    agent.jump_env(working_env);
    agent.designation_chain_mut().push_back(working_env);

    // TODO(func) Rm once we sort out the deal with InteractiveHelper holding a
    // potentially-stale Agent copy.
    let lang_env = agent.context().lang_env();
    agent.designation_chain_mut().push_front(lang_env);

    // Run agent.
    let stream = InteractiveStream::new(agent.clone());
    for _result in agent.run(stream, print_result) {}

    // Serialize.
    if let Err(err) = manager.serialize_full(META_ENV_PATH) {
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
