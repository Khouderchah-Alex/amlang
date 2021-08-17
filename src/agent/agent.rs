use super::agent_state::AgentState;
use crate::model::Eval;


pub trait Agent: Eval {
    fn state(&self) -> &AgentState;
    fn state_mut(&mut self) -> &mut AgentState;
}
