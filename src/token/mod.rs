// Public exports.
pub use token::{Token, TokenInfo};

// Public mods.
pub mod file_stream;
pub mod interactive_stream;
pub mod string_stream;
pub mod token;

// Private mods.
mod tokenize;