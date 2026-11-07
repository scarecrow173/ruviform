//! Private OS-ops boundary for staging-tree filesystem work
//! (IO-IMP-10), adapted onto the shared `ruviform-io-file::vfs`
//! backend.
//!
//! Every OS interaction the extraction pipeline performs is funneled
//! through this trait family so platform backends stay swappable
//! implementation details and the pipeline can be driven by a
//! fault-injection mock in unit tests. The real backend is the
//! workspace-internal vfs layer in `ruviform-io-file` — the same
//! authoritative implementation of handle/fd-relative traversal,
//! no-follow opens, exclusive create, and stable node identity that
//! the rooted-read capability uses. There is no per-crate OS backend
//! here anymore: `OsExtractOps` below is a thin adapter translating
//! the archive's `&str`-named staging surface onto `VfsDir`
//! component operations.
//!
//! Guarantees the trait family is built around (adapters must provide
//! them as *operations*, never as `exists()`-then-act checks):
//!
//! - **Handle/fd-relative traversal.** Every child operation takes a
//!   single component name and executes relative to an open directory
//!   handle (`openat`/`RootDirectory`) — path strings are never
//!   re-resolved below the trusted parent.
//! - **No-follow.** A name that is a symlink/reparse point is never
//!   followed by the adapter: opens fail or address the link object
//!   itself (`O_NOFOLLOW`, `OBJ_DONT_REPARSE` +
//!   `FILE_OPEN_REPARSE_POINT`).
//! - **Exclusive create.** `create_dir`/`create_file` fail
//!   `AlreadyExists` when the name is already taken (including by a
//!   link) — no overwrite, ever.
//! - **Identity.** `identity`/`probe_identity` report a stable
//!   (volume, file-index) pair so cleanup can refuse to delete objects
//!   that are not the ones the session created.
//!
//! **RT contract:** non-RT only — every operation may block on
//! filesystem I/O and allocate.

use std::ffi::{OsStr, OsString};
use std::fmt;
use std::fs::File;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub(crate) use ruviform_io_file::vfs::{NodeIdentity, NodeTypeTag};
use ruviform_io_file::vfs::{VfsDir, VfsOps, platform_vfs_ops};

/// The default platform backend factory.
///
/// The vfs factory installs the unix/Windows backend as appropriate
/// (including the pre-RS1 `OBJ_DONT_REPARSE` version gate); every
/// other target reports `Unsupported` per-operation rather than
/// falling back to path-string traversal. Shared via `Arc` so
/// recovery handles carry the same backend the session used.
pub(crate) fn platform_extract_ops() -> Arc<dyn PlatformExtractOps> {
    Arc::new(OsExtractOps {
        ops: platform_vfs_ops(),
    })
}

/// Factory surface: creates the isolated staging root.
pub(crate) trait PlatformExtractOps: Send + Sync + fmt::Debug {
    /// Creates a fresh, uniquely named staging root inside `parent`
    /// (the caller's trusted directory, resolved to an absolute path
    /// first). Exclusive-create semantics — the name is generated
    /// until `create_dir`-equivalent succeeds, so a pre-existing or
    /// concurrently planted candidate never gets opened.
    ///
    /// `io::Error` with [`io::ErrorKind::Unsupported`] reports a
    /// platform/FS that cannot provide the required guarantees.
    fn create_staging_root(&self, parent: &Path) -> io::Result<Box<dyn StagingRoot>>;

    /// Opens an existing staging root by path for recovery
    /// (no-follow on the final component). Used by
    /// `ExtractionRecoveryHandle` `inspect`/`cleanup` after the
    /// session handle was released.
    fn open_staging_root(&self, path: &Path) -> io::Result<Box<dyn StagingRoot>>;
}

/// An open directory handle inside the staging tree. All child
/// operations are relative to *this* handle and take a single
/// component name.
pub(crate) trait StagingDir: Send + fmt::Debug {
    /// Diagnostic path for messages — never an identity proof.
    fn path(&self) -> &Path;
    /// Identity of this directory (its own handle; `tag` = `Dir`,
    /// `discriminator` = 0).
    fn identity(&self) -> io::Result<NodeIdentity>;
    /// Identity of the child object named `name` without following
    /// links (`AT_SYMLINK_NOFOLLOW` / `FILE_OPEN_REPARSE_POINT`) —
    /// for a link this reports the *link's* identity (`tag` = `Other`).
    fn probe_identity(&self, name: &str) -> io::Result<NodeIdentity>;
    /// Creates a child directory exclusively; `AlreadyExists` when
    /// taken (including by a link).
    fn create_dir(&self, name: &str) -> io::Result<Box<dyn StagingDir>>;
    /// Opens an existing child directory; fails when the name is not
    /// a real directory or is a link/reparse point.
    fn open_dir(&self, name: &str) -> io::Result<Box<dyn StagingDir>>;
    /// Creates a child regular file exclusively; `AlreadyExists` when
    /// taken (including by a link).
    fn create_file(&self, name: &str) -> io::Result<Box<dyn StagingFile>>;
    /// Deletes a child file by name — deletes a link itself, never
    /// follows it.
    fn remove_file(&self, name: &str) -> io::Result<()>;
    /// Deletes a child *empty* directory by name.
    fn remove_dir(&self, name: &str) -> io::Result<()>;
}

/// The staging root: a [`StagingDir`] that also owns the handle to
/// its trusted parent so it can remove itself.
pub(crate) trait StagingRoot: StagingDir {
    /// Deletes this (empty) staging root via its parent handle.
    /// Implementations must not delete anything else and must not
    /// follow a swapped-in link at the recorded name.
    fn remove_root(self: Box<Self>) -> io::Result<()>;
}

/// An open staging file — `Write` plus identity.
pub(crate) trait StagingFile: Write + Send + fmt::Debug {
    fn identity(&self) -> io::Result<NodeIdentity>;
}

/// Shared helper: generates a collision-resistant staging name
/// `<prefix>-<16 hex>` (time ⊕ pid ⊕ counter, not cryptographic —
/// `create_new` retries cover collisions).
fn staging_name(prefix: &str) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_nanos()).unwrap_or(u64::MAX));
    let mut x = nanos ^ u64::from(std::process::id()).rotate_left(23);
    x ^= COUNTER
        .fetch_add(1, Ordering::Relaxed)
        .wrapping_mul(0x9E37_79B9_7F4A_7C15);
    x ^= x >> 33;
    x = x.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    x ^= x >> 33;
    format!("{prefix}-{x:016x}")
}

/// Maximum exclusive-name attempts before giving up.
const MAX_NAME_ATTEMPTS: u32 = 64;

// ------------------------------------------------------------------
// The real backend: archive surface → vfs adapter.
// ------------------------------------------------------------------

/// [`PlatformExtractOps`] backed by the shared vfs layer.
#[derive(Debug)]
struct OsExtractOps {
    ops: Arc<dyn VfsOps>,
}

impl PlatformExtractOps for OsExtractOps {
    /// Creates the staging root: open trusted parent → exclusive
    /// `create_dir` of a unique name (unix `mkdirat(0700)`; Windows
    /// `NtCreateFile(FILE_CREATE|FILE_DIRECTORY_FILE|
    /// FILE_OPEN_REPARSE_POINT)`), retried across name collisions.
    /// The returned root keeps the parent handle so `remove_root`
    /// can re-verify and delete itself.
    fn create_staging_root(&self, parent: &Path) -> io::Result<Box<dyn StagingRoot>> {
        let parent = std::path::absolute(parent)?;
        let parent_dir = self.ops.open_dir_path(&parent)?;
        for _ in 0..MAX_NAME_ATTEMPTS {
            let name = staging_name("rvf-staging");
            match parent_dir.create_dir(OsStr::new(&name)) {
                Ok(dir) => {
                    return Ok(Box::new(OsStagingDir {
                        dir,
                        parent: Some(ParentRef {
                            dir: parent_dir,
                            name: OsString::from(name),
                        }),
                        ops: self.ops.clone(),
                    }));
                }
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
                Err(e) => return Err(e),
            }
        }
        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "ruviform-io-archive: staging name space exhausted",
        ))
    }

    /// Re-opens an existing staging root by path (final component
    /// opened no-follow — `O_NOFOLLOW`/`FILE_OPEN_REPARSE_POINT`,
    /// never a path-level open of the leaf) for recovery handle
    /// `inspect`/`cleanup`.
    fn open_staging_root(&self, path: &Path) -> io::Result<Box<dyn StagingRoot>> {
        let path = std::path::absolute(path)?;
        let name = path
            .file_name()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no file name"))?
            .to_os_string();
        let parent_path = path
            .parent()
            .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
        let parent_dir = self.ops.open_dir_path(&parent_path)?;
        let dir = parent_dir.open_dir(&name)?;
        Ok(Box::new(OsStagingDir {
            dir,
            parent: Some(ParentRef {
                dir: parent_dir,
                name,
            }),
            ops: self.ops.clone(),
        }))
    }
}

/// An open staging directory backed by a [`VfsDir`] handle.
/// `parent` is set only on the root so it can remove itself.
#[derive(Debug)]
struct OsStagingDir {
    dir: Box<dyn VfsDir>,
    parent: Option<ParentRef>,
    ops: Arc<dyn VfsOps>,
}

/// The staging root's trusted parent handle + its own leaf name.
#[derive(Debug)]
struct ParentRef {
    dir: Box<dyn VfsDir>,
    name: OsString,
}

impl StagingDir for OsStagingDir {
    fn path(&self) -> &Path {
        self.dir.path()
    }

    fn identity(&self) -> io::Result<NodeIdentity> {
        self.dir.identity()
    }

    fn probe_identity(&self, name: &str) -> io::Result<NodeIdentity> {
        self.dir.probe(OsStr::new(name)).map(|p| p.identity)
    }

    fn create_dir(&self, name: &str) -> io::Result<Box<dyn StagingDir>> {
        let dir = self.dir.create_dir(OsStr::new(name))?;
        Ok(Box::new(OsStagingDir {
            dir,
            parent: None,
            ops: self.ops.clone(),
        }))
    }

    fn open_dir(&self, name: &str) -> io::Result<Box<dyn StagingDir>> {
        let dir = self.dir.open_dir(OsStr::new(name))?;
        Ok(Box::new(OsStagingDir {
            dir,
            parent: None,
            ops: self.ops.clone(),
        }))
    }

    fn create_file(&self, name: &str) -> io::Result<Box<dyn StagingFile>> {
        let file = self.dir.create_file(OsStr::new(name))?;
        Ok(Box::new(OsStagingFile {
            file,
            ops: self.ops.clone(),
        }))
    }

    fn remove_file(&self, name: &str) -> io::Result<()> {
        self.dir.remove_file(OsStr::new(name))
    }

    fn remove_dir(&self, name: &str) -> io::Result<()> {
        self.dir.remove_dir(OsStr::new(name))
    }
}

impl StagingRoot for OsStagingDir {
    /// Deletes the staging root via its parent dir handle. Re-checks
    /// the recorded name's identity against this handle's own
    /// identity first: a swapped-in link or foreign directory at the
    /// name reports an error instead of being deleted.
    fn remove_root(self: Box<Self>) -> io::Result<()> {
        let Some(parent) = &self.parent else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "ruviform-io-archive: remove_root on a non-root directory",
            ));
        };
        let expected = self.dir.identity()?;
        let observed = parent.dir.probe(&parent.name).map(|p| p.identity)?;
        if observed != expected {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "ruviform-io-archive: staging root identity changed; refusing removal",
            ));
        }
        parent.dir.remove_dir(&parent.name)
    }
}

/// An open staging file backed by a vfs-created `File` (unix
/// `O_WRONLY|O_CREAT|O_EXCL|O_NOFOLLOW, 0600`; Windows
/// `NtCreateFile(FILE_CREATE)` + `FILE_GENERIC_WRITE|
/// FILE_READ_ATTRIBUTES` + `ShareAccess=0`).
#[derive(Debug)]
struct OsStagingFile {
    file: File,
    ops: Arc<dyn VfsOps>,
}

impl Write for OsStagingFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.file.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

impl StagingFile for OsStagingFile {
    fn identity(&self) -> io::Result<NodeIdentity> {
        self.ops.stat_file(&self.file).map(|p| p.identity)
    }
}

/// Test-only scripted backend for fault injection.
#[cfg(test)]
pub(crate) mod mock {
    use super::*;
    use std::collections::BTreeMap;
    use std::sync::Mutex;

    /// What the mock should do at the next matching call.
    #[derive(Debug, Clone)]
    pub(crate) enum MockStep {
        Ok,
        Fail(io::ErrorKind, &'static str),
        /// `identity`/`probe_identity` returns a foreign id.
        ForeignIdentity,
    }

    /// Identity the mock always assigns to the staging root.
    pub(crate) const ROOT_ID: NodeIdentity = NodeIdentity {
        volume: 7,
        index: 1,
        tag: NodeTypeTag::Dir,
        discriminator: 0,
        handle: None,
    };

    /// What `ForeignIdentity` scripts report — never equal to any
    /// recorded identity.
    const FOREIGN_ID: NodeIdentity = NodeIdentity {
        volume: 99,
        index: 99,
        tag: NodeTypeTag::Other,
        discriminator: 99,
        handle: None,
    };

    #[derive(Debug, Default)]
    pub(crate) struct MockScript {
        /// `op` → FIFO of scripted outcomes.
        pub steps: BTreeMap<&'static str, std::collections::VecDeque<MockStep>>,
        /// Per-op call counts (assert no-fs-call plans, etc).
        pub calls: BTreeMap<&'static str, u32>,
        /// Next identity handed out for created nodes.
        pub next_id: u64,
        /// "On disk" nodes: path → identity (created/deleted tracked).
        pub nodes: BTreeMap<PathBuf, NodeIdentity>,
    }

    impl MockScript {
        fn pop(&mut self, op: &'static str) -> MockStep {
            *self.calls.entry(op).or_insert(0) += 1;
            self.steps
                .get_mut(op)
                .and_then(std::collections::VecDeque::pop_front)
                .unwrap_or(MockStep::Ok)
        }
    }

    #[derive(Debug)]
    pub(crate) struct MockExtractOps {
        pub state: Arc<Mutex<MockScript>>,
    }

    impl MockExtractOps {
        pub fn new() -> (Self, Arc<Mutex<MockScript>>) {
            let state = Arc::new(Mutex::new(MockScript::default()));
            (
                Self {
                    state: state.clone(),
                },
                state,
            )
        }
    }

    impl PlatformExtractOps for MockExtractOps {
        fn create_staging_root(&self, parent: &Path) -> io::Result<Box<dyn StagingRoot>> {
            let mut s = self.state.lock().unwrap();
            match s.pop("create_staging_root") {
                MockStep::Ok => {
                    let path = parent.join("rvf-staging-mock");
                    s.nodes.insert(path.clone(), ROOT_ID);
                    Ok(Box::new(MockDir {
                        state: self.state.clone(),
                        path,
                        id: ROOT_ID,
                    }))
                }
                MockStep::Fail(kind, msg) => Err(io::Error::new(kind, msg)),
                MockStep::ForeignIdentity => unreachable!(),
            }
        }
        fn open_staging_root(&self, path: &Path) -> io::Result<Box<dyn StagingRoot>> {
            let mut s = self.state.lock().unwrap();
            match s.pop("open_staging_root") {
                MockStep::Ok => match s.nodes.get(path) {
                    Some(id) => Ok(Box::new(MockDir {
                        state: self.state.clone(),
                        path: path.to_path_buf(),
                        id: *id,
                    })),
                    None => Err(io::Error::new(
                        io::ErrorKind::NotFound,
                        "mock: no such staging root",
                    )),
                },
                MockStep::Fail(kind, msg) => Err(io::Error::new(kind, msg)),
                MockStep::ForeignIdentity => unreachable!(),
            }
        }
    }

    #[derive(Debug)]
    struct MockDir {
        state: Arc<Mutex<MockScript>>,
        path: PathBuf,
        id: NodeIdentity,
    }

    impl StagingDir for MockDir {
        fn path(&self) -> &Path {
            &self.path
        }
        fn identity(&self) -> io::Result<NodeIdentity> {
            let mut s = self.state.lock().unwrap();
            match s.pop("identity") {
                MockStep::Ok => Ok(self.id),
                MockStep::ForeignIdentity => Ok(FOREIGN_ID),
                MockStep::Fail(kind, msg) => Err(io::Error::new(kind, msg)),
            }
        }
        fn probe_identity(&self, name: &str) -> io::Result<NodeIdentity> {
            let mut s = self.state.lock().unwrap();
            match s.pop("probe_identity") {
                MockStep::Ok => match s.nodes.get(&self.path.join(name)) {
                    Some(id) => Ok(*id),
                    None => Err(io::Error::new(
                        io::ErrorKind::NotFound,
                        "mock: no such child",
                    )),
                },
                MockStep::ForeignIdentity => Ok(FOREIGN_ID),
                MockStep::Fail(kind, msg) => Err(io::Error::new(kind, msg)),
            }
        }
        fn create_dir(&self, name: &str) -> io::Result<Box<dyn StagingDir>> {
            let mut s = self.state.lock().unwrap();
            match s.pop("create_dir") {
                MockStep::Ok => {
                    s.next_id += 1;
                    let id = NodeIdentity {
                        volume: 7,
                        index: u128::from(s.next_id),
                        tag: NodeTypeTag::Dir,
                        discriminator: 0,
                        handle: None,
                    };
                    let path = self.path.join(name);
                    s.nodes.insert(path.clone(), id);
                    Ok(Box::new(MockDir {
                        state: self.state.clone(),
                        path,
                        id,
                    }))
                }
                MockStep::Fail(kind, msg) => Err(io::Error::new(kind, msg)),
                MockStep::ForeignIdentity => unreachable!(),
            }
        }
        fn open_dir(&self, name: &str) -> io::Result<Box<dyn StagingDir>> {
            let mut s = self.state.lock().unwrap();
            match s.pop("open_dir") {
                MockStep::Ok => {
                    let path = self.path.join(name);
                    match s.nodes.get(&path) {
                        Some(id) => Ok(Box::new(MockDir {
                            state: self.state.clone(),
                            path,
                            id: *id,
                        })),
                        None => Err(io::Error::new(io::ErrorKind::NotFound, "mock: no such dir")),
                    }
                }
                MockStep::Fail(kind, msg) => Err(io::Error::new(kind, msg)),
                MockStep::ForeignIdentity => unreachable!(),
            }
        }
        fn create_file(&self, name: &str) -> io::Result<Box<dyn StagingFile>> {
            let mut s = self.state.lock().unwrap();
            match s.pop("create_file") {
                MockStep::Ok => {
                    s.next_id += 1;
                    let id = NodeIdentity {
                        volume: 7,
                        index: u128::from(s.next_id),
                        tag: NodeTypeTag::File,
                        discriminator: s.next_id,
                        handle: None,
                    };
                    s.nodes.insert(self.path.join(name), id);
                    Ok(Box::new(MockFile {
                        id,
                        written: Vec::new(),
                    }))
                }
                MockStep::Fail(kind, msg) => Err(io::Error::new(kind, msg)),
                MockStep::ForeignIdentity => unreachable!(),
            }
        }
        fn remove_file(&self, name: &str) -> io::Result<()> {
            let mut s = self.state.lock().unwrap();
            match s.pop("remove_file") {
                MockStep::Ok => {
                    s.nodes.remove(&self.path.join(name));
                    Ok(())
                }
                MockStep::Fail(kind, msg) => Err(io::Error::new(kind, msg)),
                MockStep::ForeignIdentity => unreachable!(),
            }
        }
        fn remove_dir(&self, name: &str) -> io::Result<()> {
            let mut s = self.state.lock().unwrap();
            match s.pop("remove_dir") {
                MockStep::Ok => {
                    s.nodes.remove(&self.path.join(name));
                    Ok(())
                }
                MockStep::Fail(kind, msg) => Err(io::Error::new(kind, msg)),
                MockStep::ForeignIdentity => unreachable!(),
            }
        }
    }

    impl StagingRoot for MockDir {
        fn remove_root(self: Box<Self>) -> io::Result<()> {
            let mut s = self.state.lock().unwrap();
            match s.pop("remove_root") {
                MockStep::Ok => {
                    s.nodes.remove(&self.path);
                    Ok(())
                }
                MockStep::Fail(kind, msg) => Err(io::Error::new(kind, msg)),
                MockStep::ForeignIdentity => unreachable!(),
            }
        }
    }

    #[derive(Debug)]
    struct MockFile {
        id: NodeIdentity,
        written: Vec<u8>,
    }

    impl Write for MockFile {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.written.extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl StagingFile for MockFile {
        fn identity(&self) -> io::Result<NodeIdentity> {
            Ok(self.id)
        }
    }

    /// Helper for tests: enqueue scripted outcomes for an op.
    pub(crate) fn script(state: &Arc<Mutex<MockScript>>, op: &'static str, steps: Vec<MockStep>) {
        state.lock().unwrap().steps.insert(op, steps.into());
    }
}
