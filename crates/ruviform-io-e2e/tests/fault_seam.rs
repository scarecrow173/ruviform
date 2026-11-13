//! G07 fault-seam contract tests (Issue #306): the publish pipeline's
//! classification contract driven through the **public API only**,
//! with faults injected via `ruviform-io-file`'s opt-in
//! `fault-injection` feature (`#[doc(hidden)] pub mod mock`).
//!
//! These tests cover the classes the real-OS integration tests
//! (`fault_boundary`, `staged_lifecycle`, `wav_fault`) honestly record
//! as not reproducible on a real filesystem:
//!
//! - `ApplyOutcome::Unknown` → `PublishEffect::Indeterminate`, temp
//!   not auto-deleted, explicit `RecoveryHandle` inspect/cleanup.
//! - `Applied` publish followed by a directory-sync failure →
//!   `PublishEffect::Published` + `PublishPhase::DirectorySync` +
//!   achieved `DurabilityEvidence::FileSynced`.
//! - `TempDisposition::Unknown` → `TempState::Unknown` on the
//!   post-publish cleanup path, the recovery `inspect` path, and
//!   `abort`.
//! - Per-phase `NotPublished` classification (validate / file sync /
//!   close / publish) with the matching `PublishFailureKind`.
//!
//! The mock's `MockFs` buffers and stage call log are observation
//! evidence only — every assertion targets the public result types
//! (`PublishReceipt`, `PublishFailure`, `CleanupReceipt`,
//! `CleanupFailure`, `RecoveryHandle`). Rules: G07-R1..R6 of
//! `docs/test-specs/issue-306-fault-seam-fuzz.md`.

use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};

use ruviform_io_file::mock::{
    FaultScript, MockFs, PublishReport, PublishScript, TempReport, staged_file,
};
use ruviform_io_file::{
    Durability, DurabilityEvidence, PublishEffect, PublishFailure, PublishFailureKind, PublishMode,
    PublishOptions, PublishPhase, PublishReceipt, StagedFile, TempState,
};

/// File-and-directory-sync publish — exercises every pipeline stage.
fn options() -> PublishOptions {
    PublishOptions::new(PublishMode::CreateNew, Durability::FileAndDirectorySync)
}

/// Builds a mock-backed `StagedFile` whose temp already holds
/// `payload`, plus the shared observation handle.
fn staged(script: FaultScript) -> (StagedFile, Arc<Mutex<MockFs>>) {
    staged_file(
        Path::new("dest.bin"),
        script,
        Some(b"payload".to_vec()),
        None,
    )
}

/// Writes, seals and publishes a mock-backed staged file; returns the
/// publish outcome and the observation handle.
fn run_publish(
    script: FaultScript,
) -> (Result<PublishReceipt, PublishFailure>, Arc<Mutex<MockFs>>) {
    let (mut staged, fs) = staged(script);
    staged.write_all(b"payload").expect("mock write");
    let outcome = staged.seal().publish(options());
    (outcome, fs)
}

fn lock(fs: &Arc<Mutex<MockFs>>) -> MutexGuard<'_, MockFs> {
    fs.lock().expect("mock fs poisoned")
}

/// G07-R1/G07-R5: an `Unknown` publish report classifies as
/// `Indeterminate`, the physically-applied temp is *not* auto-deleted,
/// and the attached `RecoveryHandle` owns explicit inspect/cleanup.
#[test]
fn indeterminate_publish_retains_temp_and_recovers_explicitly() {
    let script = FaultScript::new().publish(
        PublishScript::new(PublishReport::Unknown)
            .with_temp(TempReport::Retained)
            .with_error(std::io::ErrorKind::Other)
            .with_physically_applied(true),
    );
    let (outcome, fs) = run_publish(script);

    let mut failure = outcome.expect_err("indeterminate publish must fail");
    assert_eq!(failure.effect(), PublishEffect::Indeterminate);
    assert_eq!(failure.phase(), PublishPhase::Publish);
    assert_eq!(failure.kind(), PublishFailureKind::Io);
    assert_eq!(failure.info().temp_state(), TempState::Retained);

    // The leftover was NOT auto-deleted — the mock FS still holds the
    // temp bytes, and the publish physically reached the destination.
    {
        let fs = lock(&fs);
        assert!(fs.temp.is_some(), "indeterminate temp must be retained");
        assert_eq!(fs.dest.as_deref(), Some(b"payload".as_slice()));
    }

    // Recovery is explicit through the handle: inspect observes the
    // retained temp, cleanup removes it.
    let handle = failure
        .take_recovery_handle()
        .expect("indeterminate failure must carry a recovery handle");
    assert_eq!(
        handle.inspect().expect("inspect").temp_state(),
        TempState::Retained
    );
    let receipt = handle.cleanup().expect("explicit cleanup");
    assert_eq!(receipt.temp_state(), TempState::Absent);
    {
        let fs = lock(&fs);
        assert!(fs.temp.is_none(), "cleanup must remove the temp");
        assert_eq!(fs.dest.as_deref(), Some(b"payload".as_slice()));
    }
}

/// G07-R2: `Applied` followed by a directory-sync failure reports
/// `Published` (never `NotPublished`) at the `DirectorySync` phase,
/// with the file-sync evidence actually achieved.
#[test]
fn directory_sync_failure_after_publish_reports_published() {
    let script = FaultScript::new()
        .sync_directory_error(std::io::ErrorKind::PermissionDenied)
        .publish(PublishScript::new(PublishReport::Applied));
    let (outcome, fs) = run_publish(script);

    let failure = outcome.expect_err("directory-sync failure must fail");
    assert_eq!(failure.effect(), PublishEffect::Published);
    assert_eq!(failure.phase(), PublishPhase::DirectorySync);
    assert_eq!(failure.kind(), PublishFailureKind::Io);
    assert_eq!(failure.durability(), DurabilityEvidence::FileSynced);
    assert_eq!(failure.info().temp_state(), TempState::Absent);

    // The publish physically applied and the pipeline stopped at the
    // directory-sync stage — no post-publish cleanup ran.
    {
        let fs = lock(&fs);
        assert_eq!(fs.dest.as_deref(), Some(b"payload".as_slice()));
        assert!(fs.temp.is_none());
        assert_eq!(
            fs.calls.as_slice(),
            &[
                "validate",
                "sync_file",
                "prepare",
                "publish",
                "sync_directory"
            ][..]
        );
    }
}

/// G07-R3: a `Unknown` cleanup disposition after a retained-temp
/// publish surfaces `TempState::Unknown` on the receipt, and the
/// recovery handle's scripted inspect reports `Unknown` too.
#[test]
fn unknown_cleanup_disposition_surfaces_temp_state_unknown() {
    let script = FaultScript::new()
        .publish(PublishScript::new(PublishReport::Applied).with_temp(TempReport::Retained))
        .cleanup(TempReport::Unknown, Some(std::io::ErrorKind::Other))
        .inspect(Ok(TempReport::Unknown));
    let (outcome, _fs) = run_publish(script);

    let mut receipt = outcome.expect("publish itself applied");
    assert_eq!(receipt.temp_state(), TempState::Unknown);

    let handle = receipt
        .take_recovery_handle()
        .expect("unknown temp state must keep a recovery handle");
    assert_eq!(
        handle.inspect().expect("inspect").temp_state(),
        TempState::Unknown
    );
}

/// G07-R3 (abort path): `TempDisposition::Unknown` from `cleanup_temp`
/// during `abort` yields a `CleanupFailure` whose info reports
/// `TempState::Unknown` and whose handle can retry to `Absent`.
#[test]
fn abort_with_unknown_cleanup_reports_unknown_temp_state() {
    let script = FaultScript::new().cleanup(TempReport::Unknown, Some(std::io::ErrorKind::Other));
    let (staged, _fs) = staged(script);

    let mut failure = staged.abort().expect_err("unknown cleanup must fail");
    assert_eq!(failure.info().temp_state(), TempState::Unknown);

    // The scripted cleanup slot is consumed; the retry falls back to
    // the happy path and removes the temp.
    let handle = failure
        .take_recovery_handle()
        .expect("cleanup failure must carry a recovery handle");
    let receipt = handle.cleanup().expect("cleanup retry");
    assert_eq!(receipt.temp_state(), TempState::Absent);
}

/// G07-R4: a `validate` fault classifies as `NotPublished` at the
/// `Validate` phase; `Unsupported` maps to `PublishFailureKind::
/// Unsupported`, other kinds to `Io`.
#[test]
fn validate_fault_is_not_published() {
    let (outcome, _fs) =
        run_publish(FaultScript::new().validate_error(std::io::ErrorKind::Unsupported));

    let failure = outcome.expect_err("validate fault must fail");
    assert_eq!(failure.effect(), PublishEffect::NotPublished);
    assert_eq!(failure.phase(), PublishPhase::Validate);
    assert_eq!(failure.kind(), PublishFailureKind::Unsupported);
    assert_eq!(failure.durability(), DurabilityEvidence::None);
}

/// G07-R4: a `sync_file` fault is safely `NotPublished` — the publish
/// stage never ran.
#[test]
fn file_sync_fault_is_not_published() {
    let (outcome, _fs) =
        run_publish(FaultScript::new().sync_file_error(std::io::ErrorKind::PermissionDenied));

    let failure = outcome.expect_err("file-sync fault must fail");
    assert_eq!(failure.effect(), PublishEffect::NotPublished);
    assert_eq!(failure.phase(), PublishPhase::FileSync);
    assert_eq!(failure.kind(), PublishFailureKind::Io);
    assert_eq!(failure.durability(), DurabilityEvidence::None);
}

/// G07-R4: a `prepare_for_publish` (close) fault is `NotPublished`
/// with the file-sync evidence already achieved reported honestly.
#[test]
fn close_fault_is_not_published_with_file_synced_evidence() {
    let (outcome, _fs) =
        run_publish(FaultScript::new().prepare_error(std::io::ErrorKind::PermissionDenied));

    let failure = outcome.expect_err("close fault must fail");
    assert_eq!(failure.effect(), PublishEffect::NotPublished);
    assert_eq!(failure.phase(), PublishPhase::Close);
    assert_eq!(failure.kind(), PublishFailureKind::Io);
    assert_eq!(failure.durability(), DurabilityEvidence::FileSynced);
}

/// G07-R4: a `NotApplied` publish carrying `AlreadyExists` classifies
/// as `DestinationExists` + `NotPublished` — the incumbent is
/// untouched.
#[test]
fn destination_conflict_is_not_published() {
    let script = FaultScript::new().publish(
        PublishScript::new(PublishReport::NotApplied)
            .with_error(std::io::ErrorKind::AlreadyExists)
            .with_physically_applied(false),
    );
    let (outcome, fs) = run_publish(script);

    let failure = outcome.expect_err("conflict must fail");
    assert_eq!(failure.effect(), PublishEffect::NotPublished);
    assert_eq!(failure.phase(), PublishPhase::Publish);
    assert_eq!(failure.kind(), PublishFailureKind::DestinationExists);
    assert_eq!(failure.durability(), DurabilityEvidence::FileSynced);
    assert_eq!(failure.info().temp_state(), TempState::Absent);

    // NotApplied with the default `Removed` temp disposition consumed
    // the temp; nothing reached the destination.
    let fs = lock(&fs);
    assert!(fs.dest.is_none(), "conflict must not publish");
    assert!(fs.temp.is_none());
}
