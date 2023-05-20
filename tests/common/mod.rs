use amlang::agent::env_policy::{EnvPolicy, SimplePolicy};
use amlang::agent::{Agent, AmlangInterpreter, EnvManager, TransformExecutor};
use amlang::error::Error;
use amlang::parser::Parser;
use amlang::primitive::symbol_policies::policy_base;
use amlang::pull_transform;
use amlang::sexp::Sexp;
use amlang::stream::input::StringReader;
use amlang::token::Tokenizer;
use amlang::InitOptions;


pub fn setup() -> Result<(Agent, EnvManager<impl EnvPolicy>), String> {
    amlang::init(InitOptions::RootRun).unwrap();

    // Integration tests will call this method multiple times; ignore the error.
    if let Err(_err) = env_logger::try_init() {}

    // Bootstrap/deserialize.
    let manager = match amlang::agent::env_manager::EnvManager::<SimplePolicy>::bootstrap(".") {
        Ok(val) => val,
        Err(err) => return Err(format!("{}", err)),
    };

    // Prep agent.
    let pre_agent = manager.agent();
    let history_env = pre_agent.find_env("history.env").unwrap();
    let impl_env = pre_agent.find_env("impl.env").unwrap();
    let mut agent = pre_agent.fork(AmlangInterpreter::new(history_env, impl_env));
    let working_env = agent.find_env("working.env").unwrap();
    agent.jump_env(working_env);
    agent.designation_chain_mut().push_back(working_env);

    Ok((agent, manager))
}

pub fn results<S: AsRef<str>>(lang_agent: &mut Agent, s: S) -> Vec<Sexp> {
    pull_transform!(?unwrap
                    StringReader::new(s.as_ref())
                    =>> Tokenizer::new(policy_base)
                    =>. Parser::new()
                    =>. TransformExecutor::top_interpret(lang_agent))
    .map(|e| e.unwrap())
    .collect::<Vec<_>>()
}

pub fn results_with_errors<S: AsRef<str>>(
    lang_agent: &mut Agent,
    s: S,
) -> Vec<Result<Sexp, Error>> {
    pull_transform!(?unwrap
                    StringReader::new(s.as_ref())
                    =>> Tokenizer::new(policy_base)
                    =>. Parser::new()
                    =>. TransformExecutor::top_interpret(lang_agent))
    .collect::<Vec<_>>()
}
