use super::env_state::EnvState;
use crate::model::Eval;


pub trait Agent: Eval {
    fn run(&mut self) -> Result<(), String>;
    fn env_state(&mut self) -> &mut EnvState;
}
