//! Normalized [`FileError`] / [`FileErrorKind`] model for the file-I/O
//! boundary.
//!
//! Every failure crossing a platform adapter is normalized into the
//! [`FileErrorKind`] classification; adapter-specific detail is carried
//! only as a type-erased diagnostic reachable through
//! [`std::error::Error::source`]. No OS error type ever appears in the
//! public API.
//!
//! Publish-time failures do **not** use this type — they carry far
//! richer state ([`PublishFailure`]: effect, phase, achieved
//! durability, recovery ownership). `FileError` covers
//! [`StagedFile::create`], [`SealedFile::open_readback`] and
//! [`RecoveryHandle::inspect`].
//!
//! [`PublishFailure`]: crate::PublishFailure
//! [`StagedFile::create`]: crate::StagedFile::create
//! [`SealedFile::open_readback`]: crate::SealedFile::open_readback
//! [`RecoveryHandle::inspect`]: crate::RecoveryHandle::inspect

use std::fmt;

/// Normalized failure classification for file-I/O operations.
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
/// use ruviform_io_file::FileErrorKind;
///
/// assert_eq!(FileErrorKind::Unsupported.to_string(), "unsupported feature");
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileErrorKind {
    /// Underlying I/O failed.
    Io,
    /// The platform adapter does not provide the requested capability
    /// — including `StagedFile::create` while the OS backend is still
    /// unimplemented (IO-IMP-02), or a readback surface the platform
    /// cannot offer safely.
    Unsupported,
    /// A supplied argument (e.g. a destination without a file name)
    /// was invalid.
    InvalidInput,
}

impl fmt::Display for FileErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::Io => "I/O failure",
            Self::Unsupported => "unsupported feature",
            Self::InvalidInput => "invalid input",
        };
        f.write_str(message)
    }
}

/// File-I/O error: a normalized [`FileErrorKind`] classification plus
/// an optional type-erased diagnostic.
///
/// `FileError` is `Send`: it may cross thread boundaries like any
/// other data type.
///
/// **RT contract:** non-RT only — may allocate (boxed diagnostic).
///
/// # Examples
///
/// ```
/// use ruviform_io_file::{FileError, FileErrorKind};
///
/// let error = FileError::from_kind(FileErrorKind::Unsupported);
/// assert_eq!(error.kind(), FileErrorKind::Unsupported);
/// ```
#[derive(Debug)]
pub struct FileError {
    /// Normalized classification — the stable, matchable surface.
    kind: FileErrorKind,
    /// Type-erased adapter diagnostic, exposed via
    /// [`std::error::Error::source`].
    source: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
}

impl FileError {
    /// Builds an error carrying only a normalized classification.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn from_kind(kind: FileErrorKind) -> Self {
        Self { kind, source: None }
    }

    /// Builds an error with a type-erased adapter diagnostic attached.
    ///
    /// **RT contract:** non-RT only.
    pub fn with_source(
        kind: FileErrorKind,
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
    pub const fn kind(&self) -> FileErrorKind {
        self.kind
    }

    /// Maps an adapter `io::Error` to the normalized kind:
    /// `io::ErrorKind::Unsupported` → [`FileErrorKind::Unsupported`],
    /// `io::ErrorKind::InvalidInput` → [`FileErrorKind::InvalidInput`],
    /// everything else → [`FileErrorKind::Io`].
    pub(crate) fn from_ops_error(error: std::io::Error) -> Self {
        let kind = match error.kind() {
            std::io::ErrorKind::Unsupported => FileErrorKind::Unsupported,
            std::io::ErrorKind::InvalidInput => FileErrorKind::InvalidInput,
            _ => FileErrorKind::Io,
        };
        Self::with_source(kind, error)
    }
}

impl fmt::Display for FileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ruviform-io-file: {}", self.kind)?;
        if let Some(source) = &self.source {
            write!(f, ": {source}")?;
        }
        Ok(())
    }
}

impl std::error::Error for FileError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source
            .as_deref()
            .map(|source| source as &(dyn std::error::Error + 'static))
    }
}
