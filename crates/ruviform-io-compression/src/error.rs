//! Normalized [`CompressionError`] / [`CompressionErrorKind`] model for
//! the byte-stream compression boundary.
//!
//! Every failure crossing a compression backend is normalized into the
//! [`CompressionErrorKind`] classification; backend- or I/O-specific
//! detail is carried only as a type-erased diagnostic reachable through
//! [`std::error::Error::source`]. No backend type ever appears in the
//! public API.
//!
//! When a failure is surfaced through `impl std::io::Read` or
//! `impl std::io::Write`, it is wrapped in an [`std::io::Error`] whose
//! [`std::error::Error::source`] chain still leads back to the
//! [`CompressionError`] — classification is never lost for the sake of
//! trait shape.

use std::fmt;
use std::io;

/// Normalized failure classification for compression operations.
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
/// use ruviform_io_compression::CompressionErrorKind;
///
/// assert_eq!(CompressionErrorKind::NotExhausted.to_string(), "output not exhausted");
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompressionErrorKind {
    /// Underlying reader/writer I/O failed.
    Io,
    /// A supplied argument (e.g. an out-of-range compression level)
    /// was invalid.
    InvalidInput,
    /// Input bytes do not satisfy the format's structure — wrong magic,
    /// unknown method, reserved header bits set, oversized member header.
    InvalidFormat,
    /// The input ended before the current unit (member/stream) reached
    /// its format-defined end.
    TruncatedInput,
    /// The compressed body is corrupt (invalid deflate stream data).
    CorruptData,
    /// A format checksum (gzip CRC32/ISIZE) did not match the decoded
    /// bytes.
    ChecksumMismatch,
    /// With [`crate::DecodeScope::WholeInput`], non-format bytes remain
    /// after the last unit's end.
    TrailingData,
    /// A cumulative byte/unit budget was exceeded.
    LimitExceeded,
    /// The requested format or capability is not provided by this build.
    Unsupported,
    /// The operation was cancelled by an upper layer. Reserved for
    /// future cancellation plumbing — no cancellation surface exists yet.
    Cancelled,
    /// The encoder/decoder is in a terminal state (`Finished`/`Failed`)
    /// and the operation is not allowed there.
    InvalidState,
    /// `Decoder::finish` was called while unverified input or undelivered
    /// decoded output could still exist.
    NotExhausted,
}

impl CompressionErrorKind {
    /// Maps the normalized kind onto the closest [`io::ErrorKind`] for
    /// `impl Read`/`impl Write` error wrapping.
    pub(crate) const fn io_error_kind(self) -> io::ErrorKind {
        match self {
            Self::Io | Self::NotExhausted => io::ErrorKind::Other,
            Self::InvalidInput | Self::InvalidState => io::ErrorKind::InvalidInput,
            Self::InvalidFormat
            | Self::CorruptData
            | Self::ChecksumMismatch
            | Self::TrailingData => io::ErrorKind::InvalidData,
            Self::TruncatedInput => io::ErrorKind::UnexpectedEof,
            Self::LimitExceeded => io::ErrorKind::QuotaExceeded,
            Self::Unsupported => io::ErrorKind::Unsupported,
            Self::Cancelled => io::ErrorKind::Interrupted,
        }
    }
}

impl fmt::Display for CompressionErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::Io => "I/O failure",
            Self::InvalidInput => "invalid input",
            Self::InvalidFormat => "invalid format",
            Self::TruncatedInput => "truncated input",
            Self::CorruptData => "corrupt data",
            Self::ChecksumMismatch => "checksum mismatch",
            Self::TrailingData => "trailing data after stream end",
            Self::LimitExceeded => "limit exceeded",
            Self::Unsupported => "unsupported feature",
            Self::Cancelled => "operation cancelled",
            Self::InvalidState => "invalid encoder/decoder state",
            Self::NotExhausted => "output not exhausted",
        };
        f.write_str(message)
    }
}

/// Compression error: a normalized [`CompressionErrorKind`]
/// classification plus an optional type-erased diagnostic.
///
/// `CompressionError` is `Send`: it may cross thread boundaries like
/// any other data type.
///
/// **RT contract:** non-RT only — may allocate (boxed diagnostic).
///
/// # Examples
///
/// ```
/// use ruviform_io_compression::{CompressionError, CompressionErrorKind};
///
/// let error = CompressionError::from_kind(CompressionErrorKind::Unsupported);
/// assert_eq!(error.kind(), CompressionErrorKind::Unsupported);
/// ```
#[derive(Debug)]
pub struct CompressionError {
    /// Normalized classification — the stable, matchable surface.
    kind: CompressionErrorKind,
    /// Type-erased backend/I-O diagnostic, exposed via
    /// [`std::error::Error::source`].
    source: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
}

impl CompressionError {
    /// Builds an error carrying only a normalized classification.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn from_kind(kind: CompressionErrorKind) -> Self {
        Self { kind, source: None }
    }

    /// Builds an error with a type-erased backend diagnostic attached.
    ///
    /// **RT contract:** non-RT only.
    pub fn with_source(
        kind: CompressionErrorKind,
        source: impl Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
    ) -> Self {
        Self {
            kind,
            source: Some(source.into()),
        }
    }

    /// The normalized classification.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn kind(&self) -> CompressionErrorKind {
        self.kind
    }

    /// Wraps an underlying `io::Error` preserving its diagnostic as
    /// [`std::error::Error::source`].
    #[cfg(any(feature = "gzip", feature = "deflate"))]
    pub(crate) fn io(source: io::Error) -> Self {
        Self::with_source(CompressionErrorKind::Io, source)
    }

    /// Converts into an [`io::Error`] for `impl Read`/`impl Write`
    /// surfaces, keeping this error reachable via `source()`.
    pub(crate) fn into_io_error(self) -> io::Error {
        let kind = self.kind.io_error_kind();
        io::Error::new(kind, self)
    }
}

impl fmt::Display for CompressionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ruviform-io-compression: {}", self.kind)?;
        if let Some(source) = &self.source {
            write!(f, ": {source}")?;
        }
        Ok(())
    }
}

impl std::error::Error for CompressionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source
            .as_deref()
            .map(|source| source as &(dyn std::error::Error + 'static))
    }
}

impl From<CompressionError> for io::Error {
    fn from(error: CompressionError) -> Self {
        error.into_io_error()
    }
}
