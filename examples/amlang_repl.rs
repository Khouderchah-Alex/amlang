//! Basic REPL in Amlang; no special impls, single-threaded w/SimplePolicy.
//!
//! Run with saved state as:       `cargo run --example simple_repl`.
//! Reset saved state and run as:  `cargo run --example simple_repl -- -r`.
//!
//!
//! Uses envs in the .gitignore'd examples/envs/ directory and {de,}serializes,
//! so state is maintained between executions. Lacking a meta.env, the meta.env
//! used for integration tests will copied over.
//!
//! The lang env is actually the one in the top-level envs/ directory, so this
//! can be used to make changes to the env as part of a commit.

use clap::{App, Arg};
use env_logger::{Builder, Env};
use log::LevelFilter;
use std::convert::TryFrom;
use std::path::Path;

use amlang::agent::env_policy::SimplePolicy;
use amlang::agent::{
    Agent, AmlangInterpreter, EnvManager, NullInterpreter, TransformExecutor, VmInterpreter,
};
use amlang::env::LocalNode;
use amlang::error::Error;
use amlang::parser::Parser;
use amlang::primitive::{Node, Primitive};
use amlang::pull_transform;
use amlang::sexp::Sexp;
use amlang::token::cli_stream::CliStream;


const SERIALIZATION_PATH: &str = ".";

fn main() -> Result<(), String> {
    // Setup logging.
    Builder::from_env(Env::default().default_filter_or("info"))
        .filter_module("rustyline", LevelFilter::Warn)
        .init();

    // Parse args.
    let matches = App::new("Cli Amlang REPL")
        .version("0.1")
        .about("Bare-bones single-threaded Amlang REPL with persistence")
        .arg(
            Arg::with_name("reset")
                .short('r')
                .long("reset")
                .help("Reset all serialized state"),
        )
        .get_matches();

    // Use examples/envs/.
    let base_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .canonicalize()
        .unwrap();
    let examples_dir = Path::new(file!()).parent().unwrap();
    let init_options = amlang::InitOptions::IsolatedRun(
        base_dir.join(examples_dir).join("envs"),
        matches.is_present("reset"),
    );
    amlang::init(init_options).unwrap();

    // Bootstrap/deserialize.
    let mut manager = match EnvManager::<SimplePolicy>::bootstrap(SERIALIZATION_PATH) {
        Ok(val) => val,
        Err(err) => return Err(format!("{}", err)),
    };

    // Prep agent.
    let pre_agent = manager.agent();
    let history_env = pre_agent.find_env("history.env").unwrap();
    let impl_env = pre_agent.find_env("impl.env").unwrap();
    let mut agent = pre_agent.fork(VmInterpreter::new(history_env, impl_env));
    agent
        .set_eval(move |frame| {
            let mut interpreter = AmlangInterpreter::new(impl_env);
            if let Some(frame) = frame {
                interpreter.eval_state.push(frame);
            }
            Ok(Box::new(interpreter))
        })
        .unwrap();

    let working_env = agent.find_env("working.env").unwrap();
    let pos = agent.jump_env(working_env);
    agent.designation_chain_mut().push_back(pos);

    // TODO(func) Rm once we sort out the deal with CliHelper holding a
    // potentially-stale Agent copy.
    let lang_env = agent.context().lang_env();
    agent
        .designation_chain_mut()
        .push_front(Node::new(lang_env, LocalNode::default()));

    // Run agent.
    let tokens = CliStream::with_helper(agent.fork(NullInterpreter::default()));
    let sexps = pull_transform!(?unwrap
                                tokens
                                =>. Parser::new()
                                =>. TransformExecutor::custom(
                                    &mut agent,
                                    agent_handler));
    for _result in sexps {}

    // Serialize.
    if let Err(err) = manager.serialize_full(SERIALIZATION_PATH, ["lang.env"].into()) {
        return Err(err.to_string());
    }

    Ok(())
}

fn agent_handler(agent: &mut Agent, sexp: Sexp) -> Result<Sexp, Error> {
    let result = agent.interpret(sexp);
    match &result {
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
    result
}
