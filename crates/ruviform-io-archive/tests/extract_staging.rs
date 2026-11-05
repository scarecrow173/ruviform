//! Staging lifecycle — session abort/Drop, verified cleanup with
//! identity checks, foreign-swap refusal, recovery-handle
//! inspect/retry, and `into_path` ownership transfer (IO-IMP-10).

mod common;

use std::io::Cursor;

use common::extract::{TestDir, exists, read_staged};
use common::{EntrySpec, build_simple, writer_zip};
use ruviform_io_archive::extract::{ExtractOptions, ExtractionSession, RecoveryTreeState};
use ruviform_io_archive::zip::{ZipArchiveReader, ZipCompressionMethod};
use ruviform_io_archive::{ArchiveErrorKind, ArchiveReadLimits};

fn options() -> ExtractOptions {
    ExtractOptions::new()
}

fn extract_zip(
    entries: &[(&str, &[u8])],
    parent: &std::path::Path,
) -> ruviform_io_archive::extract::ExtractedTree {
    let bytes = writer_zip(
        &entries
            .iter()
            .map(|(n, d)| (*n, ZipCompressionMethod::Stored, *d))
            .collect::<Vec<_>>(),
    );
    let mut archive =
        ZipArchiveReader::new(Cursor::new(bytes), ArchiveReadLimits::default()).expect("reader");
    ExtractionSession::create(parent, options())
        .expect("session")
        .extract_zip(&mut archive)
        .expect("extract")
}

// --- lifecycle ---------------------------------------------------------

#[test]
fn abort_removes_the_staging_root() {
    let dir = TestDir::new("st-abort");
    let session = ExtractionSession::create(dir.path(), options()).expect("session");
    let root = session.staging_root().to_path_buf();
    assert!(exists(&root));
    session.abort().expect("abort");
    assert!(!exists(&root));
    assert_eq!(dir.children(), Vec::<std::path::PathBuf>::new());
}

#[test]
fn dropping_a_session_removes_its_root() {
    let dir = TestDir::new("st-sdrop");
    let root = {
        let session = ExtractionSession::create(dir.path(), options()).expect("session");
        session.staging_root().to_path_buf()
    };
    assert!(!exists(&root));
}

#[test]
fn dropping_a_tree_cleans_it() {
    let dir = TestDir::new("st-tdrop");
    let root = {
        let tree = extract_zip(&[("a.txt", b"x"), ("d/e.txt", b"y")], dir.path());
        tree.path().to_path_buf()
    };
    assert!(!exists(&root));
    assert_eq!(dir.children(), Vec::<std::path::PathBuf>::new());
}

#[test]
fn into_path_transfers_ownership_without_cleanup() {
    let dir = TestDir::new("st-into");
    let tree = extract_zip(&[("keep.txt", b"data")], dir.path());
    let root = tree.path().to_path_buf();
    let path = tree.into_path();
    assert_eq!(path, root);
    assert!(exists(&path));
    assert_eq!(
        read_staged(&path.join("keep.txt")).as_deref(),
        Some(b"data".as_slice())
    );
    // TestDir Drop removes it — the API no longer owns the tree.
}

#[test]
fn explicit_cleanup_reports_counts() {
    let dir = TestDir::new("st-count");
    let tree = extract_zip(
        &[("a.txt", b"x"), ("d/e.txt", b"y"), ("d/f.txt", b"z")],
        dir.path(),
    );
    let root = tree.path().to_path_buf();
    let receipt = tree.cleanup().expect("cleanup");
    assert_eq!(receipt.retained, 0);
    assert!(receipt.removed >= 4, "files+dirs+root: {}", receipt.removed);
    assert!(!exists(&root));
}

// --- verified cleanup: foreign-object refusal --------------------------

#[test]
fn cleanup_retains_a_foreign_file_at_a_recorded_name() {
    let dir = TestDir::new("st-foreign");
    let tree = extract_zip(&[("keep.txt", b"mine"), ("d/inner.txt", b"x")], dir.path());
    let root = tree.path().to_path_buf();
    // Replace the staged file with a different object at the same name.
    let staged = root.join("keep.txt");
    std::fs::remove_file(&staged).expect("remove staged");
    std::fs::write(&staged, b"FOREIGN").expect("plant foreign");

    let err = tree.cleanup().expect_err("cleanup must fail");
    assert!(err.failure.receipt.retained >= 1);
    // The foreign object must NOT have been deleted.
    assert_eq!(read_staged(&staged).as_deref(), Some(b"FOREIGN".as_slice()));
    // Everything else was removed honestly.
    assert!(!exists(&root.join("d")));

    // Recovery handle: inspect sees the real root still present.
    let handle = err.handle;
    assert_eq!(handle.inspect().state, RecoveryTreeState::Present);

    // Retried cleanup still refuses while the foreign file is there.
    let err2 = handle.cleanup().expect_err("still foreign");
    assert_eq!(read_staged(&staged).as_deref(), Some(b"FOREIGN".as_slice()));

    // Remove the foreign object ourselves → retry succeeds.
    std::fs::remove_file(&staged).expect("remove foreign");
    err2.handle.cleanup().expect("cleanup after manual removal");
    assert!(!exists(&root));
}

#[test]
fn cleanup_refuses_to_delete_a_swapped_root_tree() {
    let dir = TestDir::new("st-swap");
    let tree = extract_zip(&[("keep.txt", b"mine")], dir.path());
    let root = tree.path().to_path_buf();
    // Force a recovery handle by planting a foreign file first.
    let staged = root.join("keep.txt");
    std::fs::remove_file(&staged).expect("remove staged");
    std::fs::write(&staged, b"FOREIGN").expect("plant foreign");
    let err = tree.cleanup().expect_err("cleanup must fail");
    let handle = err.handle;

    // Now swap the entire staging root for a foreign directory that
    // carries a victim file we must never delete.
    let moved = dir.path().join("root-moved");
    std::fs::rename(&root, &moved).expect("move original root");
    let foreign = root.clone();
    std::fs::create_dir(&foreign).expect("create foreign root");
    let victim = foreign.join("victim.txt");
    std::fs::write(&victim, b"VICTIM").expect("write victim");

    // inspect must NOT report the recorded tree present.
    let state = handle.inspect().state;
    assert_ne!(
        state,
        RecoveryTreeState::Present,
        "foreign tree reported as the recorded tree"
    );
    // cleanup must refuse and must not touch the victim.
    let err2 = handle.cleanup().expect_err("must refuse foreign tree");
    let _ = err2;
    assert_eq!(read_staged(&victim).as_deref(), Some(b"VICTIM".as_slice()));
    assert!(exists(&foreign));
}

#[cfg(unix)]
#[test]
fn cleanup_retains_a_symlink_at_a_recorded_name() {
    let dir = TestDir::new("st-symlink");
    let tree = extract_zip(&[("f.txt", b"mine")], dir.path());
    let root = tree.path().to_path_buf();
    let staged = root.join("f.txt");
    std::fs::remove_file(&staged).expect("remove staged");
    std::os::unix::fs::symlink("/etc/hostname", &staged).expect("symlink");

    let err = tree.cleanup().expect_err("cleanup must fail");
    assert!(err.failure.receipt.retained >= 1);
    // The link object is retained — never followed, never deleted.
    let meta = std::fs::symlink_metadata(&staged).expect("symlink_metadata");
    assert!(meta.file_type().is_symlink());

    std::fs::remove_file(&staged).expect("remove link");
    err.handle.cleanup().expect("cleanup after removal");
    assert!(!exists(&root));
}

#[cfg(windows)]
#[test]
fn a_junction_at_the_root_is_never_followed_or_deleted() {
    let dir = TestDir::new("st-junction");
    let tree = extract_zip(&[("keep.txt", b"mine")], dir.path());
    let root = tree.path().to_path_buf();
    let staged = root.join("keep.txt");
    std::fs::remove_file(&staged).expect("remove staged");
    std::fs::write(&staged, b"FOREIGN").expect("plant foreign");
    let err = tree.cleanup().expect_err("cleanup must fail");
    let handle = err.handle;

    // Replace the root with a junction to the original tree.
    let moved = dir.path().join("root-moved");
    std::fs::rename(&root, &moved).expect("move original root");
    let junction = root.clone();
    let status = std::process::Command::new("cmd")
        .args([
            "/c",
            "mklink",
            "/J",
            junction.to_str().expect("utf8 path"),
            moved.to_str().expect("utf8 path"),
        ])
        .status()
        .expect("mklink");
    // mklink may be unavailable — report honestly rather than skip.
    assert!(
        status.success(),
        "mklink /J failed with {status}; cannot run junction test"
    );
    let victim = moved.join("keep.txt");

    let state = handle.inspect().state;
    assert_ne!(state, RecoveryTreeState::Present);
    let err2 = handle.cleanup().expect_err("must refuse reparse point");
    let _ = err2;
    assert_eq!(read_staged(&victim).as_deref(), Some(b"FOREIGN".as_slice()));

    // Manual teardown: junction first, then the real tree.
    std::fs::remove_dir(&junction).expect("remove junction");
    std::fs::remove_dir_all(&moved).expect("remove moved tree");
}

// --- staging isolation --------------------------------------------------

#[test]
fn staging_parent_that_does_not_exist_fails_cleanly() {
    let dir = TestDir::new("st-noparent");
    let missing = dir.path().join("no-such-dir");
    let err = ExtractionSession::create(&missing, options()).expect_err("must fail");
    assert_eq!(err.kind(), ArchiveErrorKind::Io);
}

#[test]
fn two_concurrent_sessions_do_not_collide() {
    let dir = TestDir::new("st-two");
    let s1 = ExtractionSession::create(dir.path(), options()).expect("s1");
    let s2 = ExtractionSession::create(dir.path(), options()).expect("s2");
    assert_ne!(s1.staging_root(), s2.staging_root());
    s1.abort().expect("a1");
    s2.abort().expect("a2");
}

// --- archive-source honesty ----------------------------------------------

#[test]
fn a_malformed_second_entry_fails_and_cleans() {
    let dir = TestDir::new("st-mal");
    let mut bad = EntrySpec::stored("bad", b"x");
    bad.crc32 = 0xDEAD_BEEF;
    let bytes = build_simple(&[EntrySpec::stored("ok", b"f"), bad]);
    let mut archive =
        ZipArchiveReader::new(Cursor::new(bytes), ArchiveReadLimits::default()).expect("reader");
    let f = ExtractionSession::create(dir.path(), options())
        .expect("session")
        .extract_zip(&mut archive)
        .expect_err("must fail");
    assert!(!exists(&f.staging_root));
    assert!(f.handle().is_none());
}
