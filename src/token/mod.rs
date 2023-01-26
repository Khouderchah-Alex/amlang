// Public exports.
pub use stream::{fifo_stream, file_stream, string_stream};
pub use token::{Token, TokenKind};
pub use tokenizer::TokenizeError;

// Public mods.
pub mod stream;
pub mod token;

#[cfg(feature = "cli")]
pub mod cli_helper;
#[cfg(feature = "cli")]
pub mod cli_stream;

// Private mods.
mod tokenizer;
