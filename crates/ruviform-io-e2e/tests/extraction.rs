//! ZIP/TAR secure-extraction composition tests (R4/R8).
//!
//! The boundary under test: `ExtractionSession` turns a verified-parse
//! archive into an isolated staging tree under `SafeRelativePath`
//! policy — traversal, links, special entries, canonical collisions,
//! and budgets refuse hostile entries and never leave a usable
//! staging tree behind. Hostile TAR fixtures are crafted byte-exact
//! (`common::crafted_tar_entry`) because the writer API only emits
//! regular files.

mod common;

use std::io::Cursor;

use common::{TestDir, crafted_tar_entry, exists, read};
use ruviform_io_archive::extract::{ExtractOptions, ExtractionSession};
use ruviform_io_archive::tar::{TarArchiveReader, TarArchiveWriter, TarEntryOptions};
use ruviform_io_archive::zip::{
    ZipArchiveReader, ZipArchiveWriter, ZipCompressionMethod, ZipEntryOptions,
};
use ruviform_io_archive::{ArchiveErrorKind, ArchiveReadLimits};
use std::io::Write;

/// Creates a session with its isolated staging root under `dir` —
/// the parent must exist; the session creates the root itself.
fn session(dir: &TestDir, tag: &str) -> ExtractionSession {
    let parent = dir.dest(tag);
    std::fs::create_dir(&parent).expect("staging parent");
    ExtractionSession::create(&parent, ExtractOptions::default()).expect("session")
}

/// Wraps a TAR body (entries + terminator) in a reader.
fn tar_reader(bytes: Vec<u8>) -> TarArchiveReader<Cursor<Vec<u8>>> {
    TarArchiveReader::new(Cursor::new(bytes), ArchiveReadLimits::default())
}

#[test]
fn tar_round_trip_extracts_verified_tree() {
    let dir = TestDir::new("extract-round");
    let mut body = Vec::new();
    {
        let mut writer = TarArchiveWriter::new(&mut body);
        let mut entry = writer
            .start_entry("sub/hello.txt", 5, &TarEntryOptions::default())
            .expect("entry");
        entry.write_all(b"hello").expect("payload");
        entry.finish().expect("entry finish");
        writer.finish().expect("archive finish");
    }

    let (_reader, tree) = session(&dir, "stage")
        .extract_tar(tar_reader(body))
        .expect("extract");
    let root = tree.path().to_path_buf();
    assert!(root.is_dir(), "staging root holds the extracted tree");
    assert_eq!(
        read(&root.join("sub/hello.txt")).as_deref(),
        Some(b"hello".as_slice())
    );
    assert_eq!(tree.receipt().files, 1);
    tree.cleanup().expect("cleanup");
    assert!(!exists(&root), "cleanup removes the staging root");
}

#[test]
fn traversal_entry_is_refused_and_nothing_escapes() {
    let dir = TestDir::new("extract-traversal");
    let body = crafted_tar_entry("../escape.txt", b'0', "", b"pwned");
    let escape = dir.dest("escape.txt");

    let result = session(&dir, "stage").extract_tar(tar_reader(body));
    let failure = result.expect_err("traversal must fail");
    assert_eq!(failure.error().kind(), ArchiveErrorKind::InvalidInput);
    assert!(
        !exists(&escape),
        "no payload may be written outside the staging root"
    );
    // `..` from the staging root would land inside the session parent
    // — it must not exist there either.
    assert!(!exists(&dir.dest("stage").join("escape.txt")));
    // Refused extraction must not leave a usable staging tree.
    assert!(!exists(&failure.staging_root));
}

#[test]
fn nested_traversal_is_refused() {
    let dir = TestDir::new("extract-nested");
    let body = crafted_tar_entry("a/../../b/evil.txt", b'0', "", b"x");
    let result = session(&dir, "stage").extract_tar(tar_reader(body));
    assert!(result.is_err(), "interior .. must be refused");
}

#[test]
fn absolute_path_entry_is_refused() {
    let dir = TestDir::new("extract-absolute");
    for (i, name) in ["/etc/passwd", "C:/windows/system32/x.dll"]
        .into_iter()
        .enumerate()
    {
        let body = crafted_tar_entry(name, b'0', "", b"x");
        let result = session(&dir, &format!("stage-{i}")).extract_tar(tar_reader(body));
        assert!(
            result.is_err(),
            "absolute/drive path {name} must be refused"
        );
    }
}

#[test]
fn symlink_and_special_entries_are_refused() {
    let dir = TestDir::new("extract-links");
    for (i, (typeflag, name, expected)) in [
        // link entries surface from the reader and are refused by the
        // extraction policy as InvalidInput.
        (b'2', "link.txt", ArchiveErrorKind::InvalidInput),
        (b'1', "hard.txt", ArchiveErrorKind::InvalidInput),
        // fifo/special types are rejected earlier, by the TAR reader
        // itself (Unsupported), before extraction policy sees them.
        (b'6', "fifo0", ArchiveErrorKind::Unsupported),
    ]
    .into_iter()
    .enumerate()
    {
        let body = crafted_tar_entry(name, typeflag, "/abs/target", b"");
        let failure = session(&dir, &format!("stage-{i}"))
            .extract_tar(tar_reader(body))
            .expect_err("special entry must be refused");
        assert_eq!(
            failure.error().kind(),
            expected,
            "typeflag {} refusal kind",
            char::from(typeflag)
        );
    }
}

#[test]
fn case_fold_collision_is_refused_and_tree_removed() {
    let dir = TestDir::new("extract-casefold");
    // Two names that differ only by case collide canonically. TAR
    // extraction streams entries, so the first name may already be
    // staged when the collision surfaces — the guarantee under test
    // is refuse-then-clean, not plan-phase refusal.
    let mut body = Vec::new();
    {
        let mut writer = TarArchiveWriter::new(&mut body);
        for name in ["Dir/file.txt", "dir/FILE.txt"] {
            let mut entry = writer
                .start_entry(name, 1, &TarEntryOptions::default())
                .expect("entry");
            entry.write_all(b"z").expect("payload");
            entry.finish().expect("entry finish");
        }
        writer.finish().expect("archive finish");
    }
    let failure = session(&dir, "stage")
        .extract_tar(tar_reader(body))
        .expect_err("canonical collision must fail extraction");
    assert!(
        !exists(&failure.staging_root),
        "collision failure must not leave a usable staging tree"
    );
}

#[test]
fn extraction_budget_binds_total_output_bytes() {
    let dir = TestDir::new("extract-budget");
    let payload = vec![0xAB_u8; 4096];
    let mut body = Vec::new();
    {
        let mut writer = TarArchiveWriter::new(&mut body);
        for i in 0..4 {
            let name = format!("blob-{i}.bin");
            let mut entry = writer
                .start_entry(
                    &name,
                    u64::try_from(payload.len()).expect("len"),
                    &TarEntryOptions::default(),
                )
                .expect("entry");
            entry.write_all(&payload).expect("payload");
            entry.finish().expect("entry finish");
        }
        writer.finish().expect("archive finish");
    }
    let mut options = ExtractOptions::default();
    options.limits.max_total_uncompressed_bytes = 8192; // half the payload
    let parent = dir.dest("stage");
    std::fs::create_dir(&parent).expect("staging parent");
    let result = ExtractionSession::create(&parent, options)
        .expect("session")
        .extract_tar(tar_reader(body));
    let failure = result.expect_err("budget must bind");
    assert_eq!(failure.error().kind(), ArchiveErrorKind::LimitExceeded);
}

#[test]
fn failed_extraction_cleans_or_hands_out_recovery() {
    // A first valid entry followed by a hostile one: the failure path
    // must either remove the partial tree itself or hand out a
    // recovery handle that can — never leave a silently usable tree.
    let dir = TestDir::new("extract-partial");
    let mut body = Vec::new();
    {
        let mut writer = TarArchiveWriter::new(&mut body);
        let mut entry = writer
            .start_entry("ok.txt", 3, &TarEntryOptions::default())
            .expect("entry");
        entry.write_all(b"ok!").expect("payload");
        entry.finish().expect("entry finish");
        // Drop the writer without archive finish: the two trailing
        // zero blocks are appended manually below anyway.
    }
    // Append a crafted traversal entry (header + padded data) plus the
    // archive terminator.
    let hostile = crafted_tar_entry("../evil.txt", b'0', "", b"x");
    body.extend_from_slice(&hostile[..512 + 512]);
    body.resize(body.len() + 1024, 0);

    let failure = session(&dir, "stage")
        .extract_tar(tar_reader(body))
        .expect_err("hostile tail must fail extraction");
    let root = failure.staging_root.clone();
    match failure.into_handle() {
        Some(handle) => {
            assert_eq!(handle.path(), root.as_path());
            let info = handle.inspect();
            assert_eq!(info.root, root);
            handle.cleanup().expect("recovery cleanup");
        }
        None => assert!(!exists(&root), "no handle ⇒ tree already removed"),
    }
    assert!(!exists(&root), "staging root must be gone after cleanup");
}

#[test]
fn zip_extract_verifies_extracted_tree() {
    let dir = TestDir::new("extract-zip");
    let mut bytes = Cursor::new(Vec::new());
    {
        let mut writer = ZipArchiveWriter::new(&mut bytes);
        for (name, payload) in [
            ("a.txt", b"alpha".as_slice()),
            ("b/c.txt", b"beta".as_slice()),
        ] {
            let mut entry = writer
                .start_entry(name, &ZipEntryOptions::new(ZipCompressionMethod::Deflate))
                .expect("entry");
            entry.write_all(payload).expect("payload");
            entry.finish().expect("entry finish");
        }
        writer.finish().expect("zip finish");
    }
    bytes.set_position(0);

    let mut reader =
        ZipArchiveReader::new(&mut bytes, ArchiveReadLimits::default()).expect("indexed open");
    let tree = session(&dir, "stage")
        .extract_zip(&mut reader)
        .expect("extract zip");
    let root = tree.path().to_path_buf();
    assert_eq!(
        read(&root.join("a.txt")).as_deref(),
        Some(b"alpha".as_slice())
    );
    assert_eq!(
        read(&root.join("b/c.txt")).as_deref(),
        Some(b"beta".as_slice())
    );
    assert_eq!(tree.receipt().files, 2);
    tree.cleanup().expect("cleanup");
}

#[test]
fn zip_traversal_entry_is_refused() {
    let dir = TestDir::new("extract-zip-trav");
    // Craft a hostile ZIP: the writer validates only empty/NUL names,
    // so "../x" can be stored — extraction must refuse it.
    let mut bytes = Cursor::new(Vec::new());
    {
        let mut writer = ZipArchiveWriter::new(&mut bytes);
        let mut entry = writer
            .start_entry(
                "../escape.txt",
                &ZipEntryOptions::new(ZipCompressionMethod::Stored),
            )
            .expect("entry");
        entry.write_all(b"pwned").expect("payload");
        entry.finish().expect("entry finish");
        writer.finish().expect("zip finish");
    }
    bytes.set_position(0);

    let mut reader =
        ZipArchiveReader::new(&mut bytes, ArchiveReadLimits::default()).expect("indexed open");
    let result = session(&dir, "stage").extract_zip(&mut reader);
    assert!(result.is_err(), "zip traversal entry must be refused");
    assert!(!exists(&dir.dest("escape.txt")));
}
