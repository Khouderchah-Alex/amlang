use rustyline::error::ReadlineError;
use rustyline::Editor;

use super::interactive_helper::InteractiveHelper;
use super::token::TokenInfo;
use super::tokenize::{tokenize_line, TokenStore};
use crate::agent::env_state::EnvState;
use crate::primitive::symbol_policies::policy_base;


pub struct InteractiveStream {
    editor: Editor<InteractiveHelper>,
    tokens: TokenStore,

    depth: i16,
    curr_expr: String,
}

impl InteractiveStream {
    pub fn new(env_state: EnvState) -> InteractiveStream {
        let mut editor = Editor::<InteractiveHelper>::new();
        editor.set_helper(Some(InteractiveHelper::new(env_state)));

        InteractiveStream {
            editor,
            tokens: TokenStore::default(),

            depth: 0,
            curr_expr: String::default(),
        }
    }
}


impl Iterator for InteractiveStream {
    type Item = TokenInfo;

    fn next(&mut self) -> Option<TokenInfo> {
        if let Some(token) = self.tokens.pop_front() {
            return Some(token);
        }

        while self.tokens.len() == 0 {
            let line = if self.depth <= 0 {
                self.editor.add_history_entry(self.curr_expr.as_str());
                self.curr_expr = String::default();
                self.depth = 0;
                self.editor.readline("> ")
            } else {
                self.editor
                    .readline(&format!("..{}", "  ".repeat(self.depth as usize)))
            };

            match line {
                Ok(line) => {
                    let l = self.curr_expr.len();
                    // Insert whitespace only if we don't already have any and
                    // we haven't just opened or are about to close a list.
                    if l > 0
                        && (|c: char| !c.is_whitespace() && c != '(')(
                            self.curr_expr.as_str().chars().next_back().unwrap(),
                        )
                    {
                        if let Some(')') = line.as_str().chars().next() {
                        } else {
                            self.curr_expr += " ";
                        }
                    }
                    self.curr_expr += &line;
                    // TODO(func) Make generic over policy.
                    match tokenize_line(&line, 0, &policy_base, &mut self.tokens) {
                        Ok(depth) => {
                            self.depth = self.depth.saturating_add(depth);
                        }
                        Err(err) => {
                            println!("[Tokenize Error]: {:?}", err);
                            println!("");
                            self.tokens.clear();
                            continue;
                        }
                    }
                }
                Err(ReadlineError::Interrupted) => {
                    println!("^C");
                    continue;
                }
                Err(ReadlineError::Eof) => {
                    println!("^D");
                    break;
                }
                Err(err) => {
                    println!("[Tokenize Error]: {:?}", err);
                    println!("");
                    break;
                }
            }
        }

        self.tokens.pop_front()
    }
}
