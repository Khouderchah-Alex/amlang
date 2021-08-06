use log::debug;
use std::borrow::Cow;
use std::convert::TryFrom;
use std::io::{stdout, BufWriter};
use std::iter::Peekable;

use super::agent::Agent;
use super::amlang_wrappers::*;
use super::env_state::EnvState;
use crate::environment::LocalNode;
use crate::lang_err::{ExpectedCount, LangErr};
use crate::model::{Eval, Model, Ret};
use crate::parser::{parse_sexp, ParseError};
use crate::primitive::continuation::ContinuationFrame;
use crate::primitive::{
    AmString, Continuation, Node, Number, Primitive, Procedure, Symbol, SymbolTable,
};
use crate::sexp::{Cons, HeapSexp, Sexp, SexpIntoIter};
use crate::token::TokenInfo;


#[derive(Clone)]
pub struct AmlangAgent {
    agent_state: EnvState,
    history_state: EnvState,
    cont: Continuation,
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
    pub fn from_state(mut agent_state: EnvState, history_state: EnvState) -> Self {
        // Ensure agent state designates amlang nodes first.
        let lang_env = agent_state.context().lang_env();
        agent_state.designation_chain_mut().push_front(lang_env);

        Self {
            agent_state,
            history_state,
            cont: Continuation::new(),
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
            let node = self
                .agent_state
                .env()
                .insert_atom()
                .globalize(&self.agent_state);
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
            .agent_state
            .env()
            .insert_structure(body_eval)
            .globalize(&self.agent_state);
        Ok(Procedure::Abstraction(surface, body_node))
    }

    fn exec(&mut self, meaning: Sexp) -> Ret {
        match meaning {
            Sexp::Primitive(Primitive::Procedure(proc)) => match proc {
                Procedure::Application(proc_node, arg_nodes) => {
                    let concretized_nodes = arg_nodes
                        .into_iter()
                        .map(|n| self.concretize(n))
                        .collect::<Vec<_>>();
                    let cproc = self.concretize(proc_node);
                    self.apply(cproc, concretized_nodes)
                }
                Procedure::Branch(pred, a, b) => {
                    let cpred = self.concretize(pred);
                    let ca = self.concretize(a);
                    let cb = self.concretize(b);
                    let p_des = self.agent_state.designate(Primitive::Node(cpred))?;
                    let a_des = self.agent_state.designate(Primitive::Node(ca))?;
                    let b_des = self.agent_state.designate(Primitive::Node(cb))?;

                    let cond = self.exec(p_des)?;
                    // TODO(func) Integrate actual boolean type.
                    if cond != Number::Integer(1).into() {
                        return Ok(self.exec(b_des)?);
                    }
                    Ok(self.exec(a_des)?)
                }
                lambda @ Procedure::Abstraction(..) => Ok(lambda.into()),
                _ => panic!("Unsupported procedure type: {:?}", proc),
            },
            _ => Ok(meaning),
        }
    }

    fn apply(&mut self, proc_node: Node, arg_nodes: Vec<Node>) -> Ret {
        match self.agent_state.designate(Primitive::Node(proc_node))? {
            Sexp::Primitive(Primitive::Node(node)) => {
                if node.env() == self.agent_state.context().lang_env() {
                    self.apply_special(node.local(), arg_nodes)
                } else {
                    err!(InvalidArgument {
                        given: node.into(),
                        expected: Cow::Borrowed("Procedure or special Amlang Node"),
                    })
                }
            }
            Sexp::Primitive(Primitive::BuiltIn(builtin)) => {
                let mut args = Vec::with_capacity(arg_nodes.len());
                for node in arg_nodes {
                    let structure = self.agent_state.designate(Primitive::Node(node))?;
                    let arg = if let Ok(_) = <Node>::try_from(&structure) {
                        structure
                    } else {
                        self.exec(structure)?
                    };
                    args.push(arg);
                }
                builtin.call(args, &self.cont)
            }
            Sexp::Primitive(Primitive::Procedure(Procedure::Abstraction(params, body_node))) => {
                if arg_nodes.len() != params.len() {
                    return err_ctx!(
                        self.cont,
                        WrongArgumentCount {
                            given: arg_nodes.len(),
                            // TODO(func) support variable arity.
                            expected: ExpectedCount::Exactly(params.len()),
                        }
                    );
                }

                let mut frame = ContinuationFrame::new(proc_node);
                let mut args = Vec::with_capacity(arg_nodes.len());
                for (i, node) in arg_nodes.into_iter().enumerate() {
                    let structure = self.agent_state.designate(Primitive::Node(node))?;
                    let arg = if let Ok(_) = <Node>::try_from(&structure) {
                        structure
                    } else {
                        self.exec(structure)?
                    };
                    args.push(arg);

                    frame.insert(params[i], node);
                    debug!("cont insert: {} -> {}", params[i], node);
                }

                self.cont.push(frame);

                let res = (|| {
                    let body = self.agent_state.designate(Primitive::Node(body_node))?;
                    let result = self.exec(body)?;
                    if let Ok(node) = <Node>::try_from(&result) {
                        Ok(self.concretize(node).into())
                    } else {
                        Ok(result)
                    }
                })();

                self.cont.pop();
                res
            }
            not_proc @ _ => err_ctx!(
                self.cont,
                InvalidArgument {
                    given: not_proc.clone(),
                    expected: Cow::Borrowed("Procedure"),
                }
            ),
        }
    }

    fn exec_to_node(&mut self, node: Node) -> Result<Node, LangErr> {
        let desig = self.agent_state.designate(Primitive::Node(node))?;
        let e = self.exec(desig)?;
        if let Ok(new_node) = Node::try_from(&e) {
            Ok(new_node)
        } else {
            Ok(node)
        }
    }

    fn apply_special(&mut self, special_node: LocalNode, arg_nodes: Vec<Node>) -> Ret {
        let context = self.agent_state.context();
        match special_node {
            _ if context.tell == special_node || context.ask == special_node => {
                let is_tell = context.tell == special_node;
                let (ss, pp, oo) = tell_wrapper(&arg_nodes, &self.cont)?;

                // TODO(func) Add support for cross-env triples through surrogates.
                let mut to_local = |node: Node| {
                    let placeholder = Node::new(
                        self.agent_state.context().lang_env(),
                        self.agent_state.context().placeholder,
                    );
                    let final_node = self.exec_to_node(node)?;
                    if (is_tell || final_node != placeholder)
                        && final_node.env() != self.agent_state.pos().env()
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
                    self.agent_state.tell(s, p, o)
                } else {
                    self.agent_state.ask(s, p, o)
                }
            }
            _ if context.def == special_node => {
                let (name, structure) = def_wrapper(&arg_nodes, &self.cont)?;
                self.agent_state.designate(Primitive::Node(name))?;
                if name.env() != self.agent_state.pos().env() {
                    panic!("Cross-env triples are not yet supported");
                }

                let structure_node = if let Some(s) = structure {
                    let des = self.agent_state.designate(Primitive::Node(s))?;
                    let res = self.exec(des)?;
                    self.agent_state.env().insert_structure(res)
                } else {
                    self.agent_state.env().insert_atom()
                };
                return Ok(self
                    .agent_state
                    .name_node(name.local(), structure_node)?
                    .into());
            }
            _ if context.curr == special_node => {
                if arg_nodes.len() > 0 {
                    return err_ctx!(
                        self.cont,
                        WrongArgumentCount {
                            given: arg_nodes.len(),
                            expected: ExpectedCount::Exactly(0),
                        }
                    );
                }
                self.print_curr_triples();
                return Ok(self.agent_state.pos().into());
            }
            _ if context.jump == special_node => {
                if arg_nodes.len() != 1 {
                    return err_ctx!(
                        self.cont,
                        WrongArgumentCount {
                            given: arg_nodes.len(),
                            expected: ExpectedCount::Exactly(1),
                        }
                    );
                }

                // If original expr eval + exec -> Node, use that.
                let dest = self.exec_to_node(arg_nodes[0])?;
                self.agent_state.jump(dest);
                self.print_curr_triples();
                return Ok(self.agent_state.pos().into());
            }
            _ if context.import == special_node => {
                if arg_nodes.len() != 1 {
                    return err_ctx!(
                        self.cont,
                        WrongArgumentCount {
                            given: arg_nodes.len(),
                            expected: ExpectedCount::Exactly(1),
                        }
                    );
                }

                // If original expr eval + exec -> Node, use that.
                let original = self.exec_to_node(arg_nodes[0])?;
                let imported = self.agent_state.import(original)?;
                return Ok(imported.into());
            }
            _ if context.env_find == special_node => {
                if arg_nodes.len() != 1 {
                    return err_ctx!(
                        self.cont,
                        WrongArgumentCount {
                            given: arg_nodes.len(),
                            expected: ExpectedCount::Exactly(1),
                        }
                    );
                }

                let des = self.agent_state.designate(Primitive::Node(arg_nodes[0]))?;
                let path = match <&AmString>::try_from(&des) {
                    Ok(path) => path,
                    _ => {
                        return err_ctx!(
                            self.cont,
                            InvalidArgument {
                                given: des.into(),
                                expected: Cow::Borrowed("Node containing string"),
                            }
                        );
                    }
                };

                if let Some(lnode) = self.agent_state.find_env(path.as_str()) {
                    Ok(Node::new(LocalNode::default(), lnode).into())
                } else {
                    Ok(Sexp::default())
                }
            }
            _ if context.apply == special_node => {
                let (proc_node, args_node) = apply_wrapper(&arg_nodes, &self.cont)?;
                let proc_meaning = self.agent_state.designate(Primitive::Node(proc_node))?;
                let proc_sexp = self.exec(proc_meaning)?;
                let args_meaning = self.agent_state.designate(Primitive::Node(args_node))?;
                let args_sexp = self.exec(args_meaning)?;
                debug!("applying (apply {} '{})", proc_sexp, args_sexp);

                let proc = if let Ok(node) = Node::try_from(&proc_sexp) {
                    node
                } else {
                    self.agent_state
                        .env()
                        .insert_structure(proc_sexp)
                        .globalize(&self.agent_state)
                };
                let mut args = Vec::new();
                for arg in SexpIntoIter::try_from(args_sexp)? {
                    if let Ok(node) = Node::try_from(&*arg) {
                        args.push(node);
                    } else {
                        args.push(
                            self.agent_state
                                .env()
                                .insert_structure(arg.into())
                                .globalize(&self.agent_state),
                        );
                    }
                }

                return self.apply(proc, args);
            }
            _ if context.eval == special_node || context.exec == special_node => {
                if arg_nodes.len() != 1 {
                    return err_ctx!(
                        self.cont,
                        WrongArgumentCount {
                            given: arg_nodes.len(),
                            expected: ExpectedCount::Exactly(1),
                        }
                    );
                }
                let is_eval = context.eval == special_node;
                let arg = self.agent_state.designate(Primitive::Node(arg_nodes[0]))?;
                let structure = self.exec(arg)?;
                debug!("applying (eval {})", structure);
                if is_eval {
                    self.eval(HeapSexp::new(structure))
                } else {
                    self.exec(structure)
                }
            }
            _ => err_ctx!(
                self.cont,
                InvalidArgument {
                    given: Node::new(self.agent_state.context().lang_env(), special_node).into(),
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
                            self.agent_state
                                .env()
                                .insert_structure(val)
                                .globalize(&self.agent_state),
                        );
                    }
                }
                Ok(args)
            }
        };
    }

    fn concretize(&self, node: Node) -> Node {
        if let Some(new_node) = self.cont.lookup(&node) {
            debug!("concretizing: {} -> {}", node, new_node);
            new_node
        } else {
            node
        }
    }
}


impl Agent for AmlangAgent {
    fn env_state(&mut self) -> &mut EnvState {
        &mut self.agent_state
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
                return self.agent_state.designate(primitive);
            }

            Sexp::Cons(cons) => {
                let (car, cdr) = cons.consume();
                let car = match car {
                    Some(car) => car,
                    None => return err!(InvalidSexp(Cons::new(car, cdr).into())),
                };

                let eval_car = self.eval(car)?;
                if let Ok(node) = <Node>::try_from(&eval_car) {
                    let context = self.agent_state.context();
                    match node {
                        _ if Node::new(context.lang_env(), context.quote) == node => {
                            return quote_wrapper(cdr);
                        }
                        _ if Node::new(context.lang_env(), context.lambda) == node => {
                            let (params, body) = make_lambda_wrapper(cdr, &self.cont)?;
                            let proc = self.make_lambda(params, body)?;
                            return Ok(self
                                .agent_state
                                .env()
                                .insert_structure(proc.into())
                                .globalize(&self.agent_state)
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

        let meaning_node = self
            .agent
            .history_state
            .env()
            .insert_structure(meaning)
            .globalize(&self.agent.history_state);
        let meaning = self
            .agent
            .history_state
            .designate(Primitive::Node(meaning_node))
            .unwrap();

        let res = match self.agent.exec(meaning) {
            Ok(val) => Ok(val),
            Err(err) => Err(RunError::ExecError(err)),
        };
        (self.handler)(&mut self.agent, &res);
        Some(res)
    }
}


impl AmlangAgent {
    pub fn trace_error(&mut self, err: &LangErr) {
        if let Some(cont) = err.cont() {
            println!("");
            println!("  --TRACE--");
            for (i, frame) in cont.iter().enumerate() {
                print!("   {})  ", i);
                self.print_list(&frame.context().into());
                println!("");
            }
        }
    }
    /*
       fn print_curr_nodes(&mut self) {
           let nodes = self.agent_state.env().all_nodes();
           for node in nodes {
               self.print_list(&node.into());
               println!("");
           }
       }
    */

    fn print_curr_triples(&mut self) {
        let local = self.agent_state.pos().local();
        let triples = self.agent_state.env().match_any(local);
        for triple in triples {
            print!("    ");
            let structure = triple.generate_structure(&mut self.agent_state);
            self.print_list(&structure);
            println!("");
        }
    }

    pub fn print_list(&mut self, structure: &Sexp) {
        let mut writer = BufWriter::new(stdout());
        if let Err(err) = self.print_list_internal(&mut writer, structure, 0) {
            println!("print_list error: {:?}", err);
        }
    }

    fn print_list_internal<W: std::io::Write>(
        &mut self,
        w: &mut W,
        structure: &Sexp,
        depth: usize,
    ) -> std::io::Result<()> {
        structure.write_list(w, depth, &mut |writer, primitive, depth| {
            self.write_primitive(writer, primitive, depth)
        })
    }

    fn write_primitive<W: std::io::Write>(
        &mut self,
        w: &mut W,
        primitive: &Primitive,
        depth: usize,
    ) -> std::io::Result<()> {
        const MAX_DEPTH: usize = 16;

        match primitive {
            Primitive::Node(node) => {
                // Print Nodes as their designators if possible.
                if let Some(sym) = self.agent_state.node_designator(*node) {
                    write!(w, "{}", sym.as_str())
                } else if let Some(triple) = self
                    .agent_state
                    .access_env(node.env())
                    .unwrap()
                    .node_as_triple(node.local())
                {
                    let s = triple.generate_structure(&mut self.agent_state);
                    self.print_list_internal(w, &s, depth + 1)
                } else {
                    let s = if let Some(structure) = self
                        .agent_state
                        .access_env(node.env())
                        .unwrap()
                        .node_structure(node.local())
                    {
                        write!(w, "{}->", node)?;
                        // Subtle: Cloning of Env doesn't actually copy data. In
                        // this case, the resulting Env object will be invalid
                        // and should only stand as a placeholder to determine
                        // typing.
                        //
                        // TODO(func) SharedEnv impl.
                        structure.clone()
                    } else {
                        return write!(w, "{}", node);
                    };

                    // If we recurse unconditionally, cycles will cause stack
                    // overflows.
                    if s == (*node).into() || depth > MAX_DEPTH {
                        write!(w, "{}", node)
                    } else {
                        self.print_list_internal(w, &s, depth + 1)
                    }
                }
            }
            Primitive::Procedure(procedure) => {
                let s = procedure.generate_structure(self.env_state());
                self.print_list_internal(w, &s, depth + 1)
            }
            _ => write!(w, "{}", primitive),
        }
    }
}
