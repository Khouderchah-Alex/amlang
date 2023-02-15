use std::collections::VecDeque;

use super::interpreter::InterpreterState;
use super::Agent;
use crate::error::Error;
use crate::sexp::Sexp;
use crate::stream::Transform;


/// Use Agent as Transform.
pub struct TransformExecutor<'a> {
    agent: &'a mut Agent,
    cache: VecDeque<Sexp>,
    handler: fn(&mut Agent, Sexp) -> Result<Sexp, Error>,
}

impl<'a> TransformExecutor<'a> {
    pub fn direct<I: InterpreterState + 'static>(
        agent: &'a mut Agent,
        base_interpreter: I,
    ) -> Self {
        Self {
            agent: agent.run(base_interpreter),
            cache: Default::default(),
            handler: Self::default_handler,
        }
    }

    pub fn with_handler<I: InterpreterState + 'static>(
        agent: &'a mut Agent,
        base_interpreter: I,
        handler: fn(&mut Agent, Sexp) -> Result<Sexp, Error>,
    ) -> Self {
        Self {
            agent: agent.run(base_interpreter),
            cache: Default::default(),
            handler: handler,
        }
    }

    fn default_handler(agent: &mut Agent, sexp: Sexp) -> Result<Sexp, Error> {
        agent.top_interpret(sexp)
    }
}

impl<'a> Transform<Sexp, Sexp> for TransformExecutor<'a> {
    fn input(&mut self, input: Result<Sexp, Error>) -> Result<bool, Error> {
        self.cache
            .push_back((self.handler)(&mut self.agent, input?)?);
        Ok(true)
    }

    fn output(&mut self) -> Option<Result<Sexp, Error>> {
        Some(Ok(self.cache.pop_front()?))
    }
}
