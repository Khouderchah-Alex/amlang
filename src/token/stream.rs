use std::path::Path;

use super::token::Token;
use super::tokenizer::Tokenizer;
use crate::agent::symbol_policies::SymbolPolicy;
use crate::error::Error;
use crate::stream::prelude::*;


pub fn file_stream<P: AsRef<Path>, SymbolInfo: 'static>(
    path: P,
    symbol_policy: SymbolPolicy<SymbolInfo>,
) -> Result<Stream<Token>, StreamError> {
    let input = match FileReader::new(path) {
        Ok(input) => Box::new(input),
        Err(error) => return Err(StreamError::IoError(error)),
    };

    let strategy = match Strategy::<String, Token>::new(
        StrategyKind::Eager,
        input,
        Box::new(Tokenizer::new(symbol_policy)),
        Box::new(|_error: Error| panic!()),
    ) {
        Ok(strategy) => Box::new(strategy),
        Err(error) => return Err(StreamError::Error(error)),
    };

    Ok(Stream::<Token>::new(strategy))
}

pub fn fifo_stream<P: AsRef<Path>, SymbolInfo: 'static>(
    path: P,
    symbol_policy: SymbolPolicy<SymbolInfo>,
) -> Result<Stream<Token>, StreamError> {
    let input = match FifoReader::new(path) {
        Ok(input) => Box::new(input),
        Err(error) => return Err(StreamError::IoError(error)),
    };

    let strategy = match Strategy::<String, Token>::new(
        StrategyKind::Eager,
        input,
        Box::new(Tokenizer::new(symbol_policy)),
        Box::new(|_error: Error| panic!()),
    ) {
        Ok(strategy) => Box::new(strategy),
        Err(error) => return Err(StreamError::Error(error)),
    };

    Ok(Stream::<Token>::new(strategy))
}

pub fn string_stream<S: AsRef<str>, SymbolInfo: 'static>(
    s: S,
    symbol_policy: SymbolPolicy<SymbolInfo>,
) -> Result<Stream<Token>, Error> {
    Ok(Stream::<Token>::new(Box::new(
        Strategy::<String, Token>::new(
            StrategyKind::Eager,
            Box::new(StringReader::new(s)),
            Box::new(Tokenizer::new(symbol_policy)),
            Box::new(|_error: Error| panic!()),
        )?,
    )))
}
