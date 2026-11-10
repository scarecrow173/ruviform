//! Secure extraction — ZIP: verified-tree round trips, the negative
//! path matrix, canonical collisions, kind confinement, budgets, and
//! mid-extraction failure cleanup (IO-IMP-10).

mod common;

use std::io::Cursor;

use common::extract::{TestDir, exists, read_staged};
use common::{EntrySpec, METHOD_STORED, build_simple, crc32, writer_zip};
use ruviform_io_archive::extract::{ExtractOptions, ExtractionSession};
use ruviform_io_archive::zip::{ZipArchiveReader, ZipCompressionMethod};
use ruviform_io_archive::{ArchiveErrorKind, ArchiveReadLimits};

fn options() -> ExtractOptions {
    ExtractOptions::new()
}

fn options_with(configure: impl Fn(&mut ArchiveReadLimits)) -> ExtractOptions {
    let mut o = ExtractOptions::new();
    configure(&mut o.limits);
    o
}

fn extract(
    bytes: Vec<u8>,
    parent: &std::path::Path,
    opts: ExtractOptions,
) -> Result<
    ruviform_io_archive::extract::ExtractedTree,
    ruviform_io_archive::extract::ExtractionFailure,
> {
    let mut archive =
        ZipArchiveReader::new(Cursor::new(bytes), ArchiveReadLimits::default()).expect("reader");
    ExtractionSession::create(parent, opts)
        .expect("session")
        .extract_zip(&mut archive)
}

fn dir_spec(name: &str) -> EntrySpec {
    let mut spec = EntrySpec::raw(name.as_bytes(), METHOD_STORED, b"", 0, 0);
    spec.external_attrs = 0o40_755 << 16;
    spec
}

// --- round trips ---------------------------------------------------

#[test]
fn extracts_files_and_directories_into_staging() {
    let dir = TestDir::new("zip-rt");
    let bytes = writer_zip(&[
        ("a.txt", ZipCompressionMethod::Stored, b"alpha"),
        ("d/e.bin", ZipCompressionMethod::Deflate, b"beta-beta"),
        ("empty", ZipCompressionMethod::Stored, b""),
    ]);
    let tree = extract(bytes, dir.path(), options()).expect("extract");
    let root = tree.path().to_path_buf();
    assert!(root.starts_with(dir.path()));
    assert_eq!(
        read_staged(&root.join("a.txt")).as_deref(),
        Some(b"alpha".as_slice())
    );
    assert_eq!(
        read_staged(&root.join("d").join("e.bin")).as_deref(),
        Some(b"beta-beta".as_slice())
    );
    assert_eq!(
        read_staged(&root.join("empty")).as_deref(),
        Some(b"".as_slice())
    );
    let receipt = tree.receipt();
    assert_eq!(receipt.entries, 3);
    assert_eq!(receipt.files, 3);
    assert_eq!(receipt.payload_bytes, 14);
    tree.cleanup().expect("cleanup");
    assert!(!exists(&root));
    assert_eq!(dir.children(), Vec::<std::path::PathBuf>::new());
}

#[test]
fn explicit_directory_entries_are_created() {
    let dir = TestDir::new("zip-dirs");
    let bytes = build_simple(&[
        dir_spec("top/"),
        dir_spec("top/inner/"),
        EntrySpec::stored("top/inner/f.txt", b"x"),
    ]);
    let tree = extract(bytes, dir.path(), options()).expect("extract");
    let root = tree.path().to_path_buf();
    assert!(root.join("top").join("inner").is_dir());
    assert_eq!(tree.receipt().directories, 2);
    tree.cleanup().expect("cleanup");
}

#[test]
fn staging_roots_are_unique_per_session() {
    let dir = TestDir::new("zip-iso");
    let s1 = ExtractionSession::create(dir.path(), options()).expect("s1");
    let s2 = ExtractionSession::create(dir.path(), options()).expect("s2");
    assert_ne!(s1.staging_root(), s2.staging_root());
    assert_eq!(dir.children().len(), 2);
    s1.abort().expect("abort s1");
    s2.abort().expect("abort s2");
    assert_eq!(dir.children(), Vec::<std::path::PathBuf>::new());
}

// --- negative path matrix -------------------------------------------

#[test]
fn rejects_dangerous_and_divergent_names() {
    let cases: Vec<Vec<u8>> = [
        b"../evil".as_slice(),
        b"a/../../b",
        b"a//b",
        b"/abs",
        b"C:/win",
        b"C:\\win",
        b"c:rel",
        b"\\\\srv\\share\\x",
        b"a\\b",
        b"a:b",
        b"con.txt",
        b"NUL",
        b"lpt1",
        b"aux.ini",
        b"com9",
        b"a.",
        b"a ",
        b"..",
        b".",
        b"a/./b",
        b"a/../b",
        b"x\0y",
        &[0xFF, 0xFE][..],
    ]
    .into_iter()
    .map(<[u8]>::to_vec)
    .collect();
    for name in cases {
        let dir = TestDir::new("zip-neg");
        let mut spec = EntrySpec::raw(&name, METHOD_STORED, b"x", 1, crc32(b"x"));
        if name.last() == Some(&b'/') {
            spec.external_attrs = 0o40_755 << 16;
        }
        let bytes = build_simple(&[spec]);
        let Ok(mut archive) =
            ZipArchiveReader::new(Cursor::new(bytes), ArchiveReadLimits::default())
        else {
            continue; // reader rejects some names up front — fine
        };
        let result = ExtractionSession::create(dir.path(), options())
            .expect("session")
            .extract_zip(&mut archive);
        match result {
            Err(f) => {
                assert_eq!(
                    f.error().kind(),
                    ArchiveErrorKind::InvalidInput,
                    "name {name:?}: wrong kind"
                );
                assert!(!exists(&f.staging_root), "name {name:?}: staging leaked");
                assert!(dir.children().is_empty(), "name {name:?}: children left");
            }
            Ok(tree) => panic!("name {name:?} was extracted to {}", tree.path().display()),
        }
    }
}

// --- canonical collisions -------------------------------------------

#[test]
fn rejects_exact_duplicate_paths() {
    let dir = TestDir::new("zip-dup");
    let bytes = build_simple(&[
        EntrySpec::stored("a.txt", b"1"),
        EntrySpec::stored("a.txt", b"2"),
    ]);
    let f = extract(bytes, dir.path(), options()).expect_err("must fail");
    assert_eq!(f.error().kind(), ArchiveErrorKind::InvalidInput);
    assert!(!exists(&f.staging_root));
}

#[test]
fn rejects_case_fold_collision() {
    let dir = TestDir::new("zip-case");
    let bytes = build_simple(&[
        EntrySpec::stored("Foo.txt", b"1"),
        EntrySpec::stored("foo.txt", b"2"),
    ]);
    let f = extract(bytes, dir.path(), options()).expect_err("must fail");
    assert_eq!(f.error().kind(), ArchiveErrorKind::InvalidInput);
}

#[test]
fn rejects_unicode_normalization_collision() {
    let dir = TestDir::new("zip-unicode");
    // NFC `é` vs NFD `e` + U+0301 — different stored bytes, same file.
    let bytes = build_simple(&[
        EntrySpec::stored("é.txt", b"1"),
        EntrySpec::stored("e\u{0301}.txt", b"2"),
    ]);
    let f = extract(bytes, dir.path(), options()).expect_err("must fail");
    assert_eq!(f.error().kind(), ArchiveErrorKind::InvalidInput);
}

#[test]
fn rejects_file_versus_directory_conflicts() {
    // File `a` then `a/b` — `a` as a prefix of `a/b`.
    let dir = TestDir::new("zip-fd1");
    let bytes = build_simple(&[EntrySpec::stored("a", b"1"), EntrySpec::stored("a/b", b"2")]);
    let f = extract(bytes, dir.path(), options()).expect_err("must fail");
    assert_eq!(f.error().kind(), ArchiveErrorKind::InvalidInput);

    // `a/b` then file `a` — same conflict, reversed order.
    let dir2 = TestDir::new("zip-fd2");
    let bytes2 = build_simple(&[EntrySpec::stored("a/b", b"2"), EntrySpec::stored("a", b"1")]);
    let f2 = extract(bytes2, dir2.path(), options()).expect_err("must fail");
    assert_eq!(f2.error().kind(), ArchiveErrorKind::InvalidInput);

    // File `a` then explicit dir `a/` — same canonical path, different kind.
    let dir3 = TestDir::new("zip-fd3");
    let bytes3 = build_simple(&[EntrySpec::stored("a", b"1"), dir_spec("a/")]);
    let f3 = extract(bytes3, dir3.path(), options()).expect_err("must fail");
    assert_eq!(f3.error().kind(), ArchiveErrorKind::InvalidInput);
}

// --- kind confinement -------------------------------------------------

#[test]
fn rejects_symlink_and_special_entries() {
    // Symlink: unix S_IFLNK in the high external attributes.
    let dir = TestDir::new("zip-link");
    let mut link = EntrySpec::raw(b"link", METHOD_STORED, b"target", 6, crc32(b"target"));
    link.external_attrs = 0o120_777 << 16;
    let bytes = build_simple(&[link]);
    let f = extract(bytes, dir.path(), options()).expect_err("must fail");
    assert_eq!(f.error().kind(), ArchiveErrorKind::InvalidInput);

    // FIFO (S_IFIFO) → EntryKind::Special.
    let dir2 = TestDir::new("zip-spec");
    let mut fifo = EntrySpec::stored("pipe", b"");
    fifo.external_attrs = 0o010_644 << 16;
    fifo.name = b"pipe".to_vec();
    let bytes2 = build_simple(&[fifo]);
    let f2 = extract(bytes2, dir2.path(), options()).expect_err("must fail");
    assert_eq!(f2.error().kind(), ArchiveErrorKind::InvalidInput);
}

// --- budgets ---------------------------------------------------------

#[test]
fn enforces_entry_count_budget() {
    let dir = TestDir::new("zip-count");
    let bytes = build_simple(&[EntrySpec::stored("a", b"1"), EntrySpec::stored("b", b"2")]);
    let f = extract(bytes, dir.path(), options_with(|l| l.max_entries = 1)).expect_err("must fail");
    assert_eq!(f.error().kind(), ArchiveErrorKind::LimitExceeded);
    assert!(!exists(&f.staging_root));
}

#[test]
fn enforces_path_depth_budget() {
    let dir = TestDir::new("zip-depth");
    let bytes = build_simple(&[EntrySpec::stored("a/b/c/d.txt", b"x")]);
    let f =
        extract(bytes, dir.path(), options_with(|l| l.max_path_depth = 2)).expect_err("must fail");
    assert_eq!(f.error().kind(), ArchiveErrorKind::LimitExceeded);
}

#[test]
fn enforces_entry_size_budget_before_write() {
    let dir = TestDir::new("zip-esize");
    let bytes = build_simple(&[EntrySpec::stored("big", b"0123456789")]);
    let f = extract(
        bytes,
        dir.path(),
        options_with(|l| l.max_entry_uncompressed_bytes = 4),
    )
    .expect_err("must fail");
    assert_eq!(f.error().kind(), ArchiveErrorKind::LimitExceeded);
    assert!(!exists(&f.staging_root));
}

#[test]
fn enforces_total_declared_budget_before_write() {
    let dir = TestDir::new("zip-total");
    let bytes = build_simple(&[
        EntrySpec::stored("a", b"12345"),
        EntrySpec::stored("b", b"12345"),
    ]);
    let f = extract(
        bytes,
        dir.path(),
        options_with(|l| l.max_total_uncompressed_bytes = 8),
    )
    .expect_err("must fail");
    assert_eq!(f.error().kind(), ArchiveErrorKind::LimitExceeded);
}

#[test]
fn refuses_a_declared_size_bomb_up_front() {
    let dir = TestDir::new("zip-bomb");
    // Declared size dwarfs the real data — the declared-budget check
    // must reject before any write.
    let mut spec = EntrySpec::stored("bomb", b"xx");
    // 0xFFFFFFFF is the ZIP64 sentinel — use the largest non-sentinel
    // declared size (still far above the 1 MiB extraction budget).
    spec.declared_uncompressed = u32::MAX - 1;
    let bytes = build_simple(&[spec]);
    let f = extract(
        bytes,
        dir.path(),
        options_with(|l| l.max_entry_uncompressed_bytes = 1 << 20),
    )
    .expect_err("must fail");
    assert_eq!(f.error().kind(), ArchiveErrorKind::LimitExceeded);
}

// --- mid-extraction failure cleanup -----------------------------------

#[test]
fn cleans_the_partial_tree_on_entry_verification_failure() {
    let dir = TestDir::new("zip-fail");
    // First entry valid, second has a corrupt CRC → fails at finish
    // after the first file is already staged.
    let mut bad = EntrySpec::stored("bad.txt", b"payload");
    bad.crc32 ^= 0xFFFF;
    let bytes = build_simple(&[EntrySpec::stored("good.txt", b"fine"), bad]);
    let f = extract(bytes, dir.path(), options()).expect_err("must fail");
    assert_eq!(f.error().kind(), ArchiveErrorKind::ChecksumMismatch);
    assert!(!exists(&f.staging_root), "partial tree leaked");
    assert_eq!(dir.children(), Vec::<std::path::PathBuf>::new());
}

#[test]
fn staging_root_survives_nothing_on_plan_failure() {
    let dir = TestDir::new("zip-plan");
    // Plan-phase failure (collision) must remove the staging root too.
    let bytes = build_simple(&[
        EntrySpec::stored("a.txt", b"1"),
        EntrySpec::stored("a.txt", b"2"),
    ]);
    let f = extract(bytes, dir.path(), options()).expect_err("must fail");
    assert!(!exists(&f.staging_root));
    assert!(f.handle().is_none());
}

// --- archive payloads are opaque --------------------------------------

#[test]
fn nested_archive_payload_is_extracted_opaquely() {
    // A ZIP inside a ZIP is just bytes: extraction never recurses into
    // nested archives (that would be a consumer-level decision).
    let dir = TestDir::new("zip-nested");
    let inner = build_simple(&[EntrySpec::stored("inner.txt", b"nested")]);
    let bytes = build_simple(&[EntrySpec::stored("bundle.zip", &inner)]);
    let tree = extract(bytes, dir.path(), options()).expect("extract");
    let root = tree.path().to_path_buf();
    assert_eq!(
        read_staged(&root.join("bundle.zip")).as_deref(),
        Some(inner.as_slice())
    );
    assert_eq!(tree.receipt().entries, 1);
    assert_eq!(tree.receipt().unverified_entries, 0);
    tree.cleanup().expect("cleanup");
}
