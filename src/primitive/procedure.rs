use std::convert::TryFrom;

use serde::{Deserialize, Serialize};

use super::{Node, Primitive};
use crate::agent::lang_error::{ExpectedCount, LangError};
use crate::agent::Agent;
use crate::error::Error;
use crate::model::Reflective;
use crate::sexp::{Cons, HeapSexp, Sexp};


#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Procedure {
    Application(Node, Vec<Node>),
    Abstraction(Vec<Node>, Node, bool),
    Sequence(Vec<Node>),
    Branch(Box<(Node, Node, Node)>), // Pred, A, B.
}


impl Reflective for Procedure {
    fn reify(&self, agent: &Agent) -> Sexp {
        let context = agent.context();
        match self {
            Procedure::Application(func, args) => {
                let apply_node = amlang_node!(apply, context);
                list!(apply_node, *func, args)
            }
            Procedure::Abstraction(params, body, reflect) => {
                let special_node = if *reflect {
                    amlang_node!(fexpr, context)
                } else {
                    amlang_node!(lambda, context)
                };
                list!(special_node, params, *body)
            }
            Procedure::Sequence(seq) => {
                let progn_node = amlang_node!(progn, context);
                Cons::new(progn_node, Sexp::from(seq)).into()
            }
            Procedure::Branch(t) => {
                let branch_node = amlang_node!(branch, context);
                let (pred, a, b) = **t;
                list!(branch_node, pred, a, b)
            }
        }
    }

    fn reflect<F>(structure: Sexp, agent: &Agent, resolve: F) -> Result<Self, Error>
    where
        F: Fn(&Agent, &Primitive) -> Result<Node, Error>,
    {
        let (command, cdr) = break_sexp!(structure => (Primitive; remainder), agent)?;
        let node = resolve(agent, &command)?;
        let context = agent.context();
        if !Self::valid_discriminator(node, agent) {
            return err!(
                agent,
                LangError::InvalidArgument {
                    given: command.into(),
                    expected: "Procedure variant".into()
                }
            );
        }

        if node.local() == context.apply() {
            if cdr.is_none() {
                return err!(
                    agent,
                    LangError::WrongArgumentCount {
                        given: 0,
                        expected: ExpectedCount::Exactly(2),
                    }
                );
            }

            let (func, args) = break_sexp!(cdr.unwrap() => (Primitive, HeapSexp), agent)?;
            let fnode = resolve(agent, &func)?;
            let mut arg_nodes = Vec::with_capacity(args.iter().count());
            for (arg, proper) in args {
                if !proper {
                    return err!(agent, LangError::InvalidSexp(*arg));
                }
                if let Ok(p) = <&Primitive>::try_from(&*arg) {
                    arg_nodes.push(resolve(agent, &p)?);
                } else {
                    return err!(agent, LangError::InvalidSexp(*arg));
                }
            }
            Ok(Procedure::Application(fnode, arg_nodes).into())
        } else if node.local() == context.lambda() || node.local() == context.fexpr() {
            if cdr.is_none() {
                return err!(
                    agent,
                    LangError::WrongArgumentCount {
                        given: 0,
                        expected: ExpectedCount::AtLeast(2),
                    }
                );
            }

            let reflect = node.local() == context.fexpr();
            let (params, body) = break_sexp!(cdr.unwrap() => (HeapSexp, Primitive), agent)?;
            let mut param_nodes = Vec::with_capacity(params.iter().count());
            for (param, proper) in params {
                if !proper {
                    return err!(agent, LangError::InvalidSexp(*param));
                }
                if let Ok(p) = <&Primitive>::try_from(&*param) {
                    param_nodes.push(resolve(agent, &p)?);
                } else {
                    return err!(agent, LangError::InvalidSexp(*param));
                }
            }
            let body_node = resolve(agent, &body)?;
            Ok(Procedure::Abstraction(param_nodes, body_node, reflect).into())
        } else if node.local() == context.progn() {
            let mut seq = vec![];
            if cdr.is_some() {
                for (sexp, proper) in cdr.unwrap().into_iter() {
                    if !proper {
                        return err!(
                            agent,
                            LangError::InvalidArgument {
                                given: *sexp,
                                expected: "list of Procedure nodes".into()
                            }
                        );
                    }
                    match *sexp {
                        Sexp::Primitive(p) => seq.push(resolve(agent, &p)?),
                        Sexp::Cons(c) => return err!(agent, LangError::InvalidSexp(c.into())),
                    }
                }
            }
            Ok(Procedure::Sequence(seq).into())
        } else if node.local() == context.branch() {
            if cdr.is_none() {
                return err!(
                    agent,
                    LangError::WrongArgumentCount {
                        given: 0,
                        expected: ExpectedCount::Exactly(3),
                    }
                );
            }

            let (pred, a, b) =
                break_sexp!(cdr.unwrap() => (Primitive, Primitive, Primitive), agent)?;
            Ok(Procedure::Branch(Box::new((
                resolve(agent, &pred)?,
                resolve(agent, &a)?,
                resolve(agent, &b)?,
            )))
            .into())
        } else {
            panic!()
        }
    }

    fn valid_discriminator(node: Node, agent: &Agent) -> bool {
        let context = agent.context();
        if node.env() != context.lang_env() {
            return false;
        }

        let local = node.local();
        return local == context.apply()
            || local == context.lambda()
            || local == context.fexpr()
            || local == context.progn()
            || local == context.branch();
    }
}


impl_try_from!(Procedure;
               Primitive         ->  Procedure,
               Sexp              ->  Procedure,
               HeapSexp          ->  Procedure,
               ref Sexp          ->  ref Procedure,
               Option<Sexp>      ->  Procedure,
               Option<ref Sexp>  ->  ref Procedure,
               Result<Sexp>      ->  Procedure,
               Result<ref Sexp>  ->  ref Procedure,
);
