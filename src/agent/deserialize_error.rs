use crate::error::ErrorKind;
use crate::parser;
use crate::primitive::{AmString, Symbol};
use crate::sexp::{Cons, Sexp};
use crate::token::file_stream;

#[derive(Debug)]
pub enum DeserializeError {
    FileStreamError(file_stream::FileStreamError),
    ParseError(parser::ParseError),
    MissingNodeSection,
    MissingTripleSection,
    ExtraneousSection,
    UnexpectedCommand(Sexp),
    ExpectedSymbol,
    UnrecognizedBuiltIn(Symbol),
    InvalidNodeEntry(Sexp),
}

impl ErrorKind for DeserializeError {
    // TODO(func) Model within env rather than fall back on strings.
    fn reify(&self) -> Sexp {
        let inner = match self {
            Self::FileStreamError(err) => {
                list!(
                    AmString::new("FileStreamError"),
                    AmString::new(err.to_string()),
                )
            }
            Self::ParseError(err) => err.reify(),
            Self::MissingNodeSection => {
                list!(AmString::new("MissingNodeSection"),)
            }
            Self::MissingTripleSection => {
                list!(AmString::new("MissingTripleSection"),)
            }
            Self::ExtraneousSection => {
                list!(AmString::new("ExtraneousSection"),)
            }
            Self::UnexpectedCommand(sexp) => {
                list!(AmString::new("UnexpectedCommand"), sexp.clone(),)
            }
            Self::ExpectedSymbol => {
                list!(AmString::new("ExpectedSymbol"),)
            }
            Self::UnrecognizedBuiltIn(symbol) => {
                list!(AmString::new("UnrecognizedBuiltIn"), symbol.clone(),)
            }
            Self::InvalidNodeEntry(sexp) => {
                list!(AmString::new("InvalidNodeEntry"), sexp.clone(),)
            }
        };
        Cons::new(Some(AmString::new("DeserializeError").into()), inner.into()).into()
    }
}
