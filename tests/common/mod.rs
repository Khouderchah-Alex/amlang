use std::path::Path;

use amlang::agent::amlang_agent::{AmlangAgent, RunError};
use amlang::agent::env_policy::SimplePolicy;
use amlang::primitive::symbol_policies::policy_base;
use amlang::sexp::Sexp;
use amlang::token::string_stream::StringStream;


pub fn setup() -> Result<AmlangAgent, String> {
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
    let mut agent_state = manager.state().clone();
    let working_env = agent_state.find_env("working.env").unwrap();
    agent_state.jump_env(working_env);
    agent_state.designation_chain_mut().push_back(working_env);

    let history_env = agent_state.find_env("history.env").unwrap();
    Ok(AmlangAgent::from_state(agent_state, history_env))
}

pub fn results<S: AsRef<str>>(lang_agent: &mut AmlangAgent, s: S) -> Vec<Sexp> {
    let stream = StringStream::new(s, policy_base).unwrap();
    lang_agent
        .run(stream, |_, _| {})
        .map(|e| e.unwrap())
        .collect::<Vec<_>>()
}

pub fn results_with_errors<S: AsRef<str>>(
    lang_agent: &mut AmlangAgent,
    s: S,
) -> Vec<Result<Sexp, RunError>> {
    let stream = StringStream::new(s, policy_base).unwrap();
    lang_agent.run(stream, |_, _| {}).collect::<Vec<_>>()
}
