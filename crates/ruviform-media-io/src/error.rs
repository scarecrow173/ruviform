//! Normalized [`Error`] / [`Result`] model for the media-I/O boundary.
//!
//! Every failure crossing a backend implementation is normalized into
//! the [`ErrorKind`] classification; backend-specific detail is carried
//! only as a type-erased diagnostic reachable through
//! [`std::error::Error::source`]. Boxing is permitted because this
//! crate is non-RT only. No external (backend) error type ever appears
//! in the public API.
//!
//! # End-of-stream convention
//!
//! A *normal* end of stream is **not** an error: packet and frame
//! reads report it as `Ok(None)` ([`Demuxer::read_packet`],
//! [`Decoder::receive_frame`], [`Encoder::receive_packet`]). The
//! [`ErrorKind::UnexpectedEos`] classification is reserved for
//! *abnormal* termination — truncated input or a hard container EOS in
//! the middle of a stream.
//!
//! [`Demuxer::read_packet`]: crate::Demuxer::read_packet
//! [`Decoder::receive_frame`]: crate::Decoder::receive_frame
//! [`Encoder::receive_packet`]: crate::Encoder::receive_packet

use ruviform_media_core as media;
use std::fmt;

/// Normalized failure classification for media-I/O operations.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy — new variants may be added without breaking downstream
/// `match` arms.
///
/// **RT contract:** non-RT only.
///
/// # Examples
///
/// ```
/// use ruviform_media_io::ErrorKind;
///
/// // `ErrorKind::Unsupported` is the error half of capability
/// // negotiation (the query half is `Capabilities`).
/// assert_eq!(ErrorKind::Unsupported.to_string(), "unsupported feature");
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorKind {
    /// Underlying I/O failed — a source read/seek or a sink write.
    Io,
    /// The backend does not provide the requested trait, codec,
    /// container or parameter — the error half of capability
    /// negotiation (the query half is [`Capabilities`](crate::Capabilities)).
    Unsupported,
    /// Input bytes or container structures are corrupt or violate the
    /// format contract.
    InvalidData,
    /// A supplied argument or upstream value was invalid — the
    /// normalized landing point for upstream value-type errors
    /// (`InvalidValue`, `InvalidTimeBase`, …).
    InvalidValue,
    /// Time arithmetic overflowed the representable range — normalized
    /// from the media-core `TimeOverflow` value error.
    TimeOverflow,
    /// Abnormal end of stream — truncated input or a hard container EOS
    /// in the middle of a stream. A *normal* end of stream is
    /// `Ok(None)`, never an error.
    UnexpectedEos,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::Io => "I/O failure",
            Self::Unsupported => "unsupported feature",
            Self::InvalidData => "invalid or corrupt data",
            Self::InvalidValue => "invalid value",
            Self::TimeOverflow => "time arithmetic overflow",
            Self::UnexpectedEos => "unexpected end of stream",
        };
        f.write_str(message)
    }
}

/// Media-I/O error: a normalized [`ErrorKind`] classification plus an
/// optional type-erased backend diagnostic.
///
/// Backend-specific causes (symphonia, `FFmpeg`, OS I/O, …) are boxed and
/// reachable only through [`std::error::Error::source`] — the public
/// surface stays free of external error types. `Error` is `Send`: it
/// may cross the I/O/decode-thread boundary like any other data type.
///
/// **RT contract:** non-RT only — may allocate (boxed diagnostic).
///
/// # Examples
///
/// ```
/// use ruviform_media_io::{Error, ErrorKind};
///
/// let error = Error::from_kind(ErrorKind::Unsupported);
/// assert_eq!(error.kind(), ErrorKind::Unsupported);
/// ```
#[derive(Debug)]
pub struct Error {
    /// Normalized classification — the stable, matchable surface.
    kind: ErrorKind,
    /// Type-erased backend diagnostic, exposed via
    /// [`std::error::Error::source`].
    source: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
}

impl Error {
    /// Builds an error carrying only a normalized classification.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn from_kind(kind: ErrorKind) -> Self {
        Self { kind, source: None }
    }

    /// Builds an error with a type-erased backend diagnostic attached.
    ///
    /// The diagnostic is reachable through
    /// [`std::error::Error::source`] and is rendered by
    /// [`Display`](fmt::Display); it never becomes a named type in the
    /// public API.
    ///
    /// **RT contract:** non-RT only — allocates the boxed source.
    pub fn with_source<E>(kind: ErrorKind, source: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self {
            kind,
            source: Some(Box::new(source)),
        }
    }

    /// The normalized classification of this error.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn kind(&self) -> ErrorKind {
        self.kind
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.source {
            Some(source) => write!(f, "{}: {source}", self.kind),
            None => fmt::Display::fmt(&self.kind, f),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_deref().map(|diagnostic| {
            let erased: &(dyn std::error::Error + 'static) = diagnostic;
            erased
        })
    }
}

/// Normalized mapping of upstream value-type errors
/// (`ruviform-media-core`'s `Error`, also used by the audio/video
/// domain crates): the variant collapses into an [`ErrorKind`] and the
/// original error is preserved as the type-erased diagnostic.
///
/// **RT contract:** non-RT only — allocates the boxed source.
impl From<media::Error> for Error {
    fn from(source: media::Error) -> Self {
        let kind = match source {
            media::Error::InvalidTimeBase | media::Error::InvalidValue(_) => {
                ErrorKind::InvalidValue
            }
            media::Error::TimeOverflow => ErrorKind::TimeOverflow,
            media::Error::UnsupportedDropFrameRate => ErrorKind::Unsupported,
            _ => ErrorKind::InvalidData,
        };
        Self::with_source(kind, source)
    }
}

/// Normalized mapping of `std::io::Error` — the failure channel of the
/// [`MediaSource`](crate::MediaSource) (`Read`/`Seek`) and sink
/// boundaries.
///
/// **RT contract:** non-RT only — allocates the boxed source.
impl From<std::io::Error> for Error {
    fn from(source: std::io::Error) -> Self {
        Self::with_source(ErrorKind::Io, source)
    }
}

/// Result alias for media-I/O operations.
///
/// **RT contract:** non-RT only.
pub type Result<T> = core::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;
    use ruviform_media_core::Error as MediaError;

    fn check_send<T: Send>() {}

    #[test]
    fn kind_classification_is_carried() {
        let error = Error::from_kind(ErrorKind::Unsupported);
        assert_eq!(error.kind(), ErrorKind::Unsupported);
        assert!(std::error::Error::source(&error).is_none());
        assert_eq!(error.to_string(), "unsupported feature");
    }

    #[test]
    fn source_is_carried_type_erased() {
        let error = Error::with_source(
            ErrorKind::Io,
            std::io::Error::new(std::io::ErrorKind::BrokenPipe, "disk gone"),
        );
        assert_eq!(error.kind(), ErrorKind::Io);
        assert_eq!(
            std::error::Error::source(&error).unwrap().to_string(),
            "disk gone"
        );
        // Display renders classification + diagnostic.
        assert!(error.to_string().contains("I/O failure"));
        assert!(error.to_string().contains("disk gone"));
    }

    #[test]
    fn io_error_maps_to_io_kind() {
        let error = Error::from(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "eof",
        ));
        assert_eq!(error.kind(), ErrorKind::Io);
        assert!(std::error::Error::source(&error).is_some());
    }

    #[test]
    fn upstream_value_errors_map_to_normalized_kinds() {
        for (source, expected) in [
            (MediaError::InvalidValue("frames"), ErrorKind::InvalidValue),
            (MediaError::InvalidTimeBase, ErrorKind::InvalidValue),
            (MediaError::TimeOverflow, ErrorKind::TimeOverflow),
            (MediaError::UnsupportedDropFrameRate, ErrorKind::Unsupported),
        ] {
            let error = Error::from(source);
            assert_eq!(error.kind(), expected, "mapping for {source}");
            // The upstream error stays reachable as diagnostics.
            assert!(std::error::Error::source(&error).is_some());
        }
    }

    #[test]
    fn display_covers_every_kind() {
        for kind in [
            ErrorKind::Io,
            ErrorKind::Unsupported,
            ErrorKind::InvalidData,
            ErrorKind::InvalidValue,
            ErrorKind::TimeOverflow,
            ErrorKind::UnexpectedEos,
        ] {
            assert_ne!(kind.to_string(), "");
            assert_eq!(Error::from_kind(kind).to_string(), kind.to_string());
        }
    }

    #[test]
    fn error_send_bounds() {
        check_send::<Error>();
        check_send::<ErrorKind>();
    }
}
