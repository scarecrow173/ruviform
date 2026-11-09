//! `RootedDir` boundary over real published and extracted trees:
//! no-follow reads, directory enumeration, locator grammar refusal,
//! and link-escape rejection.
//!
//! Slice-2 rule F10
//! (`docs/test-specs/issue-306-io-e2e-fault-fuzz.md`). Link fixtures
//! use the same primitives as `ruviform-io-file/tests/common`
//! (unix symlinks; Windows junctions/`symlink_file`); when the
//! platform cannot create a link the affected assertion is skipped
//! with an `eprintln!` record — never a fabricated pass.

mod common;

use std::io::{Cursor, Read, Write};

use common::{TestDir, dir_link, exists, file_link};
use ruviform_io_archive::ArchiveReadLimits;
use ruviform_io_archive::extract::{ExtractOptions, ExtractionSession};
use ruviform_io_archive::tar::{TarArchiveReader, TarArchiveWriter, TarEntryOptions};
use ruviform_io_file::rooted::{FollowMode, NodeKind, RelativePath, RootedDir};
use ruviform_io_file::{Durability, FileErrorKind, PublishMode, PublishOptions, StagedFile};

fn publish_options() -> PublishOptions {
    PublishOptions::new(PublishMode::CreateNew, Durability::FileSync)
}

fn rel(locator: &str) -> RelativePath {
    RelativePath::new(locator).expect("valid locator")
}

#[test]
fn rooted_dir_reads_published_tree_no_follow() {
    let dir = TestDir::new("rooted-pub");
    // A published file at the root plus a nested regular file —
    // both produced through ordinary filesystem/publish paths.
    let mut staged = StagedFile::create(dir.dest("doc.json")).expect("stage");
    staged.write_all(br#"{"k":1}"#).expect("write");
    staged.seal().publish(publish_options()).expect("publish");
    std::fs::create_dir(dir.dest("sub")).expect("subdir");
    std::fs::write(dir.dest("sub/hello.txt"), b"hello").expect("nested file");

    let root = RootedDir::open(dir.path()).expect("open rooted dir");

    // Verified read of a published file.
    let doc = rel("doc.json");
    let mut file = root
        .open_regular(&doc, FollowMode::NoFollow)
        .expect("open published file");
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).expect("read");
    assert_eq!(buf, br#"{"k":1}"#);

    // Facts of the resolved object.
    let info = root.metadata(&doc, FollowMode::NoFollow).expect("metadata");
    assert_eq!(info.kind, NodeKind::File);
    assert_eq!(info.leaf_kind, NodeKind::File);

    // Nested path resolves component-wise.
    let nested = rel("sub/hello.txt");
    let mut file = root
        .open_regular(&nested, FollowMode::NoFollow)
        .expect("open nested");
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).expect("read");
    assert_eq!(buf, b"hello");

    // Enumeration sees both entries with no-follow kinds.
    let entries = root
        .read_dir(&rel(""), FollowMode::NoFollow)
        .expect("read_dir root");
    let names: Vec<_> = entries
        .iter()
        .map(|e| e.name.to_string_lossy().into_owned())
        .collect();
    assert!(names.iter().any(|n| n == "doc.json"), "entries: {names:?}");
    assert!(names.iter().any(|n| n == "sub"), "entries: {names:?}");
    let sub = entries
        .iter()
        .find(|e| e.name.to_string_lossy() == "sub")
        .expect("sub entry");
    assert_eq!(sub.kind, NodeKind::Dir);

    // Missing leaf is a typed NotFound, not a panic.
    let missing = root
        .open_regular(&rel("nope.txt"), FollowMode::NoFollow)
        .expect_err("missing file must fail");
    assert_eq!(missing.kind(), FileErrorKind::NotFound);
}

#[test]
fn rooted_locator_rejects_traversal_and_unsafe_forms() {
    // Grammar is validated at `RelativePath::new` — no filesystem
    // needed.
    for bad in [
        "../escape",
        "a/../../b",
        "..",
        "/etc/passwd",
        "C:/windows/x",
        "a//b",
        "a\\b",
        "a/./b",
        ".hidden/../x",
    ] {
        let error = RelativePath::new(bad)
            .err()
            .unwrap_or_else(|| panic!("locator {bad:?} must be rejected"));
        assert_eq!(
            error.kind(),
            FileErrorKind::InvalidInput,
            "locator {bad:?} must classify as InvalidInput"
        );
    }
}

#[test]
fn rooted_dir_link_escape_is_refused() {
    let dir = TestDir::new("rooted-link");
    let outside = TestDir::new("rooted-link-out");
    std::fs::write(outside.path().join("secret.txt"), b"secret").expect("outside file");
    std::fs::create_dir(outside.path().join("sub")).expect("outside dir");
    let root = RootedDir::open(dir.path()).expect("open rooted dir");

    // Directory link (junction on Windows — unprivileged) pointing
    // outside the capability root.
    match dir_link(outside.path(), &dir.dest("escape")) {
        Ok(()) => {
            // NoFollow: the link itself is visible as Symlink and is
            // never traversed.
            let link = rel("escape");
            let info = root
                .metadata(&link, FollowMode::NoFollow)
                .expect("link metadata");
            assert_eq!(
                info.kind,
                NodeKind::Symlink,
                "name-surrogate link must classify as Symlink"
            );

            // WithinRoot: resolving through the link must refuse —
            // the target lives outside the capability root.
            let through = rel("escape/secret.txt");
            let error = root
                .open_regular(&through, FollowMode::WithinRoot)
                .expect_err("escape through link must fail");
            assert_eq!(
                error.kind(),
                FileErrorKind::EscapesRoot,
                "link to outside must be refused as EscapesRoot"
            );

            // NoFollow on the nested path: an intermediate link is
            // not traversable.
            let error = root
                .open_regular(&through, FollowMode::NoFollow)
                .expect_err("NoFollow must not traverse the link");
            assert!(
                matches!(
                    error.kind(),
                    FileErrorKind::NotDirectory | FileErrorKind::NotRegularFile
                ),
                "unexpected NoFollow kind: {}",
                error.kind()
            );
        }
        Err(error) => {
            eprintln!("skipping dir-link escape asserts: cannot create link ({error})");
        }
    }

    // File link (unix symlink / Windows symlink_file — may need
    // SeCreateSymbolicLinkPrivilege).
    let outside_file = outside.path().join("secret.txt");
    match file_link(&outside_file, &dir.dest("filelink")) {
        Ok(()) => {
            let error = root
                .open_regular(&rel("filelink"), FollowMode::WithinRoot)
                .expect_err("file-link escape must fail");
            assert_eq!(error.kind(), FileErrorKind::EscapesRoot);
            // Under NoFollow the leaf link is not a regular file.
            let error = root
                .open_regular(&rel("filelink"), FollowMode::NoFollow)
                .expect_err("NoFollow must not open the link");
            assert_eq!(
                error.kind(),
                FileErrorKind::NotRegularFile,
                "link leaf under NoFollow must be NotRegularFile"
            );
        }
        Err(error) => {
            eprintln!("skipping file-link escape asserts: cannot create link ({error})");
        }
    }
}

#[test]
fn rooted_dir_reads_extraction_tree() {
    let dir = TestDir::new("rooted-extract");

    // Produce a verified tree through ExtractionSession, then read
    // it back through the rooted capability while the tree lives.
    let mut body = Vec::new();
    {
        let mut writer = TarArchiveWriter::new(&mut body);
        let mut entry = writer
            .start_entry("sub/hello.txt", 5, &TarEntryOptions::default())
            .expect("entry");
        entry.write_all(b"hello").expect("payload");
        entry.finish().expect("entry finish");
        writer.finish().expect("tar finish");
    }
    let parent = dir.dest("stage-parent");
    std::fs::create_dir(&parent).expect("staging parent");
    let session = ExtractionSession::create(&parent, ExtractOptions::default()).expect("session");
    let reader = TarArchiveReader::new(Cursor::new(body), ArchiveReadLimits::default());
    let (_inner, tree) = session.extract_tar(reader).expect("extract");
    assert_eq!(tree.receipt().files, 1);

    {
        // Scoped: open handles must be released before cleanup — on
        // Windows an open directory handle leaves the path delete-
        // pending, which still resolves via try_exists.
        let root = RootedDir::open(tree.path()).expect("open staging root");
        let mut file = root
            .open_regular(&rel("sub/hello.txt"), FollowMode::NoFollow)
            .expect("open extracted file");
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).expect("read");
        assert_eq!(buf, b"hello");

        // Enumeration of the extracted root sees the directory entry.
        let entries = root
            .read_dir(&rel(""), FollowMode::NoFollow)
            .expect("read_dir");
        assert!(
            entries
                .iter()
                .any(|e| e.name.to_string_lossy() == "sub" && e.kind == NodeKind::Dir),
            "extracted dir must enumerate"
        );
    }

    let staging = tree.path().to_path_buf();
    tree.cleanup().expect("cleanup");
    assert!(!exists(&staging));
}
