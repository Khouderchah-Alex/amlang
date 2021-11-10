use log::debug;
use std::convert::TryFrom;

use super::agent::{Agent, ExecFrame};
use super::amlang_wrappers::*;
use super::continuation::Continuation;
use crate::agent::lang_error::{ExpectedCount, LangError};
use crate::environment::entry::EntryMutKind;
use crate::environment::LocalNode;
use crate::error::Error;
use crate::model::{Interpretation, Reflective};
use crate::primitive::prelude::*;
use crate::primitive::table::Table;
use crate::sexp::{Cons, HeapSexp, Sexp};


pub struct AmlangAgent<'a> {
    agent: &'a mut Agent,
    eval_state: Continuation<SymbolTable>,
}

impl<'a> AmlangAgent<'a> {
    pub fn from_agent(agent: &'a mut Agent) -> Self {
        // Ensure agent designates amlang nodes first.
        // TODO(func) Make idempotent.
        let lang_env = agent.context().lang_env();
        agent.designation_chain_mut().push_front(lang_env);

        Self {
            agent,
            eval_state: Continuation::new(SymbolTable::default()),
        }
    }

    pub fn agent(&self) -> &Agent {
        &self.agent
    }
    pub fn agent_mut(&mut self) -> &mut Agent {
        &mut self.agent
    }


    fn make_lambda(
        &mut self,
        params: Vec<Symbol>,
        body: HeapSexp,
        reflect: bool,
    ) -> Result<(Procedure, SymbolTable), Error> {
        let mut surface = Vec::new();
        let mut frame = SymbolTable::default();
        for symbol in params {
            let node = self.agent_mut().env().insert_atom().globalize(self.agent());
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
            let name = self
                .agent_mut()
                .env()
                .insert_structure(symbol.into())
                .globalize(self.agent());
            // Unlike amlang designation, label predicate must be imported.
            let raw_predicate = amlang_node!(self.agent().context(), label);
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
                let eval = self.construe(*elem)?;
                let node = self.node_or_insert(eval)?;
                body_nodes.push(node);
            }

            if body_nodes.len() == 1 {
                Ok(Procedure::Abstraction(surface, body_nodes[0], reflect))
            } else {
                let seq_node = self
                    .agent_mut()
                    .env()
                    .insert_structure(Procedure::Sequence(body_nodes).into())
                    .globalize(self.agent());
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
                        if cond == amlang_node!(context, t).into() {
                            Ok(self.exec(a)?)
                        } else if cond == amlang_node!(context, f).into() {
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
                let mut args = Vec::with_capacity(arg_nodes.len());
                for node in arg_nodes {
                    args.push(self.exec(node)?);
                }
                builtin.call(args, self.agent_mut())
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
            _ if context.tell == special_node || context.ask == special_node => {
                let is_tell = context.tell == special_node;
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
                    self.agent_mut().tell(s, p, o)
                } else {
                    self.agent_mut().ask(s, p, o)
                }
            }
            _ if context.def == special_node => {
                let (name, val) = def_wrapper(&arg_nodes, &self.agent())?;
                self.agent_mut().designate(name.into())?;
                if name.env() != self.agent().pos().env() {
                    panic!("Cross-env triples are not yet supported");
                }

                let val_node = if let Some(s) = val {
                    // Ensure construe maps name to this node.
                    let val_node = self.agent_mut().env().insert_atom();
                    let mut frame = SymbolTable::default();
                    if let Ok(sym) =
                        Symbol::try_from(self.agent_mut().designate(Primitive::Node(name))?)
                    {
                        frame.insert(sym, val_node.globalize(self.agent()));
                    }

                    // Construe, relying on self-evaluation of val_node.
                    let original = self.agent_mut().designate(Primitive::Node(s))?;
                    self.eval_state.push(frame);
                    let meaning = self.construe(original.into());
                    self.eval_state.pop();

                    let final_sexp = self.contemplate(meaning?)?;
                    // If final result is a Node, we name that rather
                    // than nesting abstractions. Perhaps nested
                    // abstraction is right here?
                    //
                    // TODO(func) Either nest abstractions or somehow
                    // garbage-mark/free the unused atom.
                    if let Ok(node) = <Node>::try_from(&final_sexp) {
                        node
                    } else {
                        *self.agent_mut().env().entry_mut(val_node).kind_mut() =
                            EntryMutKind::Owned(final_sexp);
                        val_node.globalize(self.agent())
                    }
                } else {
                    self.agent_mut().env().insert_atom().globalize(self.agent())
                };
                Ok(self.agent_mut().name_node(name, val_node)?.into())
            }
            _ if context.curr == special_node => {
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
            _ if context.jump == special_node => {
                if arg_nodes.len() != 1 {
                    return err!(
                        self.agent(),
                        LangError::WrongArgumentCount {
                            given: arg_nodes.len(),
                            expected: ExpectedCount::Exactly(1),
                        }
                    );
                }

                // If original expr construe + exec -> Node, use that.
                let dest = self.exec_to_node(arg_nodes[0])?;
                self.agent_mut().jump(dest);
                self.print_curr_triples();
                Ok(self.agent().pos().into())
            }
            _ if context.import == special_node => {
                if arg_nodes.len() != 1 {
                    return err!(
                        self.agent(),
                        LangError::WrongArgumentCount {
                            given: arg_nodes.len(),
                            expected: ExpectedCount::Exactly(1),
                        }
                    );
                }

                // If original expr construe + exec -> Node, use that.
                let original = self.exec_to_node(arg_nodes[0])?;
                let imported = self.agent_mut().import(original)?;
                Ok(imported.into())
            }
            _ if context.env_find == special_node => {
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
                let path = match <&AmString>::try_from(&des) {
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
            _ if context.apply == special_node => {
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
            _ if context.eval == special_node || context.exec == special_node => {
                if arg_nodes.len() != 1 {
                    return err!(
                        self.agent(),
                        LangError::WrongArgumentCount {
                            given: arg_nodes.len(),
                            expected: ExpectedCount::Exactly(1),
                        }
                    );
                }
                let is_eval = context.eval == special_node;
                let arg = self.agent_mut().designate(arg_nodes[0].into())?;
                if is_eval {
                    debug!("applying (eval {})", arg);
                    self.construe(arg)
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
            Ok(self
                .agent_mut()
                .env()
                .insert_structure(sexp)
                .globalize(self.agent()))
        }
    }

    fn evlis(
        &mut self,
        structures: Option<HeapSexp>,
        should_construe: bool,
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

            let arg_node = if should_construe {
                let val = self.construe(*structure)?;
                self.node_or_insert(val)?
            } else {
                self.agent_mut()
                    .env()
                    .insert_structure(*structure)
                    .globalize(self.agent())
            };
            args.push(arg_node);
        }
        Ok(args)
    }

    fn print_curr_triples(&mut self) {
        let local = self.agent().pos().local();
        let triples = self.agent_mut().env().match_any(local);
        for triple in triples {
            print!("    ");
            let structure = triple.reify(self.agent_mut());
            self.agent_mut().print_sexp(&structure);
            println!("");
        }
    }
}

impl<'a> Interpretation for AmlangAgent<'a> {
    fn contemplate(&mut self, structure: Sexp) -> Result<Sexp, Error> {
        let node = if let Ok(node) = <Node>::try_from(&structure) {
            node
        } else {
            self.agent_mut().history_insert(structure)
        };
        self.exec(node)
    }

    fn construe(&mut self, structure: Sexp) -> Result<Sexp, Error> {
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

                let eval_car = self.construe(*car)?;
                let node = match eval_car {
                    Sexp::Primitive(Primitive::Procedure(_))
                    | Sexp::Primitive(Primitive::Node(_)) => self.node_or_insert(eval_car)?,
                    _ => {
                        return err!(
                            self.agent(),
                            LangError::InvalidArgument {
                                given: Cons::new(Some(eval_car.into()), cdr).into(),
                                expected: "special form or Procedure application".into(),
                            }
                        );
                    }
                };
                let context = self.agent().context();
                match node {
                    _ if amlang_node!(context, quote) == node => {
                        return Ok(*quote_wrapper(cdr, self.agent())?);
                    }
                    _ if amlang_node!(context, lambda) == node
                        || amlang_node!(context, fexpr) == node =>
                    {
                        let (params, body) = make_lambda_wrapper(cdr, &self.agent())?;
                        let reflect = node.local() == context.fexpr;
                        let (proc, _) = self.make_lambda(params, body, reflect)?;
                        return Ok(proc.into());
                    }
                    _ if amlang_node!(context, let_basic) == node
                        || amlang_node!(context, let_rec) == node =>
                    {
                        let (params, exprs, body) = let_wrapper(cdr, &self.agent())?;
                        let recursive = node.local() == context.let_rec;
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
                    _ if amlang_node!(context, branch) == node => {
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
                    _ if amlang_node!(context, progn) == node => {
                        let args = self.evlis(cdr, true)?;
                        return Ok(Procedure::Sequence(args).into());
                    }
                    _ => {
                        let def_node = amlang_node!(context, def);
                        let should_construe = match self.agent_mut().designate(node.into())? {
                            // Don't evaluate args of reflective Abstractions.
                            Sexp::Primitive(Primitive::Procedure(Procedure::Abstraction(
                                _,
                                _,
                                true,
                            ))) => false,
                            _ => node != def_node,
                        };
                        let args = self.evlis(cdr, should_construe)?;
                        return Ok(Procedure::Application(node, args).into());
                    }
                }
            }
        }
    }
}
