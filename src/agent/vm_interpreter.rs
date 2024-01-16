use log::debug;
use std::convert::TryFrom;

use super::agent_frames::ExecFrame;
use super::amlang_wrappers::*;
use super::interpreter::{Interpreter, InterpreterState};
use super::Agent;
use crate::agent::lang_error::{ExpectedCount, LangError};
use crate::env::LocalNode;
use crate::error::Error;
use crate::primitive::prelude::*;
use crate::primitive::table::Table;
use crate::sexp::{ConsList, HeapSexp, Sexp};


#[derive(Debug)]
pub struct VmInterpreter {
    history_env: LocalNode,
    impl_env: LocalNode,
}

impl VmInterpreter {
    pub fn new(history_env: LocalNode, impl_env: LocalNode) -> Self {
        Self {
            history_env,
            impl_env,
        }
    }
}

impl InterpreterState for VmInterpreter {
    fn borrow_agent<'a>(&'a mut self, agent: &'a mut Agent) -> Box<dyn Interpreter + 'a> {
        Box::new(ExecutingInterpreter::from_state(self, agent))
    }
}


struct ExecutingInterpreter<'a> {
    state: &'a mut VmInterpreter,
    agent: &'a mut Agent,
}

impl<'a> ExecutingInterpreter<'a> {
    fn from_state(state: &'a mut VmInterpreter, agent: &'a mut Agent) -> Self {
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

    // TODO(func) Do we need this to be a Node over Sexp?
    fn exec(&mut self, meaning_node: Node) -> Result<Sexp, Error> {
        let meaning = self.agent_mut().concretize(meaning_node)?;
        match meaning {
            Sexp::Primitive(Primitive::Procedure(proc)) => {
                match proc {
                    Procedure::Application(proc_node, arg_nodes) => {
                        let frame = ExecFrame::new(meaning_node);
                        debug!("exec_state push: {}", meaning_node);
                        self.agent_mut().exec_state_mut().push(frame);

                        let res = self.apply(proc_node, arg_nodes);

                        debug!("exec_state pop: {}", meaning_node);
                        self.agent_mut().exec_state_mut().pop();
                        res
                    }
                    Procedure::Branch(t) => {
                        let (pred, a, b) = *t;
                        let cond = self.exec(pred)?;

                        // TODO(func) Integrate actual boolean type.
                        let context = self.agent().context();
                        if cond == amlang_node!(t, context).into() {
                            Ok(self.exec(a)?)
                        } else if cond == amlang_node!(f, context).into() {
                            Ok(self.exec(b)?)
                        } else {
                            err!(
                                self.agent(),
                                LangError::InvalidArgument {
                                    given: cond,
                                    expected: "true or false Node".into(),
                                }
                            )
                        }
                    }
                    Procedure::Sequence(seq) => {
                        let mut result = Default::default();
                        for elem in seq {
                            result = self.exec(elem)?;
                        }
                        Ok(result)
                    }
                    lambda @ Procedure::Abstraction(..) => Ok(lambda.into()),
                }
            }
            _ => Ok(meaning),
        }
    }

    fn apply(&mut self, proc_node: Node, arg_nodes: Vec<Node>) -> Result<Sexp, Error> {
        match self.agent_mut().concretize(proc_node)? {
            Sexp::Primitive(Primitive::Node(node)) => {
                if node.env() == self.agent().context().lang_env() {
                    self.apply_special(node.local(), arg_nodes)
                } else {
                    err!(
                        self.agent(),
                        LangError::InvalidArgument {
                            given: node.into(),
                            expected: "Procedure or special Amlang Node".into(),
                        }
                    )
                }
            }
            Sexp::Primitive(Primitive::BuiltIn(builtin)) => {
                let mut args = ConsList::default();
                for node in arg_nodes {
                    args.append(self.exec(node)?);
                }
                builtin.call(args.release(), self.agent_mut())
            }
            Sexp::Primitive(Primitive::Procedure(Procedure::Abstraction(params, body_node, _))) => {
                if arg_nodes.len() != params.len() {
                    return err!(
                        self.agent(),
                        LangError::WrongArgumentCount {
                            given: arg_nodes.len(),
                            // TODO(func) support variable arity.
                            expected: ExpectedCount::Exactly(params.len()),
                        }
                    );
                }

                for (i, node) in arg_nodes.into_iter().enumerate() {
                    let val = self.exec(node)?;
                    let frame = self.agent_mut().exec_state_mut().top_mut();
                    frame.insert(params[i], val);
                    debug!("exec_state insert: {} -> {}", params[i], node);
                }

                self.exec(body_node)
            }
            not_proc @ _ => err!(
                self.agent(),
                LangError::InvalidArgument {
                    given: not_proc.clone(),
                    expected: "Procedure".into(),
                }
            ),
        }
    }

    fn exec_to_node(&mut self, node: Node) -> Result<Node, Error> {
        let structure = self.exec(node)?;
        if let Ok(new_node) = Node::try_from(structure) {
            Ok(new_node)
        } else {
            Ok(node)
        }
    }

    fn apply_special(
        &mut self,
        special_node: LocalNode,
        arg_nodes: Vec<Node>,
    ) -> Result<Sexp, Error> {
        let context = self.agent().context();
        match special_node {
            _ if context.tell() == special_node || context.ask() == special_node => {
                let is_tell = context.tell() == special_node;
                let (ss, pp, oo) = tell_wrapper(&arg_nodes, &self.agent())?;
                let (s, p, o) = (
                    self.exec_to_node(ss)?,
                    self.exec_to_node(pp)?,
                    self.exec_to_node(oo)?,
                );
                debug!(
                    "({} {} {} {})",
                    if is_tell { "tell" } else { "ask" },
                    s,
                    p,
                    o
                );
                if is_tell {
                    Ok(self.agent_mut().tell(s, p, o)?.into())
                } else {
                    let resolve_placeholder = |node: Node| {
                        if node == amlang_node!(placeholder, self.agent().context()) {
                            None
                        } else {
                            Some(node)
                        }
                    };
                    let (s, p, o) = (
                        resolve_placeholder(s),
                        resolve_placeholder(p),
                        resolve_placeholder(o),
                    );
                    Ok(self
                        .agent_mut()
                        .ask(s, p, o)?
                        .triples()
                        .map(|t| t.node().globalize(self.agent()).into())
                        .collect::<Vec<Sexp>>()
                        .into())
                }
            }
            _ if context.def() == special_node || context.node() == special_node => {
                let interpreter_context = amlang_node!(def, context);
                let is_named = special_node == context.def();
                let (name, val) = if is_named {
                    let (name, val) = def_wrapper(&arg_nodes, &self.agent())?;
                    if name.env() != self.agent().pos().env() {
                        panic!("Cross-env triples are not yet supported");
                    }
                    (name, val)
                } else {
                    let val = defa_wrapper(&arg_nodes, &self.agent())?;
                    (Node::new(context.lang_env(), context.anon()), val)
                };

                let mut val_node = if let Some(s) = val {
                    // Ensure internalize maps name to this node.
                    let val_node = self.agent_mut().define(None)?;

                    // Interpret value, relying on self-evaluation of val_node.
                    let sexp = self.agent_mut().designate(Primitive::Node(s))?;
                    let mut frame = SymNodeTable::default();
                    if let Ok(sym) = Symbol::try_from(self.agent_mut().designate(name.into())?) {
                        frame.insert(sym, val_node);
                    }
                    let final_sexp = self.eval(sexp, Some(frame), interpreter_context)?;

                    // If final result is a Node, we name that rather
                    // than nesting abstractions. Perhaps nested
                    // abstraction is right here?
                    //
                    // TODO(func) Either nest abstractions or somehow
                    // garbage-mark/free the unused atom.
                    if let Ok(node) = <Node>::try_from(&final_sexp) {
                        node
                    } else {
                        self.agent_mut().set(val_node, Some(final_sexp))?;
                        val_node
                    }
                } else {
                    self.agent_mut().define(None)?
                };

                if is_named {
                    val_node = self.agent_mut().name_node(name, val_node)?;
                }
                Ok(val_node.into())
            }
            _ if context.set() == special_node => {
                // Note that unlike def, set! follows normal internalization during evlis.
                let (node, val) = def_wrapper(&arg_nodes, &self.agent())?;
                let node = Node::try_from(node).unwrap();
                let interpreter_context = amlang_node!(set, context);
                if let Some(s) = val {
                    let final_sexp = self.eval(s.into(), None, interpreter_context)?;
                    self.agent_mut().set(node, Some(final_sexp))?;
                } else {
                    self.agent_mut().set(node, None)?;
                }
                Ok(node.into())
            }
            _ if context.import() == special_node => {
                if arg_nodes.len() != 1 {
                    return err!(
                        self.agent(),
                        LangError::WrongArgumentCount {
                            given: arg_nodes.len(),
                            expected: ExpectedCount::Exactly(1),
                        }
                    );
                }

                // If original expr internalize + exec -> Node, use that.
                let original = self.exec_to_node(arg_nodes[0])?;
                let imported = self.agent_mut().import(original)?;
                Ok(imported.into())
            }
            _ if context.apply() == special_node => {
                let (proc_node, args_node) = apply_wrapper(&arg_nodes, &self.agent())?;
                let proc_sexp = self.agent_mut().designate(proc_node.into())?;
                let args_sexp = self.agent_mut().designate(args_node.into())?;
                debug!("applying (apply {} '{})", proc_sexp, args_sexp);

                let proc = self.node_or_insert(proc_sexp)?;
                let mut args = Vec::new();
                for (arg, proper) in HeapSexp::new(args_sexp).into_iter() {
                    if !proper {
                        return err!(self.agent(), LangError::InvalidSexp(*arg));
                    }
                    args.push(self.node_or_insert(*arg)?);
                }

                return self.apply(proc, args);
            }
            _ if context.eval() == special_node || context.exec() == special_node => {
                if arg_nodes.len() != 1 {
                    return err!(
                        self.agent(),
                        LangError::WrongArgumentCount {
                            given: arg_nodes.len(),
                            expected: ExpectedCount::Exactly(1),
                        }
                    );
                }
                let is_eval = context.eval() == special_node;
                let interpreter_context = amlang_node!(eval, context);
                let arg = self.agent_mut().designate(arg_nodes[0].into())?;
                if is_eval {
                    let to_inner = self.interpret(arg)?;
                    let evaled = self.eval(to_inner, None, interpreter_context)?;
                    let evaled_node = self.node_or_insert(evaled)?;
                    self.exec(evaled_node)
                } else {
                    self.interpret(arg)
                }
            }
            _ => err!(
                self.agent(),
                LangError::InvalidArgument {
                    given: Node::new(self.agent().context().lang_env(), special_node).into(),
                    expected: "special Amlang Node".into(),
                }
            ),
        }
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

    fn eval(
        &mut self,
        sexp: Sexp,
        frame: Option<SymNodeTable>,
        context: Node,
    ) -> Result<Sexp, Error> {
        let sub_interpreter = self.agent().gen_eval_interpreter(frame)?;
        let evaled = self
            .agent_mut()
            .sub_interpret(sexp, sub_interpreter, context)?;
        let evaled_node = self.node_or_insert(evaled)?;
        self.exec(evaled_node)
    }
}

impl<'a> Interpreter for ExecutingInterpreter<'a> {
    fn interpret(&mut self, structure: Sexp) -> Result<Sexp, Error> {
        debug!("Interpreting: {}", structure);
        let node = if let Ok(node) = <Node>::try_from(&structure) {
            node
        } else {
            let env = self.state.history_env;
            self.agent_mut().define_to(env, Some(structure))?
        };
        self.exec(node)
    }
}
