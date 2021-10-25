//! Helper macros for creation of primitive::Errors.

/// Creates a stateful Error.
///
/// Called as:  err!(state, error).
/// Stateful errors should always be used when possible.
#[macro_export]
macro_rules! err {
    ($state:expr, $($inner:tt)+) => {
        Err($crate::primitive::error::Error::with_state(
            $state.clone(),
            Box::new($crate::agent::lang_error::LangError::$($inner)+),
        ))
    };
}

/// Creates a stateless Error.
///
/// Called as:  err_nost!(error).
/// Stateful errors are always preferred when possible.
#[macro_export]
macro_rules! err_nost {
    ($($inner:tt)+) => {
        Err($crate::primitive::error::Error::empty_state(
            Box::new($crate::agent::lang_error::LangError::$($inner)+),
        ))
    };
}
