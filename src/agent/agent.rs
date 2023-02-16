use colored::*;
use log::debug;
use std::collections::VecDeque;
use std::convert::TryFrom;
use std::io::{self, stdout, BufWriter};

use super::agent_frames::{EnvFrame, ExecFrame};
use super::amlang_context::AmlangContext;
use super::env_prelude::EnvPrelude;
use super::interpreter::{InterpreterState, NullInterpreter};
use super::AmlangInterpreter;
use crate::agent::lang_error::LangError;
use crate::agent::symbol_policies::policy_admin;
use crate::continuation::Continuation;
use crate::env::entry::EntryMutKind;
use crate::env::LocalNode;
use crate::env::{EnvObject, TripleSet};
use crate::error::Error;
use crate::model::Reflective;
use crate::primitive::prelude::*;
use crate::primitive::table::Table;
use crate::sexp::Sexp;


#[derive(Debug)]
pub struct Agent {
    env_state: Continuation<EnvFrame>,
    exec_state: Continuation<ExecFrame>,
    interpreter_state: Continuation<Box<dyn InterpreterState>>,
    designation_chain: VecDeque<LocalNode>,

    history_env: LocalNode,

    context: AmlangContext,
}

impl Agent {
    pub(super) fn new(pos: Node, context: AmlangContext, history_env: LocalNode) -> Self {
        let env_state = Continuation::new(EnvFrame { pos });
        // TODO(func) Provide better root node.
        let exec_state = Continuation::new(ExecFrame::new(pos));
        Self {
            env_state,
            exec_state,
            interpreter_state: Continuation::new(Box::new(NullInterpreter::default())),
            designation_chain: VecDeque::new(),
            // TODO(sec) Verify as env node.
            history_env,
            context,
        }
    }

    pub fn fork<I: InterpreterState + 'static>(&self, base_interpreter: I) -> Self {
        let mut res = Self::new(self.pos(), self.context.clone(), self.history_env);
        res.interpreter_state = Continuation::new(Box::new(base_interpreter));
        res
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
            .access_env_mut(self.history_env)
            .unwrap()
            .insert_structure(structure);
        Node::new(self.history_env, local)
    }
}

// Basic frame functionality.
impl Agent {
    // SAFETY: Clients must ensure the LocalNode is indeed a part of the Agent's
    // current env. While operational design can be used to prevent access to
    // sensitive information by unprivileged Agents, the ability to use the same
    // LocalNode in different envs is something we really want to avoid.
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

    pub fn concretize(&self, node: Node) -> Result<Sexp, Error> {
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

    pub fn top_interpret(&mut self, sexp: Sexp) -> Result<Sexp, Error> {
        // TODO(perf) Can we avoid this clone plz?
        let mut interpreter_state = self.interpreter_state().top().clone();
        let mut interpreter = interpreter_state.borrow_agent(self);

        let ir = interpreter.internalize(sexp);
        interpreter.contemplate(ir?)
    }
    pub fn sub_interpret(
        &mut self,
        sexp: Sexp,
        interpreter: Box<dyn InterpreterState>,
        _context: Node,
    ) -> Result<Sexp, Error> {
        self.interpreter_state.push(interpreter);
        let res = self.top_interpret(sexp);
        self.interpreter_state.pop();
        res
    }


    pub fn top_exec(&mut self, sexp: Sexp) -> Result<Sexp, Error> {
        // TODO(perf) Can we avoid this clone plz?
        let mut interpreter_state = self.interpreter_state().top().clone();
        let mut interpreter = interpreter_state.borrow_agent(self);

        interpreter.contemplate(sexp)
    }
    pub fn sub_exec(
        &mut self,
        sexp: Sexp,
        interpreter: Box<dyn InterpreterState>,
        _context: Node,
    ) -> Result<Sexp, Error> {
        self.interpreter_state.push(interpreter);
        let res = self.top_exec(sexp);
        self.interpreter_state.pop();
        res
    }
}


// Core functionality.
impl Agent {
    /// Access arbitrary env.
    ///
    /// Prefer env() over this if possible. Unwrapping the Option here is not
    /// safe a priori.
    pub fn access_env(&self, meta_node: LocalNode) -> Option<&Box<EnvObject>> {
        let meta = self.context.meta();
        if meta_node == LocalNode::default() {
            return Some(meta.base());
        }
        meta.env(meta_node)
    }

    /// Access arbitrary env.
    ///
    /// Prefer env_mut() over this if possible. Unwrapping the Option here is
    /// not safe a priori.
    pub fn access_env_mut(&mut self, meta_node: LocalNode) -> Option<&mut Box<EnvObject>> {
        let meta = self.context.meta_mut();
        if meta_node == LocalNode::default() {
            return Some(meta.base_mut());
        }
        meta.env_mut(meta_node)
    }

    /// Access current env.
    ///
    /// Prefer over direct access_env usage, but using
    /// define/set/concretize/ask/tell is best.
    ///
    /// Note(sec) Verification of jumps makes the unwrap safe *if*
    /// we can assume that env nodes will not have their structures
    /// changed to non-envs. If needed, this can be implemented
    /// through EnvPolicy and/or Entry impls.
    pub fn env(&self) -> &Box<EnvObject> {
        self.access_env(self.pos().env()).unwrap()
    }

    /// Access current env.
    ///
    /// Prefer over direct access_env_mut usage, but using
    /// define/set/concretize/ask/tell is best.
    ///
    /// Note(sec) Verification of jumps makes the unwrap safe *if*
    /// we can assume that env nodes will not have their structures
    /// changed to non-envs. If needed, this can be implemented
    /// through EnvPolicy and/or Entry impls.
    pub fn env_mut(&mut self) -> &mut Box<EnvObject> {
        self.access_env_mut(self.pos().env()).unwrap()
    }

    /// Get the amlang designator of a Node, which is (contextually) an
    /// injective property.
    pub fn node_designator(&self, node: Node) -> Option<Symbol> {
        if let Some(prelude) = node.local().as_prelude() {
            return Some(prelude.name().to_symbol_or_panic(policy_admin));
        }

        let designation = self.context().designation();
        let names = self
            .ask_from(
                node.env(),
                Some(node),
                Some(Node::new(node.env(), designation)),
                None,
            )
            .unwrap();
        if let Some(name) = names.objects().next() {
            if let Ok(sym) = Symbol::try_from(self.concretize(Node::new(node.env(), name)).unwrap())
            {
                return Some(sym);
            }
        }
        None
    }

    /// Get the label of a Node, which need not be injective.
    pub fn node_label(&self, node: Node) -> Option<Symbol> {
        let try_import = self.get_imported(amlang_node!(label, self.context()), node.env());
        let label_predicate = match try_import {
            Some(pred) => pred,
            None => return None,
        };

        let labels = self
            .ask_from(node.env(), Some(node), Some(label_predicate), None)
            .unwrap();
        if let Some(label) = labels.objects().next() {
            if let Ok(sym) =
                Symbol::try_from(self.concretize(Node::new(node.env(), label)).unwrap())
            {
                return Some(sym);
            }
        }
        None
    }

    pub fn resolve(&self, name: &Symbol) -> Result<Node, Error> {
        // Always get prelude nodes from current env.
        if let Some(prelude) = EnvPrelude::from_name(name.as_str()) {
            return Ok(Node::new(self.pos().env(), prelude.local()));
        }

        let designation = self.context().designation();
        for i in 0..self.designation_chain.len() {
            let table = SymNodeTable::try_from(
                self.concretize(Node::new(self.designation_chain[i], designation))?,
            )
            .unwrap();
            if let Some(node) = table.lookup(name) {
                return Ok(node);
            }
        }
        err!(self, LangError::UnboundSymbol(name.clone()))
    }

    pub fn designate(&self, designator: Primitive) -> Result<Sexp, Error> {
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

        let name_sexp = self.concretize(name)?;
        let symbol = match <Symbol>::try_from(name_sexp) {
            Ok(symbol) => symbol,
            Err(sexp) => {
                return err!(
                    self,
                    LangError::InvalidArgument {
                        given: sexp,
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
            <&mut SymNodeTable>::try_from(self.env_mut().entry_mut(designation).as_option())
        {
            table.insert(symbol, node);
        } else {
            panic!("Env designation isn't a symbol table");
        }

        self.tell(node, self.globalize(designation), name)?;
        Ok(node)
    }

    pub fn define(&mut self, structure: Option<Sexp>) -> Result<Node, Error> {
        let env = self.env_mut();
        let local = match structure {
            None => env.insert_atom(),
            Some(sexp) => env.insert_structure(sexp),
        };
        Ok(self.globalize(local))
    }

    pub fn set(&mut self, node: Node, structure: Option<Sexp>) -> Result<(), Error> {
        let mut entry = self
            .access_env_mut(node.env())
            .unwrap()
            .entry_mut(node.local());
        match structure {
            None => *entry.kind_mut() = EntryMutKind::Atomic,
            Some(sexp) => *entry.kind_mut() = EntryMutKind::Owned(sexp),
        }
        Ok(())
    }

    pub fn tell(&mut self, subject: Node, predicate: Node, object: Node) -> Result<Node, Error> {
        self.tell_to(self.pos().env(), subject, predicate, object)
    }

    pub fn tell_to(
        &mut self,
        env: LocalNode,
        subject: Node,
        predicate: Node,
        object: Node,
    ) -> Result<Node, Error> {
        let original_pos = self.pos();

        if let Some(triple) = self
            .ask_from(env, Some(subject), Some(predicate), Some(object))?
            .triples()
            .next()
        {
            return err!(self, LangError::DuplicateTriple(triple.reify(self)));
        }

        // If a tell_handler exists for the predicate, ensure it passes before adding triple.
        let tell_handler = Node::new(env, self.context().tell_handler());
        if let Some(handler) = self
            .ask_from(env, Some(predicate), Some(tell_handler), None)?
            .objects()
            .next()
        {
            // TODO(feat) Decouple from AmlangInterpreter?
            let res = self.sub_exec(
                Procedure::Application(handler.globalize(self), vec![subject, predicate, object])
                    .into(),
                Box::new(AmlangInterpreter::default()),
                amlang_node!(tell, self.context()),
            )?;
            // Only allow insertion to continue if the handler returns true.
            if res != amlang_node!(t, self.context()).into() {
                return err!(
                    self,
                    LangError::RejectedTriple(list!(subject, predicate, object,), res)
                );
            }
        }

        // Note(sec) If the tell handler jumps to a different environment, the
        // local nodes will globalize into the wrong Environment without jumping
        // back to the original env.
        self.jump(original_pos);
        // Ensuring this triple didn't already exist assures that we can call
        // .local() here without any checks.
        let triple =
            self.env_mut()
                .insert_triple(subject.local(), predicate.local(), object.local());
        Ok(triple.node().globalize(&self))
    }

    pub fn ask(
        &self,
        subject: Option<Node>,
        predicate: Option<Node>,
        object: Option<Node>,
    ) -> Result<TripleSet, Error> {
        self.ask_from(self.pos().env(), subject, predicate, object)
    }

    pub fn ask_from(
        &self,
        env: LocalNode,
        subject: Option<Node>,
        predicate: Option<Node>,
        object: Option<Node>,
    ) -> Result<TripleSet, Error> {
        let to_local = |arg: Option<Node>| {
            if let Some(node) = arg {
                if node.env() != env {
                    return err!(
                        self,
                        LangError::Unsupported(
                            "Cross-env triples are not currently supported".into()
                        )
                    );
                }
                return Ok(Some(node.local()));
            }
            Ok(None)
        };
        let (s, p, o) = (to_local(subject)?, to_local(predicate)?, to_local(object)?);

        let e = self.access_env(env).unwrap();
        let res = match s {
            Some(ss) => match p {
                Some(pp) => match o {
                    Some(oo) => e.match_triple(ss, pp, oo),
                    None => e.match_but_object(ss, pp),
                },
                None => match o {
                    Some(oo) => e.match_but_predicate(ss, oo),
                    None => e.match_subject(ss),
                },
            },
            None => match p {
                Some(pp) => match o {
                    Some(oo) => e.match_but_subject(pp, oo),
                    None => e.match_predicate(pp),
                },
                None => match o {
                    Some(oo) => e.match_object(oo),
                    None => e.match_all(),
                },
            },
        };
        Ok(res.into())
    }

    pub fn ask_any(&self, node: Node) -> Result<TripleSet, Error> {
        Ok(self.access_env(node.env()).unwrap().match_any(node.local()))
    }

    pub fn import(&mut self, original: Node) -> Result<Node, Error> {
        if original.env() == self.pos().env() {
            return Ok(original);
        }

        let table_node = self.get_or_create_import_table(original.env());
        if let Ok(table) =
            <&LocalNodeTable>::try_from(self.context.meta().base().entry(table_node).as_option())
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

        let imported = self.define(Some(original.into()))?;
        let success = if let Ok(table) = <&mut LocalNodeTable>::try_from(
            self.context
                .meta_mut()
                .base_mut()
                .entry_mut(table_node)
                .as_option(),
        ) {
            table.insert(original.local(), imported.local());
            true
        } else {
            false
        };
        if success {
            Ok(imported)
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

    pub fn get_imported(&self, original: Node, target_env: LocalNode) -> Option<Node> {
        if original.env() == target_env {
            return Some(original);
        }

        let table_node = self.get_import_table(original.env(), target_env);
        if table_node.is_none() {
            return None;
        }
        if let Ok(table) = <&LocalNodeTable>::try_from(
            self.context
                .meta()
                .base()
                .entry(table_node.unwrap())
                .as_option(),
        ) {
            if let Some(imported) = table.lookup(&original.local()) {
                return Some(imported.globalize(&self));
            }
        }
        return None;
    }

    pub fn find_env<S: AsRef<str>>(&self, s: S) -> Option<LocalNode> {
        let meta = self.context.meta().base();
        let triples = self
            .ask_from(
                LocalNode::default(),
                None,
                Some(Node::new(
                    LocalNode::default(),
                    self.context().serialize_path(),
                )),
                None,
            )
            .unwrap()
            .triples();
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

    fn get_or_create_import_table(&mut self, from_env: LocalNode) -> LocalNode {
        let imports_node = self.context.imports();
        let import_table_node = self.context.import_table();
        let env = self.pos().env();
        let import_triple = {
            let meta = self.context.meta_mut().base_mut();
            if let Some(triple) = meta
                .match_triple(env, imports_node, from_env)
                .triples()
                .next()
            {
                triple
            } else {
                meta.insert_triple(env, imports_node, from_env)
            }
        };

        let matches = self
            .context
            .meta()
            .base()
            .match_but_object(import_triple.node(), import_table_node);
        match matches.len() {
            0 => {
                let table = LocalNodeTable::in_env(LocalNode::default()).into();
                let table_node = self.context.meta_mut().base_mut().insert_structure(table);
                self.context.meta_mut().base_mut().insert_triple(
                    import_triple.node(),
                    import_table_node,
                    table_node,
                );
                table_node
            }
            1 => matches.objects().next().unwrap(),
            _ => panic!("Found multiple import tables for single import triple"),
        }
    }

    fn get_import_table(&self, from_env: LocalNode, target_env: LocalNode) -> Option<LocalNode> {
        let imports_node = self.context.imports();
        let import_table_node = self.context.import_table();
        let import_triple = {
            let meta = self.context.meta();
            if let Some(triple) = meta
                .base()
                .match_triple(target_env, imports_node, from_env)
                .triples()
                .next()
            {
                triple
            } else {
                return None;
            }
        };

        let matches = self
            .context
            .meta()
            .base()
            .match_but_object(import_triple.node(), import_table_node);
        match matches.len() {
            0 => None,
            1 => Some(matches.objects().next().unwrap()),
            _ => panic!("Found multiple import tables for single import triple"),
        }
    }
}


// Print functionality.
impl Agent {
    pub fn trace_error(&mut self, err: &Error) {
        if let Some(cont) = err.cont() {
            let mut original_cont = std::mem::replace(&mut self.exec_state, cont.clone());
            println!("");
            println!("  --TRACE--");
            let end = cont.depth() - 1;
            for (i, frame) in cont.iter().enumerate() {
                if i == end {
                    break;
                }
                self.exec_state_mut().pop();
                print!("   {})  ", i);
                self.print_sexp(&frame.context().into());
                println!("");
            }
            std::mem::swap(&mut self.exec_state, &mut original_cont);
        }
    }

    pub fn print_sexp(&self, structure: &Sexp) {
        let mut writer = BufWriter::new(stdout());
        if let Err(err) = self.write_sexp(&mut writer, structure, 0, true) {
            println!("print_sexp error: {:?}", err);
        }
    }

    // TODO(func) Make show_redirects & paren_color configurable & introspectable.
    fn write_sexp<W: std::io::Write>(
        &self,
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
        &self,
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
                if show_redirects {
                    write!(w, "[Procedure]->")?;
                }
                let s = procedure.reify(self);
                self.write_sexp(w, &s, depth, true)
            }
            Primitive::SymNodeTable(table) => {
                if show_redirects {
                    write!(w, "[SymNodeTable]->")?;
                }
                let s = table.reify(self);
                self.write_sexp(w, &s, depth, false)
            }
            Primitive::LocalNodeTable(table) => {
                if show_redirects {
                    write!(w, "[LocalNodeTable]->")?;
                }
                let s = table.reify(self);
                self.write_sexp(w, &s, depth, false)
            }
            _ => write!(w, "{}", primitive),
        }
    }
}
