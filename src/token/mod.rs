pub mod file_stream;
pub mod token;

pub use token::{Token, TokenInfo};

mod tokenize;

#[cfg(test)]
mod string_stream;
