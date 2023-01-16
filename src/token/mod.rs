// Public exports.
pub use token::{Token, TokenInfo};
pub use tokenizer::TokenizeError;

// Public mods.
pub mod fifo_stream;
pub mod file_stream;
pub mod string_stream;
pub mod token;

#[cfg(feature = "cli")]
pub mod cli_helper;
#[cfg(feature = "cli")]
pub mod cli_stream;

// Private mods.
mod tokenizer;
