//! Media-wide [`Error`] type and [`Result`] alias shared by the value
//! types in this crate.
//!
//! `Error` is deliberately a `Copy` value type: it carries a static
//! context string instead of an allocated message so it can be produced,
//! stored and returned on RT paths.

use core::fmt;

/// Error type for `ruviform-media-core` value-type operations.
///
/// **RT contract:** RT-suitable — `Copy`, allocation-free.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Error {
    /// A [`TimeBase`](crate::TimeBase) was built with a zero numerator or
    /// denominator.
    InvalidTimeBase,
    /// Exact rational arithmetic overflowed the `i64`/`i128`/`u128`
    /// intermediate or `i64` result range.
    TimeOverflow,
    /// A parameter held an invalid value. The payload is a static,
    /// field-name-ish description of what was invalid (e.g. `"stride"`).
    InvalidValue(&'static str),
    /// Drop-frame timecode was requested for a rate outside the NTSC
    /// family. SMPTE drop-frame labeling is defined only for `30000/1001`
    /// and `60000/1001`.
    UnsupportedDropFrameRate,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTimeBase => {
                f.write_str("invalid time base: numerator and denominator must be non-zero")
            }
            Self::TimeOverflow => f.write_str("time arithmetic overflowed the representable range"),
            Self::InvalidValue(field) => write!(f, "invalid value for `{field}`"),
            Self::UnsupportedDropFrameRate => f.write_str(
                "drop-frame timecode is only defined for 30000/1001 and 60000/1001 rates",
            ),
        }
    }
}

impl core::error::Error for Error {}

/// Result alias for `ruviform-media-core` operations.
///
/// **RT contract:** RT-suitable — [`Error`] is `Copy` and allocation-free,
/// so producing a `Result` never allocates.
pub type Result<T> = core::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_covers_every_variant() {
        for error in [
            Error::InvalidTimeBase,
            Error::TimeOverflow,
            Error::InvalidValue("stride"),
            Error::UnsupportedDropFrameRate,
        ] {
            assert_ne!(error.to_string(), "");
        }
        assert!(Error::InvalidValue("stride").to_string().contains("stride"));
    }
}
