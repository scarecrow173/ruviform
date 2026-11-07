//! Test-only scripted vfs backend: a shared mutable pseudo-FS tree
//! that unit tests can mutate between calls (leaf/intermediate
//! swaps, link plant, delete+recreate, rename) to drive resolve
//! rules deterministically and OS-independently.
//!
//! `MockVfsDir` holds a node `Arc`, not a path — an open directory
//! handle stays valid across rename/replacement of its pathname,
//! exactly like a real dirfd/HANDLE (the node is the handle; the
//! path map entry is only the *name* under which lookups find it).
//! `TestFs::rename` moves a subtree while keeping node identity, so
//! tests can model "the held root was renamed/replaced out from
//! under us" without path-string rebinding.
//!
//! `outside = true` marks a node standing in for a root-external
//! object: its `data`/facts must never appear in an API result —
//! tests assert on that sentinel.
//!
//! `open_regular_leaf` materializes node `data` into a real temp
//! file so `RootedFile` byte assertions work against a genuine
//! `File`; the node's own probe is returned alongside, as the real
//! backends guarantee.

use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::fs::File;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use super::{
    ListingOutcome, NodeIdentity, NodeTypeTag, VfsDir, VfsEntry, VfsKind, VfsOps, VfsProbe,
    VisitFlow,
};
use crate::rooted::IdentityStrength;

/// One scripted node in the pseudo filesystem.
#[derive(Debug)]
pub(crate) struct TestNode {
    pub kind: VfsKind,
    pub id: NodeIdentity,
    /// Link target for `Symlink` nodes (OS-raw form).
    pub target: OsString,
    /// File payload (readable via `open_regular_leaf`).
    pub data: Vec<u8>,
    /// `true` = stands in for a root-external object.
    pub outside: bool,
    pub strength: IdentityStrength,
    /// The node's current path — assigned by `put` and updated by
    /// `rename`. Directory children are looked up under this path in
    /// the shared map; held handles follow the node across rename.
    path: Mutex<PathBuf>,
    /// A directory node's own child table — the node's namespace,
    /// independent of the global path map: a held handle keeps
    /// addressing *its own* entries across rename/replacement of the
    /// name, like a real dirfd. Non-dirs leave it empty. Guarded by
    /// its own mutex; always taken after the `TestFs` lock.
    children: Mutex<BTreeMap<OsString, Arc<TestNode>>>,
}

impl TestNode {
    pub(crate) fn new(kind: VfsKind, index: u64, strength: IdentityStrength) -> Self {
        Self {
            kind,
            id: NodeIdentity {
                volume: 7,
                index: u128::from(index),
                tag: match kind {
                    VfsKind::File => NodeTypeTag::File,
                    VfsKind::Dir => NodeTypeTag::Dir,
                    _ => NodeTypeTag::Other,
                },
                discriminator: index,
                handle: None,
            },
            target: OsString::new(),
            data: Vec::new(),
            outside: false,
            strength,
            path: Mutex::new(PathBuf::new()),
            children: Mutex::new(BTreeMap::new()),
        }
    }

    pub(crate) fn dir(index: u64) -> Self {
        Self::new(VfsKind::Dir, index, IdentityStrength::Strong)
    }

    pub(crate) fn file(index: u64, data: &[u8]) -> Self {
        let mut n = Self::new(VfsKind::File, index, IdentityStrength::Strong);
        n.data = data.to_vec();
        n
    }

    pub(crate) fn link(index: u64, target: &str) -> Self {
        let mut n = Self::new(VfsKind::Symlink, index, IdentityStrength::Strong);
        n.target = OsString::from(target);
        n
    }

    pub(crate) fn outside_file(index: u64, data: &[u8]) -> Self {
        let mut n = Self::file(index, data);
        n.outside = true;
        n
    }

    /// The node's current location (set by `put`/`rename`).
    fn current_path(&self) -> PathBuf {
        self.path.lock().unwrap().clone()
    }

    fn probe(&self) -> VfsProbe {
        VfsProbe {
            identity: self.id,
            kind: self.kind,
            byte_len: match self.kind {
                VfsKind::File => Some(self.data.len() as u64),
                VfsKind::Symlink => Some(self.target.len() as u64),
                _ => None,
            },
            modified: None,
            strength: self.strength,
        }
    }
}

/// The shared mutable pseudo-FS: absolute fake path → node. The map
/// is the *namespace*; a `MockVfsDir` keeps its node alive across
/// rename/replacement of the name, like a real handle.
#[derive(Debug, Default)]
pub(crate) struct TestFs {
    pub nodes: BTreeMap<PathBuf, Arc<TestNode>>,
    /// Temp files materialized for `open_regular_leaf`/`create_file`
    /// (removed on drop best-effort).
    temps: Vec<PathBuf>,
    /// Entries pulled from `list_each` enumeration state so far —
    /// the observable "enumeration progress" counter. Counts pulls,
    /// not deliveries: a pull that hits injected failure or a
    /// vanished child is still counted.
    enum_pulled: AtomicU64,
    /// Per-entry classifications inside `list_each`.
    classify_calls: AtomicU64,
    /// `probe` invocations.
    probe_calls: AtomicU64,
    /// 1-based pull index at which `list_each` injects an error
    /// (`0` = no fault). Cumulative across calls — persists until
    /// `reset_stats`/`set_list_fail_at(0)`, so it fires on the first
    /// pull at-or-past the index, not per call.
    list_fail_at: AtomicU64,
    /// When set, `canonicalize_handle` reports `Unsupported`.
    canonicalize_unsupported: AtomicBool,
}

impl Drop for TestFs {
    fn drop(&mut self) {
        for p in self.temps.drain(..) {
            let _ = std::fs::remove_file(p);
        }
    }
}

impl TestFs {
    pub(crate) fn at(self) -> (Arc<Mutex<Self>>, MockVfsOps) {
        let fs = Arc::new(Mutex::new(self));
        (fs.clone(), MockVfsOps { fs: fs.clone() })
    }

    /// Inserts a node at `path`, wiring it into its parent's child
    /// table when the parent exists.
    pub(crate) fn put(&mut self, path: impl Into<PathBuf>, node: TestNode) {
        let path = path.into();
        *node.path.lock().unwrap() = path.clone();
        let node = Arc::new(node);
        if let Some(parent) = path.parent().and_then(|p| self.nodes.get(p))
            && let Some(name) = path.file_name()
        {
            parent
                .children
                .lock()
                .unwrap()
                .insert(name.to_os_string(), Arc::clone(&node));
        }
        self.nodes.insert(path, node);
    }

    /// Removes the node at `path`, if any — also unwires it from its
    /// parent's child table.
    pub(crate) fn remove(&mut self, path: impl Into<PathBuf>) -> Option<Arc<TestNode>> {
        let path = path.into();
        let removed = self.nodes.remove(&path)?;
        if let Some(parent) = path.parent().and_then(|p| self.nodes.get(p))
            && let Some(name) = path.file_name()
        {
            parent.children.lock().unwrap().remove(name);
        }
        Some(removed)
    }

    /// Renames the subtree at `from` to `to` — every node under
    /// `from` (inclusive) keeps its identity and moves with its
    /// parent, matching real `rename(2)`/`MoveFile` semantics: held
    /// handles keep addressing the same object at its new path.
    /// Panics on `to` inside `from` — invalid input, like `EINVAL`
    /// for a directory moved into its own subtree.
    pub(crate) fn rename(&mut self, from: impl Into<PathBuf>, to: impl Into<PathBuf>) {
        let from = from.into();
        let to = to.into();
        assert!(
            from != to && !to.starts_with(&from),
            "mock: cannot rename a path into itself/its own subtree"
        );
        // Rewire the moved root's entry in the parent child tables:
        // the name under old parent leaves, the new name lands under
        // the new parent (descendants keep their own child tables —
        // they are name-keyed and move with their node).
        if let Some(old_parent) = from.parent().and_then(|p| self.nodes.get(p))
            && let Some(name) = from.file_name()
        {
            old_parent.children.lock().unwrap().remove(name);
        }
        let moves: Vec<(PathBuf, PathBuf)> = self
            .nodes
            .keys()
            .filter(|p| p.starts_with(&from))
            .map(|p| {
                let rest = p.strip_prefix(&from).expect("prefix checked");
                (p.clone(), to.join(rest))
            })
            .collect();
        for (old, new) in moves {
            if let Some(node) = self.nodes.remove(&old) {
                *node.path.lock().unwrap() = new.clone();
                self.nodes.insert(new, node);
            }
        }
        if let Some(root) = self.nodes.get(&to).cloned()
            && let Some(parent) = to.parent().and_then(|p| self.nodes.get(p))
            && let Some(name) = to.file_name()
        {
            parent
                .children
                .lock()
                .unwrap()
                .insert(name.to_os_string(), root);
        }
    }

    /// Entries pulled by `list_each` (cumulative — includes pulls
    /// that injected a fault or hit a vanished child, so it can
    /// exceed the count actually delivered to visitors).
    pub(crate) fn enum_pulled(&self) -> u64 {
        self.enum_pulled.load(Ordering::Relaxed)
    }

    /// Per-entry classifications inside `list_each` (cumulative).
    pub(crate) fn classify_calls(&self) -> u64 {
        self.classify_calls.load(Ordering::Relaxed)
    }

    /// `probe` invocations (cumulative).
    pub(crate) fn probe_calls(&self) -> u64 {
        self.probe_calls.load(Ordering::Relaxed)
    }

    /// Resets the enumeration/probe counters.
    pub(crate) fn reset_stats(&self) {
        self.enum_pulled.store(0, Ordering::Relaxed);
        self.classify_calls.store(0, Ordering::Relaxed);
        self.probe_calls.store(0, Ordering::Relaxed);
    }

    /// Injects an `io` error when the cumulative pull counter
    /// reaches `n` (`0` disables). Persists across `list_each`
    /// calls — a second call fires on its first pull if the counter
    /// already passed `n`; call `reset_stats` first for per-call
    /// semantics.
    pub(crate) fn set_list_fail_at(&self, n: u64) {
        self.list_fail_at.store(n, Ordering::Relaxed);
    }

    /// Makes `canonicalize_handle` report `Unsupported` — models a
    /// backend that cannot derive a canonical path from a handle.
    pub(crate) fn set_canonicalize_unsupported(&self, on: bool) {
        self.canonicalize_unsupported.store(on, Ordering::Relaxed);
    }
}

fn not_found() -> io::Error {
    io::Error::new(io::ErrorKind::NotFound, "mock: no such node")
}

fn not_a_dir() -> io::Error {
    io::Error::new(io::ErrorKind::NotADirectory, "mock: not a directory")
}

fn invalid_input(msg: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, msg)
}

/// The scripted [`VfsOps`].
pub(crate) struct MockVfsOps {
    fs: Arc<Mutex<TestFs>>,
}

impl MockVfsOps {
    /// A second ops handle onto an already-live pseudo-FS — for
    /// tests that need a *fresh* path-level open next to a held
    /// capability on the same tree.
    pub(crate) fn from_shared(fs: &Arc<Mutex<TestFs>>) -> Self {
        Self { fs: fs.clone() }
    }
}

impl fmt::Debug for MockVfsOps {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MockVfsOps").finish_non_exhaustive()
    }
}

impl VfsOps for MockVfsOps {
    fn open_dir_path(&self, path: &Path) -> io::Result<Box<dyn VfsDir>> {
        let fs = self.fs.lock().unwrap();
        match fs.nodes.get(path) {
            Some(n) if n.kind == VfsKind::Dir => Ok(Box::new(MockVfsDir {
                fs: self.fs.clone(),
                node: Arc::clone(n),
                path: path.to_path_buf(),
            })),
            Some(_) => Err(not_a_dir()),
            None => Err(not_found()),
        }
    }

    fn stat_file(&self, _file: &File) -> io::Result<VfsProbe> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "mock: stat_file unsupported",
        ))
    }
}

struct MockVfsDir {
    fs: Arc<Mutex<TestFs>>,
    /// The handle — survives rename/replacement of its pathname.
    node: Arc<TestNode>,
    /// Diagnostic path snapshot (never an identity proof).
    path: PathBuf,
}

impl fmt::Debug for MockVfsDir {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MockVfsDir")
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

impl MockVfsDir {
    /// Path of a prospective child (for `create_*` addresses only —
    /// lookups must use `child_node` so a held handle addresses its
    /// own node namespace, not whatever the path map now names).
    fn child(&self, name: &OsStr) -> PathBuf {
        self.node.current_path().join(name)
    }

    /// The node's own child `name` — like a `*at` lookup on a real
    /// dirfd: follows the node, not the global path namespace.
    fn child_node(&self, name: &OsStr) -> io::Result<Arc<TestNode>> {
        self.node
            .children
            .lock()
            .unwrap()
            .get(name)
            .cloned()
            .ok_or_else(not_found)
    }
}

impl VfsDir for MockVfsDir {
    fn path(&self) -> &Path {
        &self.path
    }

    fn identity(&self) -> io::Result<NodeIdentity> {
        Ok(self.node.id)
    }

    fn stat_self(&self) -> io::Result<VfsProbe> {
        Ok(self.node.probe())
    }

    fn canonicalize_handle(&self) -> io::Result<PathBuf> {
        let fs = self.fs.lock().unwrap();
        if fs.canonicalize_unsupported.load(Ordering::Relaxed) {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "mock: handle canonicalization unsupported",
            ));
        }
        Ok(self.node.current_path())
    }

    fn probe(&self, name: &OsStr) -> io::Result<VfsProbe> {
        let fs = self.fs.lock().unwrap();
        fs.probe_calls.fetch_add(1, Ordering::Relaxed);
        Ok(self.child_node(name)?.probe())
    }

    fn open_dir(&self, name: &OsStr) -> io::Result<Box<dyn VfsDir>> {
        let node = self.child_node(name)?;
        if node.kind != VfsKind::Dir {
            return Err(not_a_dir());
        }
        let path = self.child(name);
        Ok(Box::new(MockVfsDir {
            fs: self.fs.clone(),
            node,
            path,
        }))
    }

    fn open_regular_leaf(&self, name: &OsStr) -> io::Result<(File, VfsProbe)> {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let (data, probe) = {
            let n = self.child_node(name)?;
            if n.kind != VfsKind::File {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "mock: not a regular file",
                ));
            }
            (n.data.clone(), n.probe())
        };
        // Materialize into a real temp file so `Read`/`Seek` work.
        let path = std::env::temp_dir().join(format!(
            "rvf-mock-{:x}-{}-{counter:x}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos()),
            counter = COUNTER.fetch_add(1, Ordering::Relaxed),
        ));
        let mut f = File::create(&path)?;
        f.write_all(&data)?;
        drop(f);
        let f = File::open(&path)?;
        self.fs.lock().unwrap().temps.push(path);
        Ok((f, probe))
    }

    fn read_link(&self, name: &OsStr) -> io::Result<OsString> {
        let n = self.child_node(name)?;
        if n.kind != VfsKind::Symlink {
            return Err(invalid_input("mock: not a link"));
        }
        Ok(n.target.clone())
    }

    fn list_each(
        &self,
        visit: &mut dyn FnMut(VfsEntry) -> VisitFlow,
    ) -> io::Result<ListingOutcome> {
        if self.node.kind != VfsKind::Dir {
            return Err(not_a_dir());
        }
        // Name snapshot of THIS node's children — the analogue of
        // one OS batch's worth of names (a test-double
        // simplification: a real fixed-size OS batch is capped,
        // this one covers the whole directory). Each entry is then
        // pulled, classified, and visited in turn so `Stop` halts
        // observable progress.
        let names: Vec<OsString> = self.node.children.lock().unwrap().keys().cloned().collect();
        for name in names {
            let entry = {
                let fs = self.fs.lock().unwrap();
                let pulled = fs.enum_pulled.fetch_add(1, Ordering::Relaxed) + 1;
                let fail_at = fs.list_fail_at.load(Ordering::Relaxed);
                if fail_at != 0 && pulled >= fail_at {
                    return Err(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        "mock: injected list failure",
                    ));
                }
                let node = self.node.children.lock().unwrap().get(&name).cloned();
                let Some(node) = node else {
                    // Child vanished between snapshot and pull — like
                    // a fstatat ENOENT mid-readdir, skip it.
                    continue;
                };
                fs.classify_calls.fetch_add(1, Ordering::Relaxed);
                VfsEntry {
                    name,
                    kind: node.kind,
                }
            };
            if visit(entry) == VisitFlow::Stop {
                return Ok(ListingOutcome::Stopped);
            }
        }
        Ok(ListingOutcome::Complete)
    }

    fn create_dir(&self, name: &OsStr) -> io::Result<Box<dyn VfsDir>> {
        let mut fs = self.fs.lock().unwrap();
        if self.node.children.lock().unwrap().contains_key(name) {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "mock: name taken",
            ));
        }
        let path = self.child(name);
        let index = u64::try_from(fs.nodes.len() + 10_000).unwrap_or(u64::MAX);
        let node = TestNode::dir(index);
        *node.path.lock().unwrap() = path.clone();
        let node = Arc::new(node);
        self.node
            .children
            .lock()
            .unwrap()
            .insert(name.to_os_string(), Arc::clone(&node));
        fs.nodes.insert(path.clone(), Arc::clone(&node));
        Ok(Box::new(MockVfsDir {
            fs: self.fs.clone(),
            node,
            path,
        }))
    }

    fn create_file(&self, name: &OsStr) -> io::Result<File> {
        let mut fs = self.fs.lock().unwrap();
        if self.node.children.lock().unwrap().contains_key(name) {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "mock: name taken",
            ));
        }
        let path = self.child(name);
        let index = u64::try_from(fs.nodes.len() + 10_000).unwrap_or(u64::MAX);
        let node = TestNode::file(index, b"");
        *node.path.lock().unwrap() = path.clone();
        let node = Arc::new(node);
        self.node
            .children
            .lock()
            .unwrap()
            .insert(name.to_os_string(), Arc::clone(&node));
        fs.nodes.insert(path, node);
        let path = std::env::temp_dir().join(format!(
            "rvf-mock-create-{:x}-{}",
            std::process::id(),
            fs.nodes.len()
        ));
        let f = File::create(&path)?;
        fs.temps.push(path);
        Ok(f)
    }

    fn remove_file(&self, name: &OsStr) -> io::Result<()> {
        let mut fs = self.fs.lock().unwrap();
        let removed = self.node.children.lock().unwrap().remove(name);
        match removed {
            Some(n) => {
                fs.nodes.remove(&n.current_path());
                Ok(())
            }
            None => Err(not_found()),
        }
    }

    fn remove_dir(&self, name: &OsStr) -> io::Result<()> {
        self.remove_file(name)
    }
}
