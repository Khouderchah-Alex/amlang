use colored::*;
use log::debug;
use std::collections::btree_map::Entry;
use std::collections::VecDeque;
use std::convert::TryFrom;
use std::io::{self, stdout, BufWriter};

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
#[derive(Clone, Debug, PartialEq)]
pub struct ExecFrame {
    context: Node,
    map: AmlangTable<Node, Sexp>,
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

    pub fn concretize(&mut self, node: Node) -> Result<Sexp, Error> {
        for frame in self.exec_state().iter() {
            if let Some(s) = frame.lookup(node) {
                debug!("concretizing: {} -> {}", node, s);
                // TODO(perf) Shouldn't need to clone, but lifetime must be
                // constrained to that of ExecFrame.
                return Ok(s.clone());
            }
        }
        self.designate(node.into())
    }
}


// Core functionality.
impl AgentState {
    pub fn access_env(&mut self, meta_node: LocalNode) -> Option<&mut Box<EnvObject>> {
        let meta = self.context.meta_mut();
        if meta_node == LocalNode::default() {
            return Some(meta);
        }

        meta.entry_mut(meta_node).env()
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
            if let Ok(sym) = Symbol::try_from(env.entry(name).owned().unwrap()) {
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
            let entry = env.entry(designation);
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
                    .entry(node.local())
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
            _ => Ok(designator.into()),
        }
    }

    pub fn name_node(&mut self, name: Node, node: Node) -> Result<Node, Error> {
        if name.env() != node.env() {
            return err!(
                self,
                Unsupported("Cross-env triples are not currently supported".into())
            );
        }

        let name_sexp = self.access_env(name.env()).unwrap().entry(name.local());
        let symbol = match <Symbol>::try_from(name_sexp.owned()) {
            Ok(symbol) => symbol,
            Err(sexp) => {
                return err!(
                    self,
                    InvalidArgument {
                        given: sexp.unwrap_or(Sexp::default()),
                        expected: "Node abstracting Symbol".into(),
                    }
                );
            }
        };

        // TODO(func) This prevents us from using an existing designation
        // anywhere in the chain. Perhaps we should allow "overriding"
        // designations; that is, only fail if the designation exists earlier in
        // the chain than the current environment.
        if let Ok(_) = self.resolve(&symbol) {
            return err!(self, AlreadyBoundSymbol(symbol));
        }

        let designation = self.context().designation();
        // Use designation of current environment.
        if let Ok(table) =
            <&mut SymbolTable>::try_from(self.env().entry_mut(designation).as_option())
        {
            table.insert(symbol, node);
        } else {
            panic!("Env designation isn't a symbol table");
        }

        self.env()
            .insert_triple(node.local(), designation, name.local());
        Ok(node)
    }

    pub fn tell(&mut self, subject: Node, predicate: Node, object: Node) -> Result<Sexp, Error> {
        let to_local = |node: Node| {
            if node.env() != self.pos().env() {
                return err!(
                    self,
                    Unsupported("Cross-env triples are not currently supported".into())
                );
            }
            Ok(node.local())
        };
        let (s, p, o) = (to_local(subject)?, to_local(predicate)?, to_local(object)?);

        if let Some(triple) = self.env().match_triple(s, p, o).iter().next() {
            return err!(self, DuplicateTriple(triple.reify(self)));
        }

        let triple = self.env().insert_triple(s, p, o);
        Ok(triple.node().globalize(&self).into())
    }

    pub fn ask(&mut self, subject: Node, predicate: Node, object: Node) -> Result<Sexp, Error> {
        let to_local = |node: Node| {
            let placeholder = amlang_node!(self.context(), placeholder);
            if node != placeholder && node.env() != self.pos().env() {
                return err!(
                    self,
                    Unsupported("Cross-env triples are not currently supported".into())
                );
            }
            Ok(node.local())
        };
        let (s, p, o) = (to_local(subject)?, to_local(predicate)?, to_local(object)?);

        let res = if s == self.context.placeholder {
            if p == self.context.placeholder {
                if o == self.context.placeholder {
                    self.env().match_all()
                } else {
                    self.env().match_object(o)
                }
            } else {
                if o == self.context.placeholder {
                    self.env().match_predicate(p)
                } else {
                    self.env().match_but_subject(p, o)
                }
            }
        } else {
            if p == self.context.placeholder {
                if o == self.context.placeholder {
                    self.env().match_subject(s)
                } else {
                    self.env().match_but_predicate(s, o)
                }
            } else {
                if o == self.context.placeholder {
                    self.env().match_but_object(s, p)
                } else {
                    let mut set = TripleSet::new();
                    if let Some(triple) = self.env().match_triple(s, p, o) {
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
            return Ok(original);
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
            <&LocalNodeTable>::try_from(self.context.meta().entry(table_node).as_option())
        {
            if let Some(imported) = table.lookup(&original.local()) {
                return Ok(imported.globalize(&self));
            }
        } else {
            return err!(
                self,
                InvalidState {
                    actual: "import table triple object has no table".into(),
                    expected: "has table".into(),
                }
            );
        };

        let imported = self.env().insert_structure(original.into());
        let success = if let Ok(table) = <&mut LocalNodeTable>::try_from(
            self.context.meta_mut().entry_mut(table_node).as_option(),
        ) {
            table.insert(original.local(), imported);
            true
        } else {
            false
        };
        if success {
            Ok(imported.globalize(&self))
        } else {
            err!(
                self,
                InvalidState {
                    actual: "import table triple object has no table".into(),
                    expected: "has table".into(),
                }
            )
        }
    }

    pub fn find_env<S: AsRef<str>>(&self, s: S) -> Option<LocalNode> {
        let meta = self.context.meta();
        let triples = meta.match_predicate(self.context.serialize_path);
        for triple in triples {
            let object_node = meta.triple_object(triple);
            let entry = meta.entry(object_node);
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
            let mut original_state = std::mem::replace(self, state.clone());
            println!("");
            println!("  --TRACE--");
            let end = state.exec_state().depth() - 1;
            for (i, frame) in state.exec_state().iter().enumerate() {
                if i == end {
                    break;
                }
                self.exec_state_mut().pop();
                print!("   {})  ", i);
                self.print_sexp(&frame.context().into());
                println!("");
            }
            std::mem::swap(self, &mut original_state);
        }
    }

    pub fn print_sexp(&mut self, structure: &Sexp) {
        let mut writer = BufWriter::new(stdout());
        if let Err(err) = self.write_sexp(&mut writer, structure, 0, true) {
            println!("print_sexp error: {:?}", err);
        }
    }

    // TODO(func) Make show_redirects & paren_color configurable & introspectable.
    fn write_sexp<W: std::io::Write>(
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

        // Any list longer than this will simply be suffixed with "..." after these
        // many elements.
        const MAX_LENGTH: usize = 64;
        const MAX_DEPTH: usize = 16;

        structure.write(
            w,
            depth,
            &mut |writer, primitive, depth| {
                self.write_primitive(writer, primitive, depth, show_redirects)
            },
            &mut |writer, paren, depth| {
                let (r, g, b) = paren_color(depth);
                write!(writer, "{}", paren.truecolor(r, g, b))
            },
            Some(MAX_LENGTH),
            Some(MAX_DEPTH),
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
            Primitive::Node(node) => {
                // Print Nodes as their designators if possible.
                if let Some(sym) = self.node_designator(*node) {
                    write!(w, "{}", sym.as_str())
                } else if let Some(triple) = self
                    .access_env(node.env())
                    .unwrap()
                    .node_as_triple(node.local())
                {
                    let s = triple.reify(self);
                    self.write_sexp(w, &s, depth, show_redirects)
                } else {
                    let s = match self.concretize(*node) {
                        Ok(structure) => structure,
                        Err(err) => {
                            return Err(io::Error::new(io::ErrorKind::Other, err.to_string()));
                        }
                    };
                    if show_redirects {
                        write!(w, "{}->", node)?;
                    } else {
                        return write!(w, "{}", node);
                    }

                    // If we recurse unconditionally, cycles will cause stack
                    // overflows.
                    if s == (*node).into() || depth > MAX_DEPTH {
                        write!(w, "{}", node)
                    } else {
                        self.write_sexp(w, &s, depth, show_redirects)
                    }
                }
            }
            Primitive::Procedure(procedure) => {
                let s = procedure.reify(self);
                self.write_sexp(w, &s, depth, false)
            }
            Primitive::SymbolTable(table) => {
                let s = table.reify(self);
                self.write_sexp(w, &s, depth, false)
            }
            Primitive::LocalNodeTable(table) => {
                let s = table.reify(self);
                self.write_sexp(w, &s, depth, false)
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

    pub fn insert(&mut self, from: Node, to: Sexp) -> bool {
        let entry = self.map.entry(from);
        if let Entry::Occupied(..) = entry {
            false
        } else {
            entry.or_insert(to);
            true
        }
    }

    pub fn lookup(&self, key: Node) -> Option<&Sexp> {
        self.map.as_map().get(&key)
    }

    pub fn context(&self) -> Node {
        self.context
    }
}
