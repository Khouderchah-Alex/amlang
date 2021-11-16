use std::path::Path;

use amlang::agent::env_policy::{EnvPolicy, SimplePolicy};
use amlang::agent::{Agent, EnvManager};
use amlang::error::Error;
use amlang::parser::ParseIter;
use amlang::primitive::symbol_policies::policy_base;
use amlang::sexp::Sexp;
use amlang::token::string_stream::StringStream;


pub fn setup() -> Result<(Agent, EnvManager<impl EnvPolicy>), String> {
    const META_ENV_PATH: &str = "meta.env";

    // Start in this dir.
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let file_dir = Path::new(file!()).parent().unwrap();
    let start_dir = crate_dir.join(file_dir).canonicalize().unwrap();
    amlang::init(start_dir).unwrap();

    // Integration tests will call this method multiple times; ignore the error.
    if let Err(_err) = env_logger::try_init() {}

    // Bootstrap/deserialize.
    let manager =
        match amlang::agent::env_manager::EnvManager::<SimplePolicy>::bootstrap(META_ENV_PATH) {
            Ok(val) => val,
            Err(err) => return Err(format!("{}", err)),
        };

    // Prep agent.
    let mut agent = manager.agent().clone();
    let working_env = agent.find_env("working.env").unwrap();
    agent.jump_env(working_env);
    agent.designation_chain_mut().push_back(working_env);

    Ok((agent, manager))
}

pub fn results<S: AsRef<str>>(lang_agent: &mut Agent, s: S) -> Vec<Sexp> {
    let tokens = StringStream::new(s, policy_base).unwrap();
    let sexps = ParseIter::from_tokens(tokens);
    lang_agent
        .run(sexps, |_, _| {})
        .map(|e| e.unwrap())
        .collect::<Vec<_>>()
}

pub fn results_with_errors<S: AsRef<str>>(
    lang_agent: &mut Agent,
    s: S,
) -> Vec<Result<Sexp, Error>> {
    let tokens = StringStream::new(s, policy_base).unwrap();
    let sexps = ParseIter::from_tokens(tokens);
    lang_agent.run(sexps, |_, _| {}).collect::<Vec<_>>()
}
