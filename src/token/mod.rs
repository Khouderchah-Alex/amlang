// Public exports.
pub use token::{Token, TokenKind};
pub use tokenizer::{TokenizeError, Tokenizer};

// Public mods.
pub mod token;

#[cfg(feature = "cli")]
pub mod cli_helper;
#[cfg(feature = "cli")]
pub mod cli_stream;

// Private mods.
mod tokenizer;
