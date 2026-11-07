//! Real-filesystem integration tests for the rooted read
//! capability: traversal, link modes, escapes, swaps, identity,
//! nonregular leaves, and scoped browsing against actual OS
//! backends.
//!
//! Link-dependent cases use `dir_link`/`file_link` from `common`
//! (unix symlinks; Windows junctions/file links — creation may fail
//! without privilege and is then skipped honestly).

mod common;

use common::{TestDir, canonical_link_target, dir_link, file_link, write_file};

use std::ffi::OsStr;
use std::io::Read;
use std::path::{Path, PathBuf};

use ruviform_io_file::FileErrorKind;
use ruviform_io_file::rooted::{
    EntryFlow, EnumerationOutcome, FollowMode, NodeKind, RelativePath, RootedDir,
};

fn rel(s: &str) -> RelativePath {
    RelativePath::new(s).unwrap()
}

fn root_of(dir: &TestDir) -> RootedDir {
    RootedDir::open(dir.path()).expect("RootedDir::open")
}

fn read_all(file: &mut ruviform_io_file::rooted::RootedFile) -> String {
    let mut s = String::new();
    file.read_to_string(&mut s).expect("read");
    s
}

/// Builds a tree:
/// `<root>/d/f` = "deep", `<root>/f` = "top", `<root>/d/sub` = dir.
fn tree(dir: &TestDir) {
    std::fs::create_dir(dir.path().join("d")).unwrap();
    std::fs::create_dir(dir.path().join("d").join("sub")).unwrap();
    write_file(&dir.path().join("f"), b"top");
    write_file(&dir.path().join("d").join("f"), b"deep");
    write_file(&dir.path().join("d").join("sub").join("g"), b"subg");
}

// ----------------------------------------------------------------
// Basic traversal
// ----------------------------------------------------------------

#[test]
fn opens_and_enumerates_a_real_tree() {
    let dir = TestDir::new("basic");
    tree(&dir);
    let root = root_of(&dir);
    let mut f = root
        .open_regular(&rel("d/f"), FollowMode::NoFollow)
        .unwrap();
    assert_eq!(read_all(&mut f), "deep");
    let entries = root.read_dir(&rel(""), FollowMode::NoFollow).unwrap();
    let mut names: Vec<_> = entries.iter().map(|e| e.name.clone()).collect();
    names.sort();
    assert_eq!(
        names,
        vec![
            OsStr::new("d").to_os_string(),
            OsStr::new("f").to_os_string()
        ]
    );
    let info = root.metadata(&rel("d"), FollowMode::NoFollow).unwrap();
    assert_eq!(info.kind, NodeKind::Dir);
    let self_info = root.metadata(&rel(""), FollowMode::NoFollow).unwrap();
    assert_eq!(self_info.kind, NodeKind::Dir);
    assert_eq!(self_info.identity, root.dir_identity().unwrap());
}

#[test]
fn failure_classifications_on_real_fs() {
    let dir = TestDir::new("fails");
    tree(&dir);
    let root = root_of(&dir);
    assert_eq!(
        root.open_regular(&rel("missing"), FollowMode::NoFollow)
            .unwrap_err()
            .kind(),
        FileErrorKind::NotFound
    );
    assert_eq!(
        root.open_regular(&rel("d"), FollowMode::NoFollow)
            .unwrap_err()
            .kind(),
        FileErrorKind::NotRegularFile
    );
    assert_eq!(
        root.open_dir(&rel("f"), FollowMode::NoFollow)
            .unwrap_err()
            .kind(),
        FileErrorKind::NotDirectory
    );
    assert_eq!(
        root.metadata(&rel("f/g"), FollowMode::NoFollow)
            .unwrap_err()
            .kind(),
        FileErrorKind::NotDirectory
    );
    assert_eq!(
        RootedDir::open(&dir.path().join("f")).unwrap_err().kind(),
        FileErrorKind::NotDirectory
    );
}

// ----------------------------------------------------------------
// Leaf link cases
// ----------------------------------------------------------------

#[test]
fn leaf_link_nofollow_vs_withinroot() {
    let dir = TestDir::new("leaflink");
    tree(&dir);
    let root = root_of(&dir);
    // In-root absolute target spelled via the canonical root (the
    // contract's lexical anchor) — `dir.path()` is a non-canonical
    // alias on macOS/Windows and would conservatively `EscapesRoot`.
    if file_link(
        &canonical_link_target(root.canonical_root(), "f"),
        &dir.path().join("l"),
    )
    .is_err()
    {
        eprintln!("skipping: cannot create a file link on this host");
        return;
    }
    let info = root.metadata(&rel("l"), FollowMode::NoFollow).unwrap();
    assert_eq!(info.kind, NodeKind::Symlink);
    assert_eq!(info.leaf_kind, NodeKind::Symlink);
    assert_eq!(
        root.open_regular(&rel("l"), FollowMode::NoFollow)
            .unwrap_err()
            .kind(),
        FileErrorKind::NotRegularFile
    );
    let mut f = root
        .open_regular(&rel("l"), FollowMode::WithinRoot)
        .unwrap();
    assert_eq!(read_all(&mut f), "top");
    assert_eq!(f.info().leaf_kind, NodeKind::Symlink);
}

#[test]
fn leaf_link_absolute_outside_escapes() {
    let dir = TestDir::new("abs-out");
    let outside = TestDir::new("abs-out-side");
    tree(&dir);
    write_file(&outside.path().join("secret"), b"SECRET");
    // Absolute link to an outside object.
    if file_link(&outside.path().join("secret"), &dir.path().join("l")).is_err() {
        eprintln!("skipping: cannot create a file link on this host");
        return;
    }
    let root = root_of(&dir);
    assert_eq!(
        root.open_regular(&rel("l"), FollowMode::WithinRoot)
            .unwrap_err()
            .kind(),
        FileErrorKind::EscapesRoot
    );
    assert_eq!(
        read_std(&outside.path().join("secret")),
        "SECRET",
        "sanity: outside object still exists"
    );
}

#[test]
fn leaf_link_absolute_inside_resolves() {
    let dir = TestDir::new("abs-in");
    tree(&dir);
    let root = root_of(&dir);
    // An absolute link into the tree, built from the *canonical*
    // root so the TEXTUAL prefix check matches exactly.
    let target = root.canonical_root().join("d").join("f");
    if file_link(&target, &dir.path().join("l")).is_err() {
        eprintln!("skipping: cannot create a file link on this host");
        return;
    }
    let mut f = root
        .open_regular(&rel("l"), FollowMode::WithinRoot)
        .unwrap();
    assert_eq!(read_all(&mut f), "deep");
}

// The contract's documented narrowing: an absolute target spelled
// through a *non-canonical alias* of the root (here a second link
// into it — the class that bit the original tests via `/var` vs
// `/private/var` and 8.3 temp names) kernel-resolves inside the
// root but fails the lexical prefix check, so it conservatively
// `EscapesRoot`s. Both links are dir links (junctions — no
// privilege needed on Windows).
#[test]
fn absolute_link_alias_spelling_escapes_root() {
    let dir = TestDir::new("alias-out");
    let outside = TestDir::new("alias-out-side");
    tree(&dir);
    let root = root_of(&dir);
    // `outside/alias` is a non-canonical path spelling of the root.
    if dir_link(
        &canonical_link_target(root.canonical_root(), ""),
        &outside.path().join("alias"),
    )
    .is_err()
    {
        eprintln!("skipping: cannot create a dir link on this host");
        return;
    }
    // `<outside>/alias/d` kernel-resolves to `<root>/d` — inside —
    // yet the stored text never prefixes the canonical root.
    if dir_link(
        &outside.path().join("alias").join("d"),
        &dir.path().join("l"),
    )
    .is_err()
    {
        eprintln!("skipping: cannot create a dir link on this host");
        return;
    }
    assert_eq!(
        root.open_regular(&rel("l/f"), FollowMode::WithinRoot)
            .unwrap_err()
            .kind(),
        FileErrorKind::EscapesRoot
    );
    assert_eq!(
        root.metadata(&rel("l"), FollowMode::NoFollow).unwrap().kind,
        NodeKind::Symlink
    );
}

#[test]
fn leaf_link_swap_never_reads_outside() {
    let dir = TestDir::new("leafswap");
    let outside = TestDir::new("leafswap-out");
    tree(&dir);
    write_file(&outside.path().join("secret"), b"SECRET");
    let root = root_of(&dir);
    if file_link(
        &canonical_link_target(root.canonical_root(), "f"),
        &dir.path().join("l"),
    )
    .is_err()
    {
        eprintln!("skipping: cannot create a file link on this host");
        return;
    }
    // Read once for sanity, then swap the link at an outside object.
    let mut f = root
        .open_regular(&rel("l"), FollowMode::WithinRoot)
        .unwrap();
    assert_eq!(read_all(&mut f), "top");
    drop(f);
    std::fs::remove_file(dir.path().join("l")).unwrap();
    file_link(&outside.path().join("secret"), &dir.path().join("l")).unwrap();
    let e = root
        .open_regular(&rel("l"), FollowMode::WithinRoot)
        .unwrap_err();
    assert_eq!(e.kind(), FileErrorKind::EscapesRoot);
    // NoFollow still reports the link itself.
    let info = root.metadata(&rel("l"), FollowMode::NoFollow).unwrap();
    assert_eq!(info.kind, NodeKind::Symlink);
}

// ----------------------------------------------------------------
// Intermediate links / scoped `..`
// ----------------------------------------------------------------

#[test]
fn intermediate_link_descend_withinroot() {
    let dir = TestDir::new("midlink");
    tree(&dir);
    let root = root_of(&dir);
    // Same canonical-anchor rule as the leaf cases: the junction
    // target must be spelled as the canonical root or the TEXTUAL
    // prefix check (correctly, conservatively) escapes.
    if dir_link(
        &canonical_link_target(root.canonical_root(), "d"),
        &dir.path().join("ld"),
    )
    .is_err()
    {
        eprintln!("skipping: cannot create a dir link on this host");
        return;
    }
    let mut f = root
        .open_regular(&rel("ld/sub/g"), FollowMode::WithinRoot)
        .unwrap();
    assert_eq!(read_all(&mut f), "subg");
    // Same path under NoFollow: the link cannot be traversed.
    assert_eq!(
        root.open_regular(&rel("ld/sub/g"), FollowMode::NoFollow)
            .unwrap_err()
            .kind(),
        FileErrorKind::NotDirectory
    );
}

#[test]
fn intermediate_link_to_outside_escapes() {
    let dir = TestDir::new("mid-out");
    let outside = TestDir::new("mid-out-side");
    tree(&dir);
    write_file(&outside.path().join("secret"), b"SECRET");
    if dir_link(outside.path(), &dir.path().join("ld")).is_err() {
        eprintln!("skipping: cannot create a dir link on this host");
        return;
    }
    let root = root_of(&dir);
    assert_eq!(
        root.open_regular(&rel("ld/secret"), FollowMode::WithinRoot)
            .unwrap_err()
            .kind(),
        FileErrorKind::EscapesRoot
    );
}

#[test]
fn scoped_dir_dotdot_link_target() {
    let dir = TestDir::new("scoped-dd");
    tree(&dir);
    if file_link(Path::new("../f"), &dir.path().join("d").join("l")).is_err() {
        eprintln!("skipping: cannot create a file link on this host");
        return;
    }
    let root = root_of(&dir);
    let scoped = root.open_dir(&rel("d"), FollowMode::NoFollow).unwrap();
    let mut f = scoped
        .open_regular(&rel("l"), FollowMode::WithinRoot)
        .unwrap();
    assert_eq!(read_all(&mut f), "top");
    assert_eq!(scoped.canonical_root(), root.canonical_root());
}

#[test]
fn link_target_dotdot_below_floor_escapes() {
    let dir = TestDir::new("dd-floor");
    tree(&dir);
    // `..` twice from `d/` pops below the capability root.
    if file_link(
        Path::new("../../outside/f"),
        &dir.path().join("d").join("l"),
    )
    .is_err()
    {
        eprintln!("skipping: cannot create a file link on this host");
        return;
    }
    let root = root_of(&dir);
    assert_eq!(
        root.open_regular(&rel("d/l"), FollowMode::WithinRoot)
            .unwrap_err()
            .kind(),
        FileErrorKind::EscapesRoot
    );
}

// unix `symlink(2)` stores the raw target string including `..`;
// Windows normalizes the stored substitute name at link creation
// (`CreateSymbolicLinkW`/`mklink`), so the out-and-back shape cannot
// be constructed there — the resolve rule itself is covered by the
// mock-driven unit test `out_and_back_target_is_tainted_escapes_root`.
#[cfg(unix)]
#[test]
fn absolute_link_out_and_back_is_conservatively_rejected() {
    let dir = TestDir::new("outback");
    let outside = TestDir::new("outback-out");
    tree(&dir);
    let root = root_of(&dir);
    // <outside>/../<basename-of-canonical-root>/d/f — leaves the
    // prefix, returns textually: conservative EscapesRoot.
    let canon = root.canonical_root().to_path_buf();
    let Some(base) = canon.file_name() else {
        eprintln!("skipping: canonical root has no leaf name");
        return;
    };
    let target = outside.path().join("..").join(base).join("d").join("f");
    if file_link(&target, &dir.path().join("l")).is_err() {
        eprintln!("skipping: cannot create a file link on this host");
        return;
    }
    assert_eq!(
        root.open_regular(&rel("l"), FollowMode::WithinRoot)
            .unwrap_err()
            .kind(),
        FileErrorKind::EscapesRoot
    );
}

// ----------------------------------------------------------------
// Nonregular leaves
// ----------------------------------------------------------------

#[cfg(unix)]
#[test]
fn fifo_leaf_is_not_regular_file() {
    let dir = TestDir::new("fifo");
    tree(&dir);
    common::make_fifo(&dir.path().join("pipe")).expect("mkfifo");
    let root = root_of(&dir);
    assert_eq!(
        root.open_regular(&rel("pipe"), FollowMode::NoFollow)
            .unwrap_err()
            .kind(),
        FileErrorKind::NotRegularFile
    );
    let info = root.metadata(&rel("pipe"), FollowMode::NoFollow).unwrap();
    assert_eq!(info.kind, NodeKind::Other);
}

// ----------------------------------------------------------------
// pin / open_pinned identity
// ----------------------------------------------------------------

#[test]
fn pin_open_pinned_roundtrip_and_recreate_mismatch() {
    let dir = TestDir::new("pin");
    tree(&dir);
    let root = root_of(&dir);
    let pin = match root.pin(&rel("d/f"), FollowMode::NoFollow) {
        Ok(pin) => pin,
        Err(e) if e.kind() == FileErrorKind::Unsupported => {
            eprintln!("skipping: filesystem reports weak identity");
            return;
        }
        Err(e) => panic!("pin failed: {e}"),
    };
    let mut f = root
        .open_pinned(&rel("d/f"), &pin, FollowMode::NoFollow)
        .unwrap();
    assert_eq!(read_all(&mut f), "deep");
    // delete+recreate: the new object's identity must differ.
    std::fs::remove_file(dir.path().join("d").join("f")).unwrap();
    write_file(&dir.path().join("d").join("f"), b"deep");
    let e = root
        .open_pinned(&rel("d/f"), &pin, FollowMode::NoFollow)
        .unwrap_err();
    assert_eq!(e.kind(), FileErrorKind::IdentityMismatch);
    // Sanity: the recreated file reads through an unpinned open.
    let mut f2 = root
        .open_regular(&rel("d/f"), FollowMode::NoFollow)
        .unwrap();
    assert_eq!(read_all(&mut f2), "deep");
}

// ----------------------------------------------------------------
// Best-effort swap tolerance: repeated replace+read races must
// never produce outside bytes — only clean success or a typed
// error.
// ----------------------------------------------------------------

#[test]
fn concurrent_leaf_swap_tolerance() {
    let dir = TestDir::new("race");
    let outside = TestDir::new("race-out");
    tree(&dir);
    write_file(&outside.path().join("secret"), b"SECRET");
    if file_link(&outside.path().join("secret"), &outside.path().join("l")).is_err() {
        eprintln!("skipping: cannot create a file link on this host");
        return;
    }
    let root = root_of(&dir);
    let leaf = dir.path().join("victim");
    let swap_link = outside.path().join("l");
    // Flip the leaf between an in-root file and an outside link in
    // this thread while reading between swaps — the assertions are
    // only that no call returns the outside bytes.
    for i in 0..8 {
        write_file(&leaf, format!("safe-{i}").as_bytes());
        match root.open_regular(&rel("victim"), FollowMode::WithinRoot) {
            Ok(mut f) => {
                let s = read_all(&mut f);
                assert_ne!(s, "SECRET", "outside bytes surfaced");
            }
            Err(e) => {
                assert!(
                    matches!(
                        e.kind(),
                        FileErrorKind::NotFound
                            | FileErrorKind::EscapesRoot
                            | FileErrorKind::NotRegularFile
                            | FileErrorKind::IdentityMismatch
                            | FileErrorKind::NotDirectory
                            | FileErrorKind::Io
                    ),
                    "unexpected kind {}",
                    e.kind()
                );
            }
        }
        std::fs::remove_file(&leaf).unwrap();
        file_link(&swap_link, &leaf).expect("swap link");
        match root.open_regular(&rel("victim"), FollowMode::WithinRoot) {
            Ok(mut f) => {
                let s = read_all(&mut f);
                assert_ne!(s, "SECRET", "outside bytes surfaced");
            }
            Err(e) => {
                assert!(
                    matches!(
                        e.kind(),
                        FileErrorKind::NotFound
                            | FileErrorKind::EscapesRoot
                            | FileErrorKind::NotRegularFile
                            | FileErrorKind::IdentityMismatch
                            | FileErrorKind::NotDirectory
                            | FileErrorKind::Io
                    ),
                    "unexpected kind {}",
                    e.kind()
                );
            }
        }
        std::fs::remove_file(&leaf).unwrap();
    }
}

fn read_std(path: &PathBuf) -> String {
    String::from_utf8_lossy(&std::fs::read(path).unwrap_or_default()).into_owned()
}

// ----------------------------------------------------------------
// Windows junction specifics — name-surrogate reparse points are
// `Symlink`; targets arrive in `\??\` NT form (bridge covered by
// the absolute-inside case).
// ----------------------------------------------------------------

#[cfg(windows)]
#[test]
fn junction_absolute_inside_resolves_via_nt_bridge() {
    let dir = TestDir::new("junction-in");
    tree(&dir);
    let root = root_of(&dir);
    // Junction targets are stored as \??\<path> — the bridge maps it
    // onto the canonical (verbatim-stripped) component list.
    if dir_link(
        &canonical_link_target(root.canonical_root(), "d"),
        &dir.path().join("lj"),
    )
    .is_err()
    {
        eprintln!("skipping: cannot create a junction on this host");
        return;
    }
    let mut f = root
        .open_regular(&rel("lj/f"), FollowMode::WithinRoot)
        .unwrap();
    assert_eq!(read_all(&mut f), "deep");
    let info = root.metadata(&rel("lj"), FollowMode::NoFollow).unwrap();
    assert_eq!(info.kind, NodeKind::Symlink);
}

#[cfg(windows)]
#[test]
fn junction_to_outside_escapes() {
    let dir = TestDir::new("junction-out");
    let outside = TestDir::new("junction-out-side");
    tree(&dir);
    write_file(&outside.path().join("secret"), b"SECRET");
    if dir_link(outside.path(), &dir.path().join("lj")).is_err() {
        eprintln!("skipping: cannot create a junction on this host");
        return;
    }
    let root = root_of(&dir);
    assert_eq!(
        root.open_regular(&rel("lj/secret"), FollowMode::WithinRoot)
            .unwrap_err()
            .kind(),
        FileErrorKind::EscapesRoot
    );
}

// ----------------------------------------------------------------
// open_scoped_root — re-rooting on the held handle (Issue #409)
// ----------------------------------------------------------------

#[test]
fn scoped_root_bounds_links_to_new_floor() {
    let dir = TestDir::new("scope-floor");
    tree(&dir);
    // A link inside `d` that pops to the scope's parent — resolvable
    // under the original root, an escape under the new floor.
    if dir_link(&PathBuf::from("../f"), &dir.path().join("d").join("l")).is_err() {
        eprintln!("skipping: cannot create a directory link on this host");
        return;
    }
    let root = root_of(&dir);
    let scoped = root
        .open_scoped_root(&rel("d"), FollowMode::NoFollow)
        .unwrap();
    assert_eq!(scoped.canonical_root().file_name(), Some(OsStr::new("d")));
    // `..` in the link target hits the new floor.
    assert_eq!(
        scoped
            .open_regular(&rel("l"), FollowMode::WithinRoot)
            .unwrap_err()
            .kind(),
        FileErrorKind::EscapesRoot
    );
    // The same link still resolves under the parent capability.
    assert_eq!(
        read_all(
            &mut root
                .open_regular(&rel("d/l"), FollowMode::WithinRoot)
                .unwrap()
        ),
        "top"
    );
}

#[test]
fn scoped_root_absolute_links_judged_against_new_anchor() {
    let dir = TestDir::new("scope-abs");
    std::fs::create_dir(dir.path().join("d")).unwrap();
    std::fs::create_dir(dir.path().join("d").join("sub")).unwrap();
    write_file(&dir.path().join("d").join("f"), b"scoped");
    let root = root_of(&dir);
    let scoped = root
        .open_scoped_root(&rel("d"), FollowMode::NoFollow)
        .unwrap();
    // Absolute link spelled through the *scoped* anchor resolves.
    let inner = canonical_link_target(scoped.canonical_root(), "");
    if dir_link(&inner, &dir.path().join("d").join("in")).is_err() {
        eprintln!("skipping: cannot create a directory link on this host");
        return;
    }
    // Absolute link to the parent root (outside the scope anchor)
    // is rejected — sibling/ancestor escape.
    let parent_target = canonical_link_target(root.canonical_root(), "");
    if dir_link(&parent_target, &dir.path().join("d").join("out")).is_err() {
        eprintln!("skipping: cannot create a directory link on this host");
        return;
    }
    assert_eq!(
        read_all(
            &mut scoped
                .open_regular(&rel("in/f"), FollowMode::WithinRoot)
                .unwrap()
        ),
        "scoped"
    );
    assert_eq!(
        scoped
            .open_regular(&rel("out/f"), FollowMode::WithinRoot)
            .unwrap_err()
            .kind(),
        FileErrorKind::EscapesRoot
    );
}

#[test]
fn scoped_root_survives_root_rename() {
    let dir = TestDir::new("scope-rename");
    tree(&dir);
    let root = root_of(&dir);
    let renamed = dir.path().with_file_name(format!(
        "{}-moved",
        dir.path().file_name().unwrap().to_string_lossy()
    ));
    std::fs::rename(dir.path(), &renamed).expect("rename root dir");
    // Scope acquisition through the held handle still works.
    let scoped = root
        .open_scoped_root(&rel("d"), FollowMode::NoFollow)
        .unwrap();
    assert_eq!(
        read_all(
            &mut scoped
                .open_regular(&rel("f"), FollowMode::NoFollow)
                .unwrap()
        ),
        "deep"
    );
    // Handle-derived anchor follows the rename.
    assert_eq!(scoped.canonical_root().file_name(), Some(OsStr::new("d")));
    let _ = std::fs::rename(&renamed, dir.path());
}

#[test]
fn scoped_root_ignores_replacement_at_old_pathname() {
    let dir = TestDir::new("scope-replace");
    tree(&dir);
    let root = root_of(&dir);
    let moved = dir.path().with_file_name(format!(
        "{}-moved",
        dir.path().file_name().unwrap().to_string_lossy()
    ));
    std::fs::rename(dir.path(), &moved).expect("rename root dir");
    // Plant a different tree at the old pathname.
    std::fs::create_dir(dir.path()).unwrap();
    std::fs::create_dir(dir.path().join("d")).unwrap();
    write_file(&dir.path().join("d").join("f"), b"IMPOSTER");
    // The held capability still binds its own subtree — never the
    // pathname's new object.
    let scoped = root
        .open_scoped_root(&rel("d"), FollowMode::NoFollow)
        .unwrap();
    assert_eq!(
        read_all(
            &mut scoped
                .open_regular(&rel("f"), FollowMode::NoFollow)
                .unwrap()
        ),
        "deep"
    );
    // A fresh path-level open does acquire the replacement.
    let fresh = RootedDir::open(dir.path()).unwrap();
    assert_eq!(
        read_all(
            &mut fresh
                .open_regular(&rel("d/f"), FollowMode::NoFollow)
                .unwrap()
        ),
        "IMPOSTER"
    );
    let _ = std::fs::remove_dir_all(dir.path());
    let _ = std::fs::rename(&moved, dir.path());
}

#[test]
fn scoped_root_self_form_and_leaf_rules() {
    let dir = TestDir::new("scope-rules");
    tree(&dir);
    let root = root_of(&dir);
    // `rel == ""` re-roots at this directory itself.
    let scoped = root
        .open_scoped_root(&rel(""), FollowMode::NoFollow)
        .unwrap();
    assert_eq!(
        read_all(
            &mut scoped
                .open_regular(&rel("d/f"), FollowMode::NoFollow)
                .unwrap()
        ),
        "deep"
    );
    // A file leaf is NotDirectory.
    assert_eq!(
        root.open_scoped_root(&rel("f"), FollowMode::NoFollow)
            .unwrap_err()
            .kind(),
        FileErrorKind::NotDirectory
    );
}

// ----------------------------------------------------------------
// read_dir_each — bounded, stoppable enumeration (Issue #409)
// ----------------------------------------------------------------

#[test]
fn read_dir_each_stops_at_prefix() {
    let dir = TestDir::new("enum-stop");
    std::fs::create_dir(dir.path().join("d")).unwrap();
    for i in 0..50 {
        write_file(&dir.path().join("d").join(format!("f{i:02}")), b"x");
    }
    let root = root_of(&dir);
    let mut count = 0u64;
    let outcome = root
        .read_dir_each(&rel("d"), FollowMode::NoFollow, &mut |_e| {
            count += 1;
            // `remaining + 1` style prefix bound.
            if count >= 5 {
                EntryFlow::Stop
            } else {
                EntryFlow::Continue
            }
        })
        .unwrap();
    assert_eq!(outcome, EnumerationOutcome::Stopped);
    assert_eq!(count, 5);
    // A second call enumerates independently and completes.
    let mut seen = 0u64;
    let outcome = root
        .read_dir_each(&rel("d"), FollowMode::NoFollow, &mut |_e| {
            seen += 1;
            EntryFlow::Continue
        })
        .unwrap();
    assert_eq!(outcome, EnumerationOutcome::Complete);
    assert_eq!(seen, 50);
}

#[test]
fn read_dir_each_nested_enumeration_is_independent() {
    // A visitor that re-entrantly enumerates the same directory must
    // not disturb the outer enumeration — the scan position is
    // per-call state (fresh fd/FILE_OBJECT), never shared.
    let dir = TestDir::new("enum-nested");
    std::fs::create_dir(dir.path().join("d")).unwrap();
    for i in 0..20 {
        write_file(&dir.path().join("d").join(format!("f{i:02}")), b"x");
    }
    let root = root_of(&dir);
    let mut outer = 0u64;
    let outcome = root
        .read_dir_each(&rel("d"), FollowMode::NoFollow, &mut |_e| {
            outer += 1;
            // Re-entrant full enumeration of the same dir through the
            // same capability — must not steal the outer scan state.
            let inner = root
                .read_dir(&rel("d"), FollowMode::NoFollow)
                .unwrap();
            assert_eq!(inner.len(), 20);
            EntryFlow::Continue
        })
        .unwrap();
    assert_eq!(outcome, EnumerationOutcome::Complete);
    assert_eq!(outer, 20);
}

#[test]
fn read_dir_each_matches_read_dir_and_keeps_link_kinds() {
    let dir = TestDir::new("enum-kinds");
    tree(&dir);
    let link_created = dir_link(&dir.path().join("d"), &dir.path().join("lj")).is_ok();
    let root = root_of(&dir);
    let via_dir: Vec<_> = root
        .read_dir(&rel(""), FollowMode::NoFollow)
        .unwrap()
        .into_iter()
        .map(|e| (e.name, e.kind))
        .collect();
    let mut via_each = Vec::new();
    let outcome = root
        .read_dir_each(&rel(""), FollowMode::NoFollow, &mut |e| {
            via_each.push((e.name, e.kind));
            EntryFlow::Continue
        })
        .unwrap();
    assert_eq!(outcome, EnumerationOutcome::Complete);
    let mut a = via_dir;
    let mut b = via_each;
    a.sort_by(|x, y| x.0.cmp(&y.0));
    b.sort_by(|x, y| x.0.cmp(&y.0));
    assert_eq!(a, b);
    if link_created {
        assert!(b.iter().any(|(n, k)| n == "lj" && *k == NodeKind::Symlink));
    }
}

#[test]
fn read_dir_each_immediate_stop_and_empty_dir() {
    let dir = TestDir::new("enum-edge");
    std::fs::create_dir(dir.path().join("empty")).unwrap();
    write_file(&dir.path().join("f"), b"x");
    let root = root_of(&dir);
    // `max_entries = 0` analogue.
    let mut visits = 0u64;
    let outcome = root
        .read_dir_each(&rel(""), FollowMode::NoFollow, &mut |_e| {
            visits += 1;
            assert_eq!(visits, 1, "only one entry visited before Stop");
            EntryFlow::Stop
        })
        .unwrap();
    assert_eq!(outcome, EnumerationOutcome::Stopped);
    // Empty directory completes without a single visit.
    let mut visited = 0u64;
    let outcome = root
        .read_dir_each(&rel("empty"), FollowMode::NoFollow, &mut |_e| {
            visited += 1;
            EntryFlow::Continue
        })
        .unwrap();
    assert_eq!(outcome, EnumerationOutcome::Complete);
    assert_eq!(visited, 0);
    // A link leaf is NotDirectory under NoFollow — same as read_dir.
    if dir_link(dir.path(), &dir.path().join("lj")).is_ok() {
        assert_eq!(
            root.read_dir_each(&rel("lj"), FollowMode::NoFollow, &mut |_e| {
                EntryFlow::Continue
            })
            .unwrap_err()
            .kind(),
            FileErrorKind::NotDirectory
        );
    }
}

#[test]
fn read_dir_each_visitor_panic_releases_state() {
    let dir = TestDir::new("enum-panic");
    std::fs::create_dir(dir.path().join("d")).unwrap();
    for i in 0..8 {
        write_file(&dir.path().join("d").join(format!("f{i}")), b"x");
    }
    let root = root_of(&dir);
    let mut n = 0u64;
    let hit = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = root.read_dir_each(&rel("d"), FollowMode::NoFollow, &mut |_e| {
            n += 1;
            assert!(n != 2, "visitor boom");
            EntryFlow::Continue
        });
    }));
    assert!(hit.is_err());
    // A fresh enumeration completes — the unwound call left no
    // half-held enumeration state.
    let mut seen = 0u64;
    let outcome = root
        .read_dir_each(&rel("d"), FollowMode::NoFollow, &mut |_e| {
            seen += 1;
            EntryFlow::Continue
        })
        .unwrap();
    assert_eq!(outcome, EnumerationOutcome::Complete);
    assert_eq!(seen, 8);
}

/// Repeating panicked enumerations must not leak the unix `DIR*`/fd
/// or the Windows enumeration state — this loops far above the
/// typical fd soft limit, so a leak deterministically errors.
#[cfg(unix)]
#[test]
fn read_dir_each_repeated_panics_do_not_leak() {
    let dir = TestDir::new("enum-leak");
    std::fs::create_dir(dir.path().join("d")).unwrap();
    write_file(&dir.path().join("d").join("f"), b"x");
    let root = root_of(&dir);
    for _ in 0..1500 {
        let hit = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = root.read_dir_each(&rel("d"), FollowMode::NoFollow, &mut |_e| {
                panic!("visitor boom");
            });
        }));
        assert!(hit.is_err());
    }
    let mut seen = 0u64;
    root.read_dir_each(&rel("d"), FollowMode::NoFollow, &mut |_e| {
        seen += 1;
        EntryFlow::Continue
    })
    .unwrap();
    assert_eq!(seen, 1);
}
