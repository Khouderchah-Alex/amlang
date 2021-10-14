//! Helper macros for creation of primitive::Errors.

/// Creates a stateful Error.
///
/// Called as:  err!(state, error).
/// Stateful errors should always be used when possible.
macro_rules! err {
    ($state:expr, $($kind:tt)+) => {
        Err(crate::primitive::error::Error::with_state(
            $state.clone(),
            crate::primitive::error::ErrKind::$($kind)+,
        ))
    };
}

/// Creates a stateless Error.
///
/// Called as:  err_nost!(error).
/// Stateful errors are always preferred when possible.
macro_rules! err_nost {
    ($($kind:tt)+) => {
        Err(crate::primitive::error::Error::empty_state(
            crate::primitive::error::ErrKind::$($kind)+,
        ))
    };
}
