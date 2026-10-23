//! The crate-local [`Error`] type and [`Result`] alias for
//! `ruviform-edit` mutations and evaluation.
//!
//! `Error` is a `Copy` value type carrying static context strings
//! (`HostError` convention): domain failures such as `*NotFound` or
//! [`Error::KindMismatch`] cannot be expressed by
//! `ruviform_media_core::Error`, so media-core time errors are
//! normalized into [`Error::InvalidValue`] / [`Error::TimeOverflow`]
//! through the `From` impl instead (see
//! `docs/designs/edit-data-model.md`).

use core::fmt;

/// Why an edit-model mutation or evaluation failed.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy — new variants may be added without breaking downstream
/// `match` arms.
///
/// **RT contract:** non-RT only — `Copy`, allocation-free value type
/// (the crate itself is non-RT; the value itself is RT-shaped).
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Error {
    /// No [`Track`](crate::Track) with the given
    /// [`TrackId`](crate::TrackId) exists.
    TrackNotFound,
    /// The track holds no [`Clip`](crate::Clip) with the given
    /// [`ClipId`](crate::ClipId).
    ClipNotFound,
    /// A range parameter broke a structural invariant — non-positive
    /// duration, an unresolvable `end`, a base-collapsed
    /// `end <= start`, or a `range` longer than the selected
    /// `source_range`. The payload names the offending input
    /// (`"range"` / `"source_range"`).
    InvalidRange(&'static str),
    /// The [`ClipAttributes`](crate::ClipAttributes) variant does not
    /// match the target track's [`TrackKind`](crate::TrackKind).
    KindMismatch,
    /// A parameter held an invalid value. The payload is a static,
    /// field-name-ish description of what was invalid (e.g. `"gain"`).
    InvalidValue(&'static str),
    /// Time arithmetic overflowed the representable `i64` range.
    TimeOverflow,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TrackNotFound => f.write_str("no track with the given id"),
            Self::ClipNotFound => f.write_str("no clip with the given id on the track"),
            Self::InvalidRange(range) => write!(f, "invalid range for `{range}`"),
            Self::KindMismatch => f.write_str("clip attributes do not match the track kind"),
            Self::InvalidValue(field) => write!(f, "invalid value for `{field}`"),
            Self::TimeOverflow => f.write_str("time arithmetic overflowed the representable range"),
        }
    }
}

impl core::error::Error for Error {}

impl From<ruviform_media_core::Error> for Error {
    /// Normalizes media-core value-type errors: `TimeOverflow` stays
    /// `TimeOverflow`, everything else degrades to `InvalidValue`.
    fn from(error: ruviform_media_core::Error) -> Self {
        use ruviform_media_core::Error as MediaError;
        match error {
            MediaError::TimeOverflow => Self::TimeOverflow,
            MediaError::InvalidValue(field) => Self::InvalidValue(field),
            MediaError::InvalidTimeBase => Self::InvalidValue("time_base"),
            MediaError::UnsupportedDropFrameRate => Self::InvalidValue("drop_frame_rate"),
            // Unknown future variant — the payload tags the error's
            // source crate rather than a field that does not exist.
            _ => Self::InvalidValue("media_core"),
        }
    }
}

/// Result alias for `ruviform-edit` operations.
///
/// **RT contract:** non-RT only — [`Error`] is `Copy` and
/// allocation-free, so producing a `Result` never allocates.
pub type Result<T> = core::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_covers_every_variant() {
        for error in [
            Error::TrackNotFound,
            Error::ClipNotFound,
            Error::InvalidRange("range"),
            Error::KindMismatch,
            Error::InvalidValue("gain"),
            Error::TimeOverflow,
        ] {
            assert_ne!(error.to_string(), "");
        }
        assert!(Error::InvalidValue("gain").to_string().contains("gain"));
    }

    #[test]
    fn media_core_errors_normalize() {
        assert_eq!(
            Error::from(ruviform_media_core::Error::TimeOverflow),
            Error::TimeOverflow
        );
        assert_eq!(
            Error::from(ruviform_media_core::Error::InvalidValue("x")),
            Error::InvalidValue("x")
        );
        assert_eq!(
            Error::from(ruviform_media_core::Error::InvalidTimeBase),
            Error::InvalidValue("time_base")
        );
        assert_eq!(
            Error::from(ruviform_media_core::Error::UnsupportedDropFrameRate),
            Error::InvalidValue("drop_frame_rate")
        );
    }
}
