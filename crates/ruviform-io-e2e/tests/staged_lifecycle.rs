//! `StagedFile` → `SealedFile` → publish → reopen boundary (R1/R2).
//!
//! Exercises the type-state staged-publish lifecycle end to end on
//! the real OS backend: sealed readback verification gates the
//! publish, `CreateNew` conflicts are honest `NotPublished`, `Replace`
//! performs atomic owner replacement, and abort/drop recover the
//! staging temp. Adapter-level fault classes (Indeterminate,
//! directory-sync failure, Unknown temp) are covered by the crate's
//! own `#[cfg(test)]` mock suite — only real-OS-reproducible
//! behaviour is asserted here.

mod common;

use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;

use common::{TestDir, children, exists, read};
use ruviform_io_file::{
    Durability, PublishEffect, PublishFailureKind, PublishMode, PublishOptions, StagedFile,
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
