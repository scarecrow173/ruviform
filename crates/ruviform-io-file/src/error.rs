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
    /// Underlying I/O failed (including `PermissionDenied` — the raw
    /// `io::ErrorKind` is kept in `source` but not elevated to a
    /// stable kind).
    Io,
    /// The platform adapter does not provide the requested capability
    /// — including `StagedFile::create` while the OS backend is still
    /// unimplemented (IO-IMP-02), a readback surface the platform
    /// cannot offer safely, a target without a handle-relative
    /// no-follow backend, Windows builds older than RS1, Linux
    /// without `/proc`, and `pin`/`open_pinned` on a filesystem whose
    /// identity strength cannot discriminate delete+recreate.
    Unsupported,
    /// A supplied argument (e.g. a destination without a file name or
    /// a [`RelativePath`] grammar violation) was invalid.
    ///
    /// [`RelativePath`]: crate::rooted::RelativePath
    InvalidInput,
    /// No object exists at the resolved position (`ENOENT` /
    /// `STATUS_OBJECT_NAME_NOT_FOUND` family), or a link target was
    /// empty.
    NotFound,
    /// A directory was expected but the object is not one (`ENOTDIR`
    /// family), including a link where `NoFollow` forbids traversal
    /// (`open_dir`/`read_dir`, and intermediate components of any
    /// rooted API).
    NotDirectory,
    /// A regular file was required but the resolved object is not
    /// one (`EISDIR`, nonregular leaf: FIFO/device/socket/dir/link
    /// under `NoFollow`).
    NotRegularFile,
    /// Resolution would touch an object outside the capability
    /// root: an absolute link whose target does not keep the
    /// canonical-root prefix (tainted), a `..` that pops below the
    /// capability-root floor, or a link target resolving outside.
    EscapesRoot,
    /// A link expansion exceeded the hop budget or formed a loop
    /// (raised by the resolve layer — distinct from a kernel
    /// `ELOOP`).
    LoopDetected,
    /// `open_pinned` found a different object identity than the
    /// pinned one — a swapped/recreated object, never its bytes.
    IdentityMismatch,
    /// A resolve budget was exceeded: locator depth, expanded
    /// component total, or simultaneous open directory handles.
    LimitExceeded,
}

impl fmt::Display for FileErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::Io => "I/O failure",
            Self::Unsupported => "unsupported feature",
            Self::InvalidInput => "invalid input",
            Self::NotFound => "not found",
            Self::NotDirectory => "not a directory",
            Self::NotRegularFile => "not a regular file",
            Self::EscapesRoot => "path escapes the capability root",
            Self::LoopDetected => "link expansion loop detected",
            Self::IdentityMismatch => "object identity mismatch",
            Self::LimitExceeded => "resolve budget exceeded",
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
    /// everything else → [`FileErrorKind::Io`]. The rooted
    /// capability's richer mapping ([`NotFound`], [`NotDirectory`],
    /// [`NotRegularFile`], …) is applied by `rooted` at its own
    /// boundary; staged-publish `io` errors stay `Io` for backward
    /// compatibility.
    ///
    /// [`NotFound`]: FileErrorKind::NotFound
    /// [`NotDirectory`]: FileErrorKind::NotDirectory
    /// [`NotRegularFile`]: FileErrorKind::NotRegularFile
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
