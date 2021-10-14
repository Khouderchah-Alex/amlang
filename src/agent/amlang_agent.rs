use log::debug;
use std::borrow::Cow;
use std::convert::TryFrom;
use std::iter::Peekable;

use super::agent::Agent;
use super::agent_state::{AgentState, ExecFrame};
use super::amlang_wrappers::*;
use super::continuation::Continuation;
use crate::environment::LocalNode;
use crate::primitive::error::{ExpectedCount, Error};
use crate::model::{Eval, Model, Ret};
use crate::parser::{parse_sexp, ParseError};
use crate::primitive::prelude::*;
use crate::primitive::table::Table;
use crate::sexp::{Cons, HeapSexp, Sexp};
use crate::token::TokenInfo;


#[derive(Clone)]
pub struct AmlangAgent {
    state: AgentState,
    eval_state: Continuation<SymbolTable>,

    history_env: LocalNode,
}

pub struct RunIter<'a, S, F>
where
    S: Iterator<Item = TokenInfo>,
    F: FnMut(&mut AmlangAgent, &Result<Sexp, RunError>),
{
    agent: &'a mut AmlangAgent,
    stream: Peekable<S>,
    handler: F,
}

#[derive(Debug)]
pub enum RunError {
    ParseError(ParseError),
    CompileError(Error),
    ExecError(Error),
}

impl AmlangAgent {
    pub fn from_state(mut state: AgentState, history_env: LocalNode) -> Self {
        // Ensure agent state designates amlang nodes first.
        let lang_env = state.context().lang_env();
        state.designation_chain_mut().push_front(lang_env);

        Self {
            state,
            // TODO(sec) Verify as env node.
            history_env,
            eval_state: Continuation::new(SymbolTable::default()),
        }
    }

    pub fn run<'a, S, F>(&'a mut self, stream: S, handler: F) -> RunIter<'a, S, F>
    where
        S: Iterator<Item = TokenInfo>,
        F: FnMut(&mut AmlangAgent, &Result<Sexp, RunError>),
    {
        RunIter {
            agent: self,
            stream: stream.peekable(),
            handler,
        }
    }


    fn make_lambda(
        &mut self,
        params: Vec<Symbol>,
        body: HeapSexp,
        reflect: bool,
    ) -> Result<Procedure, Error> {
        let mut surface = Vec::new();
        let mut frame = SymbolTable::default();
        for symbol in params {
            let node = self.state_mut().env().insert_atom().globalize(self.state());
            if frame.contains_key(&symbol) {
                return err!(
                    self.state(),
                    InvalidArgument {
                        given: symbol.into(),
                        expected: Cow::Borrowed("unique name within argument list")
                    }
                );
            }
            frame.insert(symbol, node);
            surface.push(node);
        }

        self.eval_state.push(frame);
        let res = (|| {
            let mut body_nodes = vec![];
            for (elem, proper) in body.into_iter() {
                if !proper {
                    return err!(self.state(), InvalidSexp(*elem));
                }
                let eval = self.eval(elem)?;
                let node = self
                    .state_mut()
                    .env()
                    .insert_structure(eval)
                    .globalize(self.state());
                body_nodes.push(node);
            }

            if body_nodes.len() == 1 {
                Ok(Procedure::Abstraction(surface, body_nodes[0], reflect))
            } else {
                let seq_node = self
                    .state_mut()
                    .env()
                    .insert_structure(Procedure::Sequence(body_nodes).into())
                    .globalize(self.state());
                Ok(Procedure::Abstraction(surface, seq_node, reflect))
            }
        })();
        self.eval_state.pop();
        res
    }

    fn exec(&mut self, meaning_node: Node) -> Ret {
        let meaning = self.state_mut().designate(meaning_node.into())?;
        match meaning {
            Sexp::Primitive(Primitive::Procedure(proc)) => {
                match proc {
                    Procedure::Application(proc_node, arg_nodes) => {
                        let frame = ExecFrame::new(meaning_node);
                        self.state_mut().exec_state_mut().push(frame);

                        let res = (|| {
                            let concretized_nodes = arg_nodes
                                .into_iter()
                                .map(|n| self.state().concretize(n))
                                .collect::<Vec<_>>();
                            let cproc = self.state().concretize(proc_node);
                            self.apply(cproc, concretized_nodes)
                        })();

                        self.state_mut().exec_state_mut().pop();
                        res
                    }
                    Procedure::Branch(pred, a, b) => {
                        let cpred = self.state().concretize(pred);
                        let ca = self.state().concretize(a);
                        let cb = self.state().concretize(b);

                        let cond = self.exec(cpred)?;
                        // TODO(func) Integrate actual boolean type.
                        let context = self.state().context();
                        if cond == Node::new(context.lang_env(), context.t).into() {
                            Ok(self.exec(ca)?)
                        } else if cond == Node::new(context.lang_env(), context.f).into() {
                            Ok(self.exec(cb)?)
                        } else {
                            err!(
                                self.state(),
                                InvalidArgument {
                                    given: cond,
                                    expected: Cow::Borrowed("true or false Node"),
                                }
                            )
                        }
                    }
                    Procedure::Sequence(seq) => {
                        let mut result = Sexp::default();
                        for elem in seq {
                            let concrete = self.state().concretize(elem);
                            result = self.exec(concrete)?;
                        }
                        Ok(result)
                    }
                    lambda @ Procedure::Abstraction(..) => Ok(lambda.into()),
                }
            }
            _ => Ok(meaning),
        }
    }

    fn apply(&mut self, proc_node: Node, arg_nodes: Vec<Node>) -> Ret {
        match self.state_mut().designate(proc_node.into())? {
            Sexp::Primitive(Primitive::Node(node)) => {
                if node.env() == self.state().context().lang_env() {
                    self.apply_special(node.local(), arg_nodes)
                } else {
                    err!(
                        self.state(),
                        InvalidArgument {
                            given: node.into(),
                            expected: Cow::Borrowed("Procedure or special Amlang Node"),
                        }
                    )
                }
            }
            Sexp::Primitive(Primitive::BuiltIn(builtin)) => {
                let mut args = Vec::with_capacity(arg_nodes.len());
                for node in arg_nodes {
                    args.push(self.exec(node)?);
                }

                builtin.call(args, self.state_mut())
            }
            Sexp::Primitive(Primitive::Procedure(Procedure::Abstraction(params, body_node, _))) => {
                if arg_nodes.len() != params.len() {
                    return err!(
                        self.state(),
                        WrongArgumentCount {
                            given: arg_nodes.len(),
                            // TODO(func) support variable arity.
                            expected: ExpectedCount::Exactly(params.len()),
                        }
                    );
                }

                let mut args = Vec::with_capacity(arg_nodes.len());
                for (i, node) in arg_nodes.into_iter().enumerate() {
                    args.push(self.exec(node)?);

                    let frame = self.state_mut().exec_state_mut().top_mut();
                    frame.insert(params[i], node);
                    debug!("exec_state insert: {} -> {}", params[i], node);
                }

                let body = self.exec(body_node)?;
                if let Ok(node) = <Node>::try_from(&body) {
                    Ok(self.state().concretize(node).into())
                } else {
                    Ok(body)
                }
            }
            not_proc @ _ => err!(
                self.state(),
                InvalidArgument {
                    given: not_proc.clone(),
                    expected: Cow::Borrowed("Procedure"),
                }
            ),
        }
    }

    fn exec_to_node(&mut self, node: Node) -> Result<Node, Error> {
        let structure = self.exec(node)?;
        if let Ok(new_node) = Node::try_from(&structure) {
            Ok(new_node)
        } else {
            Ok(node)
        }
    }

    fn apply_special(&mut self, special_node: LocalNode, arg_nodes: Vec<Node>) -> Ret {
        let context = self.state().context();
        match special_node {
            _ if context.tell == special_node || context.ask == special_node => {
                let is_tell = context.tell == special_node;
                let (ss, pp, oo) = tell_wrapper(&arg_nodes, &self.state())?;

                // TODO(func) Add support for cross-env triples through surrogates.
                let mut to_local = |node: Node| {
                    let placeholder = Node::new(
                        self.state().context().lang_env(),
                        self.state().context().placeholder,
                    );
                    let final_node = self.exec_to_node(node)?;
                    if (is_tell || final_node != placeholder)
                        && final_node.env() != self.state().pos().env()
                    {
                        panic!("Cross-env triples are not yet supported");
                    }
                    Ok(final_node.local())
                };
                let (s, p, o) = (to_local(ss)?, to_local(pp)?, to_local(oo)?);
                debug!(
                    "({} {} {} {})",
                    if is_tell { "tell" } else { "ask" },
                    s,
                    p,
                    o
                );
                if is_tell {
                    self.state_mut().tell(s, p, o)
                } else {
                    self.state_mut().ask(s, p, o)
                }
            }
            _ if context.def == special_node => {
                let (name, val) = def_wrapper(&arg_nodes, &self.state())?;
                self.state_mut().designate(name.into())?;
                if name.env() != self.state().pos().env() {
                    panic!("Cross-env triples are not yet supported");
                }

                let val_node = if let Some(s) = val {
                    let original = self.state_mut().designate(s.into())?;
                    let meaning = self.eval(original.into())?;
                    let meaning_node = self.history_insert(meaning);
                    let val = self.exec(meaning_node)?;
                    self.eval_to_node(val)?.local()
                } else {
                    self.state_mut().env().insert_atom()
                };
                return Ok(self.state_mut().name_node(name.local(), val_node)?.into());
            }
            _ if context.curr == special_node => {
                if arg_nodes.len() > 0 {
                    return err!(
                        self.state(),
                        WrongArgumentCount {
                            given: arg_nodes.len(),
                            expected: ExpectedCount::Exactly(0),
                        }
                    );
                }
                self.print_curr_triples();
                return Ok(self.state().pos().into());
            }
            _ if context.jump == special_node => {
                if arg_nodes.len() != 1 {
                    return err!(
                        self.state(),
                        WrongArgumentCount {
                            given: arg_nodes.len(),
                            expected: ExpectedCount::Exactly(1),
                        }
                    );
                }

                // If original expr eval + exec -> Node, use that.
                let dest = self.exec_to_node(arg_nodes[0])?;
                self.state_mut().jump(dest);
                self.print_curr_triples();
                return Ok(self.state().pos().into());
            }
            _ if context.import == special_node => {
                if arg_nodes.len() != 1 {
                    return err!(
                        self.state(),
                        WrongArgumentCount {
                            given: arg_nodes.len(),
                            expected: ExpectedCount::Exactly(1),
                        }
                    );
                }

                // If original expr eval + exec -> Node, use that.
                let original = self.exec_to_node(arg_nodes[0])?;
                let imported = self.state_mut().import(original)?;
                return Ok(imported.into());
            }
            _ if context.env_find == special_node => {
                if arg_nodes.len() != 1 {
                    return err!(
                        self.state(),
                        WrongArgumentCount {
                            given: arg_nodes.len(),
                            expected: ExpectedCount::Exactly(1),
                        }
                    );
                }

                let des = self.state_mut().designate(arg_nodes[0].into())?;
                let path = match <&AmString>::try_from(&des) {
                    Ok(path) => path,
                    _ => {
                        return err!(
                            self.state(),
                            InvalidArgument {
                                given: des.into(),
                                expected: Cow::Borrowed("Node containing string"),
                            }
                        );
                    }
                };

                if let Some(lnode) = self.state().find_env(path.as_str()) {
                    Ok(Node::new(LocalNode::default(), lnode).into())
                } else {
                    Ok(Sexp::default())
                }
            }
            _ if context.apply == special_node => {
                let (proc_node, args_node) = apply_wrapper(&arg_nodes, &self.state())?;
                let proc_sexp = self.state_mut().designate(proc_node.into())?;
                let args_sexp = self.state_mut().designate(args_node.into())?;
                debug!("applying (apply {} '{})", proc_sexp, args_sexp);

                let proc = if let Ok(node) = Node::try_from(&proc_sexp) {
                    node
                } else {
                    self.state_mut()
                        .env()
                        .insert_structure(proc_sexp)
                        .globalize(self.state())
                };
                let mut args = Vec::new();
                for (arg, proper) in Box::new(args_sexp).into_iter() {
                    if !proper {
                        return err!(self.state(), InvalidSexp(*arg));
                    }
                    if let Ok(node) = Node::try_from(&*arg) {
                        args.push(node);
                    } else {
                        args.push(
                            self.state_mut()
                                .env()
                                .insert_structure(*arg)
                                .globalize(self.state()),
                        );
                    }
                }

                return self.apply(proc, args);
            }
            _ if context.eval == special_node || context.exec == special_node => {
                if arg_nodes.len() != 1 {
                    return err!(
                        self.state(),
                        WrongArgumentCount {
                            given: arg_nodes.len(),
                            expected: ExpectedCount::Exactly(1),
                        }
                    );
                }
                let is_eval = context.eval == special_node;
                let arg = self.state_mut().designate(arg_nodes[0].into())?;
                if is_eval {
                    debug!("applying (eval {})", arg);
                    self.eval(HeapSexp::new(arg))
                } else {
                    debug!("applying (exec {})", arg);
                    let meaning_node = self.history_insert(arg.into());
                    self.exec(meaning_node)
                }
            }
            _ => err!(
                self.state(),
                InvalidArgument {
                    given: Node::new(self.state().context().lang_env(), special_node).into(),
                    expected: Cow::Borrowed("special Amlang Node"),
                }
            ),
        }
    }

    // If we need Nodes in a particular context, we must abstract existing
    // Sexps into the env. However, if the sexp is already a Node, just use it
    // directly rather than create a stack of abstractions.
    fn eval_to_node(&mut self, sexp: Sexp) -> Result<Node, Error> {
        if let Ok(node) = <Node>::try_from(&sexp) {
            Ok(node)
        } else {
            Ok(self
                .state_mut()
                .env()
                .insert_structure(sexp)
                .globalize(self.state()))
        }
    }

    fn evlis(
        &mut self,
        structures: Option<HeapSexp>,
        should_eval: bool,
    ) -> Result<Vec<Node>, Error> {
        if structures.is_none() {
            return Ok(vec![]);
        }

        // TODO(perf) Return Cow.
        let s = structures.unwrap();
        let mut args = Vec::<Node>::with_capacity(s.iter().count());
        for (structure, proper) in s.into_iter() {
            if !proper {
                return err!(self.state(), InvalidSexp(*structure));
            }

            if !should_eval {
                args.push(
                    self.state_mut()
                        .env()
                        .insert_structure(*structure)
                        .globalize(self.state()),
                );
                continue;
            }

            let val = self.eval(structure)?;
            args.push(self.eval_to_node(val)?);
        }
        Ok(args)
    }

    fn history_insert(&mut self, structure: Sexp) -> Node {
        let env = self.history_env;
        let node = self
            .state_mut()
            .access_env(env)
            .unwrap()
            .insert_structure(structure);
        Node::new(env, node)
    }
}


impl Agent for AmlangAgent {
    fn state(&self) -> &AgentState {
        &self.state
    }
    fn state_mut(&mut self) -> &mut AgentState {
        &mut self.state
    }
}

impl Eval for AmlangAgent {
    fn eval(&mut self, structure: HeapSexp) -> Ret {
        match *structure {
            Sexp::Primitive(primitive) => {
                if let Primitive::Symbol(symbol) = &primitive {
                    for frame in self.eval_state.iter() {
                        if let Some(node) = frame.lookup(symbol) {
                            return Ok(node.into());
                        }
                    }
                }
                return self.state_mut().designate(primitive);
            }

            Sexp::Cons(cons) => {
                let (car, cdr) = cons.consume();
                let car = match car {
                    Some(car) => car,
                    None => return err!(self.state(), InvalidSexp(Cons::new(car, cdr).into())),
                };

                let eval_car = self.eval(car)?;
                let node = match eval_car {
                    Sexp::Primitive(Primitive::Procedure(_))
                    | Sexp::Primitive(Primitive::Node(_)) => self.eval_to_node(eval_car)?,
                    _ => {
                        return err!(
                            self.state(),
                            InvalidArgument {
                                given: Cons::new(Some(Box::new(eval_car)), cdr).into(),
                                expected: Cow::Borrowed("special form or Procedure application"),
                            }
                        );
                    }
                };
                let context = self.state().context();
                match node {
                    _ if Node::new(context.lang_env(), context.quote) == node => {
                        return Ok(*quote_wrapper(cdr, self.state())?);
                    }
                    _ if Node::new(context.lang_env(), context.lambda) == node
                        || Node::new(context.lang_env(), context.fexpr) == node =>
                    {
                        let (params, body) = make_lambda_wrapper(cdr, &self.state())?;
                        let reflect = node.local() == context.fexpr;
                        let proc = self.make_lambda(params, body, reflect)?;
                        return Ok(proc.into());
                    }
                    _ if Node::new(context.lang_env(), context.branch) == node => {
                        let args = self.evlis(cdr, true)?;
                        if args.len() != 3 {
                            return err!(
                                self.state(),
                                WrongArgumentCount {
                                    given: args.len(),
                                    expected: ExpectedCount::Exactly(3),
                                }
                            );
                        }
                        let proc = Procedure::Branch(args[0], args[1], args[2]);
                        return Ok(proc.into());
                    }
                    _ if Node::new(context.lang_env(), context.progn) == node => {
                        let args = self.evlis(cdr, true)?;
                        return Ok(Procedure::Sequence(args).into());
                    }
                    _ => {
                        let def_node = Node::new(context.lang_env(), context.def);
                        let should_eval = match self.state_mut().designate(node.into())? {
                            // Don't evaluate args of reflective Abstractions.
                            Sexp::Primitive(Primitive::Procedure(Procedure::Abstraction(
                                _,
                                _,
                                true,
                            ))) => false,
                            _ => node != def_node,
                        };
                        let args = self.evlis(cdr, should_eval)?;
                        return Ok(Procedure::Application(node, args).into());
                    }
                }
            }
        }
    }
}

impl<'a, S, F> Iterator for RunIter<'a, S, F>
where
    S: Iterator<Item = TokenInfo>,
    F: FnMut(&mut AmlangAgent, &Result<Sexp, RunError>),
{
    type Item = Result<Sexp, RunError>;

    fn next(&mut self) -> Option<Self::Item> {
        let sexp = match parse_sexp(&mut self.stream, 0) {
            Ok(Some(parsed)) => parsed,
            Ok(None) => return None,
            Err(err) => {
                let res = Err(RunError::ParseError(err));
                (self.handler)(&mut self.agent, &res);
                return Some(res);
            }
        };

        let meaning = match self.agent.eval(sexp) {
            Ok(meaning) => meaning,
            Err(err) => {
                let res = Err(RunError::CompileError(err));
                (self.handler)(&mut self.agent, &res);
                return Some(res);
            }
        };

        let meaning_node = self.agent.history_insert(meaning);
        let res = match self.agent.exec(meaning_node) {
            Ok(val) => Ok(val),
            Err(err) => Err(RunError::ExecError(err)),
        };
        (self.handler)(&mut self.agent, &res);
        Some(res)
    }
}


impl AmlangAgent {
    pub fn trace_error(&mut self, err: &Error) {
        if let Some(state) = err.state() {
            let mut stored_state = state.clone();
            std::mem::swap(self.state_mut(), &mut stored_state);
            println!("");
            println!("  --TRACE--");
            let end = state.exec_state().depth() - 1;
            for (i, frame) in state.exec_state().iter().enumerate() {
                if i == end {
                    break;
                }
                self.state_mut().exec_state_mut().pop();
                print!("   {})  ", i);
                self.state_mut().print_list(&frame.context().into());
                println!("");
            }
            std::mem::swap(self.state_mut(), &mut stored_state);
        }
    }

    fn print_curr_triples(&mut self) {
        let local = self.state().pos().local();
        let triples = self.state_mut().env().match_any(local);
        for triple in triples {
            print!("    ");
            let structure = triple.reify(self.state_mut());
            self.state_mut().print_list(&structure);
            println!("");
        }
    }
}
