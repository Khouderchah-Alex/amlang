use super::agent_state::AgentState;
use crate::model::Interpretation;


pub trait Agent: Interpretation {
    fn state(&self) -> &AgentState;
    fn state_mut(&mut self) -> &mut AgentState;
}
