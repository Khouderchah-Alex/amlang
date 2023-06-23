use crate::error::{Error, ErrorKind};
use crate::sexp::Sexp;


/// Encapsulation of Errors in rust's std module.
#[derive(Debug)]
pub enum StdError {
    Io(std::io::Error),
}

impl ErrorKind for StdError {
    // TODO(func) Model within env rather than fall back on strings.
    fn reify(&self) -> Sexp {
        let mut inner = match self {
            Self::Io(err) => {
                list!("IoError".to_string(), err.to_string())
            }
        };
        inner.push_front("StdError".to_string());
        inner
    }
}

/// Allow for io::Errors to be used seamlessly with amlang::Errors.
impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::no_cont(StdError::Io(err))
    }
}
