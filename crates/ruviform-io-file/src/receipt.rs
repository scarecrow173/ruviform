//! Receipts and recovery ownership: [`PublishReceipt`],
//! [`RecoveryInfo`], [`RecoveryHandle`], [`RecoveryObservation`],
//! [`CleanupReceipt`], [`CleanupFailure`].
//!
//! `RecoveryInfo` is a *diagnostic snapshot* — `OperationId` and
//! `temp_path` are not locks, CAS tokens, or proof of file identity.
//! `RecoveryHandle` is the *owned resource* needed to inspect and
//! clean a retained temp: it is held by [`PublishFailure`] and by
//! [`PublishReceipt`] when post-publish cleanup could not finish, and
//! its `Drop` never deletes anything.

use crate::error::FileError;
use crate::outcome::{DurabilityEvidence, OperationId, TempState};
use crate::platform::{PlatformFileOps, TempDisposition};
use std::fmt;
use std::path::{Path, PathBuf};

/// A non-destructive diagnostic snapshot of a staged operation's
/// recovery state.
///
/// `operation_id` / `destination` / `temp_path` are diagnostics —
/// they are not locks, CAS tokens, or same-file proofs for later
/// runs. `temp_state` is what the platform last verified.
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct RecoveryInfo {
    operation_id: OperationId,
    destination: PathBuf,
    temp_path: Option<PathBuf>,
    temp_state: TempState,
}

impl RecoveryInfo {
    /// The correlation id of the staged operation.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn operation_id(&self) -> OperationId {
        self.operation_id
    }

    /// The publish destination (diagnostic, not an identity proof).
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn destination(&self) -> &Path {
        &self.destination
    }

    /// The temp file path if the adapter tracks one (diagnostic,
    /// not an identity proof).
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn temp_path(&self) -> Option<&Path> {
        self.temp_path.as_deref()
    }

    /// The last verified temp state.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn temp_state(&self) -> TempState {
        self.temp_state
    }

    /// Updates the recorded temp state after the pipeline observes it.
    pub(crate) fn set_temp_state(&mut self, temp_state: TempState) {
        self.temp_state = temp_state;
    }

    /// Builds the snapshot carried by every staged operation.
    pub(crate) fn new(
        operation_id: OperationId,
        destination: PathBuf,
        temp_path: Option<PathBuf>,
        temp_state: TempState,
    ) -> Self {
        Self {
            operation_id,
            destination,
            temp_path,
            temp_state,
        }
    }
}

/// What [`RecoveryHandle::inspect`] observed about the staged temp.
///
/// `#[non_exhaustive]` — the observation surface may grow (e.g. size
/// or identity details) once OS adapters can verify more.
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct RecoveryObservation {
    temp_state: TempState,
}

impl RecoveryObservation {
    /// The observed temp state.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn temp_state(&self) -> TempState {
        self.temp_state
    }

    pub(crate) fn new(temp_state: TempState) -> Self {
        Self { temp_state }
    }
}

/// The owned resource needed to inspect and clean a retained temp.
///
/// Obtained from [`PublishFailure::take_recovery_handle`] or
/// [`PublishReceipt::take_recovery_handle`], or returned inside
/// [`CleanupFailure`] for retry. `Drop` **never** deletes the temp
/// file — indeterminate leftovers are removed only by an explicit
/// [`cleanup`](Self::cleanup) call.
///
/// **RT contract:** non-RT only — filesystem I/O.
///
/// [`PublishFailure::take_recovery_handle`]: crate::PublishFailure::take_recovery_handle
/// [`PublishReceipt::take_recovery_handle`]: crate::PublishReceipt::take_recovery_handle
pub struct RecoveryHandle {
    ops: Box<dyn PlatformFileOps>,
    // Boxed to keep `PublishFailure`/`CleanupFailure` small.
    info: Box<RecoveryInfo>,
}

impl RecoveryHandle {
    pub(crate) fn new(ops: Box<dyn PlatformFileOps>, info: RecoveryInfo) -> Self {
        Self {
            ops,
            info: Box::new(info),
        }
    }

    /// The diagnostic snapshot this handle was captured with.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn info(&self) -> &RecoveryInfo {
        &self.info
    }

    /// Re-inspects the staged temp's current state.
    ///
    /// This is an observation, not an identity proof — a temp name
    /// could belong to a different file than the one staged.
    ///
    /// **RT contract:** non-RT only — filesystem query.
    ///
    /// # Errors
    ///
    /// [`FileErrorKind::Io`](crate::FileErrorKind::Io) /
    /// [`FileErrorKind::Unsupported`](crate::FileErrorKind::Unsupported)
    /// when the platform cannot inspect the temp.
    pub fn inspect(&self) -> Result<RecoveryObservation, FileError> {
        let disposition = self.ops.inspect_temp().map_err(FileError::from_ops_error)?;
        Ok(RecoveryObservation::new(disposition.temp_state()))
    }

    /// Removes the retained temp after re-verifying ownership.
    ///
    /// **RT contract:** non-RT only — filesystem I/O.
    ///
    /// # Errors
    ///
    /// [`CleanupFailure`] — which hands the handle back so cleanup
    /// can be retried.
    pub fn cleanup(mut self) -> Result<CleanupReceipt, CleanupFailure> {
        let attempt = self.ops.cleanup_temp();
        match attempt.temp {
            TempDisposition::Removed => Ok(CleanupReceipt::new(
                self.info.operation_id(),
                TempState::Absent,
            )),
            TempDisposition::Retained | TempDisposition::Unknown => {
                let mut info = *self.info;
                info.set_temp_state(attempt.temp.temp_state());
                Err(CleanupFailure::new(
                    info.clone(),
                    attempt.error,
                    RecoveryHandle::new(self.ops, info),
                ))
            }
        }
    }
}

impl fmt::Debug for RecoveryHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RecoveryHandle")
            .field("operation_id", &self.info.operation_id())
            .field("destination", &self.info.destination())
            .field("temp_path", &self.info.temp_path())
            .finish_non_exhaustive()
    }
}

/// Proof that a cleanup finished.
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct CleanupReceipt {
    operation_id: OperationId,
    temp_state: TempState,
}

impl CleanupReceipt {
    /// The correlation id of the cleaned operation.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn operation_id(&self) -> OperationId {
        self.operation_id
    }

    /// The temp state after cleanup — `Absent` on success.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn temp_state(&self) -> TempState {
        self.temp_state
    }

    pub(crate) fn new(operation_id: OperationId, temp_state: TempState) -> Self {
        Self {
            operation_id,
            temp_state,
        }
    }
}

/// A failed cleanup — holds the [`RecoveryHandle`] so the caller can
/// inspect or retry.
///
/// **RT contract:** non-RT only.
///
/// # Examples
///
/// ```no_run
/// use ruviform_io_file::StagedFile;
///
/// let staged = StagedFile::create("out.bin")?;
/// if let Err(mut failure) = staged.abort() {
///     eprintln!("abort failed: {failure}");
///     if let Some(handle) = failure.take_recovery_handle() {
///         // handle.inspect() / handle.cleanup() can retry.
///         let _ = handle.cleanup();
///     }
/// }
/// # Ok::<(), ruviform_io_file::FileError>(())
/// ```
#[derive(Debug)]
pub struct CleanupFailure {
    info: RecoveryInfo,
    source: Option<std::io::Error>,
    recovery: Option<RecoveryHandle>,
}

impl CleanupFailure {
    pub(crate) fn new(
        info: RecoveryInfo,
        source: Option<std::io::Error>,
        recovery: RecoveryHandle,
    ) -> Self {
        Self {
            info,
            source,
            recovery: Some(recovery),
        }
    }

    /// The diagnostic snapshot of the failed cleanup.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn info(&self) -> &RecoveryInfo {
        &self.info
    }

    /// The recovery handle kept for retry.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn recovery_handle(&self) -> Option<&RecoveryHandle> {
        self.recovery.as_ref()
    }

    /// Moves the recovery handle out for inspection or a retry of
    /// [`RecoveryHandle::cleanup`]. Failures built by this crate
    /// always carry the handle.
    ///
    /// **RT contract:** non-RT only — filesystem I/O through the
    /// returned handle.
    pub fn take_recovery_handle(&mut self) -> Option<RecoveryHandle> {
        self.recovery.take()
    }
}

impl fmt::Display for CleanupFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ruviform-io-file: cleanup failed ({}) — temp state: {}",
            self.info.operation_id(),
            self.info.temp_state(),
        )
    }
}

impl std::error::Error for CleanupFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source
            .as_ref()
            .map(|error| error as &(dyn std::error::Error + 'static))
    }
}

/// A successful publish: what was published and what was achieved.
///
/// `Ok(PublishReceipt)` means the requested mode, visibility and
/// durability were *all* achieved — a requested sync that failed is
/// never reported as succeeded. When post-publish temp cleanup could
/// not finish, the receipt carries a [`RecoveryHandle`] — the publish
/// result stays valid and the handle owns the leftover.
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Debug)]
pub struct PublishReceipt {
    operation_id: OperationId,
    destination: PathBuf,
    durability: DurabilityEvidence,
    temp_state: TempState,
    recovery: Option<RecoveryHandle>,
}

impl PublishReceipt {
    pub(crate) fn new(
        operation_id: OperationId,
        destination: PathBuf,
        durability: DurabilityEvidence,
        temp_state: TempState,
        recovery: Option<RecoveryHandle>,
    ) -> Self {
        Self {
            operation_id,
            destination,
            durability,
            temp_state,
            recovery,
        }
    }

    /// The correlation id of the published operation.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn operation_id(&self) -> OperationId {
        self.operation_id
    }

    /// The destination that was published (diagnostic, not an
    /// identity proof — another writer may have changed it since).
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn destination(&self) -> &Path {
        &self.destination
    }

    /// The sync work actually observed to succeed — never the
    /// requested level.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn durability(&self) -> DurabilityEvidence {
        self.durability
    }

    /// The temp state after publish and cleanup.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn temp_state(&self) -> TempState {
        self.temp_state
    }

    /// The recovery handle, if post-publish temp cleanup left a
    /// retained temp.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn recovery_handle(&self) -> Option<&RecoveryHandle> {
        self.recovery.as_ref()
    }

    /// Moves the recovery handle out (present only when the receipt
    /// reports a retained temp).
    ///
    /// **RT contract:** non-RT only — filesystem I/O through the
    /// returned handle.
    pub fn take_recovery_handle(&mut self) -> Option<RecoveryHandle> {
        self.recovery.take()
    }
}
