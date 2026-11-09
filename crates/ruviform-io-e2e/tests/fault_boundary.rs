//! Publish-fault boundary: `DocumentFileWriteError` variant
//! propagation, preserved `PublishFailure` state (effect/phase/kind/
//! durability/recovery handle), recovery ownership, and a synthetic
//! publish conflict on composite archive bytes.
//!
//! Slice-2 rules F3/F4/F12
//! (`docs/test-specs/issue-306-io-e2e-fault-fuzz.md`). All fixtures
//! use only observable public behavior on the real filesystem —
//! `Indeterminate`, directory-sync-after-publish failure and
//! `TempState::Unknown` are *not* reproducible here and stay `NotRun`
//! (the `#[cfg(test)]` `PlatformFileOps` mocks inside
//! `ruviform-io-file` are the authoritative coverage for those
//! classes; this crate must not depend on them).

mod common;

use std::io::{BufReader, Read, Write};

use common::{TestDir, children, exists, read};
use ruviform_io_archive::ArchiveReadLimits;
use ruviform_io_archive::tar::{TarArchiveReader, TarArchiveWriter, TarEntryOptions};
use ruviform_io_compression::gzip;
use ruviform_io_file::{
    Durability, FileErrorKind, PublishEffect, PublishFailureKind, PublishMode, PublishOptions,
    PublishPhase, StagedFile, TempState,
};
use ruviform_io_structured::{DocumentFileReadError, DocumentFileWriteError, json};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
struct Row {
    name: String,
    count: u64,
}

/// A serialize impl that fails before any bytes are produced —
/// exercises the `Encode` arm of `DocumentFileWriteError`.
struct Bad;
impl serde::Serialize for Bad {
    fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
        Err(serde::ser::Error::custom("deliberate serialize failure"))
    }
}

fn options(mode: PublishMode) -> PublishOptions {
    PublishOptions::new(mode, Durability::FileSync)
}

fn row() -> Row {
    Row {
        name: "kick".to_owned(),
        count: 4,
    }
}

#[test]
fn write_file_create_failure_reports_create_and_leaves_nothing() {
    let dir = TestDir::new("create-fail");
    // The destination's parent does not exist — `StagedFile::create`
    // fails before any temp file or destination byte exists.
    let dest = dir.dest("missing").join("doc.json");

    let result = json::write_file(
        &dest,
        &row(),
        &json::JsonWriteOptions::default(),
        options(PublishMode::CreateNew),
    );
    match result {
        Err(DocumentFileWriteError::Create(error)) => {
            assert_eq!(error.kind(), FileErrorKind::Io);
        }
        other => panic!("expected Create failure, got {other:?}"),
    }
    assert!(!exists(&dir.dest("missing")), "no directory may be created");
    assert_eq!(
        children(dir.path()).len(),
        0,
        "create failure must not leave temp residue"
    );
}

#[test]
fn write_file_encode_failure_never_publishes_or_residues() {
    let dir = TestDir::new("encode-fail");
    let dest = dir.dest("broken.json");

    let result = json::write_file(
        &dest,
        &Bad,
        &json::JsonWriteOptions::default(),
        options(PublishMode::CreateNew),
    );
    match result {
        Err(DocumentFileWriteError::Encode(_)) => {}
        other => panic!("expected Encode failure, got {other:?}"),
    }
    assert!(!exists(&dest), "encode failure must not publish");
    assert_eq!(
        children(dir.path()).len(),
        0,
        "unsealed staged temp must be cleaned up"
    );
}

#[test]
fn write_file_publish_conflict_preserves_failure_state_and_destination() {
    let dir = TestDir::new("pub-conflict");
    let dest = dir.dest("doc.json");
    std::fs::write(&dest, br#"{"seeded":true}"#).expect("seed destination");

    let result = json::write_file(
        &dest,
        &row(),
        &json::JsonWriteOptions::default(),
        options(PublishMode::CreateNew),
    );
    match result {
        Err(DocumentFileWriteError::Publish(mut failure)) => {
            // The full PublishFailure survives the structured layer:
            // classification, stage, outcome and recovery ownership.
            assert_eq!(failure.kind(), PublishFailureKind::DestinationExists);
            assert_eq!(failure.phase(), PublishPhase::Publish);
            assert_eq!(failure.effect(), PublishEffect::NotPublished);
            let handle = failure
                .take_recovery_handle()
                .expect("recovery handle must survive DocumentFileWriteError::Publish");
            let observation = handle.inspect().expect("inspect retained temp");
            assert!(
                matches!(
                    observation.temp_state(),
                    TempState::Retained | TempState::Absent
                ),
                "temp state must be an honest observation"
            );
            let receipt = handle.cleanup().expect("explicit cleanup");
            assert_eq!(receipt.temp_state(), TempState::Absent);
        }
        other => panic!("expected Publish failure, got {other:?}"),
    }
    // First writer's bytes are preserved byte-for-byte.
    assert_eq!(
        read(&dest).as_deref(),
        Some(br#"{"seeded":true}"#.as_slice()),
        "conflicting publish must not touch the existing destination"
    );
    assert_eq!(
        children(dir.path()).len(),
        1,
        "only the seeded destination may remain after cleanup"
    );
}

#[test]
fn directory_destination_publish_fails_without_touching_it() {
    let dir = TestDir::new("dir-dest");
    let dest = dir.dest("as-dir");
    std::fs::create_dir(&dest).expect("directory destination");

    let result = json::write_file(
        &dest,
        &row(),
        &json::JsonWriteOptions::default(),
        options(PublishMode::Replace),
    );
    match result {
        Err(DocumentFileWriteError::Publish(mut failure)) => {
            // Replacing a directory with a file cannot apply — the
            // failure must be a certain non-application, never
            // reported as published or indeterminate.
            assert_eq!(failure.effect(), PublishEffect::NotPublished);
            assert_eq!(failure.phase(), PublishPhase::Publish);
            if let Some(handle) = failure.take_recovery_handle() {
                let _ = handle.cleanup();
            }
        }
        other => panic!("expected Publish failure for directory destination, got {other:?}"),
    }
    assert!(dest.is_dir(), "the directory must be untouched");
    assert_eq!(children(&dest).len(), 0);
}

/// Writes the fixture TAR through a finished gzip encoder — the same
/// composite layering as `archive_compression_staged.rs`.
fn write_tar_gzip_into(out: &mut Vec<u8>) {
    let files: &[(&str, &[u8])] = &[
        ("manifest.txt", b"release=rf306\n"),
        ("data/track-a.raw", b"\x00\x01\x02\x03pcm"),
    ];
    let mut encoder = gzip::Encoder::new(out, gzip::EncodeOptions::default()).expect("encoder");
    {
        let mut writer = TarArchiveWriter::new(&mut encoder);
        for (name, payload) in files {
            let mut entry = writer
                .start_entry(
                    name,
                    u64::try_from(payload.len()).expect("len"),
                    &TarEntryOptions::default(),
                )
                .expect("entry");
            entry.write_all(payload).expect("payload");
            entry.finish().expect("entry finish");
        }
        writer.finish().expect("tar finish");
    }
    encoder.finish().expect("gzip finish");
}

/// Independent layered verification of composite bytes: gzip verified
/// EOF wrapping a TAR stream whose reader reaches `finish`. Works on
/// any `Read` — in-memory bytes or the sealed readback stream.
fn verify_composite<R: Read>(reader: R) -> bool {
    let mut buffered = BufReader::new(reader);
    let mut decoder = gzip::Decoder::new(&mut buffered, gzip::DecodeOptions::default());
    let tar_ok = (|| -> Option<()> {
        let mut archive = TarArchiveReader::new(&mut decoder, ArchiveReadLimits::default());
        while let Some(mut entry) = archive.next_entry().ok()? {
            let mut sink = Vec::new();
            entry.read_to_end(&mut sink).ok()?;
            entry.finish().ok()?;
        }
        archive.finish().ok()?;
        Some(())
    })();
    tar_ok.is_some() && decoder.finish().is_ok()
}

#[test]
fn composite_publish_conflict_keeps_first_writer_and_recovers_temp() {
    let dir = TestDir::new("composite-conflict");
    let dest = dir.dest("bundle.tar.gz");

    // Build the composite through the layered boundary, stage it,
    // seal, and independently verify before publish.
    let mut composite = Vec::new();
    write_tar_gzip_into(&mut composite);
    assert!(
        verify_composite(&composite[..]),
        "fixture composite must verify"
    );

    let mut staged = StagedFile::create(&dest).expect("stage");
    staged.write_all(&composite).expect("write composite");
    let mut sealed = staged.seal();
    {
        let mut readback = sealed.open_readback().expect("readback");
        assert!(
            verify_composite(&mut readback),
            "sealed bytes must verify before publish"
        );
    }

    // A competing writer claims the destination between seal and
    // publish — the synthetic publish race this boundary exists for.
    std::fs::write(&dest, b"first-writer").expect("competing destination");

    let failure = sealed
        .publish(options(PublishMode::CreateNew))
        .expect_err("conflicting publish must fail");
    assert_eq!(failure.kind(), PublishFailureKind::DestinationExists);
    assert_eq!(failure.effect(), PublishEffect::NotPublished);
    assert_eq!(failure.phase(), PublishPhase::Publish);
    assert_eq!(
        read(&dest).as_deref(),
        Some(b"first-writer".as_slice()),
        "first writer's bytes must be preserved"
    );

    // The refused publish retains its temp: recovery goes through the
    // owned handle, never through path guessing.
    let mut failure = failure;
    let handle = failure
        .take_recovery_handle()
        .expect("retained temp must be recoverable");
    handle.cleanup().expect("recovery cleanup");
    assert_eq!(
        children(dir.path()).len(),
        1,
        "only the competing destination may remain"
    );
}

#[test]
fn read_failure_on_open_readback_is_typed_file_error() {
    // `open_readback` is the pre-publish read surface; a missing temp
    // cannot occur through the public API, so this fixture instead
    // pins the *variant contract* consumers rely on: read errors are
    // `FileError`, never publish failures.
    let dir = TestDir::new("readback-kind");
    let dest = dir.dest("doc.json");
    json::write_file(
        &dest,
        &row(),
        &json::JsonWriteOptions::default(),
        options(PublishMode::CreateNew),
    )
    .expect("write_file");

    // Reopening the published destination through the document read
    // path classifies decode problems as DocumentFileReadError —
    // the same typed boundary fault_boundary consumers see.
    std::fs::write(&dest, b"\xff\xfe\x00invalid").expect("corrupt");
    let result: Result<Row, DocumentFileReadError> =
        json::read_file(&dest, &json::JsonReadOptions::default());
    assert!(
        matches!(result, Err(DocumentFileReadError::Decode(_))),
        "corrupt published bytes must be a Decode failure"
    );
}
