use super::env_state::EnvState;
use crate::model::Eval;


pub trait Agent: Eval {
    fn env_state(&mut self) -> &mut EnvState;
}
