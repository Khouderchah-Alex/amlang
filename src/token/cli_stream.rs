use rustyline::error::ReadlineError;
use rustyline::Editor;

use super::cli_helper::CliHelper;
use super::token::TokenInfo;
use super::tokenizer::Tokenizer;
use crate::agent::symbol_policies::policy_base;
use crate::agent::Agent;
use crate::stream::Transform;


pub struct CliStream {
    editor: Editor<CliHelper>,
    tokenizer: Tokenizer<()>,

    curr_expr: String,
}

impl CliStream {
    // TODO(func) Use Node abstracting relevant Agent state so we can
    // follow the state of a different Agent (or use that to have the
    // Agents directly share state).
    pub fn with_helper(agent: Agent) -> Self {
        Self::new(Some(CliHelper::new(agent)))
    }

    pub fn no_helper() -> Self {
        Self::new(None)
    }

    fn new(helper: Option<CliHelper>) -> Self {
        let mut editor = Editor::<CliHelper>::new();
        editor.set_helper(helper);

        Self {
            editor,
            tokenizer: Tokenizer::new(policy_base),

            curr_expr: String::default(),
        }
    }
}


impl Iterator for CliStream {
    type Item = TokenInfo;

    fn next(&mut self) -> Option<TokenInfo> {
        loop {
            if let Some(token) = <dyn Transform<String, TokenInfo>>::output(&mut self.tokenizer) {
                return Some(token);
            }

            let line = if self.tokenizer.depth() == 0 {
                self.editor.add_history_entry(self.curr_expr.as_str());
                self.curr_expr = String::default();
                self.editor.readline("> ")
            } else {
                self.editor
                    .readline(&format!("..{}", "  ".repeat(self.tokenizer.depth())))
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
                    if let Err(err) = self.tokenizer.tokenize(&line) {
                        println!("{:?}", err);
                        println!("");
                        self.tokenizer.clear();
                        continue;
                    }
                }
                Err(ReadlineError::Interrupted) => {
                    println!("^C");
                    if self.tokenizer.depth() > 0 {
                        self.tokenizer.clear();
                        // Enable ^C to cancel an expression mid-parse.
                        return None;
                    }
                    continue;
                }
                Err(ReadlineError::Eof) => {
                    println!("^D");
                    self.tokenizer.clear();
                    return None;
                }
                Err(err) => {
                    println!("[Readline Error]: {:?}", err);
                    println!("");
                    self.tokenizer.clear();
                    continue;
                }
            }
        }
    }
}
