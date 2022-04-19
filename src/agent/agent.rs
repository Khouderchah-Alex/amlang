use colored::*;
use log::debug;
use std::collections::VecDeque;
use std::convert::TryFrom;
use std::io::{self, stdout, BufWriter};

use super::agent_frames::{EnvFrame, ExecFrame, InterpreterState};
use super::amlang_context::AmlangContext;
use super::amlang_interpreter::AmlangState;
use super::continuation::Continuation;
use super::env_prelude::EnvPrelude;
use crate::agent::lang_error::LangError;
use crate::environment::environment::{EnvObject, TripleSet};
use crate::environment::LocalNode;
use crate::error::Error;
use crate::model::Reflective;
use crate::primitive::prelude::*;
use crate::primitive::symbol_policies::policy_admin;
use crate::primitive::table::Table;
use crate::sexp::Sexp;


#[derive(Clone, Debug)]
pub struct Agent {
    env_state: Continuation<EnvFrame>,
    exec_state: Continuation<ExecFrame>,
    interpreter_state: Continuation<Box<dyn InterpreterState>>,
    designation_chain: VecDeque<LocalNode>,

    history_env: LocalNode,

    context: AmlangContext,
}

pub struct RunIter<'a, S, F>
where
    F: FnMut(&mut Agent, &Result<Sexp, Error>),
{
    agent: &'a mut Agent,
    stream: S,
    handler: F,
}

impl Agent {
    pub fn new(pos: Node, context: AmlangContext, history_env: LocalNode) -> Self {
        let env_state = Continuation::new(EnvFrame { pos });
        // TODO(func) Provide better root node.
        let exec_state = Continuation::new(ExecFrame::new(pos));
        // Base interpretation as amlang.
        let interpreter_state: Continuation<Box<dyn InterpreterState>> =
            Continuation::new(Box::new(AmlangState::default()));
        Self {
            env_state,
            exec_state,
            interpreter_state,
            designation_chain: VecDeque::new(),
            // TODO(sec) Verify as env node.
            history_env,
            context,
        }
    }

    pub fn run<'a, S, F>(&'a mut self, stream: S, handler: F) -> RunIter<'a, S, F>
    where
        S: Iterator<Item = Result<Sexp, Error>>,
        F: FnMut(&mut Agent, &Result<Sexp, Error>),
    {
        RunIter {
            agent: self,
            stream,
            handler,
        }
    }

    pub fn context(&self) -> &AmlangContext {
        &self.context
    }
    pub fn context_mut(&mut self) -> &mut AmlangContext {
        &mut self.context
    }

    pub(super) fn set_history_env(&mut self, history_env: LocalNode) {
        // TODO(sec) Verify as env node.
        self.history_env = history_env;
    }

    pub fn history_insert(&mut self, structure: Sexp) -> Node {
        let local = self
            .access_env(self.history_env)
            .unwrap()
            .insert_structure(structure);
        Node::new(self.history_env, local)
    }
}

// Basic frame functionality.
impl Agent {
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

    pub fn designation_chain(&self) -> &VecDeque<LocalNode> {
        &self.designation_chain
    }
    // Agent does not currently contain any policy; Clients populate this as
    // needed.
    // TODO(func, sec) Provide dedicated interface for d-chain mutations.
    pub fn designation_chain_mut(&mut self) -> &mut VecDeque<LocalNode> {
        &mut self.designation_chain
    }

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

    pub fn interpreter_state(&self) -> &Continuation<Box<dyn InterpreterState>> {
        &self.interpreter_state
    }
    pub fn interpreter_state_mut(&mut self) -> &mut Continuation<Box<dyn InterpreterState>> {
        &mut self.interpreter_state
    }
}


// Core functionality.
impl Agent {
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

    /// Get the amlang designator of a Node, which is (contextually) an
    /// injective property.
    pub fn node_designator(&mut self, node: Node) -> Option<Symbol> {
        if let Some(prelude) = node.local().as_prelude() {
            return Some(prelude.name().to_symbol_or_panic(policy_admin));
        }

        let designation = self.context().designation();
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

    /// Get the label of a Node, which need not be injective.
    pub fn node_label(&mut self, node: Node) -> Option<Symbol> {
        let pos = self.pos();
        self.jump(node);
        let try_import = self.get_imported(amlang_node!(self.context(), label));
        self.jump(pos);
        let label_predicate = match try_import {
            Some(pred) => pred,
            None => return None,
        };
        let env = self.access_env(node.env()).unwrap();

        let labels = env.match_but_object(node.local(), label_predicate.local());
        if let Some(name_node) = labels.iter().next() {
            let name = env.triple_object(*name_node);
            if let Ok(sym) = Symbol::try_from(env.entry(name).owned().unwrap()) {
                return Some(sym);
            }
        }
        None
    }

    pub fn resolve(&mut self, name: &Symbol) -> Result<Node, Error> {
        // Always get prelude nodes from current env.
        if let Some(prelude) = EnvPrelude::from_name(name.as_str()) {
            return Ok(Node::new(self.pos().env(), prelude.local()));
        }

        let designation = self.context().designation();
        for i in 0..self.designation_chain.len() {
            let env = self.access_env(self.designation_chain[i]).unwrap();
            let entry = env.entry(designation);
            let table = <&SymNodeTable>::try_from(entry.as_option()).unwrap();
            if let Some(node) = table.lookup(name) {
                return Ok(node);
            }
        }
        err!(self, LangError::UnboundSymbol(name.clone()))
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
            Primitive::SymNodeTable(table) => Ok(table.reify(self)),
            Primitive::LocalNodeTable(table) => Ok(table.reify(self)),
            // Base case for self-designating.
            _ => Ok(designator.into()),
        }
    }

    pub fn name_node(&mut self, name: Node, node: Node) -> Result<Node, Error> {
        if name.env() != node.env() {
            return err!(
                self,
                LangError::Unsupported("Cross-env triples are not currently supported".into())
            );
        }

        let name_sexp = self.access_env(name.env()).unwrap().entry(name.local());
        let symbol = match <Symbol>::try_from(name_sexp.owned()) {
            Ok(symbol) => symbol,
            Err(sexp) => {
                return err!(
                    self,
                    LangError::InvalidArgument {
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
            return err!(self, LangError::AlreadyBoundSymbol(symbol));
        }

        let designation = self.context().designation();
        // Use designation of current environment.
        if let Ok(table) =
            <&mut SymNodeTable>::try_from(self.env().entry_mut(designation).as_option())
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
                    LangError::Unsupported("Cross-env triples are not currently supported".into())
                );
            }
            Ok(node.local())
        };
        let (s, p, o) = (to_local(subject)?, to_local(predicate)?, to_local(object)?);
        let original_pos = self.pos();

        if let Some(triple) = self.env().match_triple(s, p, o).iter().next() {
            return err!(self, LangError::DuplicateTriple(triple.reify(self)));
        }

        // If a tell_handler exists for the predicate, ensure it passes before adding triple.
        let tell_handler = self.context().tell_handler();
        if let Some(&handler_triple) = self.env().match_but_object(p, tell_handler).iter().next() {
            let handler_lnode = self.env().triple_object(handler_triple);
            let res = self.amlang_exec(
                Procedure::Application(
                    handler_lnode.globalize(self),
                    vec![subject, predicate, object],
                )
                .into(),
            )?;
            // Only allow insertion to continue if the handler returns true.
            if res != amlang_node!(self.context(), t).into() {
                return err!(
                    self,
                    LangError::RejectedTriple(list!(subject, predicate, object,), res)
                );
            }
        }

        // Note(sec) If the tell handler jumps to a different environment, the
        // local nodes will globalize into the wrong Environment.
        self.jump(original_pos);
        let triple = self.env().insert_triple(s, p, o);
        Ok(triple.node().globalize(&self).into())
    }

    pub fn ask(&mut self, subject: Node, predicate: Node, object: Node) -> Result<Sexp, Error> {
        let to_local = |node: Node| {
            let placeholder = amlang_node!(self.context(), placeholder);
            if node != placeholder && node.env() != self.pos().env() {
                return err!(
                    self,
                    LangError::Unsupported("Cross-env triples are not currently supported".into())
                );
            }
            Ok(node.local())
        };
        let (s, p, o) = (to_local(subject)?, to_local(predicate)?, to_local(object)?);

        let res = if s == self.context.placeholder() {
            if p == self.context.placeholder() {
                if o == self.context.placeholder() {
                    self.env().match_all()
                } else {
                    self.env().match_object(o)
                }
            } else {
                if o == self.context.placeholder() {
                    self.env().match_predicate(p)
                } else {
                    self.env().match_but_subject(p, o)
                }
            }
        } else {
            if p == self.context.placeholder() {
                if o == self.context.placeholder() {
                    self.env().match_subject(s)
                } else {
                    self.env().match_but_predicate(s, o)
                }
            } else {
                if o == self.context.placeholder() {
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

        let table_node = self.get_or_create_import_table(original.env());
        if let Ok(table) =
            <&LocalNodeTable>::try_from(self.context.meta().entry(table_node).as_option())
        {
            if let Some(imported) = table.lookup(&original.local()) {
                return Ok(imported.globalize(&self));
            }
        } else {
            return err!(
                self,
                LangError::InvalidState {
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
                LangError::InvalidState {
                    actual: "import table triple object has no table".into(),
                    expected: "has table".into(),
                }
            )
        }
    }

    pub fn get_imported(&mut self, original: Node) -> Option<Node> {
        if original.env() == self.pos().env() {
            return Some(original);
        }

        let table_node = self.get_import_table(original.env());
        if table_node.is_none() {
            return None;
        }
        if let Ok(table) =
            <&LocalNodeTable>::try_from(self.context.meta().entry(table_node.unwrap()).as_option())
        {
            if let Some(imported) = table.lookup(&original.local()) {
                return Some(imported.globalize(&self));
            }
        }
        return None;
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

    /// Execute |structure| as internal Amlang structure.
    ///
    /// Allows Agents broadly to leverage previous internalize() execution of
    /// AmlangInterpreters.
    pub fn amlang_exec(&mut self, structure: Sexp) -> Result<Sexp, Error> {
        // TODO(func) Push & pop on interpreter_state?
        let mut state = AmlangState::default();
        let mut amlang = state.borrow_agent(self);
        amlang.contemplate(structure)
    }

    fn get_or_create_import_table(&mut self, from_env: LocalNode) -> LocalNode {
        let imports_node = self.context.imports();
        let import_table_node = self.context.import_table();
        let env = self.pos().env();
        let import_triple = {
            let meta = self.context.meta_mut();
            if let Some(triple) = meta.match_triple(env, imports_node, from_env) {
                triple
            } else {
                meta.insert_triple(env, imports_node, from_env)
            }
        };

        let matches = self
            .context
            .meta()
            .match_but_object(import_triple.node(), import_table_node);
        match matches.len() {
            0 => {
                let table = LocalNodeTable::in_env(LocalNode::default()).into();
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
        }
    }

    fn get_import_table(&mut self, from_env: LocalNode) -> Option<LocalNode> {
        let imports_node = self.context.imports();
        let import_table_node = self.context.import_table();
        let env = self.pos().env();
        let import_triple = {
            let meta = self.context.meta_mut();
            if let Some(triple) = meta.match_triple(env, imports_node, from_env) {
                triple
            } else {
                return None;
            }
        };

        let matches = self
            .context
            .meta()
            .match_but_object(import_triple.node(), import_table_node);
        match matches.len() {
            0 => None,
            1 => Some(
                self.context
                    .meta()
                    .triple_object(*matches.iter().next().unwrap()),
            ),
            _ => panic!("Found multiple import tables for single import triple"),
        }
    }
}


// Print functionality.
impl Agent {
    pub fn trace_error(&mut self, err: &Error) {
        if let Some(agent) = err.agent() {
            let mut original_agent = std::mem::replace(self, agent.clone());
            println!("");
            println!("  --TRACE--");
            let end = agent.exec_state().depth() - 1;
            for (i, frame) in agent.exec_state().iter().enumerate() {
                if i == end {
                    break;
                }
                self.exec_state_mut().pop();
                print!("   {})  ", i);
                self.print_sexp(&frame.context().into());
                println!("");
            }
            std::mem::swap(self, &mut original_agent);
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
            &mut |writer, _depth| write!(writer, " "),
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
                } else if let Some(sym) = self.node_label(*node) {
                    write!(w, "${}", sym.as_str())
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
                self.write_sexp(w, &s, depth, true)
            }
            Primitive::SymNodeTable(table) => {
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

impl<'a, S, F> Iterator for RunIter<'a, S, F>
where
    S: Iterator<Item = Result<Sexp, Error>>,
    F: FnMut(&mut Agent, &Result<Sexp, Error>),
{
    type Item = Result<Sexp, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        let sexp = match self.stream.next() {
            None => return None,
            Some(Ok(sexp)) => sexp,
            Some(err) => {
                (self.handler)(&mut self.agent, &err);
                return Some(err);
            }
        };

        // Clone InterpreterState to reborrow the Agent. Modifying
        // |interpreter_state| is left to Interpreter impls.
        let mut state = self.agent.interpreter_state.top().clone();
        let mut interpreter = state.borrow_agent(self.agent);
        let res = match interpreter.internalize(sexp) {
            Ok(meaning) => interpreter.contemplate(meaning),
            err @ _ => err,
        };

        // Normally, rustc is happy reasoning that |interpreter| should be
        // dropped here to allow for another mut borrow on self to happen below.
        // IIUC, this isn't happening here because the Interpreter impl may impl
        // Drop as well, which prevents the compiler from dropping anywhere it
        // likes. Without negative trait bounds, we either need to explicitly
        // drop or scope |interpreter|.
        std::mem::drop(interpreter);

        (self.handler)(&mut self.agent, &res);
        Some(res)
    }
}
