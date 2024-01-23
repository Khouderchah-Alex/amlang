use crate::error::ErrorKind;
use crate::primitive::Symbol;
use crate::sexp::{Cons, Sexp};

#[derive(Debug)]
pub enum DeserializeError {
    IoError(std::io::Error),

    // Serde usage.
    MissingData,
    ExtraneousData(Sexp),
    UnexpectedType { given: Sexp, expected: String },

    // Legacy EnvManager deserialization.
    MissingHeaderSection,
    MissingNodeSection,
    MissingTripleSection,
    UnexpectedCommand(Sexp),
    ExpectedSymbol,
    UnrecognizedBuiltIn(Symbol),
    InvalidNodeEntry(Sexp),
}

impl ErrorKind for DeserializeError {
    // TODO(func) Model within env rather than fall back on strings.
    fn reify(&self) -> Sexp {
        let inner = match self {
            Self::IoError(err) => {
                list!("IoError", format!("{}", err))
            }
            Self::MissingData => {
                list!("MissingData")
            }
            Self::ExtraneousData(sexp) => {
                list!("ExtraneousData", sexp.clone())
            }
            Self::UnexpectedType { given, expected } => {
                list!("UnexpectedType", given.clone(), expected.clone())
            }
            Self::MissingHeaderSection => {
                list!("MissingHeaderSection")
            }
            Self::MissingNodeSection => {
                list!("MissingNodeSection")
            }
            Self::MissingTripleSection => {
                list!("MissingTripleSection")
            }
            Self::UnexpectedCommand(sexp) => {
                list!("UnexpectedCommand", sexp.clone())
            }
            Self::ExpectedSymbol => {
                list!("ExpectedSymbol")
            }
            Self::UnrecognizedBuiltIn(symbol) => {
                list!("UnrecognizedBuiltIn", symbol.clone())
            }
            Self::InvalidNodeEntry(sexp) => {
                list!("InvalidNodeEntry", sexp.clone())
            }
        };
        Cons::new(Sexp::from("DeserializeError"), inner).into()
    }
}
