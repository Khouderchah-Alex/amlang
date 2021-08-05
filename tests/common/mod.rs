use std::path::Path;

use amlang::agent::agent::Agent;
use amlang::agent::amlang_agent::AmlangAgent;
use amlang::primitive::symbol_policies::policy_base;
use amlang::sexp::Sexp;
use amlang::token::string_stream::StringStream;


pub fn setup() -> Result<AmlangAgent, String> {
    const META_ENV_PATH: &str = "meta.env";

    // Start in this dir.
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let file_dir = Path::new(file!()).parent().unwrap();
    let start_dir = crate_dir.join(file_dir).canonicalize().unwrap();
    amlang::init(start_dir)?;

    // Bootstrap/deserialize.
    let mut manager = match amlang::agent::env_manager::EnvManager::bootstrap(META_ENV_PATH) {
        Ok(val) => val,
        Err(err) => return Err(format!("{:?}", err)),
    };

    // Prep agent.
    let mut history_state = manager.env_state().clone();
    let history_env = history_state.find_env("history.env").unwrap();
    history_state.jump_env(history_env);

    let mut agent_state = manager.env_state().clone();
    let working_env = agent_state.find_env("working.env").unwrap();
    agent_state.jump_env(working_env);
    agent_state.designation_chain_mut().push_back(working_env);

    Ok(AmlangAgent::from_state(agent_state, history_state))
}

pub fn results<S: AsRef<str>>(lang_agent: &mut AmlangAgent, s: S) -> Vec<Sexp> {
    let stream = StringStream::new(s, policy_base).unwrap();
    lang_agent
        .run(stream, |_, _| {})
        .map(|e| e.unwrap())
        .collect::<Vec<_>>()
}
