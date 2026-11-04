//! Private OS-ops boundary — `PlatformFileOps`.
//!
//! Every OS interaction the staged-publish pipeline performs is
//! funneled through this trait so platform backends stay swappable
//! implementation details and the pipeline can be driven entirely by
//! a fault-injection mock. OS-specific APIs, handle types and error
//! shapes never leak into the public surface: failures arrive as
//! `io::Error` with conventional `ErrorKind`s (`Unsupported` →
//! capability, `AlreadyExists` → destination conflict), and publish
//! attempts report a *classification* ([`ApplyOutcome`]) rather than a
//! bare `Result`, so an ambiguous OS outcome normalizes to
//! `Indeterminate` instead of a false `NotPublished`.
//!
//! The real OS backend is [`crate::os::OsFileOps`] (IO-IMP-02,
//! Issue #296), wired in by [`staged_file`].

use crate::options::{PublishMode, PublishOptions};
use crate::outcome::TempState;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

/// Read+Seek object surface for the sealed-file readback borrow.
///
/// A blanket impl covers any `Read + Seek` type; the trait exists
/// because trait-object upcasting of `dyn Read + Seek` is not a
/// stable-language feature.
pub(crate) trait ReadSeek: Read + Seek {}

impl<T: Read + Seek> ReadSeek for T {}

/// What the publish operation did to the destination — the adapter's
/// own classification, independent of the error value it may also
/// report.
///
/// `cfg_attr(not(test), allow(dead_code))`: variants are constructed
/// by platform adapters; on targets without an OS backend
/// (`cfg(not(any(unix, windows)))`) only the test mock constructs
/// them.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ApplyOutcome {
    /// The destination now holds the staged content.
    Applied,
    /// The publish definitively did not apply (e.g. refused by
    /// `CreateNew` semantics, permission denied).
    NotApplied,
    /// The adapter cannot determine whether the publish applied.
    /// Normalized to `PublishEffect::Indeterminate` by the pipeline.
    Unknown,
}

/// What happened to the staged temp file — reported independently of
/// the publish outcome so leftover handling can be reasoned about
/// separately.
///
/// `cfg_attr(not(test), allow(dead_code))`: same adapter-constructor
/// boundary as [`ApplyOutcome`].
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TempDisposition {
    /// The temp was consumed by the publish or removed.
    Removed,
    /// The temp is still present.
    Retained,
    /// The temp's presence could not be determined.
    Unknown,
}

impl TempDisposition {
    /// Maps to the public [`TempState`] vocabulary.
    pub(crate) const fn temp_state(self) -> TempState {
        match self {
            Self::Removed => TempState::Absent,
            Self::Retained => TempState::Retained,
            Self::Unknown => TempState::Unknown,
        }
    }
}

/// The adapter's report of a publish attempt.
///
/// `effect` and `temp` are independent axes; `error` carries the
/// OS-level diagnostic when one exists (also for `Applied`-uncertain
/// reports — `Unknown` typically carries the ambiguous error).
#[derive(Debug)]
pub(crate) struct PublishAttempt {
    pub effect: ApplyOutcome,
    pub temp: TempDisposition,
    pub error: Option<io::Error>,
}

/// The adapter's report of a cleanup attempt.
#[derive(Debug)]
pub(crate) struct CleanupAttempt {
    pub temp: TempDisposition,
    pub error: Option<io::Error>,
}

/// The private platform-ops contract every staged file delegates to.
///
/// Implementors must be `Send` so `StagedFile` is `Send`. Methods
/// report failures as `io::Error`; two `ErrorKind`s carry semantic
/// weight for the pipeline's classification:
///
/// - [`io::ErrorKind::Unsupported`] — the adapter cannot provide the
///   requested capability (→ `*Kind::Unsupported`); and
/// - [`io::ErrorKind::AlreadyExists`] — the destination exists (→
///   `PublishFailureKind::DestinationExists`).
pub(crate) trait PlatformFileOps: Send {
    /// Writes `buf` to the staged temp at the current position.
    fn write(&mut self, buf: &[u8]) -> io::Result<usize>;

    /// Flushes buffered writes to the staged temp.
    fn flush(&mut self) -> io::Result<()>;

    /// Moves the staged temp's write cursor.
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64>;

    /// Borrows a `Read + Seek` reader over the *same* staged object.
    ///
    /// The reader starts at position 0. Adapters that cannot offer a
    /// read surface on the same object — e.g. a write-only handle —
    /// return `Err` with [`io::ErrorKind::Unsupported`]; an anonymous
    /// path re-open is not a safe substitute (TOCTOU).
    fn open_readback(&mut self) -> io::Result<&mut dyn ReadSeek>;

    /// Validates the requested publish against adapter capability —
    /// pipeline phase `Validate`.
    fn validate_publish(&self, options: PublishOptions) -> io::Result<()>;

    /// Syncs the staged file's contents — pipeline phase `FileSync`,
    /// run only when `Durability >= FileSync` is requested.
    fn sync_file(&mut self) -> io::Result<()>;

    /// Flushes/closes the write side ahead of the publish —
    /// pipeline phase `Close`. Platforms whose close cannot report
    /// errors keep this a no-op rather than pretending otherwise.
    fn prepare_for_publish(&mut self) -> io::Result<()>;

    /// The atomic publish attempt — pipeline phase `Publish`.
    /// Classifies the outcome itself via [`ApplyOutcome`] and reports
    /// the resulting [`TempDisposition`] separately.
    fn publish(&mut self, mode: PublishMode) -> PublishAttempt;

    /// Syncs the destination's directory after a publish — pipeline
    /// phase `DirectorySync`, run only when
    /// `Durability::FileAndDirectorySync` is requested.
    fn sync_directory(&mut self) -> io::Result<()>;

    /// Removes the staged temp — used by `abort`, `Drop` best-effort
    /// cleanup, and post-publish leftover removal.
    fn cleanup_temp(&mut self) -> CleanupAttempt;

    /// Reports the temp's current state for
    /// [`crate::RecoveryHandle::inspect`].
    fn inspect_temp(&self) -> io::Result<TempDisposition>;

    /// The staged temp's path, if the adapter tracks one —
    /// *diagnostic* identity only (recorded in [`crate::RecoveryInfo`]
    /// for `inspect`/`cleanup`), never an identity proof or lock.
    fn temp_path(&self) -> Option<&Path>;
}

/// The default platform backend factory.
///
/// Wires `StagedFile::create` to the real OS backend
/// ([`crate::os::OsFileOps`], IO-IMP-02 / Issue #296) on unix and
/// Windows. Other targets report `Unsupported` rather than pretend a
/// backend exists.
pub(crate) fn staged_file(destination: &Path) -> io::Result<Box<dyn PlatformFileOps>> {
    os_staged_file(destination)
}

#[cfg(any(unix, windows))]
fn os_staged_file(destination: &Path) -> io::Result<Box<dyn PlatformFileOps>> {
    Ok(Box::new(crate::os::OsFileOps::create(destination)?))
}

#[cfg(not(any(unix, windows)))]
fn os_staged_file(_destination: &Path) -> io::Result<Box<dyn PlatformFileOps>> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "ruviform-io-file: no OS file backend for this target",
    ))
}
