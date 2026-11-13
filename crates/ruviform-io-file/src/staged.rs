//! [`StagedFile`] / [`SealedFile`] / [`ReadbackFile`] — the type-state
//! lifecycle — plus the internal [`StageOwner`] that owns the temp
//! across transitions and drives the publish pipeline.
//!
//! Ownership rules:
//!
//! - `StageOwner` holds the adapter object
//!   (`Option<Box<dyn PlatformFileOps>>`), the [`RecoveryInfo`]
//!   snapshot, and `cleanup_on_drop`. It is the only `Drop`
//!   implementation — `StagedFile`/`SealedFile` move it by value, so
//!   `seal`/`publish`/`abort` never run destructors twice.
//! - `publish`/`abort` take `self`, disable `cleanup_on_drop`, and
//!   move the adapter into the outcome's [`RecoveryHandle`] via
//!   `ops.take()` (fields cannot be moved out of a `Drop` type —
//!   the `Option` indirection exists for that).
//! - [`RecoveryHandle`]'s `Drop` never deletes the temp — only an
//!   explicit `cleanup()` does. An `Indeterminate` leftover is
//!   therefore never auto-deleted.

use crate::error::{FileError, FileErrorKind};
use crate::options::{Durability, PublishOptions};
use crate::outcome::{
    DurabilityEvidence, OperationId, PublishEffect, PublishFailure, PublishFailureKind,
    PublishPhase, TempState,
};
use crate::platform::{
    ApplyOutcome, CleanupAttempt, PlatformFileOps, ReadSeek, TempDisposition, staged_file,
};
use crate::receipt::{
    CleanupFailure, CleanupReceipt, PublishReceipt, RecoveryHandle, RecoveryInfo,
};
use std::fmt;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;

/// Internal owner shared by both type states: the adapter object, the
/// diagnostic snapshot, and the drop-time cleanup policy.
pub(crate) struct StageOwner {
    /// `Option` so the pipeline can move the adapter into a
    /// [`RecoveryHandle`] — a field cannot be moved out of a `Drop`
    /// type. Invariant: `Some` while the owner is reachable.
    ops: Option<Box<dyn PlatformFileOps>>,
    /// Diagnostic snapshot — `temp_state` is updated as the pipeline
    /// observes the adapter's reports.
    info: RecoveryInfo,
    /// Whether `Drop` performs best-effort temp cleanup. `true` for
    /// un-published staged/sealed files; disabled the moment publish
    /// or abort consumes the owner (the outcome types take over).
    cleanup_on_drop: bool,
}

impl StageOwner {
    fn new(ops: Box<dyn PlatformFileOps>, info: RecoveryInfo) -> Self {
        Self {
            ops: Some(ops),
            info,
            cleanup_on_drop: true,
        }
    }

    /// The adapter object — always `Some` while reachable.
    fn ops(&mut self) -> &mut dyn PlatformFileOps {
        self.ops
            .as_deref_mut()
            .expect("ruviform-io-file: stage owner lost its ops object")
    }

    /// The adapter object shared (for `inspect`-style queries).
    fn ops_ref(&self) -> &dyn PlatformFileOps {
        self.ops
            .as_deref()
            .expect("ruviform-io-file: stage owner lost its ops object")
    }

    /// Builds a failure with the adapter moved into the attached
    /// recovery handle. Consumed-owner contract: `cleanup_on_drop`
    /// was already disabled by the caller.
    fn fail(
        &mut self,
        kind: PublishFailureKind,
        phase: PublishPhase,
        effect: PublishEffect,
        durability: DurabilityEvidence,
        source: Option<io::Error>,
    ) -> PublishFailure {
        let ops = self.ops.take();
        let recovery = ops.map(|ops| RecoveryHandle::new(ops, self.info.clone()));
        PublishFailure::new(
            kind,
            phase,
            effect,
            durability,
            self.info.clone(),
            recovery,
            source,
        )
    }
}

impl Drop for StageOwner {
    /// Best-effort temp cleanup for an un-published owner — never a
    /// completion path and never a deletion of an indeterminate
    /// leftover (those arrive here only via handles, which do not
    /// share this `Drop`).
    fn drop(&mut self) {
        if self.cleanup_on_drop
            && let Some(mut ops) = self.ops.take()
        {
            let _ = ops.cleanup_temp();
        }
    }
}

impl fmt::Debug for StageOwner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StageOwner")
            .field("operation_id", &self.info.operation_id())
            .field("destination", &self.info.destination())
            .field("temp_path", &self.info.temp_path())
            .field("temp_state", &self.info.temp_state())
            .field("cleanup_on_drop", &self.cleanup_on_drop)
            .finish_non_exhaustive()
    }
}

/// A staged file: writes go to a temp sibling of the destination and
/// nothing touches the destination until [`SealedFile::publish`].
///
/// `StagedFile` implements `Write + Seek` directly and is `Send` —
/// the internal file handle is never exposed and cannot be cloned
/// out. Drop and [`abort`](Self::abort) both delete the temp;
/// `abort` exists so callers can *say* "cancelled" instead of relying
/// on drop order.
///
/// The format/codec `finish()` call that makes the bytes complete is
/// the caller's responsibility — `seal` is only the type-level end
/// of the write phase, not a claim that the content is finished or
/// valid.
///
/// **RT contract:** non-RT only — filesystem I/O.
///
/// # Examples
///
/// ```no_run
/// use ruviform_io_file::{Durability, PublishMode, PublishOptions, StagedFile};
/// use std::io::Write;
///
/// let mut staged = StagedFile::create("out.bin")?;
/// staged.write_all(b"payload")?;
/// staged.flush()?;
/// let sealed = staged.seal();
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug)]
pub struct StagedFile {
    owner: StageOwner,
}

impl StagedFile {
    /// Creates a staged file for `destination` by opening a temp
    /// sibling in the destination's directory.
    ///
    /// The real OS backend (IO-IMP-02, Issue #296) stages an
    /// exclusively-created temp sibling on unix and Windows; other
    /// targets report [`FileErrorKind::Unsupported`]. The temp path
    /// is recorded in the [`RecoveryInfo`] as a diagnostic.
    ///
    /// **RT contract:** non-RT only — filesystem I/O.
    ///
    /// # Errors
    ///
    /// [`FileErrorKind::InvalidInput`] when `destination` has no file
    /// name; [`FileErrorKind::Unsupported`] on targets with no OS
    /// backend; [`FileErrorKind::Io`] when the temp file cannot be
    /// created (e.g. missing parent directory, permissions).
    pub fn create(destination: impl AsRef<Path>) -> Result<Self, FileError> {
        let destination = destination.as_ref();
        if destination.file_name().is_none() {
            return Err(FileError::with_source(
                FileErrorKind::InvalidInput,
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "ruviform-io-file: destination has no file name",
                ),
            ));
        }
        let ops = staged_file(destination).map_err(FileError::from_ops_error)?;
        let info = RecoveryInfo::new(
            OperationId::next(),
            destination.to_path_buf(),
            ops.temp_path().map(Path::to_path_buf),
            TempState::Retained,
        );
        Ok(Self {
            owner: StageOwner::new(ops, info),
        })
    }

    /// Test-support constructor: injects the adapter and snapshot
    /// directly (used by the fault-injection mock — `cfg(test)` unit
    /// tests and the opt-in `fault-injection` feature facade).
    #[cfg(any(test, feature = "fault-injection"))]
    pub(crate) fn with_ops(ops: Box<dyn PlatformFileOps>, info: RecoveryInfo) -> Self {
        Self {
            owner: StageOwner::new(ops, info),
        }
    }

    /// The eventual destination path.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn destination(&self) -> &Path {
        self.owner.info.destination()
    }

    /// The correlation id of this staged operation.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn operation_id(&self) -> OperationId {
        self.owner.info.operation_id()
    }

    /// The temp file path, if the adapter tracks one (diagnostic,
    /// not an identity proof).
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn temp_path(&self) -> Option<&Path> {
        self.owner.info.temp_path()
    }

    /// Ends the write phase: consumes the writer and yields the
    /// sealed file. Pure type transition — no I/O sync happens here;
    /// `publish` performs the requested syncs.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn seal(self) -> SealedFile {
        SealedFile { owner: self.owner }
    }

    /// Cancels the staged file: deletes the temp and never touches
    /// the destination.
    ///
    /// **RT contract:** non-RT only — filesystem I/O.
    ///
    /// # Errors
    ///
    /// [`CleanupFailure`] when the temp could not be removed — it
    /// carries the [`RecoveryHandle`] for inspect/retry.
    pub fn abort(self) -> Result<CleanupReceipt, CleanupFailure> {
        abort_owner(self.owner)
    }
}

impl Write for StagedFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.owner.ops().write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.owner.ops().flush()
    }
}

impl Seek for StagedFile {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.owner.ops().seek(pos)
    }
}

/// A sealed staged file: the write phase provably ended.
///
/// `SealedFile` does **not** implement `Write` or `Seek` — sealed
/// content cannot be modified through this API. It exposes three
/// operations:
///
/// - [`open_readback`](Self::open_readback) — borrow a read-only
///   `Read + Seek` reader over the staged temp for format-level
///   verification;
/// - [`publish`](Self::publish) — the requested sync + atomic
///   publish + directory sync pipeline;
/// - [`abort`](Self::abort) — delete the temp, touch nothing.
///
/// While a [`ReadbackFile`] borrow is alive, `publish`/`abort` cannot
/// be called — the `&mut self` borrow blocks the by-value consume.
///
/// ```compile_fail
/// use ruviform_io_file::SealedFile;
/// use std::io::Write;
///
/// fn try_write(mut sealed: SealedFile) {
///     // `SealedFile` intentionally does not implement `Write`.
///     sealed.write_all(b"nope").unwrap();
/// }
/// ```
///
/// ```compile_fail
/// use ruviform_io_file::SealedFile;
/// use std::io::{Seek, SeekFrom};
///
/// fn try_seek(mut sealed: SealedFile) {
///     // `SealedFile` intentionally does not implement `Seek`.
///     sealed.seek(SeekFrom::Start(0)).unwrap();
/// }
/// ```
///
/// **RT contract:** non-RT only — filesystem I/O.
#[derive(Debug)]
pub struct SealedFile {
    owner: StageOwner,
}

impl SealedFile {
    /// The eventual destination path.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn destination(&self) -> &Path {
        self.owner.info.destination()
    }

    /// The correlation id of this staged operation.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn operation_id(&self) -> OperationId {
        self.owner.info.operation_id()
    }

    /// The temp file path, if the adapter tracks one (diagnostic,
    /// not an identity proof).
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn temp_path(&self) -> Option<&Path> {
        self.owner.info.temp_path()
    }

    /// Borrows a read-only `Read + Seek` reader over the staged temp
    /// — for consumers that verify the sealed bytes (e.g. a format
    /// reopen) *before* deciding to publish.
    ///
    /// The reader sees the same object the temp holds (no anonymous
    /// path re-open — the read cannot be redirected to a swapped
    /// file), starts at position 0, and grants no write capability.
    /// While the returned [`ReadbackFile`] is alive this `SealedFile`
    /// is `&mut`-borrowed, so `publish`/`abort` cannot consume it.
    ///
    /// **RT contract:** non-RT only — filesystem I/O.
    ///
    /// # Errors
    ///
    /// [`FileErrorKind::Unsupported`] when the platform adapter
    /// cannot offer a safe readback surface; [`FileErrorKind::Io`]
    /// on other failures.
    pub fn open_readback(&mut self) -> Result<ReadbackFile<'_>, FileError> {
        let reader = self
            .owner
            .ops()
            .open_readback()
            .map_err(FileError::from_ops_error)?;
        Ok(ReadbackFile { inner: reader })
    }

    /// Publishes the sealed file per `options`.
    ///
    /// Pipeline: validate capability → file sync (if requested) →
    /// handle preparation → atomic publish → directory sync (if
    /// requested) → leftover-temp cleanup. `Ok(PublishReceipt)` is
    /// returned only when *every* requested guarantee was achieved —
    /// `durability` on the receipt is the achieved evidence, never
    /// the request echoed back. Any failure yields a
    /// [`PublishFailure`] classifying the effect, the failing phase,
    /// the achieved durability, and carrying a [`RecoveryHandle`].
    ///
    /// **RT contract:** non-RT only — filesystem I/O.
    ///
    /// # Errors
    ///
    /// [`PublishFailure`] — see [`PublishEffect`] for the
    /// not-published / published / indeterminate trichotomy.
    pub fn publish(self, options: PublishOptions) -> Result<PublishReceipt, PublishFailure> {
        let mut owner = self.owner;
        owner.cleanup_on_drop = false;
        publish_pipeline(&mut owner, options)
    }

    /// Cancels the sealed file: deletes the temp and never touches
    /// the destination.
    ///
    /// **RT contract:** non-RT only — filesystem I/O.
    ///
    /// # Errors
    ///
    /// [`CleanupFailure`] when the temp could not be removed — it
    /// carries the [`RecoveryHandle`] for inspect/retry.
    pub fn abort(self) -> Result<CleanupReceipt, CleanupFailure> {
        abort_owner(self.owner)
    }
}

/// The publish pipeline — the deterministic classification core.
///
/// Order per IO-01 v0.4: validate → file sync → handle preparation →
/// publish → directory sync → cleanup. `Ok` is emitted only when all
/// requested guarantees were achieved.
fn publish_pipeline(
    owner: &mut StageOwner,
    options: PublishOptions,
) -> Result<PublishReceipt, PublishFailure> {
    // Validate — capability checking happens before any I/O stage.
    if let Err(error) = owner.ops_ref().validate_publish(options) {
        let kind = if error.kind() == io::ErrorKind::Unsupported {
            PublishFailureKind::Unsupported
        } else {
            PublishFailureKind::Io
        };
        return Err(owner.fail(
            kind,
            PublishPhase::Validate,
            PublishEffect::NotPublished,
            DurabilityEvidence::None,
            Some(error),
        ));
    }

    // File sync — only when requested. Failure here is safely
    // NotPublished: the publish stage never ran.
    let mut evidence = DurabilityEvidence::None;
    if options.durability != Durability::BestEffort {
        if let Err(error) = owner.ops().sync_file() {
            return Err(owner.fail(
                PublishFailureKind::Io,
                PublishPhase::FileSync,
                PublishEffect::NotPublished,
                evidence,
                Some(error),
            ));
        }
        evidence = DurabilityEvidence::FileSynced;
    }

    // Close — handle preparation ahead of the publish operation.
    if let Err(error) = owner.ops().prepare_for_publish() {
        return Err(owner.fail(
            PublishFailureKind::Io,
            PublishPhase::Close,
            PublishEffect::NotPublished,
            evidence,
            Some(error),
        ));
    }

    // Publish — the adapter classifies the outcome itself.
    let attempt = owner.ops().publish(options.mode);
    owner.info.set_temp_state(attempt.temp.temp_state());
    match attempt.effect {
        ApplyOutcome::NotApplied => {
            let kind = match &attempt.error {
                Some(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                    PublishFailureKind::DestinationExists
                }
                Some(error) if error.kind() == io::ErrorKind::Unsupported => {
                    PublishFailureKind::Unsupported
                }
                _ => PublishFailureKind::Io,
            };
            Err(owner.fail(
                kind,
                PublishPhase::Publish,
                PublishEffect::NotPublished,
                evidence,
                attempt.error,
            ))
        }
        ApplyOutcome::Unknown => {
            // Honest indeterminacy: neither "not published" nor
            // "published" — and the leftover is NOT auto-deleted
            // (the handle owns it; its Drop never deletes).
            Err(owner.fail(
                PublishFailureKind::Io,
                PublishPhase::Publish,
                PublishEffect::Indeterminate,
                evidence,
                attempt.error,
            ))
        }
        ApplyOutcome::Applied => publish_applied(owner, options, attempt.temp, evidence),
    }
}

/// The post-`Applied` tail of the pipeline: directory sync (when
/// requested) then leftover-temp cleanup.
fn publish_applied(
    owner: &mut StageOwner,
    options: PublishOptions,
    publish_temp: TempDisposition,
    mut evidence: DurabilityEvidence,
) -> Result<PublishReceipt, PublishFailure> {
    // Directory sync — required only for FileAndDirectorySync.
    if options.durability == Durability::FileAndDirectorySync {
        match owner.ops().sync_directory() {
            Ok(()) => evidence = DurabilityEvidence::FileAndDirectorySynced,
            Err(error) => {
                // Published, but the requested durability was
                // not achieved — Err, not a false Ok.
                return Err(owner.fail(
                    PublishFailureKind::Io,
                    PublishPhase::DirectorySync,
                    PublishEffect::Published,
                    evidence,
                    Some(error),
                ));
            }
        }
    }

    // Cleanup — post-publish leftover handling. A cleanup miss does
    // not un-publish: the receipt reports the temp state and keeps a
    // recovery handle instead.
    let temp_state = match publish_temp {
        TempDisposition::Removed => TempState::Absent,
        TempDisposition::Retained | TempDisposition::Unknown => {
            let cleanup: CleanupAttempt = owner.ops().cleanup_temp();
            match cleanup.temp {
                TempDisposition::Removed => TempState::Absent,
                disposition @ (TempDisposition::Retained | TempDisposition::Unknown) => {
                    let state = disposition.temp_state();
                    owner.info.set_temp_state(state);
                    let ops = owner.ops.take();
                    let recovery = ops.map(|ops| RecoveryHandle::new(ops, owner.info.clone()));
                    return Ok(PublishReceipt::new(
                        owner.info.operation_id(),
                        owner.info.destination().to_path_buf(),
                        evidence,
                        state,
                        recovery,
                    ));
                }
            }
        }
    };
    Ok(PublishReceipt::new(
        owner.info.operation_id(),
        owner.info.destination().to_path_buf(),
        evidence,
        temp_state,
        None,
    ))
}

/// Shared `abort` body for both type states.
fn abort_owner(mut owner: StageOwner) -> Result<CleanupReceipt, CleanupFailure> {
    owner.cleanup_on_drop = false;
    let Some(mut ops) = owner.ops.take() else {
        unreachable!("ruviform-io-file: stage owner lost its ops object")
    };
    let attempt: CleanupAttempt = ops.cleanup_temp();
    match attempt.temp {
        TempDisposition::Removed => Ok(CleanupReceipt::new(
            owner.info.operation_id(),
            TempState::Absent,
        )),
        TempDisposition::Retained | TempDisposition::Unknown => {
            let mut info = owner.info.clone();
            info.set_temp_state(attempt.temp.temp_state());
            Err(CleanupFailure::new(
                info.clone(),
                attempt.error,
                RecoveryHandle::new(ops, info),
            ))
        }
    }
}

/// A read-only `Read + Seek` borrow over the staged temp — the
/// readback surface [`SealedFile::open_readback`] lends out.
///
/// This is **not** a `File` and not cloneable: no path to the
/// underlying OS handle, no write capability, no `try_clone`. It
/// starts at position 0 and borrows the [`SealedFile`] mutably, so
/// `publish`/`abort` cannot run until it is dropped.
///
/// **RT contract:** non-RT only — filesystem I/O.
pub struct ReadbackFile<'a> {
    inner: &'a mut dyn ReadSeek,
}

impl ReadbackFile<'_> {
    /// The borrowed reader as an opaque `Read + Seek` object — for
    /// handing to verifiers that take a generic reader.
    ///
    /// **RT contract:** non-RT only.
    pub fn reader(&mut self) -> impl Read + Seek + '_ {
        &mut *self.inner
    }
}

impl Read for ReadbackFile<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}

impl Seek for ReadbackFile<'_> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.inner.seek(pos)
    }
}

impl fmt::Debug for ReadbackFile<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReadbackFile").finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{MockFileOps, MockFs, MockPublish, MockScript};
    use crate::options::PublishMode;
    use crate::platform::PublishAttempt;
    use crate::receipt::RecoveryInfo;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    /// Public-API compile-time surface contract.
    const fn assert_traits<T: Write + Seek + Send>() {}
    const _: () = assert_traits::<StagedFile>();

    /// `ReadbackFile` lends `Read + Seek`.
    const fn assert_reader<T: Read + Seek>() {}
    const _: () = assert_reader::<ReadbackFile<'_>>();

    // Compile-fail neighbors live as doctests on `SealedFile` —
    // `Write`/`Seek` are not implemented for it.
    fn dest() -> PathBuf {
        PathBuf::from("dest.bin")
    }

    fn temp_path() -> PathBuf {
        PathBuf::from("dest.bin.rvf-tmp")
    }

    /// Builds a `StagedFile` backed by the mock adapter; `initial`
    /// seeds the temp bytes, `predest` seeds an existing destination.
    fn staged_with(
        script: MockScript,
        initial: &[u8],
        predest: Option<Vec<u8>>,
    ) -> (StagedFile, Arc<Mutex<MockFs>>) {
        let fs = Arc::new(Mutex::new(MockFs {
            temp: Some(initial.to_vec()),
            dest: predest,
            calls: Vec::new(),
        }));
        let ops = Box::new(MockFileOps::new(fs.clone(), script));
        let info = RecoveryInfo::new(
            OperationId::next(),
            dest(),
            Some(temp_path()),
            TempState::Retained,
        );
        (StagedFile::with_ops(ops, info), fs)
    }

    fn fs_temp(fs: &Arc<Mutex<MockFs>>) -> Option<Vec<u8>> {
        fs.lock().expect("fs poisoned").temp.clone()
    }

    fn fs_dest(fs: &Arc<Mutex<MockFs>>) -> Option<Vec<u8>> {
        fs.lock().expect("fs poisoned").dest.clone()
    }

    fn fs_calls(fs: &Arc<Mutex<MockFs>>) -> Vec<&'static str> {
        fs.lock().expect("fs poisoned").calls.clone()
    }

    fn io_err(kind: io::ErrorKind, msg: &'static str) -> io::Error {
        io::Error::new(kind, msg)
    }

    // ---------- create ----------

    #[test]
    fn create_stages_real_temp_via_os_backend() {
        // The real OS backend (IO-IMP-02) stages an exclusive temp
        // sibling; `abort` removes it. Full real-FS coverage lives in
        // `tests/os_publish.rs`.
        let dir = std::env::temp_dir().join(format!(
            "rvf-staged-create-{}-{:x}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos())
        ));
        std::fs::create_dir_all(&dir).expect("workdir");
        let staged = StagedFile::create(dir.join("real.bin")).expect("create");
        let temp = staged.temp_path().expect("temp path").to_path_buf();
        assert!(temp.exists(), "temp sibling must exist on disk");
        staged.abort().expect("abort");
        assert!(!temp.exists(), "abort removes the temp");
        std::fs::remove_dir(&dir).expect("cleanup");
    }

    #[test]
    fn create_rejects_nameless_destination() {
        for path in ["", "..", "/"] {
            let err = StagedFile::create(path).unwrap_err();
            assert_eq!(err.kind(), FileErrorKind::InvalidInput, "path: {path:?}");
        }
    }

    // ---------- staged phase ----------

    #[test]
    fn staged_write_seek_round_trip() {
        let (mut staged, fs) = staged_with(MockScript::default(), b"", None);
        staged.write_all(b"he").unwrap();
        staged.write_all(b"llo").unwrap();
        let pos = staged.seek(SeekFrom::Start(0)).unwrap();
        assert_eq!(pos, 0);
        staged.write_all(b"HEL").unwrap(); // overwrite prefix
        assert_eq!(fs_temp(&fs).as_deref(), Some(&b"HELlo"[..]));
        assert_eq!(fs_dest(&fs), None);
    }

    #[test]
    fn staged_seek_beyond_end_zero_fills() {
        let (mut staged, fs) = staged_with(MockScript::default(), b"ab", None);
        staged.seek(SeekFrom::Start(4)).unwrap();
        staged.write_all(b"z").unwrap();
        assert_eq!(fs_temp(&fs).as_deref(), Some(&b"ab\0\0z"[..]));
    }

    #[test]
    fn staged_drop_cleans_temp() {
        let fs;
        {
            let (staged, f) = staged_with(MockScript::default(), b"x", None);
            fs = f;
            drop(staged);
        }
        assert_eq!(fs_temp(&fs), None);
        assert!(fs_calls(&fs).contains(&"cleanup"));
    }

    #[test]
    fn staged_drop_cleanup_failure_is_silent() {
        let (staged, fs) = staged_with(
            MockScript {
                cleanup: Some(CleanupAttempt {
                    temp: TempDisposition::Retained,
                    error: Some(io_err(io::ErrorKind::PermissionDenied, "denied")),
                }),
                ..MockScript::default()
            },
            b"x",
            None,
        );
        drop(staged); // best-effort: no panic, temp stays
        assert_eq!(fs_temp(&fs).as_deref(), Some(&b"x"[..]));
    }

    // ---------- seal / readback ----------

    #[test]
    fn readback_reads_sealed_bytes() {
        let (staged, _fs) = staged_with(MockScript::default(), b"sealed!", None);
        let mut sealed = staged.seal();
        {
            let mut rb = sealed.open_readback().unwrap();
            let mut buf = Vec::new();
            rb.read_to_end(&mut buf).unwrap();
            assert_eq!(buf, b"sealed!");
            // Seek works on the reader…
            let pos = rb.seek(SeekFrom::Start(2)).unwrap();
            assert_eq!(pos, 2);
            let mut byte = [0u8; 1];
            rb.read_exact(&mut byte).unwrap();
            assert_eq!(byte, *b"a");
            // …and the `reader()` accessor hands out `impl Read + Seek`.
            let mut one = [0u8; 1];
            rb.reader().read_exact(&mut one).unwrap();
            assert_eq!(one, *b"l");
        }
        // …and the sealed file is publishable after the borrow ends.
        let receipt = sealed
            .publish(PublishOptions::new(
                PublishMode::CreateNew,
                Durability::BestEffort,
            ))
            .unwrap();
        assert_eq!(receipt.durability(), DurabilityEvidence::None);
    }

    #[test]
    fn readback_unsupported_maps_error_kind() {
        let (staged, _fs) = staged_with(
            MockScript {
                readback: Some(Err(io_err(io::ErrorKind::Unsupported, "no readback"))),
                ..MockScript::default()
            },
            b"x",
            None,
        );
        let mut sealed = staged.seal();
        let err = sealed.open_readback().unwrap_err();
        assert_eq!(err.kind(), FileErrorKind::Unsupported);
        // The sealed file survives for publish or abort.
        sealed.abort().unwrap();
    }

    #[test]
    fn readback_position_starts_at_zero() {
        // After writes advanced the write cursor, readback still
        // starts at position 0.
        let (mut staged, _fs) = staged_with(MockScript::default(), b"", None);
        staged.write_all(b"012345").unwrap();
        staged.seek(SeekFrom::End(-1)).unwrap();
        let mut sealed = staged.seal();
        let mut rb = sealed.open_readback().unwrap();
        let mut byte = [0u8; 1];
        rb.read_exact(&mut byte).unwrap();
        assert_eq!(byte, *b"0");
    }

    // ---------- publish: validate ----------

    #[test]
    fn validate_unsupported_fails_not_published() {
        let (staged, fs) = staged_with(
            MockScript {
                validate: Some(Err(io_err(io::ErrorKind::Unsupported, "no atomic"))),
                ..MockScript::default()
            },
            b"data",
            None,
        );
        let failure = staged
            .seal()
            .publish(PublishOptions::new(
                PublishMode::CreateNew,
                Durability::FileSync,
            ))
            .unwrap_err();
        assert_eq!(failure.kind(), PublishFailureKind::Unsupported);
        assert_eq!(failure.phase(), PublishPhase::Validate);
        assert_eq!(failure.effect(), PublishEffect::NotPublished);
        assert_eq!(failure.durability(), DurabilityEvidence::None);
        // Nothing touched the destination; temp untouched (handle
        // owns it — its Drop does not delete).
        assert_eq!(fs_dest(&fs), None);
        assert_eq!(fs_temp(&fs).as_deref(), Some(&b"data"[..]));
        // No I/O stage ran before validation failed.
        assert_eq!(fs_calls(&fs), ["validate"]);
    }

    // ---------- publish: file sync ----------

    #[test]
    fn file_sync_failure_is_not_published() {
        let (staged, fs) = staged_with(
            MockScript {
                sync_file: Some(Err(io_err(io::ErrorKind::Other, "fsync failed"))),
                ..MockScript::default()
            },
            b"data",
            None,
        );
        let failure = staged
            .seal()
            .publish(PublishOptions::new(
                PublishMode::CreateNew,
                Durability::FileSync,
            ))
            .unwrap_err();
        assert_eq!(failure.phase(), PublishPhase::FileSync);
        assert_eq!(failure.effect(), PublishEffect::NotPublished);
        assert_eq!(failure.durability(), DurabilityEvidence::None);
        assert_eq!(fs_dest(&fs), None);
        assert_eq!(fs_temp(&fs).as_deref(), Some(&b"data"[..]));
        // Publish never ran.
        assert!(!fs_calls(&fs).contains(&"publish"));
    }

    // ---------- publish: close ----------

    #[test]
    fn prepare_failure_is_not_published_with_file_sync_evidence() {
        let (staged, _fs) = staged_with(
            MockScript {
                prepare: Some(Err(io_err(io::ErrorKind::Other, "close failed"))),
                ..MockScript::default()
            },
            b"data",
            None,
        );
        let failure = staged
            .seal()
            .publish(PublishOptions::new(
                PublishMode::Replace,
                Durability::FileSync,
            ))
            .unwrap_err();
        assert_eq!(failure.phase(), PublishPhase::Close);
        assert_eq!(failure.effect(), PublishEffect::NotPublished);
        // The completed file sync is still reported honestly.
        assert_eq!(failure.durability(), DurabilityEvidence::FileSynced);
    }

    // ---------- publish: publish stage ----------

    #[test]
    fn publish_not_applied_destination_exists() {
        let (staged, fs) = staged_with(
            MockScript {
                publish: Some(MockPublish {
                    attempt: PublishAttempt {
                        effect: ApplyOutcome::NotApplied,
                        temp: TempDisposition::Retained,
                        error: Some(io_err(io::ErrorKind::AlreadyExists, "dest exists")),
                    },
                    physically_applied: false,
                }),
                ..MockScript::default()
            },
            b"data",
            Some(b"existing".to_vec()),
        );
        let failure = staged
            .seal()
            .publish(PublishOptions::new(
                PublishMode::CreateNew,
                Durability::FileSync,
            ))
            .unwrap_err();
        assert_eq!(failure.kind(), PublishFailureKind::DestinationExists);
        assert_eq!(failure.phase(), PublishPhase::Publish);
        assert_eq!(failure.effect(), PublishEffect::NotPublished);
        assert_eq!(failure.durability(), DurabilityEvidence::FileSynced);
        // Existing destination preserved; temp retained under the
        // failure's handle.
        assert_eq!(fs_dest(&fs).as_deref(), Some(&b"existing"[..]));
        assert_eq!(fs_temp(&fs).as_deref(), Some(&b"data"[..]));
        assert!(failure.recovery_handle().is_some());
    }

    #[test]
    fn publish_indeterminate_keeps_temp_and_reports_unknown() {
        let (staged, fs) = staged_with(
            MockScript {
                publish: Some(MockPublish {
                    // OS report ambiguous; physically NOT applied and
                    // temp retained — Indeterminate must not lie.
                    attempt: PublishAttempt {
                        effect: ApplyOutcome::Unknown,
                        temp: TempDisposition::Retained,
                        error: Some(io_err(io::ErrorKind::Other, "ambiguous rename")),
                    },
                    physically_applied: false,
                }),
                ..MockScript::default()
            },
            b"data",
            None,
        );
        let mut failure = staged
            .seal()
            .publish(PublishOptions::new(
                PublishMode::Replace,
                Durability::FileSync,
            ))
            .unwrap_err();
        assert_eq!(failure.phase(), PublishPhase::Publish);
        assert_eq!(failure.effect(), PublishEffect::Indeterminate);
        assert_eq!(failure.info().temp_state(), TempState::Retained);
        // KEY INVARIANT: indeterminate temp is NOT auto-deleted —
        // even after the failure is dropped (the handle owns it and
        // its Drop never deletes).
        assert_eq!(fs_temp(&fs).as_deref(), Some(&b"data"[..]));
        assert_eq!(fs_dest(&fs), None);
        let handle = failure.take_recovery_handle().expect("handle");
        drop(failure);
        drop(handle);
        assert_eq!(fs_temp(&fs).as_deref(), Some(&b"data"[..])); // still there
    }

    #[test]
    fn publish_indeterminate_applied_physically_is_still_indeterminate() {
        // Physical: applied. Report: ambiguous. The pipeline must
        // report Indeterminate — never a false NotPublished, and the
        // retained temp stays put.
        let (staged, fs) = staged_with(
            MockScript {
                publish: Some(MockPublish {
                    attempt: PublishAttempt {
                        effect: ApplyOutcome::Unknown,
                        temp: TempDisposition::Retained,
                        error: Some(io_err(io::ErrorKind::Other, "lost connection")),
                    },
                    physically_applied: true,
                }),
                ..MockScript::default()
            },
            b"data",
            None,
        );
        let failure = staged
            .seal()
            .publish(PublishOptions::new(
                PublishMode::Replace,
                Durability::FileSync,
            ))
            .unwrap_err();
        assert_eq!(failure.effect(), PublishEffect::Indeterminate);
        assert_eq!(fs_dest(&fs).as_deref(), Some(&b"data"[..])); // physically there
        assert_eq!(fs_temp(&fs).as_deref(), Some(&b"data"[..])); // temp NOT auto-deleted
    }

    #[test]
    fn recovery_handle_inspect_and_cleanup() {
        let (staged, fs) = staged_with(
            MockScript {
                publish: Some(MockPublish {
                    attempt: PublishAttempt {
                        effect: ApplyOutcome::Unknown,
                        temp: TempDisposition::Retained,
                        error: Some(io_err(io::ErrorKind::Other, "ambiguous")),
                    },
                    physically_applied: false,
                }),
                ..MockScript::default()
            },
            b"left",
            None,
        );
        let mut failure = staged
            .seal()
            .publish(PublishOptions::new(
                PublishMode::Replace,
                Durability::FileSync,
            ))
            .unwrap_err();
        let handle = failure.take_recovery_handle().unwrap();
        let obs = handle.inspect().unwrap();
        assert_eq!(obs.temp_state(), TempState::Retained);
        assert_eq!(handle.info().temp_path(), Some(temp_path().as_path()));
        let receipt = handle.cleanup().unwrap();
        assert_eq!(receipt.temp_state(), TempState::Absent);
        assert_eq!(fs_temp(&fs), None);
    }

    #[test]
    fn recovery_cleanup_failure_returns_handle_for_retry() {
        let (staged, fs) = staged_with(
            MockScript {
                publish: Some(MockPublish {
                    attempt: PublishAttempt {
                        effect: ApplyOutcome::Unknown,
                        temp: TempDisposition::Retained,
                        error: Some(io_err(io::ErrorKind::Other, "ambiguous")),
                    },
                    physically_applied: false,
                }),
                cleanup: Some(CleanupAttempt {
                    temp: TempDisposition::Retained,
                    error: Some(io_err(io::ErrorKind::PermissionDenied, "busy")),
                }),
                ..MockScript::default()
            },
            b"left",
            None,
        );
        let mut failure = staged
            .seal()
            .publish(PublishOptions::new(
                PublishMode::Replace,
                Durability::FileSync,
            ))
            .unwrap_err();
        let handle = failure.take_recovery_handle().unwrap();
        let mut cleanup_err = handle.cleanup().unwrap_err();
        assert_eq!(cleanup_err.info().temp_state(), TempState::Retained);
        // The handle comes back for retry.
        let retry = cleanup_err.take_recovery_handle().unwrap();
        assert_eq!(fs_temp(&fs).as_deref(), Some(&b"left"[..]));
        // Second cleanup (unscripted → default Removed) succeeds.
        retry.cleanup().unwrap();
        assert_eq!(fs_temp(&fs), None);
    }

    #[test]
    fn inspect_failure_maps_file_error() {
        let (staged, _fs) = staged_with(
            MockScript {
                publish: Some(MockPublish {
                    attempt: PublishAttempt {
                        effect: ApplyOutcome::Unknown,
                        temp: TempDisposition::Unknown,
                        error: Some(io_err(io::ErrorKind::Other, "ambiguous")),
                    },
                    physically_applied: false,
                }),
                inspect: Some(Err(io_err(io::ErrorKind::Unsupported, "no stat"))),
                ..MockScript::default()
            },
            b"left",
            None,
        );
        let mut failure = staged
            .seal()
            .publish(PublishOptions::new(
                PublishMode::Replace,
                Durability::FileSync,
            ))
            .unwrap_err();
        let handle = failure.take_recovery_handle().unwrap();
        let err = handle.inspect().unwrap_err();
        assert_eq!(err.kind(), FileErrorKind::Unsupported);
    }

    // ---------- publish: applied + dir sync ----------

    #[test]
    fn dir_sync_failure_reports_published_not_ok() {
        // Publish applied; dir sync failed → Published + Err. The
        // receipt is never a false Ok: requested durability not met.
        let (staged, fs) = staged_with(
            MockScript {
                sync_directory: Some(Err(io_err(io::ErrorKind::Other, "dir fsync"))),
                ..MockScript::default()
            },
            b"data",
            None,
        );
        let failure = staged
            .seal()
            .publish(PublishOptions::new(
                PublishMode::CreateNew,
                Durability::FileAndDirectorySync,
            ))
            .unwrap_err();
        assert_eq!(failure.phase(), PublishPhase::DirectorySync);
        assert_eq!(failure.effect(), PublishEffect::Published);
        // Evidence stops at FileSynced — FileAndDirectorySync was
        // requested but not achieved.
        assert_eq!(failure.durability(), DurabilityEvidence::FileSynced);
        assert_eq!(fs_dest(&fs).as_deref(), Some(&b"data"[..])); // published
        assert_eq!(
            fs_calls(&fs),
            [
                "validate",
                "sync_file",
                "prepare",
                "publish",
                "sync_directory",
            ]
        );
    }

    #[test]
    fn dir_sync_not_run_when_not_requested() {
        let (staged, fs) = staged_with(MockScript::default(), b"data", None);
        let receipt = staged
            .seal()
            .publish(PublishOptions::new(
                PublishMode::CreateNew,
                Durability::FileSync,
            ))
            .unwrap();
        assert_eq!(receipt.durability(), DurabilityEvidence::FileSynced);
        assert!(!fs_calls(&fs).contains(&"sync_directory"));
    }

    #[test]
    fn file_sync_not_run_when_best_effort() {
        let (staged, fs) = staged_with(MockScript::default(), b"data", None);
        let receipt = staged
            .seal()
            .publish(PublishOptions::new(
                PublishMode::Replace,
                Durability::BestEffort,
            ))
            .unwrap();
        assert_eq!(receipt.durability(), DurabilityEvidence::None);
        assert!(!fs_calls(&fs).contains(&"sync_file"));
        assert!(!fs_calls(&fs).contains(&"sync_directory"));
    }

    #[test]
    fn full_durability_receipt_reports_all_synced() {
        let (staged, fs) = staged_with(MockScript::default(), b"data", None);
        let receipt = staged
            .seal()
            .publish(PublishOptions::new(
                PublishMode::CreateNew,
                Durability::FileAndDirectorySync,
            ))
            .unwrap();
        assert_eq!(
            receipt.durability(),
            DurabilityEvidence::FileAndDirectorySynced
        );
        assert_eq!(receipt.temp_state(), TempState::Absent);
        assert!(receipt.recovery_handle().is_none());
        assert_eq!(receipt.destination(), dest().as_path());
        assert_eq!(
            fs_calls(&fs),
            [
                "validate",
                "sync_file",
                "prepare",
                "publish",
                "sync_directory",
            ]
        );
        assert_eq!(fs_dest(&fs).as_deref(), Some(&b"data"[..]));
        assert_eq!(fs_temp(&fs), None);
    }

    // ---------- publish: applied + cleanup ----------

    #[test]
    fn cleanup_retained_temp_yields_receipt_with_handle() {
        // Publish applied but temp survived (hardlink-style adapter
        // report) and cleanup fails → still Ok, handle attached.
        let (staged, fs) = staged_with(
            MockScript {
                publish: Some(MockPublish {
                    attempt: PublishAttempt {
                        effect: ApplyOutcome::Applied,
                        temp: TempDisposition::Retained,
                        error: None,
                    },
                    physically_applied: true,
                }),
                cleanup: Some(CleanupAttempt {
                    temp: TempDisposition::Retained,
                    error: Some(io_err(io::ErrorKind::Other, "cleanup failed")),
                }),
                ..MockScript::default()
            },
            b"data",
            None,
        );
        let mut receipt = staged
            .seal()
            .publish(PublishOptions::new(
                PublishMode::Replace,
                Durability::FileSync,
            ))
            .unwrap();
        assert_eq!(receipt.temp_state(), TempState::Retained);
        assert_eq!(receipt.durability(), DurabilityEvidence::FileSynced);
        assert_eq!(fs_dest(&fs).as_deref(), Some(&b"data"[..]));
        // Cleanup ran and reported Retained; handle owns it.
        let handle = receipt.take_recovery_handle().expect("handle");
        assert_eq!(handle.inspect().unwrap().temp_state(), TempState::Retained);
        assert_eq!(fs_temp(&fs).as_deref(), Some(&b"data"[..]));
        handle.cleanup().unwrap();
        assert_eq!(fs_temp(&fs), None);
    }

    #[test]
    fn cleanup_unknown_temp_yields_unknown_state_with_handle() {
        let (staged, _fs) = staged_with(
            MockScript {
                publish: Some(MockPublish {
                    attempt: PublishAttempt {
                        effect: ApplyOutcome::Applied,
                        temp: TempDisposition::Unknown,
                        error: None,
                    },
                    physically_applied: true,
                }),
                cleanup: Some(CleanupAttempt {
                    temp: TempDisposition::Unknown,
                    error: Some(io_err(io::ErrorKind::Other, "lost handle")),
                }),
                ..MockScript::default()
            },
            b"data",
            None,
        );
        let receipt = staged
            .seal()
            .publish(PublishOptions::new(
                PublishMode::Replace,
                Durability::FileSync,
            ))
            .unwrap();
        assert_eq!(receipt.temp_state(), TempState::Unknown);
        assert!(receipt.recovery_handle().is_some());
    }

    // ---------- abort ----------

    #[test]
    fn abort_removes_temp_and_leaves_destination() {
        let (staged, fs) = staged_with(MockScript::default(), b"data", Some(b"keep".to_vec()));
        let receipt = staged.seal().abort().unwrap();
        assert_eq!(receipt.temp_state(), TempState::Absent);
        assert_eq!(fs_temp(&fs), None);
        assert_eq!(fs_dest(&fs).as_deref(), Some(&b"keep"[..]));
        // No publish stages ran.
        assert_eq!(fs_calls(&fs), ["cleanup"]);
    }

    #[test]
    fn abort_failure_returns_handle() {
        let (staged, fs) = staged_with(
            MockScript {
                cleanup: Some(CleanupAttempt {
                    temp: TempDisposition::Retained,
                    error: Some(io_err(io::ErrorKind::PermissionDenied, "denied")),
                }),
                ..MockScript::default()
            },
            b"data",
            None,
        );
        let mut failure = staged.abort().unwrap_err();
        assert_eq!(failure.info().temp_state(), TempState::Retained);
        assert_eq!(fs_temp(&fs).as_deref(), Some(&b"data"[..]));
        // The owner is gone — no second cleanup at drop; the handle
        // alone owns the temp.
        let handle = failure.take_recovery_handle().unwrap();
        handle.cleanup().unwrap(); // default-Ok retry
        assert_eq!(fs_temp(&fs), None);
        assert_eq!(fs_calls(&fs), ["cleanup", "cleanup"]);
    }

    // ---------- misc classification ----------

    #[test]
    fn stage_call_order_is_pipeline_order() {
        let (staged, fs) = staged_with(MockScript::default(), b"d", None);
        staged
            .seal()
            .publish(PublishOptions::new(
                PublishMode::CreateNew,
                Durability::FileAndDirectorySync,
            ))
            .unwrap();
        assert_eq!(
            fs_calls(&fs),
            [
                "validate",
                "sync_file",
                "prepare",
                "publish",
                "sync_directory",
            ]
        );
    }

    #[test]
    fn operation_ids_are_unique() {
        let (a, _) = staged_with(MockScript::default(), b"", None);
        let (b, _) = staged_with(MockScript::default(), b"", None);
        assert_ne!(a.operation_id(), b.operation_id());
    }
}
