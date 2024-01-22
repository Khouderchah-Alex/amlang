use std::convert::TryFrom;

use log::debug;

use super::amlang_wrappers::*;
use super::interpreter::{Interpreter, InterpreterState};
use super::Agent;
use crate::agent::lang_error::{ExpectedCount, LangError};
use crate::continuation::Continuation;
use crate::env::LocalNode;
use crate::error::Error;
use crate::primitive::prelude::*;
use crate::primitive::table::Table;
use crate::sexp::{Cons, HeapSexp, Sexp};


#[derive(Debug)]
pub struct AmlangInterpreter {
    pub eval_state: Continuation<SymNodeTable>,
    impl_env: LocalNode,
}

impl AmlangInterpreter {
    pub fn new(impl_env: LocalNode) -> Self {
        Self {
            eval_state: Continuation::new(SymNodeTable::default()),
            impl_env,
        }
    }
}

impl InterpreterState for AmlangInterpreter {
    fn borrow_agent<'a>(&'a mut self, agent: &'a mut Agent) -> Box<dyn Interpreter + 'a> {
        Box::new(ExecutingInterpreter::from_state(self, agent))
    }
}


struct ExecutingInterpreter<'a> {
    state: &'a mut AmlangInterpreter,
    agent: &'a mut Agent,
}

impl<'a> ExecutingInterpreter<'a> {
    fn from_state(state: &'a mut AmlangInterpreter, agent: &'a mut Agent) -> Self {
        // Ensure agent designates amlang nodes first.
        let lang_env = agent.context().lang_env();
        if agent
            .designation_chain()
            .front()
            .cloned()
            .unwrap_or_default()
            .env()
            != lang_env
        {
            agent
                .designation_chain_mut()
                .push_front(Node::new(lang_env, LocalNode::default()));
        }

        Self { state, agent }
    }

    fn agent(&self) -> &Agent {
        &self.agent
    }
    fn agent_mut(&mut self) -> &mut Agent {
        &mut self.agent
    }

    fn make_lambda(
        &mut self,
        params: Vec<Symbol>,
        body: HeapSexp,
        reflect: bool,
    ) -> Result<(Procedure, SymNodeTable), Error> {
        let mut surface = Vec::new();
        let mut frame = SymNodeTable::default();
        let impl_env = self.state.impl_env;
        for symbol in params {
            let node = self.agent_mut().define_to(impl_env, None)?;
            if frame.contains_key(&symbol) {
                return err!(
                    self.agent(),
                    LangError::InvalidArgument {
                        given: symbol.into(),
                        expected: "unique name within argument list".into()
                    }
                );
            }
            frame.insert(symbol.clone(), node);
            surface.push(node);
            let _name = self.agent_mut().define_to(impl_env, Some(symbol.into()))?;
            // TODO(feat) Bring back when we have multi-env triples.
            // // Unlike amlang designation, label predicate must be imported.
            // let raw_predicate = amlang_node!(label, self.agent().context());
            // let label_predicate = self.agent_mut().import(raw_predicate)?;
            // self.agent_mut().tell(node, label_predicate, name)?;
        }

        self.state.eval_state.push(frame);
        let res = (|| {
            let mut body_nodes = vec![];
            for (elem, proper) in body.into_iter() {
                if !proper {
                    return err!(self.agent(), LangError::InvalidSexp(*elem));
                }
                let eval = self.interpret(*elem)?;
                let node = self.node_or_insert(eval)?;
                body_nodes.push(node);
            }

            let body = if body_nodes.len() == 1 {
                body_nodes[0]
            } else {
                self.agent_mut()
                    .define_to(impl_env, Some(Procedure::Sequence(body_nodes).into()))?
            };

            if reflect {
                Ok(Procedure::InterpreterAbstraction(surface, body))
            } else {
                Ok(Procedure::UserAbstraction(surface, body))
            }
        })();
        let frame = self.state.eval_state.pop().unwrap();
        Ok((res?, frame))
    }

    // If we need Nodes in a particular context, we must abstract existing
    // Sexps into the env. However, if the sexp is already a Node, just use it
    // directly rather than create a stack of abstractions.
    fn node_or_insert(&mut self, sexp: Sexp) -> Result<Node, Error> {
        if let Ok(node) = <Node>::try_from(&sexp) {
            Ok(node)
        } else {
            let env = self.state.impl_env;
            self.agent_mut().define_to(env, Some(sexp))
        }
    }

    fn evlis(
        &mut self,
        structures: Option<HeapSexp>,
        should_interpret: bool,
    ) -> Result<Vec<Node>, Error> {
        if structures.is_none() {
            return Ok(vec![]);
        }

        // TODO(perf) Return Cow.
        let s = structures.unwrap();
        let mut args = Vec::<Node>::with_capacity(s.iter().count());
        for (structure, proper) in s.into_iter() {
            if !proper {
                return err!(self.agent(), LangError::InvalidSexp(*structure));
            }

            let arg_node = if should_interpret {
                let val = self.interpret(*structure)?;
                self.node_or_insert(val)?
            } else {
                let env = self.state.impl_env;
                self.agent_mut().define_to(env, Some(*structure))?
            };
            args.push(arg_node);
        }
        Ok(args)
    }

    fn evlis_def(
        &mut self,
        structures: Option<HeapSexp>,
        first_interface: bool,
    ) -> Result<Vec<Node>, Error> {
        if structures.is_none() {
            return Ok(vec![]);
        }

        // TODO(perf) Return Cow.
        let s = structures.unwrap();
        let mut args = Vec::<Node>::with_capacity(s.iter().count());
        for (i, (structure, proper)) in s.into_iter().enumerate() {
            if !proper {
                return err!(self.agent(), LangError::InvalidSexp(*structure));
            }

            let arg_node = if i == 0 && first_interface {
                self.agent_mut().define(Some(*structure))?
            } else {
                let env = self.state.impl_env;
                self.agent_mut().define_to(env, Some(*structure))?
            };
            args.push(arg_node);
        }
        Ok(args)
    }
}

impl<'a> Interpreter for ExecutingInterpreter<'a> {
    fn interpret(&mut self, structure: Sexp) -> Result<Sexp, Error> {
        debug!("Interpreting: {}", structure);
        match structure {
            Sexp::Primitive(primitive) => {
                if let Primitive::Symbol(symbol) = &primitive {
                    for frame in self.state.eval_state.iter() {
                        if let Some(node) = frame.lookup(symbol) {
                            return Ok(node.into());
                        }
                    }
                }
                return self.agent_mut().designate(primitive);
            }

            Sexp::Cons(cons) => {
                let (car, cdr) = cons.consume();
                let car = match car {
                    Some(car) => car,
                    None => {
                        return err!(
                            self.agent(),
                            LangError::InvalidSexp(Cons::new(car, cdr).into())
                        );
                    }
                };

                let eval_car = self.interpret(*car)?;
                let node = match eval_car {
                    Sexp::Primitive(Primitive::Procedure(_))
                    | Sexp::Primitive(Primitive::Node(_)) => self.node_or_insert(eval_car)?,
                    _ => {
                        return err!(
                            self.agent(),
                            LangError::InvalidArgument {
                                given: Cons::new(eval_car, cdr).into(),
                                expected: "special form or Procedure application".into(),
                            }
                        );
                    }
                };
                let context = self.agent().context();
                match node {
                    _ if amlang_node!(quote, context) == node => {
                        return Ok(*quote_wrapper(cdr, self.agent())?);
                    }
                    _ if amlang_node!(lambda, context) == node
                        || amlang_node!(fexpr, context) == node =>
                    {
                        let (params, body) = make_lambda_wrapper(cdr, &self.agent())?;
                        let reflect = node.local() == context.fexpr();
                        let (proc, _) = self.make_lambda(params, body, reflect)?;
                        return Ok(proc.into());
                    }
                    _ if amlang_node!(let_basic, context) == node
                        || amlang_node!(let_rec, context) == node =>
                    {
                        let (params, exprs, body) = let_wrapper(cdr, &self.agent())?;
                        let recursive = node.local() == context.let_rec();
                        let (proc, frame) = self.make_lambda(params, body, false)?;
                        let proc_node = self.node_or_insert(proc.into())?;

                        let args = if recursive {
                            self.state.eval_state.push(frame);
                            let res = self.evlis(Some(exprs), true);
                            self.state.eval_state.pop();
                            res?
                        } else {
                            self.evlis(Some(exprs), true)?
                        };
                        return Ok(Procedure::Application(proc_node, args).into());
                    }
                    _ if amlang_node!(branch, context) == node => {
                        let args = self.evlis(cdr, true)?;
                        if args.len() != 3 {
                            return err!(
                                self.agent(),
                                LangError::WrongArgumentCount {
                                    given: args.len(),
                                    expected: ExpectedCount::Exactly(3),
                                }
                            );
                        }
                        let proc = Procedure::Branch((args[0], args[1], args[2]).into());
                        return Ok(proc.into());
                    }
                    _ if amlang_node!(progn, context) == node => {
                        let args = self.evlis(cdr, true)?;
                        return Ok(Procedure::Sequence(args).into());
                    }
                    _ if amlang_node!(def, context) == node
                        || amlang_node!(node, context) == node =>
                    {
                        let args = self.evlis_def(cdr, node == amlang_node!(def, context))?;
                        return Ok(Procedure::Application(node, args).into());
                    }
                    _ => {
                        let should_interpret = match self.agent_mut().designate(node.into())? {
                            Sexp::Primitive(Primitive::Procedure(
                                Procedure::InterpreterAbstraction(_, _),
                            )) => false,
                            _ => true,
                        };
                        let args = self.evlis(cdr, should_interpret)?;
                        return Ok(Procedure::Application(node, args).into());
                    }
                }
            }
        }
    }
}
