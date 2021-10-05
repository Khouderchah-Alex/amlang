use rustyline::completion::{Candidate, Completer};
use rustyline::highlight::Highlighter;
use rustyline::hint::{Hint, Hinter};
use rustyline::line_buffer::LineBuffer;
use rustyline::validate::Validator;
use rustyline::{Context, Helper};
use std::cell::RefCell;
use std::convert::TryFrom;

use crate::agent::agent_state::AgentState;
use crate::primitive::{Symbol, SymbolTable};


// Rustyline Helper for InteractiveStream.
pub struct InteractiveHelper {
    state: RefCell<AgentState>,
}

pub struct InteractiveCandidate {
    symbol: Symbol,
}

impl InteractiveHelper {
    pub fn new(state: AgentState) -> Self {
        Self {
            state: RefCell::new(state),
        }
    }

    fn designation_prefix(&self, prefix: &str) -> Vec<Symbol> {
        let mut state = self.state.borrow_mut();
        let designation = state.context().designation();

        let mut res = Vec::<Symbol>::new();
        for env in state.designation_chain().clone() {
            let entry = state.access_env(env).unwrap().node_structure(designation);
            let table = <&SymbolTable>::try_from(entry.as_option()).unwrap();
            res.extend(
                table
                    .as_map()
                    .range(prefix.to_string()..)
                    .take_while(|(k, _)| k.as_str().starts_with(prefix))
                    .map(|(k, _)| k.clone()),
            );
        }
        res
    }

    fn word_bounds<'a>(&self, line: &'a str, pos: usize) -> (usize, usize) {
        let mut start: usize = 0;
        let mut end: usize = line.len();
        for (i, c) in line.char_indices() {
            // TODO(flex) Use symbol policy here.
            if !c.is_alphabetic() && c != '_' && c != '+' && c != '-' && c != '*' && c != '/' {
                if i < pos {
                    start = i + 1;
                } else {
                    end = i;
                    break;
                }
            }
        }
        (start, end)
    }
}


impl Completer for InteractiveHelper {
    type Candidate = InteractiveCandidate;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Self::Candidate>)> {
        let (start, end) = self.word_bounds(line, pos);
        let symbols = self.designation_prefix(&line[start..end]);
        Ok((
            0,
            symbols
                .into_iter()
                .map(|symbol| InteractiveCandidate { symbol })
                .collect(),
        ))
    }

    fn update(&self, line: &mut LineBuffer, _: usize, elected: &str) {
        let (start, end) = self.word_bounds(line.as_str(), line.pos());
        line.delete_range(start..end);
        line.insert_str(start, elected);
        line.set_pos(start + elected.len());
    }
}


impl Helper for InteractiveHelper {}
impl Hinter for InteractiveHelper {
    type Hint = InteractiveCandidate;
}
impl Highlighter for InteractiveHelper {}
impl Validator for InteractiveHelper {}


impl Candidate for InteractiveCandidate {
    fn display(&self) -> &str {
        self.symbol.as_str()
    }

    fn replacement(&self) -> &str {
        self.symbol.as_str()
    }
}

impl Hint for InteractiveCandidate {
    fn display(&self) -> &str {
        self.symbol.as_str()
    }

    fn completion(&self) -> Option<&str> {
        Some(self.symbol.as_str())
    }
}
