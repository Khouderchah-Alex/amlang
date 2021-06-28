use std::borrow::Cow;
use std::collections::VecDeque;
use std::convert::TryFrom;
use std::sync::Arc;

use super::amlang_context::AmlangContext;
use crate::environment::environment::{EnvObject, TripleSet};
use crate::environment::NodeId;
use crate::function::EvalErr::{self, *};
use crate::model::Model;
use crate::primitive::{Primitive, Symbol, SymbolTable, ToSymbol};
use crate::sexp::{HeapSexp, Sexp};


#[derive(Clone)]
pub struct EnvState {
    env: NodeId,
    pos: NodeId,

    // Ordered list of Env Nodes.
    designation_chain: VecDeque<NodeId>,
    context: Arc<AmlangContext>,
}

pub const AMLANG_DESIGNATION: &str = "__designatedBy";

impl EnvState {
    pub fn new(env: NodeId, pos: NodeId, context: Arc<AmlangContext>) -> Self {
        Self {
            env,
            pos,
            designation_chain: VecDeque::new(),
            context,
        }
    }

    pub fn pos(&self) -> NodeId {
        self.pos
    }

    pub fn context(&self) -> &AmlangContext {
        &*self.context
    }

    pub fn designation(&self) -> NodeId {
        self.context.designation()
    }

    pub fn designation_chain(&self) -> &VecDeque<NodeId> {
        &self.designation_chain
    }

    // EnvState does not currently contain any policy; Agents populate this as
    // needed.
    // TODO(func, sec) Provide dedicated interface for d-chain mutations.
    pub fn designation_chain_mut(&mut self) -> &mut VecDeque<NodeId> {
        &mut self.designation_chain
    }

    pub fn jump(&mut self, node: NodeId) {
        // TODO(sec) Verify.
        self.pos = node;
    }

    pub fn jump_env(&mut self, node: NodeId) {
        // TODO(sec) Verify.
        self.env = node;
    }

    pub fn access_env(&mut self, meta_node: NodeId) -> Option<&mut EnvObject> {
        let meta = self.context.meta();
        if let Some(Sexp::Primitive(Primitive::Env(env))) = meta.node_structure(meta_node) {
            Some(env.as_mut())
        } else {
            None
        }
    }

    pub fn env(&mut self) -> &mut EnvObject {
        // TODO(sec) Verify.
        self.access_env(self.env).unwrap()
    }

    pub fn node_designator(&mut self, node: NodeId) -> Option<HeapSexp> {
        let designation = self.designation();
        if node == designation {
            return Some(HeapSexp::new(
                AMLANG_DESIGNATION.to_symbol_or_panic().into(),
            ));
        }

        for i in 0..self.designation_chain.len() {
            let env = self.access_env(self.designation_chain[i]).unwrap();
            let names = env.match_but_object(node, designation);
            if let Some(name_node) = names.iter().next() {
                let name = env.triple_object(*name_node);
                return Some(HeapSexp::new(env.node_structure(name).cloned().unwrap()));
            }
        }
        None
    }

    pub fn resolve(&mut self, name: &Symbol) -> Result<NodeId, EvalErr> {
        let designation = self.designation();

        for i in 0..self.designation_chain.len() {
            let env = self.access_env(self.designation_chain[i]).unwrap();
            let table = <&mut SymbolTable>::try_from(env.node_structure(designation)).unwrap();
            if let Ok(node) = table.lookup(name) {
                return Ok(node);
            }
        }
        Err(EvalErr::UnboundSymbol(name.clone()))
    }

    pub fn designate(&mut self, designator: Primitive) -> Result<Sexp, EvalErr> {
        match designator {
            // Symbol -> Node
            Primitive::Symbol(symbol) => Ok(self.resolve(&symbol)?.into()),
            // Node -> Structure
            Primitive::Node(node) => {
                if let Some(structure) = self.env().node_structure(node) {
                    Ok(structure.clone())
                } else if let Some(triple) = self.env().node_as_triple(node) {
                    Ok(*triple.generate_structure(self))
                } else {
                    // Atoms are self-designating.
                    Ok(node.into())
                }
            }
            // Base case for self-designating.
            _ => Ok(designator.clone().into()),
        }
    }

    pub fn def_node(&mut self, name: NodeId, structure: Option<NodeId>) -> Result<NodeId, EvalErr> {
        let name_sexp = self.env().node_structure(name);
        let symbol = if let Ok(symbol) = <Symbol>::try_from(name_sexp.cloned()) {
            symbol
        } else {
            return Err(InvalidArgument {
                given: self
                    .env()
                    .node_structure(name)
                    .cloned()
                    .unwrap_or(Sexp::default()),
                expected: Cow::Borrowed("Node abstracting Symbol"),
            });
        };

        // TODO(func) This prevents us from using an existing designation
        // anywhere in the chain. Perhaps we should allow "overriding"
        // designations; that is, only fail if the designation exists earlier in
        // the chain than the current environment.
        if let Ok(_) = self.resolve(&symbol) {
            return Err(AlreadyBoundSymbol(symbol));
        }

        let node = if let Some(node) = structure {
            node
        } else {
            self.env().insert_atom()
        };

        let designation = self.designation();
        // Use designation of current environment.
        if let Ok(table) = <&mut SymbolTable>::try_from(self.env().node_structure(designation)) {
            table.insert(symbol, node);
        } else {
            panic!("Env designation isn't a symbol table");
        }

        self.env().insert_triple(node, designation, name);
        Ok(node)
    }

    pub fn tell(
        &mut self,
        subject: NodeId,
        predicate: NodeId,
        object: NodeId,
    ) -> Result<Sexp, EvalErr> {
        if let Some(triple) = self
            .env()
            .match_triple(subject, predicate, object)
            .iter()
            .next()
        {
            return Err(EvalErr::DuplicateTriple(*triple.generate_structure(self)));
        }

        let triple = self.env().insert_triple(subject, predicate, object);
        Ok(triple.node().into())
    }

    pub fn ask(
        &mut self,
        subject: NodeId,
        predicate: NodeId,
        object: NodeId,
    ) -> Result<Sexp, EvalErr> {
        let res = if subject == self.context.placeholder {
            if predicate == self.context.placeholder {
                if object == self.context.placeholder {
                    self.env().match_all()
                } else {
                    self.env().match_object(object)
                }
            } else {
                if object == self.context.placeholder {
                    self.env().match_predicate(predicate)
                } else {
                    self.env().match_but_subject(predicate, object)
                }
            }
        } else {
            if predicate == self.context.placeholder {
                if object == self.context.placeholder {
                    self.env().match_subject(subject)
                } else {
                    self.env().match_but_predicate(subject, object)
                }
            } else {
                if object == self.context.placeholder {
                    self.env().match_but_object(subject, predicate)
                } else {
                    let mut set = TripleSet::new();
                    if let Some(triple) = self.env().match_triple(subject, predicate, object) {
                        set.insert(triple);
                    }
                    set
                }
            }
        }
        .into_iter()
        .map(|t| t.node().into())
        .collect::<Vec<Sexp>>();

        Ok(res.into())
    }
}
