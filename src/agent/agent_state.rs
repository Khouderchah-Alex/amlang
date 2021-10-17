use colored::*;
use log::debug;
use std::borrow::Cow;
use std::collections::btree_map::Entry;
use std::collections::VecDeque;
use std::convert::TryFrom;
use std::io::{stdout, BufWriter};

use super::amlang_context::AmlangContext;
use super::continuation::Continuation;
use crate::environment::environment::{EnvObject, TripleSet};
use crate::environment::LocalNode;
use crate::model::Reflective;
use crate::primitive::prelude::*;
use crate::primitive::symbol_policies::policy_admin;
use crate::primitive::table::{AmlangTable, Table};
use crate::sexp::Sexp;


#[derive(Clone, Debug)]
pub struct AgentState {
    env_state: Continuation<EnvFrame>,
    exec_state: Continuation<ExecFrame>,
    designation_chain: VecDeque<LocalNode>,

    context: AmlangContext,
}

pub const AMLANG_DESIGNATION: &str = "__designatedBy";

// TODO(func) Allow for more than dynamic Node lookups (e.g. static tables).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecFrame {
    context: Node,
    map: AmlangTable<Node, Node>,
}

#[derive(Clone, Debug)]
struct EnvFrame {
    pos: Node,
}


impl AgentState {
    pub fn new(pos: Node, context: AmlangContext) -> Self {
        let env_state = Continuation::new(EnvFrame { pos });
        // TODO(func) Provide better root node.
        let exec_state = Continuation::new(ExecFrame::new(pos));
        Self {
            env_state,
            exec_state,
            designation_chain: VecDeque::new(),
            context,
        }
    }

    pub fn context(&self) -> &AmlangContext {
        &self.context
    }
    pub fn context_mut(&mut self) -> &mut AmlangContext {
        &mut self.context
    }
}

// Env-state-only functionality.
impl AgentState {
    pub fn globalize(&self, local: LocalNode) -> Node {
        Node::new(self.pos().env(), local)
    }

    pub fn pos(&self) -> Node {
        self.env_state.top().pos
    }
    fn pos_mut(&mut self) -> &mut Node {
        &mut self.env_state.top_mut().pos
    }

    pub fn jump(&mut self, node: Node) {
        // TODO(sec) Verify.
        *self.pos_mut() = node;
    }

    /// Jump to self node of indicated env.
    pub fn jump_env(&mut self, env_node: LocalNode) {
        // TODO(sec) Verify.
        let node = Node::new(env_node, self.context.self_node());
        *self.pos_mut() = node;
    }
}

// Designation-state-only functionality.
impl AgentState {
    pub fn designation_chain(&self) -> &VecDeque<LocalNode> {
        &self.designation_chain
    }
    // AgentState does not currently contain any policy; Agents populate this as
    // needed.
    // TODO(func, sec) Provide dedicated interface for d-chain mutations.
    pub fn designation_chain_mut(&mut self) -> &mut VecDeque<LocalNode> {
        &mut self.designation_chain
    }
}

// Exec-state-only functionality.
impl AgentState {
    pub fn exec_state(&self) -> &Continuation<ExecFrame> {
        &self.exec_state
    }
    pub fn exec_state_mut(&mut self) -> &mut Continuation<ExecFrame> {
        &mut self.exec_state
    }

    pub fn concretize(&self, node: Node) -> Node {
        for frame in self.exec_state().iter() {
            if let Some(n) = frame.lookup(node) {
                debug!("concretizing: {} -> {}", node, n);
                return n;
            }
        }
        node
    }
}


// Core functionality.
impl AgentState {
    pub fn access_env(&mut self, meta_node: LocalNode) -> Option<&mut Box<EnvObject>> {
        let meta = self.context.meta_mut();
        if meta_node == LocalNode::default() {
            return Some(meta);
        }

        meta.node_structure_mut(meta_node).env()
    }

    pub fn env(&mut self) -> &mut Box<EnvObject> {
        // Note(sec) Verification of jumps makes this unwrap safe *if*
        // we can assume that env nodes will not have their structures
        // changed to non-envs. If needed, this can be implemented
        // through EnvPolicy and/or Entry impls.
        self.access_env(self.pos().env()).unwrap()
    }

    pub fn node_designator(&mut self, node: Node) -> Option<Symbol> {
        let designation = self.context().designation();
        if node.local() == designation {
            return Some(AMLANG_DESIGNATION.to_symbol_or_panic(policy_admin));
        }

        let env = self.access_env(node.env()).unwrap();
        let names = env.match_but_object(node.local(), designation);
        if let Some(name_node) = names.iter().next() {
            let name = env.triple_object(*name_node);
            if let Ok(sym) = Symbol::try_from(env.node_structure(name).owned().unwrap()) {
                return Some(sym);
            }
        }
        None
    }

    pub fn resolve(&mut self, name: &Symbol) -> Result<Node, Error> {
        let designation = self.context().designation();
        // Always get self_* nodes from current env.
        match name.as_str() {
            "self_env" => return Ok(Node::new(self.pos().env(), LocalNode::default())),
            "self_des" => return Ok(Node::new(self.pos().env(), designation)),
            _ => {}
        }

        for i in 0..self.designation_chain.len() {
            let env = self.access_env(self.designation_chain[i]).unwrap();
            let entry = env.node_structure(designation);
            let table = <&SymbolTable>::try_from(entry.as_option()).unwrap();
            if let Some(node) = table.lookup(name) {
                return Ok(node);
            }
        }
        err!(self, UnboundSymbol(name.clone()))
    }

    pub fn designate(&mut self, designator: Primitive) -> Result<Sexp, Error> {
        match designator {
            // Symbol -> Node
            Primitive::Symbol(symbol) => Ok(self.resolve(&symbol)?.into()),
            // Node -> Structure
            Primitive::Node(node) => {
                if let Some(structure) = self
                    .access_env(node.env())
                    .unwrap()
                    .node_structure(node.local())
                    .owned()
                {
                    Ok(structure)
                } else if let Some(triple) = self
                    .access_env(node.env())
                    .unwrap()
                    .node_as_triple(node.local())
                {
                    Ok(triple.reify(self))
                } else {
                    // Atoms are self-designating.
                    Ok(node.into())
                }
            }
            // Reify Reflectives.
            Primitive::Procedure(proc) => Ok(proc.reify(self)),
            Primitive::SymbolTable(table) => Ok(table.reify(self)),
            Primitive::LocalNodeTable(table) => Ok(table.reify(self)),
            // Base case for self-designating.
            _ => Ok(designator.clone().into()),
        }
    }

    pub fn name_node(&mut self, name: LocalNode, node: LocalNode) -> Result<Node, Error> {
        let name_sexp = self.env().node_structure(name);
        let symbol = if let Ok(symbol) = <Symbol>::try_from(name_sexp.owned()) {
            symbol
        } else {
            return err!(
                self,
                InvalidArgument {
                    given: self
                        .env()
                        .node_structure(name)
                        .owned()
                        .unwrap_or(Sexp::default()),
                    expected: Cow::Borrowed("Node abstracting Symbol"),
                }
            );
        };

        // TODO(func) This prevents us from using an existing designation
        // anywhere in the chain. Perhaps we should allow "overriding"
        // designations; that is, only fail if the designation exists earlier in
        // the chain than the current environment.
        if let Ok(_) = self.resolve(&symbol) {
            return err!(self, AlreadyBoundSymbol(symbol));
        }

        let global_node = node.globalize(&self);

        let designation = self.context().designation();
        // Use designation of current environment.
        if let Ok(table) =
            <&mut SymbolTable>::try_from(self.env().node_structure_mut(designation).as_option())
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
    ) -> Result<Sexp, Error> {
        if let Some(triple) = self
            .env()
            .match_triple(subject, predicate, object)
            .iter()
            .next()
        {
            return err!(self, DuplicateTriple(triple.reify(self)));
        }

        let triple = self.env().insert_triple(subject, predicate, object);
        Ok(triple.node().globalize(&self).into())
    }

    pub fn ask(
        &mut self,
        subject: LocalNode,
        predicate: LocalNode,
        object: LocalNode,
    ) -> Result<Sexp, Error> {
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

    pub fn import(&mut self, original: Node) -> Result<Node, Error> {
        if original.env() == self.pos().env() {
            return err!(
                self,
                InvalidArgument {
                    given: original.into(),
                    expected: Cow::Borrowed("Node outside of current env"),
                }
            );
        }

        let imports_node = self.context.imports;
        let import_table_node = self.context.import_table;
        let env = self.pos().env();
        let import_triple = {
            let meta = self.context.meta_mut();
            if let Some(triple) = meta.match_triple(env, imports_node, original.env()) {
                triple
            } else {
                meta.insert_triple(env, imports_node, original.env())
            }
        };

        let matches = self
            .context
            .meta()
            .match_but_object(import_triple.node(), import_table_node);
        let table_node = match matches.len() {
            0 => {
                let table = LocalNodeTable::default().into();
                let table_node = self.context.meta_mut().insert_structure(table);
                self.context.meta_mut().insert_triple(
                    import_triple.node(),
                    import_table_node,
                    table_node,
                );
                table_node
            }
            1 => self
                .context
                .meta()
                .triple_object(*matches.iter().next().unwrap()),
            _ => panic!("Found multiple import tables for single import triple"),
        };

        if let Ok(table) =
            <&LocalNodeTable>::try_from(self.context.meta().node_structure(table_node).as_option())
        {
            if let Some(imported) = table.lookup(&original.local()) {
                return Ok(imported.globalize(&self));
            }
        } else {
            return err!(
                self,
                InvalidState {
                    actual: Cow::Borrowed("import table triple object has no table"),
                    expected: Cow::Borrowed("has table"),
                }
            );
        };

        let imported = self.env().insert_structure(original.into());
        if let Ok(table) = <&mut LocalNodeTable>::try_from(
            self.context
                .meta_mut()
                .node_structure_mut(table_node)
                .as_option(),
        ) {
            table.insert(original.local(), imported);
        } else {
            return err!(
                self,
                InvalidState {
                    actual: Cow::Borrowed("import table triple object has no table"),
                    expected: Cow::Borrowed("has table"),
                }
            );
        };
        Ok(imported.globalize(&self))
    }

    pub fn find_env<S: AsRef<str>>(&self, s: S) -> Option<LocalNode> {
        let meta = self.context.meta();
        let triples = meta.match_predicate(self.context.serialize_path);
        for triple in triples {
            let object_node = meta.triple_object(triple);
            let entry = meta.node_structure(object_node);
            let object = entry.structure();
            if let Ok(path) = <&Path>::try_from(object) {
                if path.as_std_path().ends_with(s.as_ref()) {
                    return Some(meta.triple_subject(triple));
                }
            }
        }
        None
    }
}


// Print functionality.
impl AgentState {
    pub fn trace_error(&mut self, err: &Error) {
        if let Some(state) = err.state() {
            let mut stored_state = state.clone();
            std::mem::swap(self, &mut stored_state);
            println!("");
            println!("  --TRACE--");
            let end = state.exec_state().depth() - 1;
            for (i, frame) in state.exec_state().iter().enumerate() {
                if i == end {
                    break;
                }
                self.exec_state_mut().pop();
                print!("   {})  ", i);
                self.print_list(&frame.context().into());
                println!("");
            }
            std::mem::swap(self, &mut stored_state);
        }
    }

    pub fn print_list(&mut self, structure: &Sexp) {
        let mut writer = BufWriter::new(stdout());
        if let Err(err) = self.write_list_internal(&mut writer, structure, 0, true) {
            println!("print_list error: {:?}", err);
        }
    }

    // TODO(func) Make show_redirects & paren_color configurable & introspectable.
    fn write_list_internal<W: std::io::Write>(
        &mut self,
        w: &mut W,
        structure: &Sexp,
        depth: usize,
        show_redirects: bool,
    ) -> std::io::Result<()> {
        fn paren_color(depth: usize) -> (u8, u8, u8) {
            match depth % 6 {
                0 => (0, 255, 204),
                1 => (204, 51, 0),
                2 => (153, 255, 102),
                3 => (153, 102, 255),
                4 => (255, 255, 102),
                _ => (255, 179, 179),
            }
        }

        structure.write_list(
            w,
            depth,
            &mut |writer, primitive, depth| {
                self.write_primitive(writer, primitive, depth, show_redirects)
            },
            &mut |writer, paren, depth| {
                let (r, g, b) = paren_color(depth);
                write!(writer, "{}", paren.truecolor(r, g, b))
            },
        )
    }

    fn write_primitive<W: std::io::Write>(
        &mut self,
        w: &mut W,
        primitive: &Primitive,
        depth: usize,
        show_redirects: bool,
    ) -> std::io::Result<()> {
        const MAX_DEPTH: usize = 16;

        match primitive {
            Primitive::Node(raw_node) => {
                let node = self.concretize(*raw_node);
                // Print Nodes as their designators if possible.
                if let Some(sym) = self.node_designator(node) {
                    write!(w, "{}", sym.as_str())
                } else if let Some(triple) = self
                    .access_env(node.env())
                    .unwrap()
                    .node_as_triple(node.local())
                {
                    let s = triple.reify(self);
                    self.write_list_internal(w, &s, depth, show_redirects)
                } else {
                    let s = if let Some(structure) = self
                        .access_env(node.env())
                        .unwrap()
                        .node_structure(node.local())
                        .owned()
                    {
                        if show_redirects {
                            write!(w, "{}->", node)?;
                        }
                        structure
                    } else {
                        return write!(w, "{}", node);
                    };

                    // If we recurse unconditionally, cycles will cause stack
                    // overflows.
                    if s == node.into() || depth > MAX_DEPTH {
                        write!(w, "{}", node)
                    } else {
                        self.write_list_internal(w, &s, depth, show_redirects)
                    }
                }
            }
            Primitive::Procedure(procedure) => {
                let s = procedure.reify(self);
                self.write_list_internal(w, &s, depth, false)
            }
            Primitive::SymbolTable(table) => {
                let s = table.reify(self);
                self.write_list_internal(w, &s, depth, false)
            }
            Primitive::LocalNodeTable(table) => {
                let s = table.reify(self);
                self.write_list_internal(w, &s, depth, false)
            }
            _ => write!(w, "{}", primitive),
        }
    }
}


impl ExecFrame {
    pub fn new(context: Node) -> Self {
        Self {
            context,
            map: Default::default(),
        }
    }

    pub fn insert(&mut self, from: Node, to: Node) -> bool {
        let entry = self.map.entry(from);
        if let Entry::Occupied(..) = entry {
            false
        } else {
            entry.or_insert(to);
            true
        }
    }

    pub fn lookup(&self, key: Node) -> Option<Node> {
        self.map.lookup(&key)
    }

    pub fn context(&self) -> Node {
        self.context
    }
}
