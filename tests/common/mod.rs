use amlang::agent::env_policy::{EnvPolicy, SimplePolicy};
use amlang::agent::symbol_policies::policy_base;
use amlang::agent::{Agent, EnvManager};
use amlang::error::Error;
use amlang::parser::Parser;
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
    let mut agent = manager.agent().clone();
    let working_env = agent.find_env("working.env").unwrap();
    agent.jump_env(working_env);
    agent.designation_chain_mut().push_back(working_env);

    Ok((agent, manager))
}

pub fn results<S: AsRef<str>>(lang_agent: &mut Agent, s: S) -> Vec<Sexp> {
    let sexps = stream(s).unwrap();
    lang_agent
        .run(sexps, |_, _| {})
        .map(|e| e.unwrap())
        .collect::<Vec<_>>()
}

pub fn results_with_errors<S: AsRef<str>>(
    lang_agent: &mut Agent,
    s: S,
) -> Vec<Result<Sexp, Error>> {
    let sexps = stream(s).unwrap();
    lang_agent.run(sexps, |_, _| {}).collect::<Vec<_>>()
}


fn stream<S: AsRef<str>>(input: S) -> Result<impl Iterator<Item = Result<Sexp, Error>>, Error> {
    Ok(pull_transform!(StringReader::new(input.as_ref())
                       =>> Tokenizer::new(policy_base)
                       =>. Parser::new()))
}
