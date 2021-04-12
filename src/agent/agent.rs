use super::designation::Designation;
use super::env_state::EnvState;


pub trait Agent: Designation + Default {
    fn run(&mut self) -> Result<(), String>;
    fn env_state(&mut self) -> &mut EnvState;
}
