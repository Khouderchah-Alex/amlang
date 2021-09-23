use std::borrow::Cow;
use std::convert::TryFrom;

use super::{Node, Primitive};
use crate::agent::agent_state::AgentState;
use crate::agent::amlang_context::AmlangContext;
use crate::lang_err::{ExpectedCount, LangErr};
use crate::model::Model;
use crate::sexp::{cons, Cons, HeapSexp, Sexp};


#[derive(Clone, Debug, PartialEq)]
pub enum Procedure {
    Application(Node, Vec<Node>),
    Abstraction(Vec<Node>, Node, bool),
    Sequence(Vec<Node>),
    Branch(Node, Node, Node), // Pred, A, B.
}


impl Model for Procedure {
    fn reify(&self, state: &mut AgentState) -> HeapSexp {
        let context = state.context();
        match self {
            Procedure::Application(func, args) => {
                let apply_node = Node::new(context.lang_env(), context.apply);
                list!(apply_node, *func, args,)
            }
            Procedure::Abstraction(params, body, reflect) => {
                let special_node = if *reflect {
                    Node::new(context.lang_env(), context.fexpr)
                } else {
                    Node::new(context.lang_env(), context.lambda)
                };
                list!(special_node, params, *body,)
            }
            Procedure::Sequence(seq) => {
                let progn_node = Node::new(context.lang_env(), context.progn);
                cons(
                    Some(HeapSexp::new(progn_node.into())),
                    Some(HeapSexp::new(seq.into())),
                )
                .unwrap()
            }
            Procedure::Branch(pred, a, b) => {
                let branch_node = Node::new(context.lang_env(), context.branch);
                list!(branch_node, *pred, *a, *b,)
            }
        }
    }

    fn reflect<F>(
        structure: HeapSexp,
        context: &mut AmlangContext,
        mut process_primitive: F,
    ) -> Result<Self, LangErr>
    where
        F: FnMut(&Primitive) -> Result<Node, LangErr>,
    {
        let (command, cdr) = break_by_types!(*structure, Primitive; remainder)?;
        let node = process_primitive(&command)?;
        if node.local() == context.apply {
            if cdr.is_none() {
                return err!(WrongArgumentCount {
                    given: 0,
                    expected: ExpectedCount::Exactly(2),
                });
            }

            let (func, args) = break_by_types!(*cdr.unwrap(), Primitive, Cons)?;
            let fnode = process_primitive(&func)?;
            let mut arg_nodes = Vec::with_capacity(args.iter().count());
            for arg in args {
                if let Ok(p) = <&Primitive>::try_from(&*arg) {
                    arg_nodes.push(process_primitive(&p)?);
                } else {
                    return err!(InvalidSexp(*arg));
                }
            }
            Ok(Procedure::Application(fnode, arg_nodes).into())
        } else if node.local() == context.lambda || node.local() == context.fexpr {
            if cdr.is_none() {
                return err!(WrongArgumentCount {
                    given: 0,
                    expected: ExpectedCount::AtLeast(2),
                });
            }

            let reflect = node.local() == context.fexpr;
            let (params, body) = break_by_types!(*cdr.unwrap(), Cons, Primitive)?;
            let mut param_nodes = Vec::with_capacity(params.iter().count());
            for param in params {
                if let Ok(p) = <&Primitive>::try_from(&*param) {
                    param_nodes.push(process_primitive(&p)?);
                } else {
                    return err!(InvalidSexp(*param));
                }
            }
            let body_node = process_primitive(&body)?;
            Ok(Procedure::Abstraction(param_nodes, body_node, reflect).into())
        } else if node.local() == context.progn {
            let mut seq = vec![];
            if cdr.is_some() {
                match *cdr.unwrap() {
                    Sexp::Cons(list) => {
                        for sexp in list.into_iter() {
                            match *sexp {
                                Sexp::Primitive(p) => seq.push(process_primitive(&p)?),
                                Sexp::Cons(c) => return err!(InvalidSexp(c.into())),
                            }
                        }
                    }
                    s @ _ => {
                        return err!(InvalidArgument {
                            given: s,
                            expected: Cow::Borrowed("list of Procedure nodes")
                        });
                    }
                }
            }
            Ok(Procedure::Sequence(seq).into())
        } else if node.local() == context.branch {
            if cdr.is_none() {
                return err!(WrongArgumentCount {
                    given: 0,
                    expected: ExpectedCount::Exactly(3),
                });
            }

            let (pred, a, b) = break_by_types!(*cdr.unwrap(), Primitive, Primitive, Primitive)?;
            Ok(Procedure::Branch(
                process_primitive(&pred)?,
                process_primitive(&a)?,
                process_primitive(&b)?,
            )
            .into())
        } else {
            err!(InvalidArgument {
                given: command.into(),
                expected: Cow::Borrowed("Procedure variant")
            })
        }
    }
}


impl_try_from!(Sexp              ->  Procedure,      Procedure;
               ref Sexp          ->  ref Procedure,  Procedure;
               Option<Sexp>      ->  Procedure,      Procedure;
               Option<ref Sexp>  ->  ref Procedure,  Procedure;
               Result<Sexp>      ->  Procedure,      Procedure;
               Result<ref Sexp>  ->  ref Procedure,  Procedure;
);
