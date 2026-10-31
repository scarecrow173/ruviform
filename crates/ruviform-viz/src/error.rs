//! Error vocabulary for `ruviform-viz` — [`VizError`].
//!
//! Three failure families are distinguished: caller-side parameter
//! validation ([`VizError::Invalid`]), backend failures reported by the
//! application's data-source implementations ([`VizError::Source`],
//! carrying an owned message — mirroring `ruviform-ui`'s
//! `DataSource::read -> Result<usize, String>` shape), cancellation
//! ([`VizError::Cancelled`]), and capability mismatches where a source
//! does not provide what an operation needs ([`VizError::Unavailable`]).

use std::error::Error;
use std::fmt::{Display, Formatter};

/// Error type for `ruviform-viz` operations.
///
/// `#[non_exhaustive]` per the public-enum policy — new variants may be
/// added without breaking downstream matches.
///
/// **RT contract:** non-RT only (`Source` owns a `String`).
#[non_exhaustive]
#[derive(Debug)]
pub enum VizError {
    /// A parameter or request held an invalid value; the payload names
    /// the offending field (e.g. `"bins"`).
    Invalid(&'static str),
    /// The application's data source failed a bounded read.
    Source(String),
    /// A [`ruviform_exec::CancellationToken`] was observed during a
    /// chunked build or analysis.
    Cancelled,
    /// The source does not provide data the operation requires (for
    /// example complex bins from a magnitude-only spectrum source).
    Unavailable(&'static str),
}

impl Display for VizError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(field) => write!(f, "invalid value for `{field}`"),
            Self::Source(message) => write!(f, "data source read failed: {message}"),
            Self::Cancelled => f.write_str("operation cancelled"),
            Self::Unavailable(what) => write!(f, "data not provided by source: {what}"),
        }
    }
}

impl Error for VizError {}

impl From<ruviform_numeric::Error> for VizError {
    /// Numeric-kernel validation failures map to [`VizError::Invalid`],
    /// preserving the kernel's field payload when it is one.
    fn from(e: ruviform_numeric::Error) -> Self {
        match e {
            ruviform_numeric::Error::InvalidValue(field) => Self::Invalid(field),
            _ => Self::Invalid("numeric"),
        }
    }
}

impl From<ruviform_media_core::Error> for VizError {
    /// `media-core` time/value validation failures map to
    /// [`VizError::Invalid`].
    fn from(e: ruviform_media_core::Error) -> Self {
        match e {
            ruviform_media_core::Error::InvalidValue(field) => Self::Invalid(field),
            _ => Self::Invalid("time"),
        }
    }
}

/// `Result` alias for `ruviform-viz` operations.
pub type Result<T> = core::result::Result<T, VizError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_covers_every_variant() {
        for e in [
            VizError::Invalid("bins"),
            VizError::Source("backend down".to_string()),
            VizError::Cancelled,
            VizError::Unavailable("complex bins"),
        ] {
            assert_ne!(e.to_string(), "");
        }
    }

    #[test]
    fn numeric_error_maps_to_invalid() {
        let e = VizError::from(ruviform_numeric::Error::InvalidValue("n"));
        assert!(matches!(e, VizError::Invalid("n")));
    }
}
