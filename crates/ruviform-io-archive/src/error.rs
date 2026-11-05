//! Normalized [`ArchiveError`] / [`ArchiveErrorKind`] model for the
//! archive boundary.
//!
//! Every failure crossing a format reader/writer is normalized into the
//! [`ArchiveErrorKind`] classification; backend-specific detail is carried
//! only as a type-erased diagnostic reachable through
//! [`std::error::Error::source`]. No backend (`flate2`/`crc32fast`) or OS
//! error type ever appears in the public API.
//!
//! **RT contract:** non-RT only.

use std::borrow::Cow;
use std::fmt;

/// Normalized failure classification for archive operations.
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
/// use ruviform_io_archive::ArchiveErrorKind;
///
/// assert_eq!(ArchiveErrorKind::ChecksumMismatch.to_string(), "checksum mismatch");
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArchiveErrorKind {
    /// Underlying I/O on the supplied reader/writer failed.
    Io,
    /// A supplied argument (e.g. an out-of-range entry index or an
    /// invalid entry name) was invalid.
    InvalidInput,
    /// The operation was invoked in a state that does not allow it —
    /// e.g. finishing an archive while an entry writer borrowed earlier
    /// was dropped without `finish`, or re-entering after a poisoned
    /// drop.
    InvalidState,
    /// The input ended before a required structure completed
    /// (short file, central directory or local header cut mid-record).
    Truncated,
    /// The byte structure is not a well-formed archive (bad signature,
    /// out-of-range offset, declared counts that do not match the
    /// record stream, trailing garbage after the end record).
    Malformed,
    /// Two descriptions of the same fact disagree — e.g. a ZIP local
    /// header whose name/method/size contradicts the central directory.
    Inconsistent,
    /// The measured content checksum does not match the declared one
    /// (ZIP CRC32). This is an integrity-detection result, never a
    /// proof of authenticity.
    ChecksumMismatch,
    /// The measured uncompressed size differs from the declared one.
    SizeMismatch,
    /// A configured [`crate::ArchiveReadLimits`] budget was exceeded.
    /// Which budget is carried in the error message.
    LimitExceeded,
    /// Two byte regions that must be disjoint overlap — e.g. an entry
    /// data region reaching into the ZIP central directory, or two
    /// entries claiming the same bytes.
    Overlap,
    /// The archive uses a feature this crate deliberately does not
    /// implement yet — encryption, unknown compression methods,
    /// multi-disk archives, or ZIP64-required structures.
    Unsupported,
}

impl fmt::Display for ArchiveErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::Io => "I/O failure",
            Self::InvalidInput => "invalid input",
            Self::InvalidState => "invalid state",
            Self::Truncated => "truncated archive",
            Self::Malformed => "malformed archive structure",
            Self::Inconsistent => "inconsistent archive metadata",
            Self::ChecksumMismatch => "checksum mismatch",
            Self::SizeMismatch => "size mismatch",
            Self::LimitExceeded => "limit exceeded",
            Self::Overlap => "overlapping archive regions",
            Self::Unsupported => "unsupported feature",
        };
        f.write_str(message)
    }
}

/// Archive error: a normalized [`ArchiveErrorKind`] classification plus a
/// message and an optional type-erased diagnostic.
///
/// `ArchiveError` is `Send`: it may cross thread boundaries like any
/// other data type.
///
/// **RT contract:** non-RT only — may allocate (boxed diagnostic).
///
/// # Examples
///
/// ```
/// use ruviform_io_archive::{ArchiveError, ArchiveErrorKind};
///
/// let error = ArchiveError::new(ArchiveErrorKind::Unsupported, "zip64 required");
/// assert_eq!(error.kind(), ArchiveErrorKind::Unsupported);
/// ```
#[derive(Debug)]
pub struct ArchiveError {
    /// Normalized classification — the stable, matchable surface.
    kind: ArchiveErrorKind,
    /// Human-readable detail (entry index, offset, which budget, ...).
    message: Cow<'static, str>,
    /// Type-erased backend/IO diagnostic, exposed via
    /// [`std::error::Error::source`].
    source: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
}

impl ArchiveError {
    /// Builds an error carrying a normalized classification and a
    /// detail message.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn new(kind: ArchiveErrorKind, message: impl Into<Cow<'static, str>>) -> Self {
        Self {
            kind,
            message: message.into(),
            source: None,
        }
    }

    /// Builds an error carrying only a normalized classification (the
    /// kind's own display text is used as the message).
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn from_kind(kind: ArchiveErrorKind) -> Self {
        Self {
            kind,
            message: Cow::Borrowed(""),
            source: None,
        }
    }

    /// Builds an error with a type-erased backend diagnostic attached.
    ///
    /// **RT contract:** non-RT only.
    pub fn with_source(
        kind: ArchiveErrorKind,
        message: impl Into<Cow<'static, str>>,
        source: impl Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
    ) -> Self {
        Self {
            kind,
            message: message.into(),
            source: Some(source.into()),
        }
    }

    /// The normalized classification.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn kind(&self) -> ArchiveErrorKind {
        self.kind
    }

    /// The detail message (empty when only a kind was supplied).
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Maps an underlying `io::Error` to the normalized
    /// [`ArchiveErrorKind::Io`] kind while preserving it as `source()`.
    pub(crate) fn from_io(error: std::io::Error) -> Self {
        Self::with_source(
            ArchiveErrorKind::Io,
            "underlying reader/writer failed",
            error,
        )
    }
}

impl fmt::Display for ArchiveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ruviform-io-archive: {}", self.kind)?;
        if !self.message.is_empty() {
            write!(f, ": {}", self.message)?;
        }
        if let Some(source) = &self.source {
            write!(f, ": {source}")?;
        }
        Ok(())
    }
}

impl std::error::Error for ArchiveError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source
            .as_deref()
            .map(|source| source as &(dyn std::error::Error + 'static))
    }
}

impl From<ArchiveError> for std::io::Error {
    fn from(error: ArchiveError) -> Self {
        std::io::Error::new(std::io::ErrorKind::InvalidData, error)
    }
}
