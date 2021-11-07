use std::convert::TryFrom;

use super::{Node, Primitive};
use crate::agent::agent_state::AgentState;
use crate::agent::lang_error::{ExpectedCount, LangError};
use crate::error::Error;
use crate::model::Reflective;
use crate::sexp::{Cons, HeapSexp, Sexp};


#[derive(Clone, Debug, PartialEq)]
pub enum Procedure {
    Application(Node, Vec<Node>),
    Abstraction(Vec<Node>, Node, bool),
    Sequence(Vec<Node>),
    Branch(Box<(Node, Node, Node)>), // Pred, A, B.
}


impl Reflective for Procedure {
    fn reify(&self, state: &mut AgentState) -> Sexp {
        let context = state.context();
        match self {
            Procedure::Application(func, args) => {
                let apply_node = amlang_node!(context, apply);
                list!(apply_node, *func, args,)
            }
            Procedure::Abstraction(params, body, reflect) => {
                let special_node = if *reflect {
                    amlang_node!(context, fexpr)
                } else {
                    amlang_node!(context, lambda)
                };
                list!(special_node, params, *body,)
            }
            Procedure::Sequence(seq) => {
                let progn_node = amlang_node!(context, progn);
                Cons::new(
                    Some(HeapSexp::new(progn_node.into())),
                    Some(HeapSexp::new(seq.into())),
                )
                .into()
            }
            Procedure::Branch(t) => {
                let branch_node = amlang_node!(context, branch);
                let (pred, a, b) = **t;
                list!(branch_node, pred, a, b,)
            }
        }
    }

    fn reflect<F>(
        structure: Sexp,
        state: &mut AgentState,
        mut process_primitive: F,
    ) -> Result<Self, Error>
    where
        F: FnMut(&mut AgentState, &Primitive) -> Result<Node, Error>,
    {
        let (command, cdr) = break_sexp!(structure => (Primitive; remainder), state)?;
        let node = process_primitive(state, &command)?;
        let context = state.context();
        if !Self::valid_discriminator(node, state) {
            return err!(
                state,
                LangError::InvalidArgument {
                    given: command.into(),
                    expected: "Procedure variant".into()
                }
            );
        }

        if node.local() == context.apply {
            if cdr.is_none() {
                return err!(
                    state,
                    LangError::WrongArgumentCount {
                        given: 0,
                        expected: ExpectedCount::Exactly(2),
                    }
                );
            }

            let (func, args) = break_sexp!(cdr.unwrap() => (Primitive, HeapSexp), state)?;
            let fnode = process_primitive(state, &func)?;
            let mut arg_nodes = Vec::with_capacity(args.iter().count());
            for (arg, proper) in args {
                if !proper {
                    return err!(state, LangError::InvalidSexp(*arg));
                }
                if let Ok(p) = <&Primitive>::try_from(&*arg) {
                    arg_nodes.push(process_primitive(state, &p)?);
                } else {
                    return err!(state, LangError::InvalidSexp(*arg));
                }
            }
            Ok(Procedure::Application(fnode, arg_nodes).into())
        } else if node.local() == context.lambda || node.local() == context.fexpr {
            if cdr.is_none() {
                return err!(
                    state,
                    LangError::WrongArgumentCount {
                        given: 0,
                        expected: ExpectedCount::AtLeast(2),
                    }
                );
            }

            let reflect = node.local() == context.fexpr;
            let (params, body) = break_sexp!(cdr.unwrap() => (HeapSexp, Primitive), state)?;
            let mut param_nodes = Vec::with_capacity(params.iter().count());
            for (param, proper) in params {
                if !proper {
                    return err!(state, LangError::InvalidSexp(*param));
                }
                if let Ok(p) = <&Primitive>::try_from(&*param) {
                    param_nodes.push(process_primitive(state, &p)?);
                } else {
                    return err!(state, LangError::InvalidSexp(*param));
                }
            }
            let body_node = process_primitive(state, &body)?;
            Ok(Procedure::Abstraction(param_nodes, body_node, reflect).into())
        } else if node.local() == context.progn {
            let mut seq = vec![];
            if cdr.is_some() {
                for (sexp, proper) in cdr.unwrap().into_iter() {
                    if !proper {
                        return err!(
                            state,
                            LangError::InvalidArgument {
                                given: *sexp,
                                expected: "list of Procedure nodes".into()
                            }
                        );
                    }
                    match *sexp {
                        Sexp::Primitive(p) => seq.push(process_primitive(state, &p)?),
                        Sexp::Cons(c) => return err!(state, LangError::InvalidSexp(c.into())),
                    }
                }
            }
            Ok(Procedure::Sequence(seq).into())
        } else if node.local() == context.branch {
            if cdr.is_none() {
                return err!(
                    state,
                    LangError::WrongArgumentCount {
                        given: 0,
                        expected: ExpectedCount::Exactly(3),
                    }
                );
            }

            let (pred, a, b) =
                break_sexp!(cdr.unwrap() => (Primitive, Primitive, Primitive), state)?;
            Ok(Procedure::Branch(Box::new((
                process_primitive(state, &pred)?,
                process_primitive(state, &a)?,
                process_primitive(state, &b)?,
            )))
            .into())
        } else {
            panic!()
        }
    }

    fn valid_discriminator(node: Node, state: &AgentState) -> bool {
        let context = state.context();
        if node.env() != context.lang_env() {
            return false;
        }

        let local = node.local();
        return local == context.apply
            || local == context.lambda
            || local == context.fexpr
            || local == context.progn
            || local == context.branch;
    }
}


impl_try_from!(Sexp              ->  Procedure,      Procedure;
               HeapSexp          ->  Procedure,      Procedure;
               ref Sexp          ->  ref Procedure,  Procedure;
               Option<Sexp>      ->  Procedure,      Procedure;
               Option<ref Sexp>  ->  ref Procedure,  Procedure;
               Result<Sexp>      ->  Procedure,      Procedure;
               Result<ref Sexp>  ->  ref Procedure,  Procedure;
);
