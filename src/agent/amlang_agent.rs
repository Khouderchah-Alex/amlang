use log::debug;
use std::borrow::Cow;
use std::convert::TryFrom;
use std::iter::Peekable;

use super::agent::Agent;
use super::agent_state::{AgentState, ExecFrame};
use super::amlang_wrappers::*;
use crate::environment::LocalNode;
use crate::lang_err::{ExpectedCount, LangErr};
use crate::model::{Eval, Model, Ret};
use crate::parser::{parse_sexp, ParseError};
use crate::primitive::{AmString, Node, Number, Primitive, Procedure, Symbol, SymbolTable};
use crate::sexp::{Cons, HeapSexp, Sexp, SexpIntoIter};
use crate::token::TokenInfo;


#[derive(Clone)]
pub struct AmlangAgent {
    state: AgentState,
    history_env: LocalNode,
    eval_symbols: SymbolTable,
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
    CompileError(LangErr),
    ExecError(LangErr),
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
            eval_symbols: SymbolTable::default(),
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


    fn make_lambda(&mut self, params: Vec<Symbol>, body: Sexp) -> Result<Procedure, LangErr> {
        let mut surface = Vec::new();
        for symbol in params {
            let node = self.state_mut().env().insert_atom().globalize(self.state());
            // TODO(func) Use actual deep environment representation (including popping off).
            self.eval_symbols.insert(symbol, node);
            surface.push(node);
        }

        let cons = match body {
            Sexp::Primitive(primitive) => {
                return err!(InvalidSexp(primitive.clone().into()));
            }
            Sexp::Cons(cons) => cons,
        };
        // TODO(func) Allow for sequence.
        let body_eval = self.eval(Box::new(cons.car().unwrap().clone()))?;
        let body_node = self
            .state_mut()
            .env()
            .insert_structure(body_eval)
            .globalize(self.state());
        Ok(Procedure::Abstraction(surface, body_node))
    }

    fn exec(&mut self, meaning_node: Node) -> Ret {
        let meaning = self.state_mut().designate(Primitive::Node(meaning_node))?;
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
                        if cond != Number::Integer(1).into() {
                            return Ok(self.exec(cb)?);
                        }
                        Ok(self.exec(ca)?)
                    }
                    lambda @ Procedure::Abstraction(..) => Ok(lambda.into()),
                    _ => panic!("Unsupported procedure type: {:?}", proc),
                }
            }
            _ => Ok(meaning),
        }
    }

    fn apply(&mut self, proc_node: Node, arg_nodes: Vec<Node>) -> Ret {
        match self.state_mut().designate(Primitive::Node(proc_node))? {
            Sexp::Primitive(Primitive::Node(node)) => {
                if node.env() == self.state().context().lang_env() {
                    self.apply_special(node.local(), arg_nodes)
                } else {
                    err_ctx!(
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

                builtin.call(args, &self.state())
            }
            Sexp::Primitive(Primitive::Procedure(Procedure::Abstraction(params, body_node))) => {
                if arg_nodes.len() != params.len() {
                    return err_ctx!(
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
            not_proc @ _ => err_ctx!(
                self.state(),
                InvalidArgument {
                    given: not_proc.clone(),
                    expected: Cow::Borrowed("Procedure"),
                }
            ),
        }
    }

    fn exec_to_node(&mut self, node: Node) -> Result<Node, LangErr> {
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
                self.state_mut().designate(Primitive::Node(name))?;
                if name.env() != self.state().pos().env() {
                    panic!("Cross-env triples are not yet supported");
                }

                let val_node = if let Some(s) = val {
                    let val = self.exec(s)?;
                    self.state_mut().env().insert_structure(val)
                } else {
                    self.state_mut().env().insert_atom()
                };
                return Ok(self.state_mut().name_node(name.local(), val_node)?.into());
            }
            _ if context.curr == special_node => {
                if arg_nodes.len() > 0 {
                    return err_ctx!(
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
                    return err_ctx!(
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
                    return err_ctx!(
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
                    return err_ctx!(
                        self.state(),
                        WrongArgumentCount {
                            given: arg_nodes.len(),
                            expected: ExpectedCount::Exactly(1),
                        }
                    );
                }

                let des = self.state_mut().designate(Primitive::Node(arg_nodes[0]))?;
                let path = match <&AmString>::try_from(&des) {
                    Ok(path) => path,
                    _ => {
                        return err_ctx!(
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
                let proc_sexp = self.exec(proc_node)?;
                let args_sexp = self.exec(args_node)?;
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
                for arg in SexpIntoIter::try_from(args_sexp)? {
                    if let Ok(node) = Node::try_from(&*arg) {
                        args.push(node);
                    } else {
                        args.push(
                            self.state_mut()
                                .env()
                                .insert_structure(arg.into())
                                .globalize(self.state()),
                        );
                    }
                }

                return self.apply(proc, args);
            }
            _ if context.eval == special_node || context.exec == special_node => {
                if arg_nodes.len() != 1 {
                    return err_ctx!(
                        self.state(),
                        WrongArgumentCount {
                            given: arg_nodes.len(),
                            expected: ExpectedCount::Exactly(1),
                        }
                    );
                }
                let is_eval = context.eval == special_node;
                let arg = self.exec(arg_nodes[0])?;
                if is_eval {
                    debug!("applying (eval {})", arg);
                    self.eval(HeapSexp::new(arg))
                } else {
                    debug!("applying (exec {})", arg);
                    let meaning_node = self.history_insert(arg.into());
                    self.exec(meaning_node)
                }
            }
            _ => err_ctx!(
                self.state(),
                InvalidArgument {
                    given: Node::new(self.state().context().lang_env(), special_node).into(),
                    expected: Cow::Borrowed("special Amlang Node"),
                }
            ),
        }
    }

    fn evlis(&mut self, structures: Option<HeapSexp>) -> Result<Vec<Node>, LangErr> {
        if structures.is_none() {
            return Ok(vec![]);
        }

        return match *structures.unwrap() {
            Sexp::Primitive(primitive) => err!(InvalidSexp(primitive.clone().into())),

            Sexp::Cons(cons) => {
                // TODO(perf) Return Cow.
                let mut args = Vec::<Node>::with_capacity(cons.iter().count());
                for structure in cons.into_iter() {
                    let val = self.eval(structure)?;
                    // Don't recreate existing Nodes.
                    if let Ok(node) = <Node>::try_from(&val) {
                        args.push(node.into());
                    } else {
                        args.push(
                            self.state_mut()
                                .env()
                                .insert_structure(val)
                                .globalize(self.state()),
                        );
                    }
                }
                Ok(args)
            }
        };
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
                    if let Some(node) = self.eval_symbols.lookup(symbol) {
                        return Ok(node.into());
                    }
                }
                return self.state_mut().designate(primitive);
            }

            Sexp::Cons(cons) => {
                let (car, cdr) = cons.consume();
                let car = match car {
                    Some(car) => car,
                    None => return err!(InvalidSexp(Cons::new(car, cdr).into())),
                };

                let eval_car = self.eval(car)?;
                if let Ok(node) = <Node>::try_from(&eval_car) {
                    let context = self.state().context();
                    match node {
                        _ if Node::new(context.lang_env(), context.quote) == node => {
                            return quote_wrapper(cdr);
                        }
                        _ if Node::new(context.lang_env(), context.lambda) == node => {
                            let (params, body) = make_lambda_wrapper(cdr, &self.state())?;
                            let proc = self.make_lambda(params, body)?;
                            return Ok(self
                                .state_mut()
                                .env()
                                .insert_structure(proc.into())
                                .globalize(self.state())
                                .into());
                        }
                        _ if Node::new(context.lang_env(), context.branch) == node => {
                            let args = self.evlis(cdr)?;
                            if args.len() != 3 {
                                return err!(WrongArgumentCount {
                                    given: args.len(),
                                    expected: ExpectedCount::Exactly(3),
                                });
                            }

                            let proc = Procedure::Branch(args[0], args[1], args[2]);
                            return Ok(proc.into());
                        }
                        _ => {
                            let args = self.evlis(cdr)?;
                            return Ok(Procedure::Application(node, args).into());
                        }
                    }
                }
                return err!(InvalidArgument {
                    given: Cons::new(Some(Box::new(eval_car)), cdr).into(),
                    expected: Cow::Borrowed("special form or functional application"),
                });
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
    pub fn trace_error(&mut self, err: &LangErr) {
        if let Some(state) = err.state() {
            let mut stored_state = state.clone();
            std::mem::swap(self.state_mut(), &mut stored_state);
            println!("");
            println!("  --TRACE--");
            for (i, frame) in state.exec_state().iter().enumerate() {
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
            let structure = triple.generate_structure(self.state_mut());
            self.state_mut().print_list(&structure);
            println!("");
        }
    }
}
