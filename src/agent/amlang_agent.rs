use std::borrow::Cow;
use std::convert::TryFrom;

use super::agent::Agent;
use super::amlang_wrappers::*;
use super::env_state::EnvState;
use crate::builtins::{add, div, mul, sub};
use crate::function::{
    EvalErr::{self, *},
    ExpectedCount, Func, Ret,
};
use crate::model::{Designation, Eval};
use crate::parser::parse_sexp;
use crate::primitive::procedure::{BProcedure, Bindings, Procedure, SProcedure};
use crate::primitive::{BuiltIn, NodeId, Primitive, Symbol, SymbolTable, ToSymbol};
use crate::sexp::{Cons, HeapSexp, Sexp};
use crate::syntax;
use crate::token::interactive_stream::InteractiveStream;


pub struct AmlangAgent {
    env_state: EnvState,
}

impl AmlangAgent {
    pub fn new() -> Self {
        let env_state = EnvState::new();
        Self { env_state }
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

        let cons = match *args.unwrap() {
            Sexp::Primitive(primitive) => {
                return Err(InvalidSexp(primitive.clone().into()));
            }
            Sexp::Cons(cons) => cons,
        };

        let mut iter = cons.iter();
        let node = if let Ok(symbol) = <&Symbol>::try_from(iter.next()) {
            self.resolve(symbol)?
        } else {
            return Err(InvalidArgument {
                given: cons.clone().into(),
                expected: Cow::Borrowed("node"),
            });
        };

        if iter.next().is_some() {
            return Err(WrongArgumentCount {
                given: iter.count() + 2,
                expected: ExpectedCount::Exactly(1),
            });
        }

        self.env_state().jump(node);
        self.print_curr_triples();
        Ok(self.env_state().pos().into())
    }

    fn env_insert_node(
        &mut self,
        name: &Symbol,
        structure: Option<HeapSexp>,
    ) -> Result<NodeId, EvalErr> {
        let designation = self.env_state().designation();

        if let Ok(table) =
            <&mut SymbolTable>::try_from(self.env_state().env().node_structure(designation))
        {
            if table.contains_key(name) {
                return Err(AlreadyBoundSymbol(name.clone()));
            }
        } else {
            panic!("Env designation isn't a symbol table");
        }

        let node = if let Some(sexp) = structure {
            self.env_state().env().insert_structure(*sexp)
        } else {
            self.env_state().env().insert_atom()
        };
        let env = self.env_state().env();
        let name_node = env.insert_structure(name.clone().into());

        if let Ok(table) = <&mut SymbolTable>::try_from(env.node_structure(designation)) {
            table.insert(name.clone(), node);
        } else {
            panic!("Env designation isn't a symbol table");
        }

        env.insert_triple(node, designation, name_node);

        for triple in env.match_all() {
            print!("    ");
            let structure = self.env_state().triple_structure(triple);
            self.print_list(&structure, 0);
            println!("");
        }
        Ok(node)
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
                self.env_state().triple_structure(*triple).into(),
            ));
        }

        let triple = env.insert_triple(s, p, o);
        Ok(self.env_state().triple_structure(triple).into())
    }

    fn print_curr_triples(&mut self) {
        let node = self.env_state().pos();
        let triples = self.env_state().env().match_any(node);
        for triple in triples {
            print!("    ");
            let structure = self.env_state().triple_structure(triple);
            self.print_list(&structure, 0);
            println!("");
        }
    }

    fn resolve(&mut self, name: &Symbol) -> Result<NodeId, EvalErr> {
        let designation = self.env_state().designation();
        let env = self.env_state().env();

        let table = <&mut SymbolTable>::try_from(env.node_structure(designation)).unwrap();
        let node = table.lookup(name)?;
        Ok(node.into())
    }

    // TODO This needs to be merged with list_fmt. Struggling to make generic
    // over io:: and fmt::Write led to this duplication.
    fn print_list(&mut self, sexp: &Sexp, depth: usize) {
        // Any list longer than this will simply be suffixed with "..." after these
        // many elements.
        const MAX_DISPLAY_LENGTH: usize = 64;
        const MAX_DISPLAY_DEPTH: usize = 32;

        if let Sexp::Primitive(primitive) = sexp {
            match primitive {
                Primitive::Symbol(symbol) => print!("[Symbol_\"{}\"]", symbol),
                Primitive::Node(node) => {
                    // Print Nodes as their designators if possible.
                    if let Some(designator) = self.env_state().node_designator(*node) {
                        print!("{}", designator);
                    } else {
                        let s =
                            if let Some(structure) = self.env_state().env().node_structure(*node) {
                                structure.clone()
                            } else {
                                print!("{}", node);
                                return;
                            };
                        self.print_list(&s, depth + 1);
                    }
                }
                _ => print!("{}", primitive),
            }
            return;
        };

        if depth >= MAX_DISPLAY_DEPTH {
            return print!("(..)");
        }

        let mut pos: usize = 0;
        let mut outer_quote = false;
        for val in sexp.cons().iter() {
            if pos == 0 {
                if let Ok(symbol) = <&Symbol>::try_from(val) {
                    if symbol.as_str() == "quote" {
                        outer_quote = true;
                        print!("'");
                        pos += 1;
                        continue;
                    }
                }
                print!("(");
            }

            if pos >= MAX_DISPLAY_LENGTH {
                print!("...");
                break;
            }

            if pos > 0 && !outer_quote {
                print!(" ");
            }
            self.print_list(val, depth + 1);

            pos += 1;
        }

        if pos == 0 {
            print!("(");
        }
        if !outer_quote {
            print!(")");
        }
    }

    fn exec(&mut self, meaning: &Sexp) -> Ret {
        if let Ok(proc) = <&Procedure>::try_from(meaning) {
            match proc.body() {
                BProcedure::Application(node) => {
                    let builtin =
                        <BuiltIn>::try_from(self.designate(Primitive::Node(*node))?).unwrap();

                    let surface_args = proc.surface_args();
                    let mut cont = Vec::with_capacity(surface_args.len());
                    for node in surface_args {
                        let proc = self.designate(Primitive::Node(*node))?;
                        cont.push(self.exec(&proc)?);
                    }
                    let args = proc.generate_args(cont);
                    builtin.call(&args)
                }
                _ => panic!("Invalid proc"),
            }
        } else {
            Ok(meaning.clone())
        }
    }

    fn evlis(&mut self, args: Option<HeapSexp>) -> Result<(SProcedure, Bindings), EvalErr> {
        let mut surface = SProcedure::new();
        let mut bindings = Bindings::new();
        if args.is_none() {
            return Ok((surface, bindings));
        }

        match *args.unwrap() {
            Sexp::Primitive(primitive) => {
                return Err(InvalidSexp(primitive.clone().into()));
            }

            Sexp::Cons(cons) => {
                for (i, arg) in cons.into_iter().enumerate() {
                    let val = self.eval(arg)?;
                    match val {
                        Sexp::Primitive(Primitive::Procedure(proc)) => {
                            let proc_node = self.env_state().env().insert_structure(proc.into());
                            surface.push(proc_node);
                        }
                        _ => {
                            bindings.insert(i, val);
                        }
                    }
                }
            }
        }
        Ok((surface, bindings))
    }
}


impl Default for AmlangAgent {
    fn default() -> Self {
        Self::new()
    }
}

macro_rules! insert_builtins {
    [$self:ident, $($n:tt : $x:expr),*] => {
        {
            $(
                {
                    let fun = HeapSexp::new(BuiltIn::new(stringify!($x), $x).into());
                    $self.env_insert_node(&$n.to_symbol_or_panic(), Some(fun)).unwrap();
                }
            )*
        }
    };
    [$($n:tt : $x:expr),+ ,] => {
        builtins![$($n : $x),*]
    };
}

impl Agent for AmlangAgent {
    fn run(&mut self) -> Result<(), String> {
        insert_builtins![self, "+": add, "-": sub, "*": mul, "/": div];

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

            match self.exec(&meaning) {
                Ok(val) => {
                    print!("-> ");
                    self.print_list(&val, 0);
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

impl Designation for AmlangAgent {
    fn designate(&mut self, designator: Primitive) -> Ret {
        match designator {
            // Symbol -> Node
            Primitive::Symbol(symbol) => Ok(self.resolve(&symbol)?.into()),
            // Node -> Structure
            Primitive::Node(node) => {
                if let Some(structure) = self.env_state().env().node_structure(node) {
                    Ok(structure.clone())
                } else {
                    // Atoms are self-designating.
                    Ok(node.into())
                }
            }
            // Base case for self-designating.
            _ => Ok(designator.clone().into()),
        }
    }
}

impl Eval for AmlangAgent {
    fn eval(&mut self, structure: HeapSexp) -> Ret {
        match *structure {
            Sexp::Primitive(primitive) => {
                return self.designate(primitive);
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
                            return syntax::quote(cdr);
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
                    let (surface, bindings) = self.evlis(cdr)?;
                    return Ok(
                        Procedure::new(surface, bindings, BProcedure::Application(node)).into(),
                    );
                }
                return Err(InvalidArgument {
                    given: Cons::new(Some(Box::new(eval_car)), cdr).into(),
                    expected: Cow::Borrowed("special form or functional application"),
                });
            }
        }
    }
}
