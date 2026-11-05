//! Private OS-ops boundary for staging-tree filesystem work
//! (IO-IMP-10).
//!
//! Every OS interaction the extraction pipeline performs is funneled
//! through this trait family so platform backends stay swappable
//! implementation details and the pipeline can be driven by a
//! fault-injection mock in unit tests. The same private-adapter shape
//! as IO-01 `PlatformFileOps` is used deliberately: IO-IMP-02's
//! `OsFileOps` is `pub(crate)` to `ruviform-io-file` and scoped to
//! *file* publish — it cannot be shared here without confusing a
//! single-file transaction with a directory-tree transaction.
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

use std::fmt;
use std::io::{self, Write};
use std::path::Path;

/// Stable filesystem identity of a node: (volume, index).
///
/// unix: (`st_dev`, `st_ino`). Windows: `FILE_ID_INFO`
/// (`VolumeSerialNumber`, `FileId`). Compared for equality only —
/// the values are opaque and never interpreted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NodeIdentity {
    pub volume: u64,
    pub index: u128,
}

/// The default platform backend factory.
///
/// unix and Windows have real backends ([`crate::extract::unix`],
/// [`crate::extract::windows`]). Every other target reports
/// `Unsupported` rather than falling back to path-string traversal.
/// Shared via `Arc` so recovery handles carry the same backend the
/// session used.
pub(crate) fn platform_extract_ops() -> std::sync::Arc<dyn PlatformExtractOps> {
    os_extract_ops()
}

#[cfg(unix)]
fn os_extract_ops() -> std::sync::Arc<dyn PlatformExtractOps> {
    std::sync::Arc::new(crate::extract::unix::UnixExtractOps)
}

#[cfg(windows)]
fn os_extract_ops() -> std::sync::Arc<dyn PlatformExtractOps> {
    std::sync::Arc::new(crate::extract::windows::WindowsExtractOps)
}

#[cfg(not(any(unix, windows)))]
fn os_extract_ops() -> std::sync::Arc<dyn PlatformExtractOps> {
    std::sync::Arc::new(UnsupportedExtractOps)
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
    /// Identity of this directory (its own handle).
    fn identity(&self) -> io::Result<NodeIdentity>;
    /// Identity of the child object named `name` without following
    /// links (`AT_SYMLINK_NOFOLLOW` / `FILE_OPEN_REPARSE_POINT`) —
    /// for a link this reports the *link's* identity.
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
pub(crate) fn staging_name(prefix: &str) -> String {
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
pub(crate) const MAX_NAME_ATTEMPTS: u32 = 64;

/// Backend for targets without a real adapter.
#[cfg(not(any(unix, windows)))]
#[derive(Debug)]
struct UnsupportedExtractOps;

#[cfg(not(any(unix, windows)))]
impl PlatformExtractOps for UnsupportedExtractOps {
    fn create_staging_root(&self, _parent: &Path) -> io::Result<Box<dyn StagingRoot>> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "ruviform-io-archive: secure extraction has no OS backend for this target",
        ))
    }
    fn open_staging_root(&self, _path: &Path) -> io::Result<Box<dyn StagingRoot>> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "ruviform-io-archive: secure extraction has no OS backend for this target",
        ))
    }
}

/// Test-only scripted backend for fault injection.
#[cfg(test)]
pub(crate) mod mock {
    use super::*;
    use std::collections::BTreeMap;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

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
                MockStep::ForeignIdentity => Ok(NodeIdentity {
                    volume: 99,
                    index: 99,
                }),
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
                MockStep::ForeignIdentity => Ok(NodeIdentity {
                    volume: 99,
                    index: 99,
                }),
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
