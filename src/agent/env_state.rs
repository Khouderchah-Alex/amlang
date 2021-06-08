use std::convert::TryFrom;
use std::sync::Arc;

use super::amlang_context::AmlangContext;
use crate::environment::environment::EnvObject;
use crate::environment::NodeId;
use crate::function::EvalErr::{self, *};
use crate::primitive::{Primitive, Symbol, SymbolTable, ToSymbol};
use crate::sexp::{HeapSexp, Sexp};


#[derive(Clone)]
pub struct EnvState {
    env: NodeId,
    pos: NodeId,

    context: Arc<AmlangContext>,
}

pub const AMLANG_DESIGNATION: &str = "__designatedBy";

impl EnvState {
    pub fn new(context: Arc<AmlangContext>, pos: NodeId) -> Self {
        Self {
            env: context.base_env(),
            pos,
            context,
        }
    }

    pub fn pos(&self) -> NodeId {
        self.pos
    }

    pub fn designation(&self) -> NodeId {
        self.context.designation()
    }

    pub fn jump(&mut self, node: NodeId) {
        // TODO(sec) Verify.
        self.pos = node;
    }

    // TODO(func) impl
    // pub fn teleport(&mut self, portal: Portal)

    pub fn env(&mut self) -> &mut EnvObject {
        let meta = self.context.meta();
        meta.access_env(self.env)
    }

    pub fn node_designator(&mut self, node: NodeId) -> Option<HeapSexp> {
        let designation = self.designation();
        if node == designation {
            return Some(HeapSexp::new(
                AMLANG_DESIGNATION.to_symbol_or_panic().into(),
            ));
        }

        let env = self.env();
        let names = env.match_but_object(node, designation);
        if let Some(name_node) = names.iter().next() {
            let name = env.triple_object(*name_node);
            return Some(HeapSexp::new(env.node_structure(name).cloned().unwrap()));
        }
        None
    }

    pub fn resolve(&mut self, name: &Symbol) -> Result<NodeId, EvalErr> {
        let designation = self.designation();
        let env = self.env();

        let table = <&mut SymbolTable>::try_from(env.node_structure(designation)).unwrap();
        let node = table.lookup(name)?;
        Ok(node.into())
    }

    pub fn designate(&mut self, designator: Primitive) -> Result<Sexp, EvalErr> {
        match designator {
            // Symbol -> Node
            Primitive::Symbol(symbol) => Ok(self.resolve(&symbol)?.into()),
            // Node -> Structure
            Primitive::Node(node) => {
                if let Some(structure) = self.env().node_structure(node) {
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

    pub fn def_node(
        &mut self,
        name: Symbol,
        structure: Option<HeapSexp>,
    ) -> Result<NodeId, EvalErr> {
        let designation = self.designation();

        if let Ok(table) = <&mut SymbolTable>::try_from(self.env().node_structure(designation)) {
            if table.contains_key(&name) {
                return Err(AlreadyBoundSymbol(name));
            }
        } else {
            panic!("Env designation isn't a symbol table");
        }

        let name_node = self.env().insert_structure(name.clone().into());
        let node = if let Some(sexp) = structure {
            self.env().insert_structure(*sexp)
        } else {
            self.env().insert_atom()
        };

        if let Ok(table) = <&mut SymbolTable>::try_from(self.env().node_structure(designation)) {
            table.insert(name.clone(), node);
        } else {
            panic!("Env designation isn't a symbol table");
        }

        self.env().insert_triple(node, designation, name_node);
        Ok(node)
    }
}
