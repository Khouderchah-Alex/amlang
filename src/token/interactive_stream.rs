use rustyline::error::ReadlineError;
use rustyline::Editor;

use super::token::TokenInfo;
use super::tokenize::{tokenize_line, TokenStore};


pub struct InteractiveStream {
    editor: Editor<()>,
    tokens: TokenStore,
}

impl InteractiveStream {
    pub fn new() -> InteractiveStream {
        // TODO: Add completer.
        let editor = Editor::<()>::new();

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
                        println!("Error: {:?}", err);
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
                    println!("Error: {:?}", err);
                    break;
                }
            }
        }

        self.tokens.pop_front()
    }
}
