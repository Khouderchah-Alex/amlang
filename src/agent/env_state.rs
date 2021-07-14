use std::borrow::Cow;
use std::collections::VecDeque;
use std::convert::TryFrom;
use std::sync::Arc;

use super::amlang_context::AmlangContext;
use crate::environment::environment::{EnvObject, TripleSet};
use crate::environment::LocalNode;
use crate::function::EvalErr::{self, *};
use crate::model::Model;
use crate::primitive::symbol_policies::policy_admin;
use crate::primitive::{LocalNodeTable, Node, Path, Primitive, Symbol, SymbolTable, ToSymbol};
use crate::sexp::Sexp;


#[derive(Clone)]
pub struct EnvState {
    env: LocalNode,
    pos: LocalNode,

    // Ordered list of Env Nodes.
    designation_chain: VecDeque<LocalNode>,
    context: Arc<AmlangContext>,
}

pub const AMLANG_DESIGNATION: &str = "__designatedBy";

impl EnvState {
    pub fn new(env: LocalNode, pos: LocalNode, context: Arc<AmlangContext>) -> Self {
        Self {
            env,
            pos,
            designation_chain: VecDeque::new(),
            context,
        }
    }

    pub fn globalize(&self, local: LocalNode) -> Node {
        Node::new(self.env, local)
    }

    pub fn pos(&self) -> Node {
        Node::new(self.env, self.pos)
    }

    pub fn jump(&mut self, node: Node) {
        // TODO(sec) Verify.
        self.env = node.env();
        self.pos = node.local();
    }

    /// Jump to self node of indicated env.
    pub fn jump_env(&mut self, env_node: LocalNode) {
        // TODO(sec) Verify.
        self.env = env_node;
        self.pos = self.context.self_node();
    }


    pub fn context(&self) -> &AmlangContext {
        &*self.context
    }

    pub fn designation(&self) -> LocalNode {
        self.context.designation()
    }

    pub fn designation_chain(&self) -> &VecDeque<LocalNode> {
        &self.designation_chain
    }

    // EnvState does not currently contain any policy; Agents populate this as
    // needed.
    // TODO(func, sec) Provide dedicated interface for d-chain mutations.
    pub fn designation_chain_mut(&mut self) -> &mut VecDeque<LocalNode> {
        &mut self.designation_chain
    }

    pub fn access_env(&mut self, meta_node: LocalNode) -> Option<&mut EnvObject> {
        let meta = self.context.meta();
        if meta_node == LocalNode::default() {
            return Some(meta);
        }

        if let Some(Sexp::Primitive(Primitive::Env(env))) = meta.node_structure_mut(meta_node) {
            Some(env.as_mut())
        } else {
            None
        }
    }

    pub fn env(&mut self) -> &mut EnvObject {
        // TODO(sec) Verify.
        self.access_env(self.env).unwrap()
    }

    pub fn node_designator(&mut self, node: Node) -> Option<Symbol> {
        let designation = self.designation();
        if node.local() == designation {
            return Some(AMLANG_DESIGNATION.to_symbol_or_panic(policy_admin));
        }

        let env = self.access_env(node.env()).unwrap();
        let names = env.match_but_object(node.local(), designation);
        if let Some(name_node) = names.iter().next() {
            let name = env.triple_object(*name_node);
            if let Ok(sym) = Symbol::try_from(env.node_structure(name).cloned().unwrap()) {
                return Some(sym);
            }
        }
        None
    }

    pub fn resolve(&mut self, name: &Symbol) -> Result<Node, EvalErr> {
        let designation = self.designation();

        for i in 0..self.designation_chain.len() {
            let env = self.access_env(self.designation_chain[i]).unwrap();
            let table = <&SymbolTable>::try_from(env.node_structure(designation)).unwrap();
            if let Some(node) = table.lookup(name) {
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
                if let Some(structure) = self
                    .access_env(node.env())
                    .unwrap()
                    .node_structure(node.local())
                {
                    Ok(structure.clone())
                } else if let Some(triple) = self
                    .access_env(node.env())
                    .unwrap()
                    .node_as_triple(node.local())
                {
                    Ok(*triple.generate_structure(self))
                } else {
                    // Atoms are self-designating.
                    Ok(node.into())
                }
            }
            // Procedure -> Structure
            Primitive::Procedure(proc) => Ok(*proc.generate_structure(self)),
            // Base case for self-designating.
            _ => Ok(designator.clone().into()),
        }
    }

    pub fn name_node(&mut self, name: LocalNode, node: LocalNode) -> Result<Node, EvalErr> {
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

        let global_node = node.globalize(&self);

        let designation = self.designation();
        // Use designation of current environment.
        if let Ok(table) = <&mut SymbolTable>::try_from(self.env().node_structure_mut(designation))
        {
            table.insert(symbol, global_node);
        } else {
            panic!("Env designation isn't a symbol table");
        }

        self.env()
            .insert_triple(global_node.local(), designation, name);
        Ok(global_node)
    }

    pub fn tell(
        &mut self,
        subject: LocalNode,
        predicate: LocalNode,
        object: LocalNode,
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
        Ok(triple.node().globalize(&self).into())
    }

    pub fn ask(
        &mut self,
        subject: LocalNode,
        predicate: LocalNode,
        object: LocalNode,
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
        .map(|t| t.node().globalize(&self).into())
        .collect::<Vec<Sexp>>();

        Ok(res.into())
    }

    pub fn import(&mut self, original: Node) -> Result<Node, EvalErr> {
        if original.env() == self.pos().env() {
            return Err(InvalidArgument {
                given: original.into(),
                expected: Cow::Borrowed("Node outside of current env"),
            });
        }

        let meta = self.context.meta();
        let import_triple =
            meta.get_or_insert_triple(self.pos().env(), self.context.imports, original.env());
        let matches = meta.match_but_object(import_triple.node(), self.context.import_table);
        let table_node = match matches.len() {
            0 => {
                let table = LocalNodeTable::default().into();
                let table_node = meta.insert_structure(table);
                meta.insert_triple(import_triple.node(), self.context.import_table, table_node);
                table_node
            }
            1 => meta.triple_object(*matches.iter().next().unwrap()),
            _ => panic!("Found multiple import tables for single import triple"),
        };

        if let Ok(table) =
            <&LocalNodeTable>::try_from(self.context.meta().node_structure(table_node))
        {
            if let Some(imported) = table.lookup(&original.local()) {
                return Ok(imported.globalize(&self));
            }
        } else {
            return Err(InvalidState {
                actual: Cow::Borrowed("import table triple object has no table"),
                expected: Cow::Borrowed("has table"),
            });
        };

        let imported = self.env().insert_structure(original.into());
        if let Ok(table) =
            <&mut LocalNodeTable>::try_from(self.context.meta().node_structure_mut(table_node))
        {
            table.insert(original.local(), imported);
        } else {
            return Err(InvalidState {
                actual: Cow::Borrowed("import table triple object has no table"),
                expected: Cow::Borrowed("has table"),
            });
        };
        Ok(imported.globalize(&self))
    }

    pub fn find_env<S: AsRef<str>>(&self, s: S) -> Option<LocalNode> {
        let meta = self.context.meta();
        let triples = meta.match_predicate(self.context.serialize_path);
        for triple in triples {
            let object_node = meta.triple_object(triple);
            let object = meta.node_structure(object_node).unwrap();
            if let Ok(path) = <&Path>::try_from(object) {
                if path.as_std_path().ends_with(s.as_ref()) {
                    return Some(meta.triple_subject(triple));
                }
            }
        }
        None
    }
}
