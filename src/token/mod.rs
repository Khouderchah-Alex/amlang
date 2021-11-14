// Public exports.
pub use token::{Token, TokenInfo};
pub use tokenizer::TokenizeError;

// Public mods.
pub mod file_stream;
pub mod string_stream;
pub mod token;

#[cfg(feature = "interactive")]
pub mod interactive_helper;
#[cfg(feature = "interactive")]
pub mod interactive_stream;

// Private mods.
mod tokenizer;
