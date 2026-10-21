//! Shared [`Error`] type and [`Result`] alias for `ruviform-rt` public
//! APIs.
//!
//! `Error` is deliberately a `Copy` value type: it carries a static
//! context string instead of an allocated message so it can be produced,
//! stored and returned on RT paths.

use core::fmt;

/// Error type for `ruviform-rt` operations.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy — new variants may be added without breaking downstream
/// `match` arms.
///
/// **RT contract:** RT-suitable — `Copy`, allocation-free.
///
/// # Examples
///
/// ```
/// use ruviform_rt::{Error, MasterClock};
///
/// assert_eq!(
///     MasterClock::new(0, 48_000).unwrap_err(),
///     Error::InvalidValue("timebase")
/// );
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Error {
    /// A parameter held an invalid value. The payload is a static,
    /// field-name-ish description of what was invalid (e.g.
    /// `"timebase"`, `"capacity"`).
    InvalidValue(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidValue(field) => write!(f, "invalid value for `{field}`"),
        }
    }
}

impl core::error::Error for Error {}

/// Result alias for `ruviform-rt` operations.
///
/// **RT contract:** RT-suitable — [`Error`] is `Copy` and
/// allocation-free, so producing a `Result` never allocates.
///
/// # Examples
///
/// ```
/// use ruviform_rt::{Error, Result};
///
/// fn check_capacity(capacity: usize) -> Result<usize> {
///     if capacity == 0 {
///         Err(Error::InvalidValue("capacity"))
///     } else {
///         Ok(capacity)
///     }
/// }
///
/// assert_eq!(check_capacity(0), Err(Error::InvalidValue("capacity")));
/// assert_eq!(check_capacity(8), Ok(8));
/// ```
pub type Result<T> = core::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_covers_every_variant() {
        let error = Error::InvalidValue("capacity");
        assert_ne!(error.to_string(), "");
        assert!(error.to_string().contains("capacity"));
    }
}
