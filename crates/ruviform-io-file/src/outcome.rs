//! Publish outcome vocabulary: [`PublishEffect`] classification,
//! [`PublishPhase`], [`DurabilityEvidence`], [`TempState`],
//! [`OperationId`], and the [`PublishFailure`] error type.
//!
//! The vocabulary exists to keep three facts un-confusable: whether
//! the destination was published, how much durability was *actually*
//! achieved, and what happened to the staged temp file. A rename
//! after which directory sync failed is `Published` with
//! `FileSynced` evidence — neither "not published" nor fully durable.

use crate::receipt::{RecoveryHandle, RecoveryInfo};
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

/// Correlation id carried by receipts, failures and recovery
/// diagnostics.
///
/// This is a *diagnostic* identifier only — it is not a lock, not a
/// CAS token, and not proof of file identity. Values are unique
/// within the process.
///
/// **RT contract:** non-RT only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OperationId(u64);

impl OperationId {
    /// The raw id value.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Allocates the next process-unique id.
    pub(crate) fn next() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self(NEXT.fetch_add(1, Ordering::Relaxed))
    }
}

impl fmt::Display for OperationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "op-{}", self.0)
    }
}

/// Classification of what a failed or succeeded `publish` did to the
/// destination.
///
/// This is the honest trichotomy the whole API is built around:
/// "applied", "definitively not applied", and "cannot tell".
/// Ambiguous outcomes normalize to [`Indeterminate`](Self::Indeterminate)
/// — never to a comforting but false `NotPublished`.
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PublishEffect {
    /// The destination was *not* published — stated only when the OS
    /// contract makes non-application certain (the publish stage was
    /// never reached, or the adapter reported `NotApplied`).
    NotPublished,
    /// The destination *was* published. Used on success — and on
    /// failures that happen *after* the publish applied (e.g.
    /// directory-sync failure): the file exists, whatever else went
    /// wrong.
    Published,
    /// The outcome cannot be determined (e.g. the OS publish call
    /// reported an ambiguous error or its result is unknowable).
    /// Leftovers are **not** automatically deleted in this state;
    /// inspect and clean up through the attached
    /// [`RecoveryHandle`](crate::RecoveryHandle).
    Indeterminate,
}

impl fmt::Display for PublishEffect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::NotPublished => "not published",
            Self::Published => "published",
            Self::Indeterminate => "indeterminate",
        };
        f.write_str(message)
    }
}

/// The pipeline stage at which a publish failed.
///
/// The stages are intentionally distinct: `Write::flush`, file sync,
/// handle/close preparation, the atomic publish itself, directory
/// sync and temp cleanup are different operations with different
/// failure semantics. OS close-time error detection is not
/// over-trusted — `Close` is a defined pipeline stage, not a promise
/// that every platform can observe close errors.
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PublishPhase {
    /// Request/capability validation before any I/O stage ran.
    Validate,
    /// The pre-publish file sync (`Durability::FileSync` and up).
    FileSync,
    /// Handle preparation/close before the publish operation.
    Close,
    /// The atomic publish operation itself.
    Publish,
    /// The post-publish directory sync
    /// (`Durability::FileAndDirectorySync` only).
    DirectorySync,
    /// Post-publish temp cleanup.
    Cleanup,
}

impl fmt::Display for PublishPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::Validate => "validate",
            Self::FileSync => "file sync",
            Self::Close => "close",
            Self::Publish => "publish",
            Self::DirectorySync => "directory sync",
            Self::Cleanup => "cleanup",
        };
        f.write_str(message)
    }
}

/// The sync work that was *actually observed to succeed* — evidence,
/// not the requested level.
///
/// Reported on both [`PublishReceipt`](crate::PublishReceipt) and
/// [`PublishFailure`]. Even `FileAndDirectorySynced` is not a claim
/// of total power-loss immunity; it is what the platform reported
/// completing.
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DurabilityEvidence {
    /// No sync was performed (or requested).
    None,
    /// The staged file's contents were synced before publish.
    FileSynced,
    /// File sync plus a post-publish directory sync both completed.
    FileAndDirectorySynced,
}

impl fmt::Display for DurabilityEvidence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::None => "no sync performed",
            Self::FileSynced => "file synced",
            Self::FileAndDirectorySynced => "file and directory synced",
        };
        f.write_str(message)
    }
}

/// Known state of the staged temp file — the public half of the
/// adapter's `TempDisposition` vocabulary.
///
/// Paths in [`RecoveryInfo`](crate::RecoveryInfo) are diagnostics
/// only; `TempState` is what the platform could actually verify at
/// the time it was recorded — it is not a promise about the future.
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TempState {
    /// The temp file is gone (consumed by publish, or removed).
    Absent,
    /// The temp file is still present.
    Retained,
    /// The temp file's presence could not be determined.
    Unknown,
}

impl fmt::Display for TempState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::Absent => "temp absent",
            Self::Retained => "temp retained",
            Self::Unknown => "temp state unknown",
        };
        f.write_str(message)
    }
}

/// Why a publish failed — the matchable surface of
/// [`PublishFailure`]. The stage and outcome live in
/// [`PublishFailure::phase`] / [`PublishFailure::effect`]; `kind`
/// names the reason category.
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PublishFailureKind {
    /// `PublishMode::CreateNew` was refused because the destination
    /// already exists — the destination was left untouched.
    DestinationExists,
    /// The requested mode/visibility/durability is beyond the
    /// platform adapter's capability — reported, never silently
    /// degraded.
    Unsupported,
    /// Any other underlying I/O failure (see `source()` for the
    /// OS-level diagnostic).
    Io,
}

impl fmt::Display for PublishFailureKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::DestinationExists => "destination already exists",
            Self::Unsupported => "unsupported feature",
            Self::Io => "I/O failure",
        };
        f.write_str(message)
    }
}

/// A failed publish: classification, stage, achieved durability,
/// recovery ownership, and the OS-level diagnostic.
///
/// `PublishFailure` *owns* the recovery path: [`info`](Self::info)
/// is a non-destructive [`RecoveryInfo`] snapshot and
/// [`take_recovery_handle`](Self::take_recovery_handle) moves out the
/// [`RecoveryHandle`] needed to inspect or clean the staged temp.
/// Dropping a `PublishFailure` never deletes the temp file.
///
/// **RT contract:** non-RT only — may allocate and perform
/// filesystem I/O through the recovery handle.
///
/// # Examples
///
/// ```no_run
/// use ruviform_io_file::{Durability, PublishEffect, PublishMode, PublishOptions, StagedFile};
///
/// let staged = StagedFile::create("out.bin")?;
/// let sealed = staged.seal();
/// if let Err(mut failure) = sealed.publish(PublishOptions::new(
///     PublishMode::CreateNew,
///     Durability::FileSync,
/// )) {
///     eprintln!("publish failed at {}: effect={}", failure.phase(), failure.effect());
///     if failure.effect() != PublishEffect::Indeterminate {
///         // Safe to clean up: non-indeterminate outcomes keep no
///         // ambiguous temp state.
///         if let Some(handle) = failure.take_recovery_handle() {
///             let _ = handle.cleanup();
///         }
///     }
/// }
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug)]
pub struct PublishFailure {
    kind: PublishFailureKind,
    phase: PublishPhase,
    effect: PublishEffect,
    durability: DurabilityEvidence,
    info: RecoveryInfo,
    recovery: Option<RecoveryHandle>,
    source: Option<std::io::Error>,
}

impl PublishFailure {
    /// Builds a failure with full classification. Crate-internal:
    /// only the pipeline knows the honest classification.
    pub(crate) fn new(
        kind: PublishFailureKind,
        phase: PublishPhase,
        effect: PublishEffect,
        durability: DurabilityEvidence,
        info: RecoveryInfo,
        recovery: Option<RecoveryHandle>,
        source: Option<std::io::Error>,
    ) -> Self {
        Self {
            kind,
            phase,
            effect,
            durability,
            info,
            recovery,
            source,
        }
    }

    /// Why the publish failed.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn kind(&self) -> PublishFailureKind {
        self.kind
    }

    /// The pipeline stage that failed.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn phase(&self) -> PublishPhase {
        self.phase
    }

    /// What the publish did to the destination.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn effect(&self) -> PublishEffect {
        self.effect
    }

    /// The sync work that was observed to succeed before the failure.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn durability(&self) -> DurabilityEvidence {
        self.durability
    }

    /// The non-destructive recovery snapshot.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn info(&self) -> &RecoveryInfo {
        &self.info
    }

    /// The recovery handle, if still attached.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn recovery_handle(&self) -> Option<&RecoveryHandle> {
        self.recovery.as_ref()
    }

    /// Moves the recovery handle out for inspection/cleanup.
    ///
    /// **RT contract:** non-RT only — filesystem I/O through the
    /// returned handle.
    pub fn take_recovery_handle(&mut self) -> Option<RecoveryHandle> {
        self.recovery.take()
    }
}

impl fmt::Display for PublishFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ruviform-io-file: publish failed in {} phase: {} ({}) — effect: {}, durability: {}",
            self.phase,
            self.kind,
            self.info.operation_id(),
            self.effect,
            self.durability,
        )
    }
}

impl std::error::Error for PublishFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source
            .as_ref()
            .map(|error| error as &(dyn std::error::Error + 'static))
    }
}
