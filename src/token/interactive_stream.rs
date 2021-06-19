use rustyline::error::ReadlineError;
use rustyline::Editor;

use super::interactive_helper::InteractiveHelper;
use super::token::TokenInfo;
use super::tokenize::{tokenize_line, TokenStore};
use crate::agent::env_state::EnvState;


pub struct InteractiveStream {
    editor: Editor<InteractiveHelper>,
    tokens: TokenStore,
}

impl InteractiveStream {
    pub fn new(env_state: EnvState) -> InteractiveStream {
        let mut editor = Editor::<InteractiveHelper>::new();
        editor.set_helper(Some(InteractiveHelper::new(env_state)));

        InteractiveStream {
            editor,
            tokens: TokenStore::default(),
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
            match self.editor.readline("> ") {
                Ok(line) => {
                    self.editor.add_history_entry(line.as_str());
                    if let Err(err) = tokenize_line(&line, 0, &mut self.tokens) {
                        println!("[Tokenize Error]: {:?}", err);
                        println!("");
                        self.tokens.clear();
                        continue;
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
