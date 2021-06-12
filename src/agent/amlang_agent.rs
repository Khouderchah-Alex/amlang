use std::borrow::Cow;
use std::convert::TryFrom;
use std::io::{stdout, BufWriter};

use super::agent::Agent;
use super::amlang_wrappers::*;
use super::env_state::EnvState;
use crate::function::{
    EvalErr::{self, *},
    ExpectedCount, Func, Ret,
};
use crate::model::{Eval, Model};
use crate::parser::parse_sexp;
use crate::primitive::procedure::Procedure;
use crate::primitive::{NodeId, Primitive, Symbol, SymbolTable};
use crate::sexp::{Cons, HeapSexp, Sexp};
use crate::token::interactive_stream::InteractiveStream;


pub type Continuation = std::collections::HashMap<NodeId, NodeId>;

pub struct AmlangAgent {
    env_state: EnvState,
    eval_symbols: SymbolTable,
}

impl AmlangAgent {
    pub fn from_env(env_state: EnvState) -> Self {
        let eval_symbols = SymbolTable::default();
        Self {
            env_state,
            eval_symbols,
        }
    }

    fn make_procedure(&mut self, params: Vec<Symbol>, body: Sexp) -> Result<Procedure, EvalErr> {
        let mut surface = Vec::new();
        for symbol in params {
            let node = self.env_state().env().insert_atom();
            // TODO(func) Use actual deep environment representation (including popping off).
            self.eval_symbols.insert(symbol, node);
            surface.push(node);
        }

        let cons = match body {
            Sexp::Primitive(primitive) => {
                return Err(InvalidSexp(primitive.clone().into()));
            }
            Sexp::Cons(cons) => cons,
        };
        // TODO(func) Allow for sequence.
        let body_eval = self.eval(Box::new(cons.car().unwrap().clone()))?;
        let body_node = self.env_state().env().insert_structure(body_eval);
        Ok(Procedure::Abstraction(surface, body_node))
    }

    fn env_insert_triple(&mut self, subject: NodeId, predicate: NodeId, object: NodeId) -> Ret {
        let env = self.env_state().env();

        if let Some(triple) = env.match_triple(subject, predicate, object).iter().next() {
            return Err(EvalErr::DuplicateTriple(
                *triple.generate_structure(self.env_state()),
            ));
        }

        let triple = env.insert_triple(subject, predicate, object);
        Ok(*triple.generate_structure(self.env_state()))
    }

    fn print_curr_nodes(&mut self) {
        let nodes = self.env_state().env().all_nodes();
        for node in nodes {
            self.print_list(&node.into());
            println!("");
        }
    }

    fn print_curr_triples(&mut self) {
        let node = self.env_state().pos();
        let triples = self.env_state().env().match_any(node);
        for triple in triples {
            print!("    ");
            let structure = triple.generate_structure(self.env_state());
            self.print_list(&structure);
            println!("");
        }
    }

    fn print_list(&mut self, structure: &Sexp) {
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
        match primitive {
            Primitive::Node(node) => {
                // Print Nodes as their designators if possible.
                if let Some(designator) = self.env_state().node_designator(*node) {
                    if let Ok(sym) = <&Symbol>::try_from(&*designator) {
                        write!(w, "{}", sym.as_str())
                    } else {
                        write!(w, "{}", designator)
                    }
                } else {
                    let s = if let Some(structure) = self.env_state().env().node_structure(*node) {
                        structure.clone()
                    } else {
                        return write!(w, "{}", node);
                    };
                    self.print_list_internal(w, &s, depth + 1)
                }
            }
            _ => write!(w, "{}", primitive),
        }
    }

    fn exec(&mut self, meaning: &Sexp, cont: &mut Continuation) -> Ret {
        match meaning {
            Sexp::Primitive(Primitive::Procedure(proc)) => match proc {
                Procedure::Application(proc_node, arg_nodes) => {
                    let concretize = |node| {
                        if let Some(new_node) = cont.get(node) {
                            new_node
                        } else {
                            node
                        }
                        .clone()
                    };

                    let final_nodes = arg_nodes.iter().map(concretize).collect::<Vec<_>>();
                    self.apply(concretize(proc_node), final_nodes, cont)
                }
                _ => panic!("Unsupported procedure type: {:?}", proc),
            },
            _ => Ok(meaning.clone()),
        }
    }

    fn apply(&mut self, proc_node: NodeId, arg_nodes: Vec<NodeId>, cont: &mut Continuation) -> Ret {
        match self.env_state().designate(Primitive::Node(proc_node))? {
            Sexp::Primitive(Primitive::Node(node)) => {
                let context = self.env_state().context();
                match node {
                    _ if context.tell == node => {
                        let (a, b, c) = env_insert_triple_wrapper(&arg_nodes)?;
                        return self.env_insert_triple(a, b, c);
                    }
                    _ if context.def == node => {
                        let (name, structure) = env_insert_node_wrapper(&arg_nodes)?;
                        self.env_state().designate(Primitive::Node(name))?;
                        return Ok(self.env_state().def_node(name, structure)?.into());
                    }
                    _ if context.curr == node => {
                        if arg_nodes.len() > 0 {
                            return Err(WrongArgumentCount {
                                given: arg_nodes.len(),
                                expected: ExpectedCount::Exactly(0),
                            });
                        }
                        self.print_curr_triples();
                        return Ok(self.env_state().pos().into());
                    }
                    _ if context.jump == node => {
                        if arg_nodes.len() != 1 {
                            return Err(WrongArgumentCount {
                                given: arg_nodes.len(),
                                expected: ExpectedCount::Exactly(1),
                            });
                        }
                        self.env_state().jump(arg_nodes[0]);
                        self.print_curr_triples();
                        return Ok(self.env_state().pos().into());
                    }
                    _ => panic!(),
                }
            }
            Sexp::Primitive(Primitive::BuiltIn(builtin)) => {
                let mut args = Vec::with_capacity(arg_nodes.len());
                for node in arg_nodes {
                    let structure = self.env_state().designate(Primitive::Node(node))?;
                    let arg = if let Ok(node) = <NodeId>::try_from(&structure) {
                        node.into()
                    } else {
                        self.exec(&structure, cont)?
                    };
                    args.push(arg);
                }
                builtin.call(&args)
            }
            // TODO(func) Allow for abstraction outside of
            // application (e.g. returning a lambda).
            Sexp::Primitive(Primitive::Procedure(Procedure::Abstraction(params, body_node))) => {
                if arg_nodes.len() != params.len() {
                    return Err(WrongArgumentCount {
                        given: arg_nodes.len(),
                        // TODO(func) support variable arity.
                        expected: ExpectedCount::Exactly(params.len()),
                    });
                }

                let mut args = Vec::with_capacity(arg_nodes.len());
                for (i, node) in arg_nodes.into_iter().enumerate() {
                    let structure = self.env_state().designate(Primitive::Node(node))?;
                    let arg = if let Ok(node) = <NodeId>::try_from(&structure) {
                        node.into()
                    } else {
                        self.exec(&structure, cont)?
                    };
                    args.push(arg);

                    // TODO(func) Use actual deep continuation
                    // representation (including popping off).
                    cont.insert(params[i], node);
                }

                let body = self.env_state().designate(Primitive::Node(body_node))?;
                self.exec(&body, cont)
            }
            not_proc @ _ => Err(InvalidArgument {
                given: not_proc.clone(),
                expected: Cow::Borrowed("Procedure"),
            }),
        }
    }

    fn evlis(&mut self, structures: Option<HeapSexp>) -> Result<Vec<NodeId>, EvalErr> {
        if structures.is_none() {
            return Ok(vec![]);
        }

        return match *structures.unwrap() {
            Sexp::Primitive(primitive) => Err(InvalidSexp(primitive.clone().into())),

            Sexp::Cons(cons) => {
                // TODO(perf) Return Cow.
                let mut args = Vec::<NodeId>::with_capacity(cons.iter().count());
                for structure in cons.into_iter() {
                    let val = self.eval(structure)?;
                    // Don't recreate existing Nodes.
                    if let Ok(node) = <NodeId>::try_from(&val) {
                        args.push(node.into());
                    } else {
                        args.push(self.env_state().env().insert_structure(val));
                    }
                }
                Ok(args)
            }
        };
    }
}


impl Agent for AmlangAgent {
    fn run(&mut self) -> Result<(), String> {
        let stream = InteractiveStream::new();
        let mut peekable = stream.peekable();

        loop {
            let sexp = match parse_sexp(&mut peekable, 0) {
                Ok(Some(parsed)) => parsed,
                Ok(None) => return Ok(()),
                Err(err) => {
                    println!(" {}", err);
                    println!("");
                    continue;
                }
            };

            let meaning = match self.eval(sexp) {
                Ok(meaning) => meaning,
                Err(err) => {
                    println!("[Compile error] {}", err);
                    continue;
                }
            };

            let mut cont = Continuation::default();
            match self.exec(&meaning, &mut cont) {
                Ok(val) => {
                    print!("-> ");
                    if let Ok(node) = <NodeId>::try_from(&val) {
                        let designated = self.env_state().designate(Primitive::Node(node)).unwrap();
                        self.print_list(&designated);
                    } else {
                        self.print_list(&val);
                    }
                    println!("");
                }
                Err(err) => {
                    println!(" {}", err);
                    continue;
                }
            };

            println!();
        }
    }

    fn env_state(&mut self) -> &mut EnvState {
        &mut self.env_state
    }
}

impl Eval for AmlangAgent {
    fn eval(&mut self, structure: HeapSexp) -> Ret {
        match *structure {
            Sexp::Primitive(primitive) => {
                if let Primitive::Symbol(symbol) = &primitive {
                    if let Ok(node) = self.eval_symbols.lookup(symbol) {
                        return Ok(node.into());
                    }
                }
                return self.env_state().designate(primitive);
            }

            Sexp::Cons(cons) => {
                let (car, cdr) = cons.consume();
                let car = match car {
                    Some(car) => car,
                    None => return Err(InvalidSexp(Cons::new(car, cdr).into())),
                };

                let eval_car = self.eval(car)?;
                if let Ok(node) = <NodeId>::try_from(&eval_car) {
                    let context = self.env_state().context();
                    match node {
                        _ if context.quote == node => return quote_wrapper(cdr),
                        _ if context.lambda == node => {
                            let (params, body) = make_procedure_wrapper(cdr)?;
                            let proc = self.make_procedure(params, body)?;
                            return Ok(self.env_state().env().insert_structure(proc.into()).into());
                        }
                        _ => {
                            let args = self.evlis(cdr)?;
                            return Ok(Procedure::Application(node, args).into());
                        }
                    }
                }
                return Err(InvalidArgument {
                    given: Cons::new(Some(Box::new(eval_car)), cdr).into(),
                    expected: Cow::Borrowed("special form or functional application"),
                });
            }
        }
    }
}
