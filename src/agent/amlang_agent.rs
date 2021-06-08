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


pub type Continuation = std::collections::HashMap<NodeId, Sexp>;

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

    fn curr_wrapper(&mut self, args: Option<HeapSexp>) -> Ret {
        if let Some(arg) = args {
            return match *arg {
                Sexp::Primitive(primitive) => Err(InvalidSexp(primitive.clone().into())),
                Sexp::Cons(cons) => Err(WrongArgumentCount {
                    given: cons.iter().count(),
                    expected: ExpectedCount::Exactly(0),
                }),
            };
        }
        self.print_curr_triples();
        Ok(self.env_state().pos().into())
    }

    fn jump_wrapper(&mut self, args: Option<HeapSexp>) -> Ret {
        if args.is_none() {
            return Err(WrongArgumentCount {
                given: 0,
                expected: ExpectedCount::Exactly(1),
            });
        }

        let (node_name,) = break_by_types!(*args.unwrap(), Symbol)?;
        let node = self.env_state().resolve(&node_name)?;

        self.env_state().jump(node);
        self.print_curr_triples();
        Ok(self.env_state().pos().into())
    }

    fn env_insert_node(
        &mut self,
        name: Symbol,
        structure: Option<HeapSexp>,
    ) -> Result<NodeId, EvalErr> {
        let ret = self.env_state().def_node(name, structure)?;

        for triple in self.env_state().env().match_all() {
            print!("    ");
            let structure = triple.generate_structure(self.env_state());
            self.print_list(&structure);
            println!("");
        }

        Ok(ret)
    }

    fn env_insert_triple(&mut self, subject: &Symbol, predicate: &Symbol, object: &Symbol) -> Ret {
        let designation = self.env_state().designation();
        let env = self.env_state().env();
        let table = <&mut SymbolTable>::try_from(env.node_structure(designation)).unwrap();

        let s = table.lookup(subject)?;
        let p = table.lookup(predicate)?;
        let o = table.lookup(object)?;

        if let Some(triple) = env.match_triple(s, p, o).iter().next() {
            return Err(EvalErr::DuplicateTriple(
                *triple.generate_structure(self.env_state()),
            ));
        }

        let triple = env.insert_triple(s, p, o);
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
            Primitive::Symbol(symbol) => write!(w, "[Symbol_\"{}\"]", symbol),
            Primitive::Node(node) => {
                // Print Nodes as their designators if possible.
                if let Some(designator) = self.env_state().node_designator(*node) {
                    write!(w, "{}", designator)
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
                    let mut args = Vec::with_capacity(arg_nodes.len());
                    for node in arg_nodes {
                        let structure = self.env_state().designate(Primitive::Node(*node))?;
                        let arg = if let Ok(node) = <NodeId>::try_from(&structure) {
                            cont.get(&node).unwrap().clone()
                        } else {
                            self.exec(&structure, cont)?
                        };
                        args.push(arg);
                    }

                    match self.env_state().designate(Primitive::Node(*proc_node))? {
                        Sexp::Primitive(Primitive::BuiltIn(builtin)) => builtin.call(&args),
                        // TODO(func) Allow for abstraction outside of
                        // application (e.g. returning a lambda).
                        Sexp::Primitive(Primitive::Procedure(Procedure::Abstraction(
                            params,
                            body_node,
                        ))) => {
                            if args.len() != params.len() {
                                return Err(WrongArgumentCount {
                                    given: args.len(),
                                    // TODO(func) support variable arity.
                                    expected: ExpectedCount::Exactly(params.len()),
                                });
                            }
                            for (i, node) in params.iter().enumerate() {
                                // TODO(func) Use actual deep continuation
                                // representation (including popping off).
                                cont.insert(*node, args[i].clone());
                            }
                            let body = self.env_state().designate(Primitive::Node(body_node))?;
                            self.exec(&body, cont)
                        }
                        _ => panic!(),
                    }
                }
                _ => panic!("Unsupported procedure type: {:?}", proc),
            },
            _ => Ok(meaning.clone()),
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
                    // Don't create new node for paramater nodes.
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
                    self.print_list(&val);
                    println!("");
                }
                Err(err) => {
                    println!(" {}", err);
                    continue;
                }
            };

            println!();
            self.print_curr_nodes();
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

                if let Ok(first) = <&Symbol>::try_from(&*car) {
                    match first.as_str() {
                        "quote" => {
                            return quote_wrapper(cdr);
                        }
                        "lambda" => {
                            let (params, body) = make_procedure_wrapper(cdr)?;
                            let proc = self.make_procedure(params, body)?;
                            return Ok(self.env_state().env().insert_structure(proc.into()).into());
                        }
                        /*
                        "def" => {
                            let (name, structure) = env_insert_node_wrapper(cons.cdr())?;
                            return Ok(self.env_insert_node(name, structure)?.into());
                        }
                        "tell" => {
                            let (s, p, o) = env_insert_triple_wrapper(cons.cdr())?;
                            return self.env_insert_triple(s, p, o);
                        }
                        "curr" => {
                            return self.curr_wrapper(cons.cdr());
                        }
                        "jump" => {
                            return self.jump_wrapper(cons.cdr());
                        }
                         */
                        _ => { /* Fallthrough */ }
                    }
                }

                let eval_car = self.eval(car.clone())?;
                if let Ok(node) = <NodeId>::try_from(&eval_car) {
                    let args = self.evlis(cdr)?;
                    return Ok(Procedure::Application(node, args).into());
                }
                return Err(InvalidArgument {
                    given: Cons::new(Some(Box::new(eval_car)), cdr).into(),
                    expected: Cow::Borrowed("special form or functional application"),
                });
            }
        }
    }
}
