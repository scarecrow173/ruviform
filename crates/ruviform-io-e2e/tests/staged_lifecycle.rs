//! `StagedFile` → `SealedFile` → publish → reopen boundary (R1/R2).
//!
//! Exercises the type-state staged-publish lifecycle end to end on
//! the real OS backend: sealed readback verification gates the
//! publish, `CreateNew` conflicts are honest `NotPublished`, `Replace`
//! performs atomic owner replacement, and abort/drop recover the
//! staging temp. External interference with the staging temp between
//! seal and publish, and the recovery-handle cleanup retry path, are
//! asserted where the real filesystem reproduces them. Adapter-level
//! fault classes that need injected faults (Indeterminate,
//! directory-sync failure, Unknown temp) are covered by the crate's
//! own `#[cfg(test)]` mock suite — only real-OS-reproducible
//! behaviour is asserted here.

mod common;

use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;

use common::{TestDir, children, exists, read};
use ruviform_io_file::{
    Durability, PublishEffect, PublishFailureKind, PublishMode, PublishOptions, StagedFile,
    TempState,
};

fn options(mode: PublishMode) -> PublishOptions {
    PublishOptions::new(mode, Durability::FileAndDirectorySync)
}

#[test]
fn write_seal_readback_publish_reopen_round_trip() {
    let dir = TestDir::new("lifecycle");
    let dest = dir.dest("payload.bin");
    let payload = b"staged bytes across the publish boundary".to_vec();

    let mut staged = StagedFile::create(&dest).expect("stage");
    staged.write_all(&payload).expect("write");
    let mut sealed = staged.seal();

    // Pre-publish readback: the sealed bytes are verified before
    // publish, exactly as a format layer would gate the release.
    let mut verified = Vec::new();
    sealed
        .open_readback()
        .expect("readback")
        .read_to_end(&mut verified)
        .expect("read sealed");
    assert_eq!(verified, payload, "sealed bytes must verify");

    let receipt = sealed
        .publish(options(PublishMode::CreateNew))
        .expect("publish");
    assert_eq!(receipt.destination(), dest.as_path());
    assert_eq!(read(&dest).as_deref(), Some(payload.as_slice()));
    // Temp consumed by publish: only the destination remains.
    assert_eq!(children(dir.path()), vec![dest.clone()]);
}

#[test]
fn create_new_conflict_is_not_published_and_preserves_destination() {
    let dir = TestDir::new("conflict");
    let dest = dir.dest("out.bin");

    let mut staged = StagedFile::create(&dest).expect("stage");
    staged.write_all(b"staged-new").expect("write");
    let sealed = staged.seal();

    // External owner wins the destination between staging and publish.
    std::fs::write(&dest, b"incumbent").expect("external write");

    let mut failure = sealed
        .publish(options(PublishMode::CreateNew))
        .expect_err("create-new must refuse an existing destination");
    assert_eq!(failure.effect(), PublishEffect::NotPublished);
    assert_eq!(failure.kind(), PublishFailureKind::DestinationExists);
    // The incumbent bytes are untouched — no clobber on conflict.
    assert_eq!(read(&dest).as_deref(), Some(b"incumbent".as_slice()));
    // Non-indeterminate failures keep no ambiguous temp state; the
    // owned recovery handle cleans the retained staging temp.
    if let Some(handle) = failure.take_recovery_handle() {
        let temp = handle.info().temp_path().map(PathBuf::from);
        handle.cleanup().expect("cleanup retained temp");
        if let Some(temp) = temp {
            assert!(!exists(&temp), "cleanup removes the retained temp");
        }
    }
    assert_eq!(children(dir.path()), vec![dest.clone()]);
}

#[test]
fn replace_publish_performs_atomic_owner_replacement() {
    let dir = TestDir::new("replace");
    let dest = dir.dest("out.bin");
    std::fs::write(&dest, b"old-owner").expect("seed destination");

    let mut staged = StagedFile::create(&dest).expect("stage");
    staged.write_all(b"new-owner-content").expect("write");
    let receipt = staged
        .seal()
        .publish(options(PublishMode::Replace))
        .expect("replace publish");

    assert_eq!(receipt.destination(), dest.as_path());
    assert_eq!(
        read(&dest).as_deref(),
        Some(b"new-owner-content".as_slice())
    );
    assert_eq!(children(dir.path()), vec![dest.clone()]);
}

#[test]
fn sealed_abort_leaves_no_destination_and_removes_temp() {
    let dir = TestDir::new("abort-sealed");
    let dest = dir.dest("abort.bin");

    let mut staged = StagedFile::create(&dest).expect("stage");
    staged.write_all(b"never published").expect("write");
    let temp = staged.temp_path().map(std::path::Path::to_path_buf);
    let receipt = staged.seal().abort().expect("abort");

    assert!(!exists(&dest), "abort must not publish the destination");
    if let Some(temp) = temp {
        assert!(!exists(&temp), "abort removes the staging temp");
    }
    assert_eq!(children(dir.path()), Vec::<PathBuf>::new());
    let _ = receipt;
}

#[test]
fn unsealed_drop_best_effort_removes_temp() {
    let dir = TestDir::new("drop");
    let dest = dir.dest("drop.bin");

    let temp = {
        let mut staged = StagedFile::create(&dest).expect("stage");
        staged.write_all(b"dropped mid-stage").expect("write");
        staged.temp_path().map(std::path::Path::to_path_buf)
    };

    assert!(!exists(&dest));
    if let Some(temp) = temp {
        assert!(!exists(&temp), "dropped staging temp must be gone");
    }
    assert_eq!(children(dir.path()), Vec::<PathBuf>::new());
}

#[test]
fn readback_starts_at_zero_and_seek_works_on_sealed_bytes() {
    let dir = TestDir::new("readback-seek");
    let dest = dir.dest("seek.bin");
    let payload: Vec<u8> = (0_u16..4096).map(|v| (v % 251) as u8).collect();

    let mut staged = StagedFile::create(&dest).expect("stage");
    staged.write_all(&payload).expect("write");
    let mut sealed = staged.seal();

    let mut readback = sealed.open_readback().expect("readback");
    let mut head = [0_u8; 16];
    readback.read_exact(&mut head).expect("head");
    assert_eq!(&head, &payload[..16]);
    readback.seek(SeekFrom::End(-4)).expect("seek tail");
    let mut tail = [0_u8; 4];
    readback.read_exact(&mut tail).expect("tail");
    assert_eq!(&tail, &payload[payload.len() - 4..]);

    sealed
        .publish(options(PublishMode::CreateNew))
        .expect("publish after readback");
    assert_eq!(read(&dest).as_deref(), Some(payload.as_slice()));
}

#[test]
fn post_seal_temp_rewrite_is_not_reverified_before_publish() {
    let dir = TestDir::new("temp-tamper");
    let dest = dir.dest("payload.bin");
    let sealed_bytes = b"bytes verified at seal time".to_vec();
    // Different length on purpose: publish performs no content
    // re-verification, so size is not silently re-checked either.
    let tampered = b"EXTERNAL-WRITER replaced the staging temp".to_vec();

    let mut staged = StagedFile::create(&dest).expect("stage");
    staged.write_all(&sealed_bytes).expect("write");
    let sealed = staged.seal();
    let temp = sealed
        .temp_path()
        .map(std::path::Path::to_path_buf)
        .expect("os backend exposes the temp path");

    // An external actor rewrites the staging temp after seal. The
    // seal-time verification already ran; the publish pipeline moves
    // whatever bytes sit at the temp path.
    std::fs::write(&temp, &tampered).expect("external temp rewrite");

    let receipt = sealed
        .publish(options(PublishMode::CreateNew))
        .expect("publish succeeds — temp content is not re-verified");
    assert_eq!(receipt.destination(), dest.as_path());
    // Documents the integrity boundary honestly: post-seal external
    // mutation of the temp file is outside the seal guarantee, and the
    // destination receives the externally rewritten bytes.
    assert_eq!(read(&dest).as_deref(), Some(tampered.as_slice()));
    assert_eq!(children(dir.path()), vec![dest.clone()]);
}

#[test]
fn post_seal_temp_delete_fails_publish_not_published() {
    let dir = TestDir::new("temp-delete");
    let dest = dir.dest("payload.bin");

    let mut staged = StagedFile::create(&dest).expect("stage");
    staged.write_all(b"sealed bytes").expect("write");
    let sealed = staged.seal();
    let temp = sealed
        .temp_path()
        .map(std::path::Path::to_path_buf)
        .expect("os backend exposes the temp path");

    // External actor removes the staging temp before publish. The
    // publish primitive cannot rename a missing source, so the
    // outcome is honestly NotPublished on every platform.
    std::fs::remove_file(&temp).expect("external temp delete");

    // BestEffort keeps the assertion on the publish stage itself —
    // durability syncs are irrelevant to this fault.
    let mut failure = sealed
        .publish(PublishOptions::new(
            PublishMode::CreateNew,
            Durability::BestEffort,
        ))
        .expect_err("publish cannot succeed without its temp file");
    assert_eq!(failure.effect(), PublishEffect::NotPublished);
    assert!(!exists(&dest), "failed publish must not create destination");
    // The failure still hands out the recovery handle; with the temp
    // already gone externally, cleanup is an honest no-op success.
    let handle = failure
        .take_recovery_handle()
        .expect("failed publish always carries its recovery handle");
    let receipt = handle.cleanup().expect("cleanup of absent temp succeeds");
    assert_eq!(receipt.temp_state(), TempState::Absent);
    assert!(!exists(&temp));
    assert_eq!(children(dir.path()), Vec::<PathBuf>::new());
}

#[test]
fn recovery_cleanup_failure_returns_handle_for_retry() {
    let dir = TestDir::new("recovery-retry");
    let dest = dir.dest("out.bin");
    // An incumbent destination makes the CreateNew publish fail with
    // the staging temp retained — the recovery handle owns the
    // leftover.
    std::fs::write(&dest, b"incumbent").expect("seed destination");

    let mut staged = StagedFile::create(&dest).expect("stage");
    staged.write_all(b"needs recovery").expect("write");
    let sealed = staged.seal();

    let mut failure = sealed
        .publish(options(PublishMode::CreateNew))
        .expect_err("create-new must refuse an existing destination");
    assert_eq!(failure.effect(), PublishEffect::NotPublished);
    let handle = failure
        .take_recovery_handle()
        .expect("failed publish retains the temp and its handle");
    let temp = handle
        .info()
        .temp_path()
        .map(PathBuf::from)
        .expect("os backend reports the temp path");
    assert!(exists(&temp), "temp retained for recovery");

    // Sabotage the retained temp so remove-file cleanup cannot
    // succeed: move the file aside and plant a directory at its path.
    // The rename comes first because the ops object still holds the
    // temp handle open, and Windows refuses name reuse while a delete
    // is pending.
    let moved = dir.dest("moved-temp.bin");
    std::fs::rename(&temp, &moved).expect("move temp aside");
    std::fs::create_dir(&temp).expect("plant directory at temp path");

    let mut cleanup_failure = handle
        .cleanup()
        .expect_err("cleanup cannot remove a directory as a file");
    let handle = cleanup_failure
        .take_recovery_handle()
        .expect("failed cleanup returns the handle for retry");

    // Restore a removable state and retry: cleanup succeeds and the
    // incumbent destination is untouched throughout.
    std::fs::remove_dir(&temp).expect("remove planted directory");
    std::fs::rename(&moved, &temp).expect("restore temp path");
    let receipt = handle.cleanup().expect("retried cleanup removes the temp");
    assert_eq!(receipt.temp_state(), TempState::Absent);
    assert!(!exists(&temp));
    assert_eq!(read(&dest).as_deref(), Some(b"incumbent".as_slice()));
    assert_eq!(children(dir.path()), vec![dest.clone()]);
}
