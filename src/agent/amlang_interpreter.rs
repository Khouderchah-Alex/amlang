use log::debug;
use std::convert::TryFrom;

use super::agent_frames::ExecFrame;
use super::amlang_wrappers::*;
use super::interpreter::{Interpreter, InterpreterState};
use super::Agent;
use crate::agent::lang_error::{ExpectedCount, LangError};
use crate::continuation::Continuation;
use crate::env::LocalNode;
use crate::error::Error;
use crate::model::Reflective;
use crate::primitive::prelude::*;
use crate::primitive::table::Table;
use crate::sexp::{Cons, ConsList, HeapSexp, Sexp};


#[derive(Debug)]
pub struct AmlangInterpreter {
    eval_state: Continuation<SymNodeTable>,
}

impl Default for AmlangInterpreter {
    fn default() -> Self {
        Self {
            eval_state: Continuation::new(SymNodeTable::default()),
        }
    }
}

impl InterpreterState for AmlangInterpreter {
    fn borrow_agent<'a>(&'a mut self, agent: &'a mut Agent) -> Box<dyn Interpreter + 'a> {
        Box::new(ExecutingInterpreter::from_state(self, agent))
    }
}


struct ExecutingInterpreter<'a> {
    eval_state: &'a mut Continuation<SymNodeTable>,
    agent: &'a mut Agent,
}

impl<'a> ExecutingInterpreter<'a> {
    fn from_state(state: &'a mut AmlangInterpreter, agent: &'a mut Agent) -> Self {
        // Ensure agent designates amlang nodes first.
        let lang_env = agent.context().lang_env();
        if agent.designation_chain().front().cloned() != Some(lang_env) {
            agent.designation_chain_mut().push_front(lang_env);
        }

        Self {
            eval_state: &mut state.eval_state,
            agent,
        }
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
        for symbol in params {
            let node = self.agent_mut().define(None)?;
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
            let name = self.agent_mut().define(Some(symbol.into()))?;
            // Unlike amlang designation, label predicate must be imported.
            let raw_predicate = amlang_node!(label, self.agent().context());
            let label_predicate = self.agent_mut().import(raw_predicate)?;
            self.agent_mut().tell(node, label_predicate, name)?;
        }

        self.eval_state.push(frame);
        let res = (|| {
            let mut body_nodes = vec![];
            for (elem, proper) in body.into_iter() {
                if !proper {
                    return err!(self.agent(), LangError::InvalidSexp(*elem));
                }
                let eval = self.internalize(*elem)?;
                let node = self.node_or_insert(eval)?;
                body_nodes.push(node);
            }

            if body_nodes.len() == 1 {
                Ok(Procedure::Abstraction(surface, body_nodes[0], reflect))
            } else {
                let seq_node = self
                    .agent_mut()
                    .define(Some(Procedure::Sequence(body_nodes).into()))?;
                Ok(Procedure::Abstraction(surface, seq_node, reflect))
            }
        })();
        let frame = self.eval_state.pop().unwrap();
        Ok((res?, frame))
    }

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

                    let mut frame = SymNodeTable::default();
                    if let Ok(sym) = Symbol::try_from(self.agent_mut().designate(name.into())?) {
                        frame.insert(sym, val_node);
                    }

                    // Interpret value, relying on self-evaluation of val_node.
                    let original = self.agent_mut().designate(Primitive::Node(s))?;
                    let mut sub_interpreter = Box::new(AmlangInterpreter::default());
                    sub_interpreter.eval_state.push(frame);
                    let final_sexp = self.agent_mut().sub_interpret(
                        original,
                        sub_interpreter,
                        interpreter_context,
                    )?;

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
                    let final_sexp = self.agent_mut().sub_interpret(
                        s.into(),
                        Box::new(AmlangInterpreter::default()),
                        interpreter_context,
                    )?;
                    self.agent_mut().set(node, Some(final_sexp))?;
                } else {
                    self.agent_mut().set(node, None)?;
                }
                Ok(node.into())
            }
            _ if context.curr() == special_node => {
                if arg_nodes.len() > 0 {
                    return err!(
                        self.agent(),
                        LangError::WrongArgumentCount {
                            given: arg_nodes.len(),
                            expected: ExpectedCount::Exactly(0),
                        }
                    );
                }
                self.print_curr_triples();
                Ok(self.agent().pos().into())
            }
            _ if context.jump() == special_node => {
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
                let dest = self.exec_to_node(arg_nodes[0])?;
                self.agent_mut().jump(dest);
                self.print_curr_triples();
                Ok(self.agent().pos().into())
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
            _ if context.env_find() == special_node => {
                if arg_nodes.len() != 1 {
                    return err!(
                        self.agent(),
                        LangError::WrongArgumentCount {
                            given: arg_nodes.len(),
                            expected: ExpectedCount::Exactly(1),
                        }
                    );
                }

                let des = self.agent_mut().designate(arg_nodes[0].into())?;
                let path = match <&LangString>::try_from(&des) {
                    Ok(path) => path,
                    _ => {
                        return err!(
                            self.agent(),
                            LangError::InvalidArgument {
                                given: des.into(),
                                expected: "Node containing string".into(),
                            }
                        );
                    }
                };

                let res = if let Some(lnode) = self.agent().find_env(path.as_str()) {
                    Node::new(LocalNode::default(), lnode).into()
                } else {
                    Sexp::default()
                };
                Ok(res)
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
                    debug!("applying (eval {})", arg);
                    let to_inner = self.contemplate(arg)?;
                    self.agent_mut().sub_interpret(
                        to_inner,
                        Box::new(AmlangInterpreter::default()),
                        interpreter_context,
                    )
                } else {
                    debug!("applying (exec {})", arg);
                    self.contemplate(arg)
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
            self.agent_mut().define(Some(sexp))
        }
    }

    fn evlis(
        &mut self,
        structures: Option<HeapSexp>,
        should_internalize: bool,
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

            let arg_node = if should_internalize {
                let val = self.internalize(*structure)?;
                self.node_or_insert(val)?
            } else {
                self.agent_mut().define(Some(*structure))?
            };
            args.push(arg_node);
        }
        Ok(args)
    }

    fn print_curr_triples(&mut self) {
        let triples = self.agent().ask_any(self.agent().pos()).unwrap();
        for triple in triples.triples() {
            print!("    ");
            let structure = triple.reify(self.agent_mut());
            self.agent_mut().print_sexp(&structure);
            println!("");
        }
    }
}

impl<'a> Interpreter for ExecutingInterpreter<'a> {
    fn contemplate(&mut self, structure: Sexp) -> Result<Sexp, Error> {
        let node = if let Ok(node) = <Node>::try_from(&structure) {
            node
        } else {
            self.agent_mut().history_insert(structure)
        };
        self.exec(node)
    }

    fn internalize(&mut self, structure: Sexp) -> Result<Sexp, Error> {
        match structure {
            Sexp::Primitive(primitive) => {
                if let Primitive::Symbol(symbol) = &primitive {
                    for frame in self.eval_state.iter() {
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

                let eval_car = self.internalize(*car)?;
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
                            self.eval_state.push(frame);
                            let res = self.evlis(Some(exprs), true);
                            self.eval_state.pop();
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
                    _ => {
                        let def_node = amlang_node!(def, context);
                        let node_node = amlang_node!(node, context);
                        let should_internalize = match self.agent_mut().designate(node.into())? {
                            // Don't evaluate args of reflective Abstractions.
                            Sexp::Primitive(Primitive::Procedure(Procedure::Abstraction(
                                _,
                                _,
                                true,
                            ))) => false,
                            _ => node != def_node && node != node_node,
                        };
                        let args = self.evlis(cdr, should_internalize)?;
                        return Ok(Procedure::Application(node, args).into());
                    }
                }
            }
        }
    }
}
