//! Resolve-rule unit tests against the scripted vfs backend
//! (`vfs::mock`). The mock pseudo-FS can be mutated between calls so
//! leaf/intermediate swaps, planted links and delete+recreate are
//! deterministic — and `outside = true` nodes stand in for
//! root-external objects whose bytes/facts must never surface.
//!
//! Absolute link targets are platform-shaped (unix `/r/x` vs
//! Windows `C:\r\x`), so those cases use `abs()`/`outside_abs()`
//! helpers; the FD machinery itself is platform-agnostic.

use std::ffi::OsStr;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use super::{
    EntryFlow, EnumerationOutcome, FollowMode, IdentityStrength, NodeKind, RelativePath, RootedDir,
    RootedOptions,
};
use crate::FileErrorKind;
use crate::vfs::VfsKind;
use crate::vfs::mock::{TestFs, TestNode};

/// The fake capability root path for this platform.
fn root_path() -> &'static Path {
    #[cfg(windows)]
    {
        Path::new("C:\\r")
    }
    #[cfg(not(windows))]
    {
        Path::new("/r")
    }
}

/// An absolute link target *inside* the root (`/r/<tail>` or
/// `C:\r\<tail>`), in OS-raw link-target form.
fn abs(tail: &str) -> String {
    #[cfg(windows)]
    {
        format!("C:\\r\\{}", tail.replace('/', "\\"))
    }
    #[cfg(not(windows))]
    {
        format!("/r/{tail}")
    }
}

/// An absolute link target *outside* the root.
fn outside_abs(tail: &str) -> String {
    #[cfg(windows)]
    {
        format!("C:\\outside\\{}", tail.replace('/', "\\"))
    }
    #[cfg(not(windows))]
    {
        format!("/outside/{tail}")
    }
}

/// The fake outside-root path matching [`outside_abs`].
fn outside_dir() -> PathBuf {
    #[cfg(windows)]
    {
        PathBuf::from("C:\\outside")
    }
    #[cfg(not(windows))]
    {
        PathBuf::from("/outside")
    }
}

fn node_path(parts: &[&str]) -> PathBuf {
    let mut p = root_path().to_path_buf();
    for part in parts {
        p.push(part);
    }
    p
}

fn rel(s: &str) -> RelativePath {
    RelativePath::new(s).unwrap()
}

/// Builds a `RootedDir` on the mock with a root dir node at
/// [`root_path`].
fn mount(fs: TestFs) -> (Arc<Mutex<TestFs>>, RootedDir) {
    mount_with(fs, RootedOptions::default())
}

fn mount_with(fs: TestFs, options: RootedOptions) -> (Arc<Mutex<TestFs>>, RootedDir) {
    mount_at(root_path(), fs, options)
}

/// Mounts the mock FS at an arbitrary root (for canonical-prefix
/// shapes other than the default).
fn mount_at(path: &Path, fs: TestFs, options: RootedOptions) -> (Arc<Mutex<TestFs>>, RootedDir) {
    let (fs, ops) = fs.at();
    let root = RootedDir::open_with_ops(path, options, Arc::new(ops)).unwrap();
    (fs, root)
}

fn fs_root() -> TestFs {
    let mut fs = TestFs::default();
    fs.put(root_path(), TestNode::dir(1));
    fs
}

fn read_all(file: &mut crate::rooted::RootedFile) -> String {
    let mut s = String::new();
    file.read_to_string(&mut s).unwrap();
    s
}

// ----------------------------------------------------------------
// RelativePath grammar
// ----------------------------------------------------------------

#[test]
fn locator_grammar_accepts_plain_forms() {
    assert_eq!(rel("").depth(), 0);
    assert_eq!(rel("a").depth(), 1);
    assert_eq!(rel("a/b/c").depth(), 3);
    assert_eq!(rel("a/").depth(), 1, "trailing `/` is a dir marker");
    assert_eq!(rel("日本語/ファイル").depth(), 2);
}

#[test]
fn locator_grammar_rejects_escape_forms() {
    for bad in [
        "/abs",
        "a/../b",
        "..",
        "a/./b",
        "a//b",
        "a\\b",
        "a:b",
        "a\0b",
        "a\u{7f}b",
        "CON",
        "con.txt",
        "lpt1",
        "com9/ok".split('/').next().unwrap(),
        "trailing.",
        "trailing ",
        " lead",
        "a\u{1f}b",
        "aux",
        "NUL",
        "com9",
        "com\u{00b9}", // com¹ — NTFS superscript mapping
    ] {
        assert!(
            RelativePath::new(bad).is_err(),
            "accepted invalid locator {bad:?}"
        );
        assert_eq!(
            RelativePath::new(bad).unwrap_err().kind(),
            FileErrorKind::InvalidInput
        );
    }
    let long = "x".repeat(256);
    assert!(RelativePath::new(&long).is_err());
}

// ----------------------------------------------------------------
// Basic traversal
// ----------------------------------------------------------------

#[test]
fn plain_file_opens_and_reports_same_object_facts() {
    let mut fs = fs_root();
    fs.put(node_path(&["f.txt"]), TestNode::file(2, b"hello"));
    let (_fs, root) = mount(fs);
    let mut file = root
        .open_regular(&rel("f.txt"), FollowMode::NoFollow)
        .unwrap();
    assert_eq!(read_all(&mut file), "hello");
    assert_eq!(file.info().kind, NodeKind::File);
    assert_eq!(file.info().byte_len, Some(5));
    assert_eq!(file.info().identity_strength, IdentityStrength::Strong);
}

#[test]
fn missing_leaf_is_not_found() {
    let (_fs, root) = mount(fs_root());
    let e = root
        .open_regular(&rel("nope"), FollowMode::NoFollow)
        .unwrap_err();
    assert_eq!(e.kind(), FileErrorKind::NotFound);
    let e = root
        .metadata(&rel("nope"), FollowMode::NoFollow)
        .unwrap_err();
    assert_eq!(e.kind(), FileErrorKind::NotFound);
}

#[test]
fn nonregular_leaves_are_not_regular_file() {
    let mut fs = fs_root();
    fs.put(
        node_path(&["fifo"]),
        TestNode::new(VfsKind::Other, 2, IdentityStrength::Strong),
    );
    fs.put(node_path(&["d"]), TestNode::dir(3));
    let (_fs, root) = mount(fs);
    assert_eq!(
        root.open_regular(&rel("fifo"), FollowMode::WithinRoot)
            .unwrap_err()
            .kind(),
        FileErrorKind::NotRegularFile
    );
    assert_eq!(
        root.open_regular(&rel("d"), FollowMode::WithinRoot)
            .unwrap_err()
            .kind(),
        FileErrorKind::NotRegularFile
    );
}

#[test]
fn metadata_self_and_dir_report_dir_kind() {
    let mut fs = fs_root();
    fs.put(node_path(&["d"]), TestNode::dir(2));
    let (_fs, root) = mount(fs);
    let info = root.metadata(&rel(""), FollowMode::NoFollow).unwrap();
    assert_eq!(info.kind, NodeKind::Dir);
    let info = root.metadata(&rel("d"), FollowMode::NoFollow).unwrap();
    assert_eq!(info.kind, NodeKind::Dir);
    assert_eq!(info.leaf_kind, NodeKind::Dir);
}

#[test]
fn read_dir_enumerates_self_and_classifies_nofollow() {
    let mut fs = fs_root();
    fs.put(node_path(&["f"]), TestNode::file(2, b"x"));
    fs.put(node_path(&["d"]), TestNode::dir(3));
    fs.put(node_path(&["l"]), TestNode::link(4, "f"));
    let (_fs, root) = mount(fs);
    let entries = root.read_dir(&rel(""), FollowMode::NoFollow).unwrap();
    let mut kinds: Vec<_> = entries.iter().map(|e| (e.name.clone(), e.kind)).collect();
    kinds.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(
        kinds,
        vec![
            (OsStr::new("d").to_os_string(), NodeKind::Dir),
            (OsStr::new("f").to_os_string(), NodeKind::File),
            (OsStr::new("l").to_os_string(), NodeKind::Symlink),
        ]
    );
}

// ----------------------------------------------------------------
// NoFollow vs WithinRoot
// ----------------------------------------------------------------

#[test]
fn nofollow_leaf_link_reports_symlink_and_refuses_open() {
    let mut fs = fs_root();
    fs.put(node_path(&["f"]), TestNode::file(2, b"payload"));
    fs.put(node_path(&["l"]), TestNode::link(3, "f"));
    let (_fs, root) = mount(fs);
    let info = root.metadata(&rel("l"), FollowMode::NoFollow).unwrap();
    assert_eq!(info.kind, NodeKind::Symlink);
    assert_eq!(info.leaf_kind, NodeKind::Symlink);
    assert_eq!(
        root.open_regular(&rel("l"), FollowMode::NoFollow)
            .unwrap_err()
            .kind(),
        FileErrorKind::NotRegularFile
    );
    assert_eq!(
        root.open_dir(&rel("l"), FollowMode::NoFollow)
            .unwrap_err()
            .kind(),
        FileErrorKind::NotDirectory
    );
    assert_eq!(
        root.read_dir(&rel("l"), FollowMode::NoFollow)
            .unwrap_err()
            .kind(),
        FileErrorKind::NotDirectory
    );
}

#[test]
fn intermediate_link_is_notdirectory_under_nofollow() {
    let mut fs = fs_root();
    fs.put(node_path(&["real"]), TestNode::dir(2));
    fs.put(node_path(&["real", "f"]), TestNode::file(3, b"inner"));
    fs.put(node_path(&["l"]), TestNode::link(4, "real"));
    let (_fs, root) = mount(fs);
    assert_eq!(
        root.open_regular(&rel("l/f"), FollowMode::NoFollow)
            .unwrap_err()
            .kind(),
        FileErrorKind::NotDirectory
    );
    assert_eq!(
        root.metadata(&rel("l/f"), FollowMode::NoFollow)
            .unwrap_err()
            .kind(),
        FileErrorKind::NotDirectory
    );
}

#[test]
fn withinroot_relative_link_reads_target_and_reports_leaf_link() {
    let mut fs = fs_root();
    fs.put(node_path(&["f"]), TestNode::file(2, b"payload"));
    fs.put(node_path(&["l"]), TestNode::link(3, "f"));
    let (_fs, root) = mount(fs);
    let mut file = root
        .open_regular(&rel("l"), FollowMode::WithinRoot)
        .unwrap();
    assert_eq!(read_all(&mut file), "payload");
    assert_eq!(file.info().kind, NodeKind::File);
    assert_eq!(file.info().leaf_kind, NodeKind::Symlink);
}

#[test]
fn withinroot_link_through_dir_descends() {
    let mut fs = fs_root();
    fs.put(node_path(&["real"]), TestNode::dir(2));
    fs.put(node_path(&["real", "f"]), TestNode::file(3, b"inner"));
    fs.put(node_path(&["l"]), TestNode::link(4, "real"));
    let (_fs, root) = mount(fs);
    let mut file = root
        .open_regular(&rel("l/f"), FollowMode::WithinRoot)
        .unwrap();
    assert_eq!(read_all(&mut file), "inner");
    let scoped = root.open_dir(&rel("l"), FollowMode::WithinRoot).unwrap();
    let entries = scoped.read_dir(&rel(""), FollowMode::NoFollow).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name, "f");
}

#[test]
fn withinroot_link_to_dir_metadata_reports_resolved() {
    let mut fs = fs_root();
    fs.put(node_path(&["real"]), TestNode::dir(2));
    fs.put(node_path(&["l"]), TestNode::link(3, "real"));
    let (_fs, root) = mount(fs);
    let info = root.metadata(&rel("l"), FollowMode::WithinRoot).unwrap();
    assert_eq!(info.kind, NodeKind::Dir);
    assert_eq!(info.leaf_kind, NodeKind::Symlink);
}

// ----------------------------------------------------------------
// `..` in link targets (kernel post-substitution semantics)
// ----------------------------------------------------------------

#[test]
fn relative_dotdot_target_pops_resolved_ancestors() {
    let mut fs = fs_root();
    fs.put(node_path(&["a"]), TestNode::dir(2));
    fs.put(node_path(&["a", "sub"]), TestNode::dir(3));
    fs.put(node_path(&["a", "sub", "l"]), TestNode::link(4, "../sib/f"));
    fs.put(node_path(&["a", "sib"]), TestNode::dir(5));
    fs.put(node_path(&["a", "sib", "f"]), TestNode::file(6, b"target"));
    let (_fs, root) = mount(fs);
    let mut file = root
        .open_regular(&rel("a/sub/l"), FollowMode::WithinRoot)
        .unwrap();
    assert_eq!(read_all(&mut file), "target");
}

#[test]
fn dotdot_past_capability_root_floor_escapes() {
    let mut fs = fs_root();
    fs.put(node_path(&["a"]), TestNode::dir(2));
    fs.put(node_path(&["a", "l"]), TestNode::link(3, "../../outside/f"));
    let (_fs, root) = mount(fs);
    assert_eq!(
        root.open_regular(&rel("a/l"), FollowMode::WithinRoot)
            .unwrap_err()
            .kind(),
        FileErrorKind::EscapesRoot
    );
    // One `..` lands at the root itself — inside.
    let mut fs2 = fs_root();
    fs2.put(node_path(&["a"]), TestNode::dir(2));
    fs2.put(node_path(&["a", "l"]), TestNode::link(3, "../f"));
    fs2.put(node_path(&["f"]), TestNode::file(4, b"kept"));
    let (_fs2, root2) = mount(fs2);
    let mut file = root2
        .open_regular(&rel("a/l"), FollowMode::WithinRoot)
        .unwrap();
    assert_eq!(read_all(&mut file), "kept");
}

// ----------------------------------------------------------------
// Absolute link targets — two-state walk + tainted
// ----------------------------------------------------------------

#[test]
fn absolute_link_inside_root_resolves() {
    let mut fs = fs_root();
    fs.put(node_path(&["d"]), TestNode::dir(2));
    fs.put(node_path(&["d", "f"]), TestNode::file(3, b"deep"));
    fs.put(node_path(&["l"]), TestNode::link(4, &abs("d/f")));
    let (_fs, root) = mount(fs);
    let mut file = root
        .open_regular(&rel("l"), FollowMode::WithinRoot)
        .unwrap();
    assert_eq!(read_all(&mut file), "deep");
}

#[test]
fn absolute_link_to_exact_root_resolves_to_root() {
    let mut fs = fs_root();
    #[cfg(windows)]
    fs.put(node_path(&["l"]), TestNode::link(2, "C:\\r"));
    #[cfg(not(windows))]
    fs.put(node_path(&["l"]), TestNode::link(2, "/r"));
    let (_fs, root) = mount(fs);
    let scoped = root.open_dir(&rel("l"), FollowMode::WithinRoot).unwrap();
    assert_eq!(scoped.canonical_root(), root.canonical_root());
    let info = root.metadata(&rel("l"), FollowMode::WithinRoot).unwrap();
    assert_eq!(info.kind, NodeKind::Dir);
    assert_eq!(info.leaf_kind, NodeKind::Symlink);
}

#[test]
fn absolute_link_outside_root_escapes() {
    let mut fs = fs_root();
    fs.put(node_path(&["l"]), TestNode::link(2, &outside_abs("f")));
    let (_fs, root) = mount(fs);
    assert_eq!(
        root.open_regular(&rel("l"), FollowMode::WithinRoot)
            .unwrap_err()
            .kind(),
        FileErrorKind::EscapesRoot
    );
    // The outside object's bytes/facts never surface — the error is
    // all a caller learns.
    assert_eq!(
        root.metadata(&rel("l"), FollowMode::WithinRoot)
            .unwrap_err()
            .kind(),
        FileErrorKind::EscapesRoot
    );
}

#[test]
fn absolute_link_prefix_only_escapes() {
    // Mounted at <root>/sub: a target of <root> itself is a strict
    // prefix of the canonical root — TEXTUAL ends without reaching R
    // → EscapesRoot.
    let mut fs = TestFs::default();
    fs.put(root_path(), TestNode::dir(1));
    let sub_root = node_path(&["sub"]);
    fs.put(&sub_root, TestNode::dir(2));
    fs.put(sub_root.join("l"), TestNode::link(3, root_target()));
    let (_fs, root) = mount_at(&sub_root, fs, RootedOptions::default());
    assert_eq!(
        root.metadata(&rel("l"), FollowMode::WithinRoot)
            .unwrap_err()
            .kind(),
        FileErrorKind::EscapesRoot
    );
}

/// The fake capability-root path as a link-target string.
#[cfg(windows)]
fn root_target() -> &'static str {
    "C:\\r"
}
#[cfg(not(windows))]
fn root_target() -> &'static str {
    "/r"
}

#[test]
fn out_and_back_target_is_tainted_escapes_root() {
    // `/outside/../r/f` leaves and returns — conservative EscapesRoot
    // (and the inside `f` node must never be reached).
    let mut fs = fs_root();
    fs.put(node_path(&["f"]), TestNode::file(2, b"in-root"));
    #[cfg(windows)]
    fs.put(
        node_path(&["l"]),
        TestNode::link(3, "C:\\outside\\..\\r\\f"),
    );
    #[cfg(not(windows))]
    fs.put(node_path(&["l"]), TestNode::link(3, "/outside/../r/f"));
    let (_fs, root) = mount(fs);
    assert_eq!(
        root.open_regular(&rel("l"), FollowMode::WithinRoot)
            .unwrap_err()
            .kind(),
        FileErrorKind::EscapesRoot
    );
}

#[test]
fn dotdot_at_fd_floor_in_absolute_target_escapes() {
    // `<R>/..` pops the capability-root floor → tainted even though
    // the lexical result would re-enter R.
    let mut fs = fs_root();
    #[cfg(windows)]
    fs.put(node_path(&["l"]), TestNode::link(2, "C:\\r\\..\\r\\f"));
    #[cfg(not(windows))]
    fs.put(node_path(&["l"]), TestNode::link(2, "/r/../r/f"));
    fs.put(node_path(&["f"]), TestNode::file(3, b"in-root"));
    let (_fs, root) = mount(fs);
    assert_eq!(
        root.open_regular(&rel("l"), FollowMode::WithinRoot)
            .unwrap_err()
            .kind(),
        FileErrorKind::EscapesRoot
    );
}

#[test]
fn absolute_target_dotdot_inside_root_is_kernel_faithful() {
    // `<R>/d/../f` — the `..` pops a *pushed* ancestor, stays inside.
    let mut fs = fs_root();
    fs.put(node_path(&["d"]), TestNode::dir(2));
    fs.put(node_path(&["f"]), TestNode::file(3, b"landed"));
    #[cfg(windows)]
    fs.put(node_path(&["l"]), TestNode::link(4, "C:\\r\\d\\..\\f"));
    #[cfg(not(windows))]
    fs.put(node_path(&["l"]), TestNode::link(4, "/r/d/../f"));
    let (_fs, root) = mount(fs);
    let mut file = root
        .open_regular(&rel("l"), FollowMode::WithinRoot)
        .unwrap();
    assert_eq!(read_all(&mut file), "landed");
}

/// Two-component root for the TEXTUAL `..` cases (`/a/b` vs
/// `C:\a\b`): a `..` consumed while `virtual` is still a proper
/// prefix of R exercises the TEXTUAL pop arm the single-component
/// `/r` root can never reach.
#[cfg(windows)]
fn nested_root_path() -> &'static Path {
    Path::new("C:\\a\\b")
}
#[cfg(not(windows))]
fn nested_root_path() -> &'static Path {
    Path::new("/a/b")
}

#[test]
fn absolute_target_dotdot_within_prefix_is_kernel_faithful() {
    // `<R-parent>/../<R>/f` — the `..` folds a component that is a
    // *proper prefix* of R (kernel: `/a/../a/b/f` = `/a/b/f`), then
    // the walk re-enters R and resolves FD-relative. Pins the
    // TEXTUAL `..` pop on the accept arm; the tainted sibling
    // (`<R-parent>/../<else>`) escapes because the folded result
    // stops prefixing R.
    let mut fs = TestFs::default();
    let r = nested_root_path();
    fs.put(r, TestNode::dir(1));
    fs.put(r.join("f"), TestNode::file(2, b"landed"));
    #[cfg(windows)]
    fs.put(r.join("l"), TestNode::link(3, "C:\\a\\..\\a\\b\\f"));
    #[cfg(not(windows))]
    fs.put(r.join("l"), TestNode::link(3, "/a/../a/b/f"));
    #[cfg(windows)]
    fs.put(r.join("m"), TestNode::link(4, "C:\\a\\..\\x\\f"));
    #[cfg(not(windows))]
    fs.put(r.join("m"), TestNode::link(4, "/a/../x/f"));
    let (_fs, root) = mount_at(r, fs, RootedOptions::default());
    let mut file = root
        .open_regular(&rel("l"), FollowMode::WithinRoot)
        .unwrap();
    assert_eq!(read_all(&mut file), "landed");
    // `/a/../x/f` — the fold leaves the canonical prefix → tainted.
    assert_eq!(
        root.open_regular(&rel("m"), FollowMode::WithinRoot)
            .unwrap_err()
            .kind(),
        FileErrorKind::EscapesRoot
    );
}

#[test]
fn absolute_target_dotdot_at_namespace_root_clamps_and_resolves() {
    // `/../r/f` — `..` on an empty virtual clamps to the namespace
    // root (kernel `/..` = `/`), then re-enters R → accept.
    let mut fs = fs_root();
    fs.put(node_path(&["f"]), TestNode::file(2, b"landed"));
    #[cfg(windows)]
    fs.put(node_path(&["l"]), TestNode::link(3, "C:\\..\\r\\f"));
    #[cfg(not(windows))]
    fs.put(node_path(&["l"]), TestNode::link(3, "/../r/f"));
    let (_fs, root) = mount(fs);
    #[cfg(windows)]
    {
        // Windows narrows this arm: folding `..` pops the `C:`
        // component and `r` alone can never re-prefix `[C:, r]`
        // — conservative EscapesRoot (kernel would accept; same
        // wrong-reject class as non-canonical alias spellings).
        assert_eq!(
            root.open_regular(&rel("l"), FollowMode::WithinRoot)
                .unwrap_err()
                .kind(),
            FileErrorKind::EscapesRoot
        );
    }
    #[cfg(not(windows))]
    {
        let mut file = root
            .open_regular(&rel("l"), FollowMode::WithinRoot)
            .unwrap();
        assert_eq!(read_all(&mut file), "landed");
    }
}

// ----------------------------------------------------------------
// Loops and budgets
// ----------------------------------------------------------------

#[test]
fn link_loop_is_loop_detected() {
    let mut fs = fs_root();
    fs.put(node_path(&["a"]), TestNode::link(2, "b"));
    fs.put(node_path(&["b"]), TestNode::link(3, "a"));
    let (_fs, root) = mount(fs);
    assert_eq!(
        root.open_regular(&rel("a"), FollowMode::WithinRoot)
            .unwrap_err()
            .kind(),
        FileErrorKind::LoopDetected
    );
}

#[test]
fn self_link_is_loop_detected() {
    let mut fs = fs_root();
    fs.put(node_path(&["a"]), TestNode::link(2, "a"));
    let (_fs, root) = mount(fs);
    assert_eq!(
        root.metadata(&rel("a"), FollowMode::WithinRoot)
            .unwrap_err()
            .kind(),
        FileErrorKind::LoopDetected
    );
}

#[test]
fn hop_budget_limits_expansions() {
    let mut fs = fs_root();
    fs.put(node_path(&["f"]), TestNode::file(2, b"x"));
    for i in 0..8u64 {
        let next = if i == 7 {
            "f".to_owned()
        } else {
            format!("l{}", i + 1)
        };
        fs.put(
            node_path(&[&format!("l{i}")]),
            TestNode::link(10 + i, &next),
        );
    }
    let options = RootedOptions {
        max_hops: 3,
        ..RootedOptions::default()
    };
    let (_fs, root) = mount_with(fs, options);
    assert_eq!(
        root.open_regular(&rel("l0"), FollowMode::WithinRoot)
            .unwrap_err()
            .kind(),
        FileErrorKind::LoopDetected
    );
}

#[test]
fn depth_and_component_budgets_limit() {
    let fs = fs_root();
    let deep: Vec<String> = (0..45).map(|i| format!("d{i}")).collect();
    let locator = deep.join("/");
    assert!(RelativePath::new(&locator).is_ok());
    let (_fs, root) = mount(fs);
    assert_eq!(
        root.metadata(&rel(&locator), FollowMode::NoFollow)
            .unwrap_err()
            .kind(),
        FileErrorKind::LimitExceeded
    );
    // Raised budget still fails — but on the missing dir, not the
    // budget.
    let options = RootedOptions {
        max_depth: 100,
        ..RootedOptions::default()
    };
    let mut fs = fs_root();
    fs.put(node_path(&["d0"]), TestNode::dir(2));
    let (_fs, root) = mount_with(fs, options);
    assert_eq!(
        root.metadata(&rel(&locator), FollowMode::NoFollow)
            .unwrap_err()
            .kind(),
        FileErrorKind::NotFound
    );
}

// ----------------------------------------------------------------
// Scoped dirs, pin, identity
// ----------------------------------------------------------------

#[test]
fn scoped_dir_shares_capability_root() {
    let mut fs = fs_root();
    fs.put(node_path(&["a"]), TestNode::dir(2));
    fs.put(node_path(&["a", "f"]), TestNode::file(3, b"scoped"));
    let (_fs, root) = mount(fs);
    let scoped = root.open_dir(&rel("a"), FollowMode::NoFollow).unwrap();
    assert_eq!(scoped.canonical_root(), root.canonical_root());
    let mut file = scoped
        .open_regular(&rel("f"), FollowMode::NoFollow)
        .unwrap();
    assert_eq!(read_all(&mut file), "scoped");
}

#[test]
fn scoped_link_dotdot_pops_inside_chain_but_not_below_root() {
    let mut fs = fs_root();
    fs.put(node_path(&["a"]), TestNode::dir(2));
    fs.put(node_path(&["a", "l"]), TestNode::link(3, "../f"));
    fs.put(node_path(&["f"]), TestNode::file(4, b"from-scoped"));
    let (_fs, root) = mount(fs);
    let scoped = root.open_dir(&rel("a"), FollowMode::NoFollow).unwrap();
    let mut file = scoped
        .open_regular(&rel("l"), FollowMode::WithinRoot)
        .unwrap();
    assert_eq!(read_all(&mut file), "from-scoped");
}

#[test]
fn open_pinned_roundtrip_and_recreate_mismatch() {
    let mut fs = fs_root();
    fs.put(node_path(&["f"]), TestNode::file(2, b"pinned"));
    let (fs, root) = mount(fs);
    let pin = root.pin(&rel("f"), FollowMode::NoFollow).unwrap();
    let mut file = root
        .open_pinned(&rel("f"), &pin, FollowMode::NoFollow)
        .unwrap();
    assert_eq!(read_all(&mut file), "pinned");
    // Delete + recreate → new identity → IdentityMismatch, no bytes.
    fs.lock().unwrap().remove(node_path(&["f"]));
    fs.lock()
        .unwrap()
        .put(node_path(&["f"]), TestNode::file(99, b"pinned"));
    let e = root
        .open_pinned(&rel("f"), &pin, FollowMode::NoFollow)
        .unwrap_err();
    assert_eq!(e.kind(), FileErrorKind::IdentityMismatch);
}

#[test]
fn weak_identity_refuses_pin_and_open_pinned() {
    let mut fs = fs_root();
    let mut weak = TestNode::file(2, b"weak-fs");
    weak.strength = IdentityStrength::Weak;
    fs.put(node_path(&["f"]), weak);
    let (_fs, root) = mount(fs);
    // metadata/open_regular still work on weak identity.
    let info = root.metadata(&rel("f"), FollowMode::NoFollow).unwrap();
    assert_eq!(info.identity_strength, IdentityStrength::Weak);
    let mut file = root.open_regular(&rel("f"), FollowMode::NoFollow).unwrap();
    assert_eq!(read_all(&mut file), "weak-fs");
    // pin/open_pinned refuse honestly.
    assert_eq!(
        root.pin(&rel("f"), FollowMode::NoFollow)
            .unwrap_err()
            .kind(),
        FileErrorKind::Unsupported
    );
}

#[test]
fn pin_on_dir_is_not_regular_file() {
    let mut fs = fs_root();
    fs.put(node_path(&["d"]), TestNode::dir(2));
    let (_fs, root) = mount(fs);
    assert_eq!(
        root.pin(&rel("d"), FollowMode::NoFollow)
            .unwrap_err()
            .kind(),
        FileErrorKind::NotRegularFile
    );
}

// ----------------------------------------------------------------
// Swaps — the capability's core promise
// ----------------------------------------------------------------

#[test]
fn leaf_swapped_to_link_never_opens_outside_object() {
    let mut fs = fs_root();
    fs.put(node_path(&["f"]), TestNode::file(2, b"safe"));
    let (fs, root) = mount(fs);
    // Swap f → link to an outside object between calls: the next
    // resolve reports the link (NoFollow) or EscapesRoot
    // (WithinRoot) — never outside bytes.
    fs.lock().unwrap().remove(node_path(&["f"]));
    fs.lock()
        .unwrap()
        .put(node_path(&["f"]), TestNode::link(9, &outside_abs("secret")));
    let info = root.metadata(&rel("f"), FollowMode::NoFollow).unwrap();
    assert_eq!(info.kind, NodeKind::Symlink);
    assert_eq!(
        root.open_regular(&rel("f"), FollowMode::WithinRoot)
            .unwrap_err()
            .kind(),
        FileErrorKind::EscapesRoot
    );
}

#[test]
fn intermediate_dir_swapped_to_link_is_notdirectory() {
    let mut fs = fs_root();
    fs.put(node_path(&["d"]), TestNode::dir(2));
    fs.put(node_path(&["d", "f"]), TestNode::file(3, b"safe"));
    let (fs, root) = mount(fs);
    // d → link to outside.
    fs.lock().unwrap().remove(node_path(&["d"]));
    fs.lock()
        .unwrap()
        .put(node_path(&["d"]), TestNode::link(9, &outside_abs("x")));
    assert_eq!(
        root.open_regular(&rel("d/f"), FollowMode::WithinRoot)
            .unwrap_err()
            .kind(),
        FileErrorKind::EscapesRoot
    );
    assert_eq!(
        root.open_regular(&rel("d/f"), FollowMode::NoFollow)
            .unwrap_err()
            .kind(),
        FileErrorKind::NotDirectory
    );
}

#[test]
fn outside_node_bytes_never_surface() {
    // An outside-marked node planted where a link points: resolution
    // must fail before its data can be returned.
    let mut fs = fs_root();
    fs.put(node_path(&["l"]), TestNode::link(2, &outside_abs("s")));
    fs.put(
        outside_dir().join("s"),
        TestNode::outside_file(50, b"SECRET"),
    );
    let (_fs, root) = mount(fs);
    assert_eq!(
        root.open_regular(&rel("l"), FollowMode::WithinRoot)
            .unwrap_err()
            .kind(),
        FileErrorKind::EscapesRoot
    );
}

// ----------------------------------------------------------------
// open_dir / read_dir leaf rules
// ----------------------------------------------------------------

#[test]
fn dir_api_leaf_rules() {
    let mut fs = fs_root();
    fs.put(node_path(&["f"]), TestNode::file(2, b"x"));
    let (_fs, root) = mount(fs);
    assert_eq!(
        root.open_dir(&rel("f"), FollowMode::NoFollow)
            .unwrap_err()
            .kind(),
        FileErrorKind::NotDirectory
    );
    assert_eq!(
        root.read_dir(&rel("f"), FollowMode::NoFollow)
            .unwrap_err()
            .kind(),
        FileErrorKind::NotDirectory
    );
}

#[test]
fn file_intermediate_is_notdirectory() {
    let mut fs = fs_root();
    fs.put(node_path(&["f"]), TestNode::file(2, b"x"));
    let (_fs, root) = mount(fs);
    assert_eq!(
        root.open_regular(&rel("f/g"), FollowMode::NoFollow)
            .unwrap_err()
            .kind(),
        FileErrorKind::NotDirectory
    );
}

#[test]
fn dir_identity_returns_stable_key() {
    let mut fs = fs_root();
    fs.put(node_path(&["d"]), TestNode::dir(2));
    let (_fs, root) = mount(fs);
    let scoped = root.open_dir(&rel("d"), FollowMode::NoFollow).unwrap();
    let a = scoped.dir_identity().unwrap();
    let b = scoped.dir_identity().unwrap();
    assert_eq!(a, b);
    assert_ne!(a, root.dir_identity().unwrap());
}

// ----------------------------------------------------------------
// open_scoped_root — re-rooting on the held handle (Issue #409)
// ----------------------------------------------------------------

/// The scoped dir's own canonical spelling (`<root>/a`) — the new
/// lexical anchor after re-rooting.
fn scoped_abs(tail: &str) -> String {
    #[cfg(windows)]
    {
        format!("C:\\r\\a\\{}", tail.replace('/', "\\"))
    }
    #[cfg(not(windows))]
    {
        format!("/r/a/{tail}")
    }
}

#[test]
fn scoped_root_establishes_a_new_floor() {
    let mut fs = fs_root();
    fs.put(node_path(&["a"]), TestNode::dir(2));
    fs.put(node_path(&["a", "f"]), TestNode::file(3, b"scoped"));
    // Link inside `a` whose target pops above `a` — allowed under
    // the parent root, an escape under the new floor.
    fs.put(node_path(&["a", "l"]), TestNode::link(4, "../f"));
    fs.put(node_path(&["f"]), TestNode::file(5, b"root-file"));
    let (_fs, root) = mount(fs);

    let scoped = root
        .open_scoped_root(&rel("a"), FollowMode::NoFollow)
        .unwrap();
    assert_eq!(
        read_all(
            &mut scoped
                .open_regular(&rel("f"), FollowMode::NoFollow)
                .unwrap()
        ),
        "scoped"
    );
    // `..` in the link target hits the new floor → EscapesRoot.
    assert_eq!(
        scoped
            .open_regular(&rel("l"), FollowMode::WithinRoot)
            .unwrap_err()
            .kind(),
        FileErrorKind::EscapesRoot
    );
    // The parent capability resolves the same link fine — the
    // narrow floor belongs only to the scoped capability.
    assert_eq!(
        read_all(
            &mut root
                .open_regular(&rel("a/l"), FollowMode::WithinRoot)
                .unwrap()
        ),
        "root-file"
    );
}

#[test]
fn scoped_root_judges_absolute_links_against_new_anchor() {
    let mut fs = fs_root();
    fs.put(node_path(&["a"]), TestNode::dir(2));
    fs.put(node_path(&["a", "f"]), TestNode::file(3, b"scoped"));
    fs.put(node_path(&["a", "in"]), TestNode::link(4, &scoped_abs("f")));
    fs.put(node_path(&["a", "out"]), TestNode::link(5, &abs("f")));
    fs.put(node_path(&["f"]), TestNode::file(6, b"outside-scope"));
    let (_fs, root) = mount(fs);

    let scoped = root
        .open_scoped_root(&rel("a"), FollowMode::NoFollow)
        .unwrap();
    // Absolute target inside the new anchor resolves.
    assert_eq!(
        read_all(
            &mut scoped
                .open_regular(&rel("in"), FollowMode::WithinRoot)
                .unwrap()
        ),
        "scoped"
    );
    // Absolute target outside the new anchor (a sibling subtree of
    // the scope) is EscapesRoot — not a silent alias acceptance.
    assert_eq!(
        scoped
            .open_regular(&rel("out"), FollowMode::WithinRoot)
            .unwrap_err()
            .kind(),
        FileErrorKind::EscapesRoot
    );
}

#[test]
fn scoped_root_requires_no_identity_strength() {
    let mut fs = fs_root();
    let mut weak_dir = TestNode::dir(2);
    weak_dir.strength = IdentityStrength::Weak;
    fs.put(node_path(&["a"]), weak_dir);
    fs.put(node_path(&["a", "f"]), TestNode::file(3, b"weak"));
    let (_fs, root) = mount(fs);
    // Handle authority — no NodeId/strength comparison — so a weak
    // filesystem object still scopes.
    let scoped = root
        .open_scoped_root(&rel("a"), FollowMode::NoFollow)
        .unwrap();
    assert_eq!(
        read_all(
            &mut scoped
                .open_regular(&rel("f"), FollowMode::NoFollow)
                .unwrap()
        ),
        "weak"
    );
}

#[test]
fn scoped_root_survives_root_rename() {
    let mut fs = fs_root();
    fs.put(node_path(&["a"]), TestNode::dir(2));
    fs.put(node_path(&["a", "f"]), TestNode::file(3, b"original"));
    let (fs, root) = mount(fs);
    // The held root's own pathname is renamed underneath it.
    let renamed = outside_dir().join("r-renamed");
    fs.lock().unwrap().rename(root_path(), &renamed);
    // Scope acquisition still resolves through the held handle.
    let scoped = root
        .open_scoped_root(&rel("a"), FollowMode::NoFollow)
        .unwrap();
    assert_eq!(
        read_all(
            &mut scoped
                .open_regular(&rel("f"), FollowMode::NoFollow)
                .unwrap()
        ),
        "original"
    );
    // The new anchor is the handle-derived renamed path.
    assert_eq!(scoped.canonical_root(), renamed.join("a").as_path());
}

#[test]
fn scoped_root_empty_rel_after_rename_rescopes_held_root() {
    let mut fs = fs_root();
    fs.put(node_path(&["f"]), TestNode::file(2, b"kept"));
    let (fs, root) = mount(fs);
    // Rename the held root's pathname, then re-scope to the held
    // root itself (`""` = self).
    let renamed = outside_dir().join("r-self");
    fs.lock().unwrap().rename(root_path(), &renamed);
    let scoped = root
        .open_scoped_root(&rel(""), FollowMode::NoFollow)
        .unwrap();
    assert_eq!(scoped.canonical_root(), renamed.as_path());
    assert_eq!(
        read_all(
            &mut scoped
                .open_regular(&rel("f"), FollowMode::NoFollow)
                .unwrap()
        ),
        "kept"
    );
}

#[test]
fn scoped_root_does_not_acquire_replaced_pathname() {
    let mut fs = fs_root();
    fs.put(node_path(&["a"]), TestNode::dir(2));
    fs.put(node_path(&["a", "f"]), TestNode::file(3, b"original"));
    let (fs, root) = mount(fs);
    // Move the held tree away, then plant a different tree at the
    // old pathname.
    let renamed = outside_dir().join("r-moved");
    fs.lock().unwrap().rename(root_path(), &renamed);
    {
        let mut g = fs.lock().unwrap();
        g.put(root_path(), TestNode::dir(90));
        g.put(node_path(&["a"]), TestNode::dir(91));
        g.put(node_path(&["a", "f"]), TestNode::file(92, b"IMPOSTER"));
    }
    // The held capability still sees its own subtree — never the
    // imposter planted at the old pathname.
    let scoped = root
        .open_scoped_root(&rel("a"), FollowMode::NoFollow)
        .unwrap();
    assert_eq!(
        read_all(
            &mut scoped
                .open_regular(&rel("f"), FollowMode::NoFollow)
                .unwrap()
        ),
        "original"
    );
    // A *fresh* path-level open does see the replacement — proving
    // the two authorities really differ.
    let ops = crate::vfs::mock::MockVfsOps::from_shared(&fs);
    let fresh =
        RootedDir::open_with_ops(root_path(), RootedOptions::default(), Arc::new(ops)).unwrap();
    assert_eq!(
        read_all(
            &mut fresh
                .open_regular(&rel("a/f"), FollowMode::NoFollow)
                .unwrap()
        ),
        "IMPOSTER"
    );
}

#[test]
fn scoped_root_without_handle_anchor_is_unsupported() {
    let mut fs = fs_root();
    fs.put(node_path(&["a"]), TestNode::dir(2));
    let (fs, root) = mount(fs);
    fs.lock().unwrap().set_canonicalize_unsupported(true);
    // The anchor cannot be derived from the handle → honest
    // Unsupported, never a path-string rebind or an empty anchor.
    assert_eq!(
        root.open_scoped_root(&rel("a"), FollowMode::NoFollow)
            .unwrap_err()
            .kind(),
        FileErrorKind::Unsupported
    );
}

#[test]
fn scoped_root_leaf_rules() {
    let mut fs = fs_root();
    fs.put(node_path(&["f"]), TestNode::file(2, b"x"));
    let (_fs, root) = mount(fs);
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

/// Puts `n` files `f00`…`f{n-1}` under `node_path(&[dir])`.
fn many_files(fs: &mut TestFs, dir: &str, n: usize) {
    fs.put(node_path(&[dir]), TestNode::dir(1000));
    for i in 0..n {
        fs.put(
            node_path(&[dir, &format!("f{i:02}")]),
            TestNode::file(1001 + i as u64, b"x"),
        );
    }
}

#[test]
fn read_dir_each_completes_with_nofollow_kinds() {
    let mut fs = fs_root();
    fs.put(node_path(&["d"]), TestNode::dir(2));
    fs.put(node_path(&["d", "f"]), TestNode::file(3, b"x"));
    fs.put(node_path(&["d", "l"]), TestNode::link(4, "f"));
    let (_fs, root) = mount(fs);
    let mut seen = Vec::new();
    let outcome = root
        .read_dir_each(&rel("d"), FollowMode::NoFollow, &mut |e| {
            seen.push((e.name.clone(), e.kind));
            EntryFlow::Continue
        })
        .unwrap();
    assert_eq!(outcome, EnumerationOutcome::Complete);
    seen.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(
        seen,
        vec![
            (OsStr::new("f").to_os_string(), NodeKind::File),
            (OsStr::new("l").to_os_string(), NodeKind::Symlink),
        ]
    );
}

#[test]
fn read_dir_each_stop_halts_enumeration_progress() {
    let mut fs = fs_root();
    many_files(&mut fs, "d", 100);
    let (fs, root) = mount(fs);
    fs.lock().unwrap().reset_stats();
    let mut count = 0u64;
    let outcome = root
        .read_dir_each(&rel("d"), FollowMode::NoFollow, &mut |_e| {
            count += 1;
            // Consumer's `remaining + 1` prefix bound.
            if count >= 4 {
                EntryFlow::Stop
            } else {
                EntryFlow::Continue
            }
        })
        .unwrap();
    assert_eq!(outcome, EnumerationOutcome::Stopped);
    assert_eq!(count, 4);
    // No enumeration progress after Stop: exactly the visited
    // prefix was pulled and classified — the other 96 entries were
    // never touched.
    let g = fs.lock().unwrap();
    assert_eq!(g.enum_pulled(), 4);
    assert_eq!(g.classify_calls(), 4);
}

#[test]
fn read_dir_each_immediate_stop() {
    let mut fs = fs_root();
    many_files(&mut fs, "d", 10);
    let (fs, root) = mount(fs);
    fs.lock().unwrap().reset_stats();
    // `max_entries = 0` analogue: the first callback already stops.
    let outcome = root
        .read_dir_each(&rel("d"), FollowMode::NoFollow, &mut |_e| EntryFlow::Stop)
        .unwrap();
    assert_eq!(outcome, EnumerationOutcome::Stopped);
    let g = fs.lock().unwrap();
    assert_eq!(g.enum_pulled(), 1);
    assert_eq!(g.classify_calls(), 1);
}

#[test]
fn read_dir_each_empty_dir_completes() {
    let mut fs = fs_root();
    fs.put(node_path(&["d"]), TestNode::dir(2));
    let (_fs, root) = mount(fs);
    let mut visited = 0u64;
    let outcome = root
        .read_dir_each(&rel("d"), FollowMode::NoFollow, &mut |_e| {
            visited += 1;
            EntryFlow::Continue
        })
        .unwrap();
    assert_eq!(outcome, EnumerationOutcome::Complete);
    assert_eq!(visited, 0);
}

#[test]
fn read_dir_each_calls_are_independent() {
    let mut fs = fs_root();
    many_files(&mut fs, "d", 7);
    let (fs, root) = mount(fs);
    fs.lock().unwrap().reset_stats();
    // First call stops early…
    let mut n = 0u64;
    root.read_dir_each(&rel("d"), FollowMode::NoFollow, &mut |_e| {
        n += 1;
        if n >= 2 {
            EntryFlow::Stop
        } else {
            EntryFlow::Continue
        }
    })
    .unwrap();
    // …the next call owns a fresh enumeration state and sees all.
    let mut seen = Vec::new();
    let outcome = root
        .read_dir_each(&rel("d"), FollowMode::NoFollow, &mut |e| {
            seen.push(e.name.clone());
            EntryFlow::Continue
        })
        .unwrap();
    assert_eq!(outcome, EnumerationOutcome::Complete);
    assert_eq!(seen.len(), 7);
}

#[test]
fn read_dir_each_os_failure_is_error_not_stopped() {
    let mut fs = fs_root();
    many_files(&mut fs, "d", 10);
    let (fs, root) = mount(fs);
    fs.lock().unwrap().reset_stats();
    fs.lock().unwrap().set_list_fail_at(3);
    let mut count = 0u64;
    let err = root
        .read_dir_each(&rel("d"), FollowMode::NoFollow, &mut |_e| {
            count += 1;
            EntryFlow::Continue
        })
        .unwrap_err();
    // An enumeration failure is `Err` — never reported `Stopped`.
    assert_eq!(err.kind(), FileErrorKind::Io);
    assert_eq!(count, 2);
    // Recovery: with the fault cleared the same call works.
    fs.lock().unwrap().set_list_fail_at(0);
    let mut seen = 0u64;
    let outcome = root
        .read_dir_each(&rel("d"), FollowMode::NoFollow, &mut |_e| {
            seen += 1;
            EntryFlow::Continue
        })
        .unwrap();
    assert_eq!(outcome, EnumerationOutcome::Complete);
    assert_eq!(seen, 10);
}

#[test]
fn read_dir_each_visitor_panic_releases_enumeration_state() {
    let mut fs = fs_root();
    many_files(&mut fs, "d", 5);
    let (_fs, root) = mount(fs);
    let mut n = 0u64;
    let hit = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = root.read_dir_each(&rel("d"), FollowMode::NoFollow, &mut |_e| {
            n += 1;
            assert!(n != 2, "visitor boom");
            EntryFlow::Continue
        });
    }));
    assert!(hit.is_err());
    // The enumeration state was released during unwind — a fresh
    // call enumerates independently and completes.
    let mut seen = 0u64;
    let outcome = root
        .read_dir_each(&rel("d"), FollowMode::NoFollow, &mut |_e| {
            seen += 1;
            EntryFlow::Continue
        })
        .unwrap();
    assert_eq!(outcome, EnumerationOutcome::Complete);
    assert_eq!(seen, 5);
}

#[test]
fn read_dir_collects_identically() {
    let mut fs = fs_root();
    many_files(&mut fs, "d", 6);
    let (_fs, root) = mount(fs);
    // `read_dir` is the collect form of the same enumeration.
    let via_dir: Vec<_> = root
        .read_dir(&rel("d"), FollowMode::NoFollow)
        .unwrap()
        .into_iter()
        .map(|e| e.name)
        .collect();
    let mut via_each = Vec::new();
    let outcome = root
        .read_dir_each(&rel("d"), FollowMode::NoFollow, &mut |e| {
            via_each.push(e.name);
            EntryFlow::Continue
        })
        .unwrap();
    assert_eq!(outcome, EnumerationOutcome::Complete);
    assert_eq!(via_dir, via_each);
}
