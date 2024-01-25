//! Load & save envs (useful for migrations or validating unchanged semantics).

use clap::App;
use env_logger::{Builder, Env};
use log::LevelFilter;

use amlang::agent::env_policy::SimplePolicy;
use amlang::agent::EnvManager;


const SERIALIZATION_PATH: &str = ".";

fn main() -> Result<(), String> {
    Builder::from_env(Env::default().default_filter_or("info"))
        .filter_module("rustyline", LevelFilter::Warn)
        .init();

    let _matches = App::new("Amlang Env Reload")
        .version("0.1")
        .about("Load & save envs (useful for migrations)")
        .get_matches();


    amlang::init(amlang::InitOptions::RootRun).unwrap();

    let mut manager = match EnvManager::<SimplePolicy>::bootstrap(SERIALIZATION_PATH) {
        Ok(val) => val,
        Err(err) => return Err(format!("{}", err)),
    };

    if let Err(err) = manager.serialize_full(
        SERIALIZATION_PATH,
        // TODO(func) Serialized meta env with implicit envs.
        ["working.env", "history.env", "impl.env"].into(),
    ) {
        return Err(err.to_string());
    }

    Ok(())
}
