use crate::error::ErrorKind;
use crate::primitive::{Symbol, ToLangString};
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
                list!(
                    "IoError".to_lang_string(),
                    format!("{}", err).to_lang_string(),
                )
            }
            Self::MissingData => {
                list!("MissingData".to_lang_string())
            }
            Self::ExtraneousData(sexp) => {
                list!("ExtraneousData".to_lang_string(), sexp.clone())
            }
            Self::UnexpectedType { given, expected } => {
                list!(
                    "UnexpectedType".to_lang_string(),
                    given.clone(),
                    expected.clone()
                )
            }
            Self::MissingHeaderSection => {
                list!("MissingHeaderSection".to_lang_string())
            }
            Self::MissingNodeSection => {
                list!("MissingNodeSection".to_lang_string())
            }
            Self::MissingTripleSection => {
                list!("MissingTripleSection".to_lang_string())
            }
            Self::UnexpectedCommand(sexp) => {
                list!("UnexpectedCommand".to_lang_string(), sexp.clone())
            }
            Self::ExpectedSymbol => {
                list!("ExpectedSymbol".to_lang_string())
            }
            Self::UnrecognizedBuiltIn(symbol) => {
                list!("UnrecognizedBuiltIn".to_lang_string(), symbol.clone())
            }
            Self::InvalidNodeEntry(sexp) => {
                list!("InvalidNodeEntry".to_lang_string(), sexp.clone())
            }
        };
        Cons::new("DeserializeError".to_lang_string(), inner).into()
    }
}
