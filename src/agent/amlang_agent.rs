use std::borrow::Cow;
use std::convert::TryFrom;

use super::agent::Agent;
use super::env_state::EnvState;
use crate::builtins::{add, div, mul, sub};
use crate::function::{
    EvalErr::{self, *},
    ExpectedCount, Func, Ret,
};
use crate::model::{Designation, Eval};
use crate::parser::parse_sexp;
use crate::primitive::{BuiltIn, NodeId, Primitive, Symbol, SymbolTable, ToSymbol};
use crate::sexp::Sexp;
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

    fn env_insert_node_wrapper(&mut self, args: Option<&Sexp>) -> Ret {
        if args.is_none() {
            return Err(WrongArgumentCount {
                given: 0,
                expected: ExpectedCount::AtLeast(1),
            });
        }

        let cons = match args.unwrap() {
            Sexp::Primitive(primitive) => {
                return Err(InvalidSexp(primitive.clone().into()));
            }
            Sexp::Cons(cons) => cons,
        };

        let mut iter = cons.iter();
        let name = if let Ok(symbol) = <&Symbol>::try_from(iter.next()) {
            symbol
        } else {
            return Err(InvalidArgument {
                given: cons.clone().into(),
                expected: Cow::Borrowed("symbol"),
            });
        };

        let structure = iter.next();
        if structure.is_some() && iter.next().is_some() {
            return Err(WrongArgumentCount {
                given: iter.count() + 3,
                expected: ExpectedCount::AtMost(2),
            });
        }

        self.env_insert_node(name, structure)
    }

    fn env_insert_triple_wrapper(&mut self, args: Option<&Sexp>) -> Ret {
        if args.is_none() {
            return Err(WrongArgumentCount {
                given: 0,
                expected: ExpectedCount::Exactly(3),
            });
        }

        let cons = match args.unwrap() {
            Sexp::Primitive(primitive) => {
                return Err(InvalidSexp(primitive.clone().into()));
            }
            Sexp::Cons(cons) => cons,
        };

        fn extract_symbol<'a, I: Iterator<Item = &'a Sexp>>(
            i: usize,
            iter: &mut I,
        ) -> Result<&'a Symbol, EvalErr> {
            if let Some(elem) = iter.next() {
                if let Ok(symbol) = <&Symbol>::try_from(elem) {
                    Ok(symbol)
                } else {
                    Err(InvalidArgument {
                        given: elem.clone().into(),
                        expected: Cow::Borrowed("symbol"),
                    })
                }
            } else {
                Err(WrongArgumentCount {
                    given: i,
                    expected: ExpectedCount::Exactly(3),
                })
            }
        }

        let mut iter = cons.iter();
        let subject = extract_symbol(0, &mut iter)?;
        let predicate = extract_symbol(1, &mut iter)?;
        let object = extract_symbol(2, &mut iter)?;

        if let Some(_) = iter.next() {
            return Err(WrongArgumentCount {
                given: cons.iter().count(),
                expected: ExpectedCount::Exactly(3),
            });
        }

        self.env_insert_triple(subject, predicate, object)
    }

    fn env_insert_node(&mut self, name: &Symbol, structure: Option<&Sexp>) -> Ret {
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
            let s = self.eval(sexp)?;
            self.env_state().env().insert_structure(s)
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
            println!("    {}", self.env_state().triple_inner_designators(triple));
        }
        Ok(name.clone().into())
    }

    fn env_insert_triple(&mut self, subject: &Symbol, predicate: &Symbol, object: &Symbol) -> Ret {
        fn lookup(table: &mut SymbolTable, name: &Symbol) -> Result<NodeId, EvalErr> {
            if let Some(node) = table.lookup(name) {
                Ok(*node)
            } else {
                Err(EvalErr::UnboundSymbol(name.clone()))
            }
        }

        let designation = self.env_state().designation();
        let env = self.env_state().env();
        let table = <&mut SymbolTable>::try_from(env.node_structure(designation)).unwrap();

        let s = lookup(table, subject)?;
        let p = lookup(table, predicate)?;
        let o = lookup(table, object)?;

        let triple = self.env_state().env().insert_triple(s, p, o);
        Ok(self.env_state().triple_inner_designators(triple).into())
    }

    fn resolve(&mut self, name: &Symbol) -> Ret {
        let designation = self.env_state().designation();
        let env = self.env_state().env();

        let node = if let Ok(table) = <&mut SymbolTable>::try_from(env.node_structure(designation))
        {
            if let Some(node) = table.lookup(name) {
                *node
            } else {
                return Err(EvalErr::UnboundSymbol(name.clone()));
            }
        } else {
            panic!("Env designation isn't a symbol table");
        };

        Ok(node.into())
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
                    let fun: Sexp = BuiltIn::new(stringify!($x), $x).into();
                    $self.env_insert_node(&$n.to_symbol_or_panic(), Some(&fun)).unwrap();
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

            let result = self.eval(&sexp);
            match result {
                Ok(val) => {
                    println!("-> {}", val);
                }
                Err(err) => {
                    println!(" {}", err);
                }
            }
            println!();
        }
    }

    fn env_state(&mut self) -> &mut EnvState {
        &mut self.env_state
    }
}

impl Designation for AmlangAgent {
    fn designate(&mut self, designator: &Primitive) -> Ret {
        let node = if let Primitive::Symbol(symbol) = designator {
            <NodeId>::try_from(self.resolve(symbol)?).unwrap()
        } else if let Primitive::Node(node) = designator {
            *node
        } else {
            // Base case for self-designating.
            return Ok(designator.clone().into());
        };

        if let Some(structure) = self.env_state().env().node_structure(node) {
            Ok(structure.clone())
        } else {
            // Atoms are self-designating; retain original context of Symbol or Node.
            Ok(designator.clone().into())
        }
    }
}

impl Eval for AmlangAgent {
    fn eval(&mut self, structure: &Sexp) -> Ret {
        match structure {
            Sexp::Primitive(primitive) => {
                return self.designate(primitive);
            }

            Sexp::Cons(cons) => {
                let car = match cons.car() {
                    Some(car) => car,
                    None => return Err(InvalidSexp(cons.clone().into())),
                };

                if let Ok(first) = <&Symbol>::try_from(car) {
                    match first.as_str() {
                        "quote" => {
                            return syntax::quote(cons.cdr());
                        }
                        "def" => {
                            return self.env_insert_node_wrapper(cons.cdr());
                        }
                        "tell" => {
                            return self.env_insert_triple_wrapper(cons.cdr());
                        }
                        _ => { /* Fallthrough */ }
                    }
                }

                if let Ok(builtin) = <BuiltIn>::try_from(self.eval(car)?) {
                    let args = syntax::evlis(cons.cdr(), self)?;
                    return builtin.call(&args);
                }
                return Err(InvalidArgument {
                    given: structure.clone(),
                    expected: Cow::Borrowed("special form or functional application"),
                });
            }
        }
    }
}
