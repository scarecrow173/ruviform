//! Secure extraction — TAR: verified-tree round trips, the negative
//! path matrix, canonical collisions (streaming detection), link and
//! special-entry rejection, budgets, and archive-end verification
//! (IO-IMP-10).

mod common;

use std::io::Cursor;

use common::extract::{TestDir, exists, read_staged};
use common::tar::{HeaderSpec, entry_with, ext_entry, file_entry, pax_record, tar, writer_tar};
use ruviform_io_archive::extract::{ExtractOptions, ExtractionSession};
use ruviform_io_archive::tar::TarArchiveReader;
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
    (Cursor<Vec<u8>>, ruviform_io_archive::extract::ExtractedTree),
    ruviform_io_archive::extract::ExtractionFailure,
> {
    let archive = TarArchiveReader::new(Cursor::new(bytes), ArchiveReadLimits::default());
    ExtractionSession::create(parent, opts)
        .expect("session")
        .extract_tar(archive)
}

fn dir_entry(name: &str) -> Vec<u8> {
    entry_with(&HeaderSpec::new(b'5', name.as_bytes(), 0), b"")
}

fn link_entry(typeflag: u8, name: &str, link: &str) -> Vec<u8> {
    let mut spec = HeaderSpec::new(typeflag, name.as_bytes(), 0);
    spec.linkname = link.as_bytes();
    entry_with(&spec, b"")
}

// --- round trips ---------------------------------------------------

#[test]
fn extracts_files_and_directories_into_staging() {
    let dir = TestDir::new("tar-rt");
    let bytes = writer_tar(&[
        ("a.txt", b"alpha"),
        ("d/e.bin", b"beta-beta"),
        ("empty", b""),
    ]);
    let (inner, tree) = extract(bytes, dir.path(), options()).expect("extract");
    assert_eq!(inner.position(), inner.get_ref().len() as u64);
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
    assert_eq!(tree.receipt().entries, 3);
    tree.cleanup().expect("cleanup");
    assert!(!exists(&root));
}

#[test]
fn explicit_directory_entries_are_created() {
    let dir = TestDir::new("tar-dirs");
    let bytes = tar(&[
        dir_entry("top/"),
        dir_entry("top/inner/"),
        file_entry("top/inner/f.txt", b"x"),
    ]);
    let (_inner, tree) = extract(bytes, dir.path(), options()).expect("extract");
    assert!(tree.path().join("top").join("inner").is_dir());
    assert_eq!(tree.receipt().directories, 2);
    tree.cleanup().expect("cleanup");
}

// --- negative path matrix -------------------------------------------

#[test]
fn rejects_dangerous_and_divergent_names() {
    for name in [
        &b"../evil"[..],
        b"a/../../b",
        b"a//b",
        b"/abs",
        b"C:/win",
        b"C:\\win",
        b"\\\\srv\\share\\x",
        b"a\\b",
        b"a:b",
        b"con.txt",
        b"NUL",
        b"a.",
        b"a ",
        b"..",
        b".",
        b"a/./b",
        &[0xFF, 0xFE][..],
    ] {
        let dir = TestDir::new("tar-neg");
        let bytes = tar(&[entry_with(&HeaderSpec::new(b'0', name, 1), b"x")]);
        match extract(bytes, dir.path(), options()) {
            Err(f) => {
                assert_eq!(
                    f.error().kind(),
                    ArchiveErrorKind::InvalidInput,
                    "name {name:?}: wrong kind"
                );
                assert!(!exists(&f.staging_root), "name {name:?}: staging leaked");
            }
            Ok((_i, tree)) => {
                panic!("name {name:?} was extracted to {}", tree.path().display())
            }
        }
    }
}

// --- canonical collisions (streaming) ---------------------------------

#[test]
fn rejects_exact_duplicate_paths() {
    let dir = TestDir::new("tar-dup");
    let bytes = tar(&[file_entry("a.txt", b"1"), file_entry("a.txt", b"2")]);
    let f = extract(bytes, dir.path(), options()).expect_err("must fail");
    assert_eq!(f.error().kind(), ArchiveErrorKind::InvalidInput);
    assert!(!exists(&f.staging_root));
}

#[test]
fn rejects_case_fold_and_unicode_collisions() {
    let dir = TestDir::new("tar-fold");
    let bytes = tar(&[file_entry("Foo.txt", b"1"), file_entry("foo.txt", b"2")]);
    let f = extract(bytes, dir.path(), options()).expect_err("must fail");
    assert_eq!(f.error().kind(), ArchiveErrorKind::InvalidInput);

    let dir2 = TestDir::new("tar-unicode");
    let bytes2 = tar(&[file_entry("é.txt", b"1"), file_entry("e\u{0301}.txt", b"2")]);
    let f2 = extract(bytes2, dir2.path(), options()).expect_err("must fail");
    assert_eq!(f2.error().kind(), ArchiveErrorKind::InvalidInput);
}

#[test]
fn rejects_file_versus_directory_conflicts_in_stream_order() {
    // File `a` then `a/b` — detected when `a/b` surfaces.
    let dir = TestDir::new("tar-fd1");
    let bytes = tar(&[file_entry("a", b"1"), file_entry("a/b", b"2")]);
    let f = extract(bytes, dir.path(), options()).expect_err("must fail");
    assert_eq!(f.error().kind(), ArchiveErrorKind::InvalidInput);
    assert!(!exists(&f.staging_root));

    // `a/b` then file `a` — implicit dir `a` vs file `a`.
    let dir2 = TestDir::new("tar-fd2");
    let bytes2 = tar(&[file_entry("a/b", b"2"), file_entry("a", b"1")]);
    let f2 = extract(bytes2, dir2.path(), options()).expect_err("must fail");
    assert_eq!(f2.error().kind(), ArchiveErrorKind::InvalidInput);
}

// --- link / special rejection ------------------------------------------

#[test]
fn rejects_symlink_and_hardlink_entries() {
    let dir = TestDir::new("tar-links");
    let bytes = tar(&[link_entry(b'2', "link", "/etc/passwd")]);
    let f = extract(bytes, dir.path(), options()).expect_err("must fail");
    assert_eq!(f.error().kind(), ArchiveErrorKind::InvalidInput);
    assert!(!exists(&f.staging_root));

    let dir2 = TestDir::new("tar-hard");
    let bytes2 = tar(&[
        file_entry("orig", b"data"),
        link_entry(b'1', "hard", "orig"),
    ]);
    let f2 = extract(bytes2, dir2.path(), options()).expect_err("must fail");
    assert_eq!(f2.error().kind(), ArchiveErrorKind::InvalidInput);
    assert!(!exists(&f2.staging_root));
}

#[test]
fn special_typeflags_are_rejected_by_the_reader() {
    // The TAR reader surfaces `Unsupported` for fifo/char/block
    // typeflags before extraction sees them — the secure default is
    // preserved end-to-end.
    let dir = TestDir::new("tar-fifo");
    let bytes = tar(&[entry_with(&HeaderSpec::new(b'6', b"pipe", 0), b"")]);
    let f = extract(bytes, dir.path(), options()).expect_err("must fail");
    assert_eq!(f.error().kind(), ArchiveErrorKind::Unsupported);
}

// --- budgets ---------------------------------------------------------

#[test]
fn enforces_entry_count_and_depth_budgets() {
    let dir = TestDir::new("tar-count");
    let bytes = tar(&[file_entry("a", b"1"), file_entry("b", b"2")]);
    let f = extract(bytes, dir.path(), options_with(|l| l.max_entries = 1)).expect_err("must fail");
    assert_eq!(f.error().kind(), ArchiveErrorKind::LimitExceeded);

    let dir2 = TestDir::new("tar-depth");
    let bytes2 = tar(&[file_entry("a/b/c/d.txt", b"x")]);
    let f2 = extract(bytes2, dir2.path(), options_with(|l| l.max_path_depth = 2))
        .expect_err("must fail");
    assert_eq!(f2.error().kind(), ArchiveErrorKind::LimitExceeded);
}

#[test]
fn enforces_entry_and_total_size_budgets_before_write() {
    let dir = TestDir::new("tar-esize");
    let bytes = tar(&[file_entry("big", b"0123456789")]);
    let f = extract(
        bytes,
        dir.path(),
        options_with(|l| l.max_entry_uncompressed_bytes = 4),
    )
    .expect_err("must fail");
    assert_eq!(f.error().kind(), ArchiveErrorKind::LimitExceeded);
    assert!(!exists(&f.staging_root));

    let dir2 = TestDir::new("tar-total");
    let bytes2 = tar(&[file_entry("a", b"12345"), file_entry("b", b"12345")]);
    let f2 = extract(
        bytes2,
        dir2.path(),
        options_with(|l| l.max_total_uncompressed_bytes = 8),
    )
    .expect_err("must fail");
    assert_eq!(f2.error().kind(), ArchiveErrorKind::LimitExceeded);
}

// --- mid-extraction failure cleanup -----------------------------------

#[test]
fn cleans_the_partial_tree_when_a_later_entry_fails() {
    let dir = TestDir::new("tar-fail");
    // First file stages, second entry collides → partial cleanup.
    let bytes = tar(&[file_entry("a.txt", b"1"), file_entry("a.txt", b"2")]);
    let f = extract(bytes, dir.path(), options()).expect_err("must fail");
    assert!(!exists(&f.staging_root));
    assert_eq!(dir.children(), Vec::<std::path::PathBuf>::new());
}

#[test]
fn a_truncated_archive_end_fails_instead_of_handing_out_a_tree() {
    let dir = TestDir::new("tar-trunc");
    // Entries but no two-zero end blocks → finish() reports truncation.
    let bytes = file_entry("a.txt", b"data");
    let f = extract(bytes, dir.path(), options()).expect_err("must fail");
    assert_eq!(f.error().kind(), ArchiveErrorKind::Truncated);
    assert!(!exists(&f.staging_root));
}

// --- extension-provided effective names -------------------------------

#[test]
fn pax_path_override_extracts_under_the_effective_name() {
    // The ustar header name is fully replaced by the PAX `path`
    // record — extraction must use the effective name everywhere.
    let dir = TestDir::new("tar-pax-path");
    let records = pax_record(b"path", b"sub/dir/real.txt");
    let bytes = tar(&[
        ext_entry(b'x', "PaxHeader", &records),
        file_entry("hdr-name.txt", b"payload"),
    ]);
    let (_inner, tree) = extract(bytes, dir.path(), options()).expect("extract");
    let root = tree.path().to_path_buf();
    assert_eq!(
        read_staged(&root.join("sub").join("dir").join("real.txt")).as_deref(),
        Some(b"payload".as_slice())
    );
    assert!(!exists(&root.join("hdr-name.txt")));
    assert_eq!(tree.receipt().unverified_entries, 0);
    tree.cleanup().expect("cleanup");
}

#[test]
fn pax_path_override_still_passes_path_policy() {
    // A hostile `path` record is the *effective* name — it must hit
    // the same SafeRelativePath policy as a plain header name.
    let dir = TestDir::new("tar-pax-evil");
    let records = pax_record(b"path", b"../evil.txt");
    let bytes = tar(&[
        ext_entry(b'x', "PaxHeader", &records),
        file_entry("innocent.txt", b"x"),
    ]);
    let f = extract(bytes, dir.path(), options()).expect_err("must fail");
    assert_eq!(f.error().kind(), ArchiveErrorKind::InvalidInput);
    assert!(!exists(&f.staging_root));
}
