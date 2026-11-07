//! Test-only scripted vfs backend: a shared mutable pseudo-FS tree
//! that unit tests can mutate between calls (leaf/intermediate
//! swaps, link plant, delete+recreate) to drive resolve rules
//! deterministically and OS-independently.
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
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use super::{NodeIdentity, NodeTypeTag, VfsDir, VfsEntry, VfsKind, VfsOps, VfsProbe};
use crate::rooted::IdentityStrength;

/// One scripted node in the pseudo filesystem.
#[derive(Debug, Clone)]
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

/// The shared mutable pseudo-FS: absolute fake path → node.
#[derive(Debug, Default)]
pub(crate) struct TestFs {
    pub nodes: BTreeMap<PathBuf, TestNode>,
    /// Temp files materialized for `open_regular_leaf` (removed on
    /// drop best-effort).
    temps: Vec<PathBuf>,
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

    /// Inserts a node at `path`.
    pub(crate) fn put(&mut self, path: impl Into<PathBuf>, node: TestNode) {
        self.nodes.insert(path.into(), node);
    }

    /// Removes the node at `path`, if any.
    pub(crate) fn remove(&mut self, path: impl Into<PathBuf>) -> Option<TestNode> {
        self.nodes.remove(&path.into())
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
    fn child(&self, name: &OsStr) -> PathBuf {
        self.path.join(name)
    }
}

impl VfsDir for MockVfsDir {
    fn path(&self) -> &Path {
        &self.path
    }

    fn identity(&self) -> io::Result<NodeIdentity> {
        let fs = self.fs.lock().unwrap();
        fs.nodes.get(&self.path).map(|n| n.id).ok_or_else(not_found)
    }

    fn stat_self(&self) -> io::Result<VfsProbe> {
        let fs = self.fs.lock().unwrap();
        fs.nodes
            .get(&self.path)
            .map(TestNode::probe)
            .ok_or_else(not_found)
    }

    fn canonicalize_handle(&self) -> io::Result<PathBuf> {
        Ok(self.path.clone())
    }

    fn probe(&self, name: &OsStr) -> io::Result<VfsProbe> {
        let fs = self.fs.lock().unwrap();
        fs.nodes
            .get(&self.child(name))
            .map(TestNode::probe)
            .ok_or_else(not_found)
    }

    fn open_dir(&self, name: &OsStr) -> io::Result<Box<dyn VfsDir>> {
        let fs = self.fs.lock().unwrap();
        let path = self.child(name);
        match fs.nodes.get(&path) {
            Some(n) if n.kind == VfsKind::Dir => Ok(Box::new(MockVfsDir {
                fs: self.fs.clone(),
                path,
            })),
            Some(_) => Err(not_a_dir()),
            None => Err(not_found()),
        }
    }

    fn open_regular_leaf(&self, name: &OsStr) -> io::Result<(File, VfsProbe)> {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let (data, probe) = {
            let fs = self.fs.lock().unwrap();
            let n = fs.nodes.get(&self.child(name)).ok_or_else(not_found)?;
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
        let fs = self.fs.lock().unwrap();
        match fs.nodes.get(&self.child(name)) {
            Some(n) if n.kind == VfsKind::Symlink => Ok(n.target.clone()),
            Some(_) => Err(invalid_input("mock: not a link")),
            None => Err(not_found()),
        }
    }

    fn list(&self) -> io::Result<Vec<VfsEntry>> {
        let fs = self.fs.lock().unwrap();
        if !fs
            .nodes
            .get(&self.path)
            .is_some_and(|n| n.kind == VfsKind::Dir)
        {
            return Err(not_a_dir());
        }
        let mut out = Vec::new();
        for (path, node) in &fs.nodes {
            if path.parent() == Some(self.path.as_path())
                && let Some(name) = path.file_name()
            {
                out.push(VfsEntry {
                    name: name.to_os_string(),
                    kind: node.kind,
                });
            }
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    fn create_dir(&self, name: &OsStr) -> io::Result<Box<dyn VfsDir>> {
        let mut fs = self.fs.lock().unwrap();
        let path = self.child(name);
        if fs.nodes.contains_key(&path) {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "mock: name taken",
            ));
        }
        let index = u64::try_from(fs.nodes.len() + 10_000).unwrap_or(u64::MAX);
        fs.nodes.insert(path.clone(), TestNode::dir(index));
        Ok(Box::new(MockVfsDir {
            fs: self.fs.clone(),
            path,
        }))
    }

    fn create_file(&self, name: &OsStr) -> io::Result<File> {
        let mut fs = self.fs.lock().unwrap();
        let path = self.child(name);
        if fs.nodes.contains_key(&path) {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "mock: name taken",
            ));
        }
        let index = u64::try_from(fs.nodes.len() + 10_000).unwrap_or(u64::MAX);
        fs.nodes.insert(path.clone(), TestNode::file(index, b""));
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
        fs.nodes
            .remove(&self.child(name))
            .map_or_else(|| Err(not_found()), |_| Ok(()))
    }

    fn remove_dir(&self, name: &OsStr) -> io::Result<()> {
        self.remove_file(name)
    }
}
