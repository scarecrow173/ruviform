//! Rooted read capability — handle-relative traversal that never
//! resolves a path string below the trusted root.
//!
//! [`RootedDir`] owns an open directory handle (the capability root)
//! and performs every component lookup, open, metadata query and
//! enumeration *relative to that handle*. The only trust boundary is
//! [`RootedDir::open`]'s own root resolution (open-first, then a
//! handle-derived canonical path where the OS offers one); afterwards
//! no path string is ever re-resolved inside the tree. A swapped
//! leaf, a swapped intermediate directory, or a link target pointing
//! outside the authority can produce `NotFound`/`EscapesRoot`/
//! `IdentityMismatch`/`NotRegularFile` — never the bytes or facts of
//! an object outside the held handles' reach.
//!
//! Authority is delegated to the acquired directory handles (inode
//! subtrees), not to continued path-namespace containment: a scoped
//! directory renamed outside the root after acquisition is still
//! legitimately read through its handle (standard capability
//! semantics — re-canonicalizing per operation would reintroduce the
//! TOCTOU this capability exists to remove).
//!
//! Guarantees are scoped to the filesystem tree the root inode
//! belongs to (unix mount/bind-mount crossings pass through as
//! kernel-namespace reality — documented limit, same as any
//! canonicalize+containment check).
//!
//! **RT contract:** non-RT only — filesystem I/O, allocation.
//!
//! # Examples
//!
//! ```no_run
//! use ruviform_io_file::rooted::{FollowMode, RelativePath, RootedDir};
//! use std::io::Read;
//! use std::path::Path;
//!
//! let root = RootedDir::open(Path::new("/srv/data"))?;
//! let rel = RelativePath::new("logs/today.txt")?;
//! let mut file = root.open_regular(&rel, FollowMode::WithinRoot)?;
//! let mut buf = String::new();
//! file.read_to_string(&mut buf)?;
//! assert_eq!(file.info().byte_len, Some(buf.len() as u64));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use std::ffi::{OsStr, OsString};
use std::fmt;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use crate::error::{FileError, FileErrorKind};
use crate::vfs::{
    self, ListingOutcome, VfsDir, VfsKind, VfsOps, VfsProbe, VisitFlow, platform_vfs_ops,
};

mod resolve;
#[cfg(test)]
mod tests;

/// Maximum byte length of a single path component (POSIX `NAME_MAX`).
const MAX_COMPONENT_BYTES: usize = 255;

/// Windows device names rejected regardless of case (the stem before
/// the first `.` is compared — `con.txt` is still `CON`).
const RESERVED_STEMS: &[&str] = &["CON", "PRN", "AUX", "NUL", "CLOCK$", "CONIN$", "CONOUT$"];

/// Characters Windows forbids in file names.
const FORBIDDEN_CHARS: &[char] = &['<', '>', '|', '"', '?', '*'];

fn invalid(message: &'static str) -> FileError {
    FileError::with_source(
        FileErrorKind::InvalidInput,
        io::Error::new(io::ErrorKind::InvalidInput, message),
    )
}

fn kind_err(kind: FileErrorKind) -> FileError {
    FileError::from_kind(kind)
}

/// The rooted boundary's `io::Error` → [`FileErrorKind`] mapping
/// (design table): `NotFound`/`NotADirectory`/`IsADirectory`/
/// `Unsupported`/`InvalidInput` keep dedicated kinds; a unix `ELOOP`
/// (a kernel-level link that reached a no-follow op) classifies as
/// `NotDirectory` — distinct from the resolve layer's own
/// `LoopDetected` hop exhaustion. Everything else is `Io` with the
/// raw error in `source`.
pub(super) fn ops_error(error: io::Error) -> FileError {
    #[cfg(unix)]
    if error.raw_os_error() == Some(libc::ELOOP) {
        return FileError::from_kind(FileErrorKind::NotDirectory);
    }
    let kind = match error.kind() {
        io::ErrorKind::NotFound => FileErrorKind::NotFound,
        io::ErrorKind::NotADirectory => FileErrorKind::NotDirectory,
        io::ErrorKind::IsADirectory => FileErrorKind::NotRegularFile,
        io::ErrorKind::Unsupported => FileErrorKind::Unsupported,
        io::ErrorKind::InvalidInput => FileErrorKind::InvalidInput,
        _ => FileErrorKind::Io,
    };
    FileError::with_source(kind, error)
}

/// State shared between the root and every scoped `RootedDir`
/// derived from it: the capability's canonical root anchor and the
/// resolve budgets. `Arc`-shared so `open_dir` never duplicates.
#[derive(Debug)]
struct RootShared {
    /// Canonical root path — the lexical anchor for absolute-link
    /// prefix policy only (the pinned inode is the real guarantee).
    canonical: PathBuf,
    /// `canonical` pre-split into normalized components for the
    /// two-state walk (`\\?\`/`/` markers removed).
    canonical_comps: Vec<OsString>,
    options: RootedOptions,
}

/// A read-only capability on a directory tree: the open root handle
/// plus the resolved ancestor chain of `self` (chain\[0\] is always
/// the capability root; `chain.last()` is this directory).
///
/// `Clone` is deliberately absent — a scoped clone is obtained via
/// [`RootedDir::open_dir`] with an empty [`RelativePath`], which
/// shares the chain `Arc`s without opening new handles.
///
/// **RT contract:** non-RT only.
#[derive(Debug)]
pub struct RootedDir {
    ops: Arc<dyn VfsOps>,
    /// `root → … → self` resolved ancestor chain; `chain[0]` is the
    /// capability root handle, `chain.last()` this directory. `Arc`
    /// sharing keeps scoped dirs free of extra fds/handles.
    chain: Vec<Arc<dyn VfsDir>>,
    shared: Arc<RootShared>,
}

impl RootedDir {
    /// Opens `path` and returns a rooted capability on it.
    ///
    /// The open happens **first** and the canonical root path is then
    /// taken from the opened handle where the OS supports it (Linux
    /// `/proc/self/fd/N` readlink, Windows
    /// `GetFinalPathNameByHandleW`, Apple `fcntl(F_GETPATH)`), keeping
    /// the (path → object, canonical string) pair near-atomic.
    /// Backends without handle-derived canonicalization fall back to
    /// `fs::canonicalize` — the only documented residual race in the
    /// capability.
    ///
    /// `path` must name a directory (`NotDirectory` otherwise); all
    /// later lookups are relative to the opened handle, never to the
    /// path string.
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// `NotFound`/`NotDirectory`/`Io` for the open itself,
    /// `Unsupported` on targets without a handle-relative no-follow
    /// backend (including pre-RS1 Windows).
    pub fn open(path: &Path) -> Result<Self, FileError> {
        Self::open_with(path, RootedOptions::default())
    }

    /// [`open`](Self::open) with explicit resolve budgets.
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// Same as [`open`](Self::open).
    pub fn open_with(path: &Path, options: RootedOptions) -> Result<Self, FileError> {
        Self::open_with_ops(path, options, platform_vfs_ops())
    }

    pub(crate) fn open_with_ops(
        path: &Path,
        options: RootedOptions,
        ops: Arc<dyn VfsOps>,
    ) -> Result<Self, FileError> {
        let path = std::path::absolute(path).map_err(ops_error)?;
        let dir = ops.open_dir_path(&path).map_err(ops_error)?;
        let dir: Arc<dyn VfsDir> = dir.into();
        let canonical = dir
            .canonicalize_handle()
            .ok()
            .filter(|p| !p.as_os_str().is_empty())
            .map_or_else(
                || {
                    // Fallback: path-level canonicalize — the documented
                    // residual trust boundary on backends without
                    // handle-derived canonicalization.
                    std::fs::canonicalize(&path)
                },
                Ok,
            )
            .map_err(ops_error)?;
        let canonical_comps = normalize_canonical(&canonical);
        Ok(Self {
            ops,
            chain: vec![dir],
            shared: Arc::new(RootShared {
                canonical,
                canonical_comps,
                options,
            }),
        })
    }

    /// The capability root's canonical path. Used only as the lexical
    /// anchor for absolute-link prefix policy — the pinned root
    /// handle, not this string, is the security guarantee. An
    /// absolute link target is accepted only when spelled through
    /// this exact component sequence; another spelling of the same
    /// directory (`/var` vs `/private/var`, 8.3 names, bind mounts,
    /// `subst` drives) is a non-canonical alias — out-of-contract —
    /// and conservatively fails `EscapesRoot`.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn canonical_root(&self) -> &Path {
        &self.shared.canonical
    }

    /// Identity of *this* directory (for browse loop guards — the
    /// `NodeId` is an equality/hash key, not a path substitute).
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Panics
    ///
    /// Never — the chain invariant guarantees a non-empty chain;
    /// the `expect` documents it.
    pub fn dir_identity(&self) -> Result<NodeId, FileError> {
        self.chain
            .last()
            .expect("chain is never empty")
            .identity()
            .map(|id| NodeId { inner: id })
            .map_err(ops_error)
    }

    /// Facts of the object `rel` resolves to: leaf no-follow kind,
    /// resolved kind, byte length, mtime, identity and its strength —
    /// one call gives both the "was the leaf a link" signal and the
    /// resolved object's state.
    ///
    /// `rel == ""` reports this directory itself. `NoFollow` reports a
    /// leaf link as `kind = Symlink` (its own identity/len);
    /// `WithinRoot` follows it — a link target escaping the root is
    /// `EscapesRoot` (to still learn that the leaf *is* a link in
    /// that case, call `metadata(rel, NoFollow)` first — the
    /// `via_symlink` two-call pattern).
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// `NotFound`/`NotDirectory`/`EscapesRoot`/`LoopDetected`/
    /// `LimitExceeded`/`Unsupported`/`Io` per the resolve rules.
    ///
    /// # Panics
    ///
    /// Never — the chain invariant guarantees a non-empty chain;
    /// the `expect` documents it.
    pub fn metadata(&self, rel: &RelativePath, mode: FollowMode) -> Result<NodeInfo, FileError> {
        match self.resolve(rel, mode)? {
            resolve::Resolved::Dir(d) => {
                let probe = d
                    .chain
                    .last()
                    .expect("chain is never empty")
                    .stat_self()
                    .map_err(ops_error)?;
                Ok(node_info(&probe, d.leaf_probe.as_ref()))
            }
            resolve::Resolved::Leaf(l) => Ok(node_info(&l.probe, Some(&l.leaf_probe))),
        }
    }

    /// Verified read-open of the regular file `rel` resolves to.
    ///
    /// The returned [`RootedFile`] pairs the `Read + Seek` reader
    /// with facts of the *same opened object* — never a path
    /// re-resolution. Nonregular leaves (dir, link under `NoFollow`,
    /// FIFO/device/socket) fail `NotRegularFile` without reaching an
    /// ordinary blocking open.
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// `NotRegularFile` for non-regular leaves;
    /// `NotFound`/`NotDirectory`/`EscapesRoot`/`LoopDetected`/
    /// `LimitExceeded`/`Unsupported`/`Io` per resolve and open.
    pub fn open_regular(
        &self,
        rel: &RelativePath,
        mode: FollowMode,
    ) -> Result<RootedFile, FileError> {
        let (chain_top, name, leaf_probe) = self.resolve_leaf_name(rel, mode)?;
        let (file, probe) = chain_top
            .open_regular_leaf(&name)
            .map_err(map_leaf_open_error)?;
        if probe.kind != VfsKind::File {
            return Err(kind_err(FileErrorKind::NotRegularFile));
        }
        Ok(RootedFile {
            file,
            info: node_info(&probe, Some(&leaf_probe)),
        })
    }

    /// Object-identity pin of the regular file `rel` resolves to.
    ///
    /// The pin is an *object identity* token (volume + index + kind +
    /// discriminator + kernel handle), not a content digest —
    /// consumers still verify digests on the returned reader. On
    /// filesystems whose identity cannot discriminate
    /// delete+recreate (`IdentityStrength::Weak`) this fails
    /// `Unsupported` honestly.
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// `NotRegularFile` for non-regular leaves, `Unsupported` on
    /// weak-identity filesystems, plus the usual resolve errors.
    pub fn pin(&self, rel: &RelativePath, mode: FollowMode) -> Result<NodePin, FileError> {
        match self.resolve(rel, mode)? {
            resolve::Resolved::Leaf(l) => {
                if l.probe.kind != VfsKind::File {
                    return Err(kind_err(FileErrorKind::NotRegularFile));
                }
                if l.probe.strength != IdentityStrength::Strong {
                    return Err(kind_err(FileErrorKind::Unsupported));
                }
                Ok(NodePin {
                    id: NodeId {
                        inner: l.probe.identity,
                    },
                })
            }
            resolve::Resolved::Dir { .. } => Err(kind_err(FileErrorKind::NotRegularFile)),
        }
    }

    /// Resolve-then-open with identity verification: the opened
    /// object's identity must equal `pin`, else `IdentityMismatch`
    /// and no bytes are returned. Requires `Strong` identity
    /// (`Unsupported` on weak filesystems).
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// `IdentityMismatch`, `NotRegularFile`, `Unsupported`, plus the
    /// usual resolve/open errors.
    pub fn open_pinned(
        &self,
        rel: &RelativePath,
        pin: &NodePin,
        mode: FollowMode,
    ) -> Result<RootedFile, FileError> {
        let (chain_top, name, leaf_probe) = self.resolve_leaf_name(rel, mode)?;
        let (file, probe) = chain_top
            .open_regular_leaf(&name)
            .map_err(map_leaf_open_error)?;
        if probe.kind != VfsKind::File {
            return Err(kind_err(FileErrorKind::NotRegularFile));
        }
        if probe.strength != IdentityStrength::Strong {
            return Err(kind_err(FileErrorKind::Unsupported));
        }
        if probe.identity != pin.id.inner {
            return Err(kind_err(FileErrorKind::IdentityMismatch));
        }
        Ok(RootedFile {
            file,
            info: node_info(&probe, Some(&leaf_probe)),
        })
    }

    /// Entries of the directory `rel` resolves to (`.`/`..`
    /// excluded), each with its no-follow kind — links are visible as
    /// `Symlink`. `rel == ""` enumerates this directory itself
    /// (root enumeration).
    ///
    /// Every call takes an independent enumeration state; non-UTF-8
    /// names surface as `OsString` (callers using [`RelativePath`]
    /// locators skip them, matching the UTF-8-only locator rule).
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// `NotDirectory` when `rel` resolves to a non-directory (or a
    /// link under `NoFollow`), plus the usual resolve errors.
    pub fn read_dir(
        &self,
        rel: &RelativePath,
        mode: FollowMode,
    ) -> Result<Vec<DirEntry>, FileError> {
        let mut out = Vec::new();
        self.read_dir_each(rel, mode, &mut |entry| {
            out.push(entry);
            EntryFlow::Continue
        })?;
        Ok(out)
    }

    /// Enumerates the directory `rel` resolves to one entry at a
    /// time, delivering each [`DirEntry`] to `visit` without
    /// materializing the whole directory first.
    ///
    /// The visitor returns [`EntryFlow::Continue`] to receive the
    /// next entry or [`EntryFlow::Stop`] to end the enumeration
    /// early — so a consumer can stop after `remaining + 1` entries,
    /// and observe its own cancellation/deadline between entries by
    /// returning `Stop`. After `Stop` (or an OS failure) the backend
    /// makes no further enumeration progress: no additional
    /// directory reads, per-entry probes, or visitor calls —
    /// entries already fetched inside a fixed-size OS batch are
    /// simply discarded.
    ///
    /// The returned [`EnumerationOutcome`] tells the two normal
    /// endings apart honestly — `Complete` means the directory was
    /// walked to exhaustion, `Stopped` means the visitor ended it.
    /// OS failures are `Err`, never a reported outcome. A visitor
    /// panic propagates with the enumeration resources released.
    ///
    /// Every call takes an independent enumeration state; `rel ==
    /// ""` enumerates this directory itself; `.`/`..` are never
    /// reported and entry kinds are no-follow (links stay
    /// `Symlink`). Enumeration order is the backend's — not sorted.
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// `NotDirectory` when `rel` resolves to a non-directory (or a
    /// link under `NoFollow`), plus the usual resolve errors, and
    /// `Io`/`Unsupported` when the backend enumeration fails.
    pub fn read_dir_each(
        &self,
        rel: &RelativePath,
        mode: FollowMode,
        visit: &mut dyn FnMut(DirEntry) -> EntryFlow,
    ) -> Result<EnumerationOutcome, FileError> {
        let dir = self.resolve_dir(rel, mode)?;
        dir.list_each(&mut |e| match visit(DirEntry {
            name: e.name,
            kind: e.kind.into(),
        }) {
            EntryFlow::Continue => VisitFlow::Continue,
            EntryFlow::Stop => VisitFlow::Stop,
        })
        .map(|outcome| match outcome {
            ListingOutcome::Complete => EnumerationOutcome::Complete,
            ListingOutcome::Stopped => EnumerationOutcome::Stopped,
        })
        .map_err(ops_error)
    }

    /// Scoped capability on the directory `rel` resolves to. The
    /// result shares the capability root (`canonical_root`,
    /// options) and keeps the resolved ancestor chain — link targets
    /// containing `..` may pop inside it, but never below the
    /// capability root (`EscapesRoot`). `rel == ""` returns a scoped
    /// clone of `self` without opening new handles.
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// `NotDirectory` for non-directory leaves (link under
    /// `NoFollow` included), plus the usual resolve errors.
    pub fn open_dir(&self, rel: &RelativePath, mode: FollowMode) -> Result<Self, FileError> {
        match self.resolve(rel, mode)? {
            resolve::Resolved::Dir(d) => Ok(Self {
                ops: self.ops.clone(),
                chain: d.chain,
                shared: self.shared.clone(),
            }),
            resolve::Resolved::Leaf { .. } => Err(kind_err(FileErrorKind::NotDirectory)),
        }
    }

    /// Re-roots this capability: resolves `rel` under `self` and
    /// returns a new `RootedDir` whose capability floor is the
    /// **resolved directory handle itself** — not this capability's
    /// root. `rel == ""` re-roots at this directory.
    ///
    /// Unlike [`open_dir`](Self::open_dir) (which shares the
    /// original capability root as the floor), the returned
    /// capability knows nothing about the ancestors of `rel`: a
    /// link target containing `..` that would pop above the
    /// resolved directory fails `EscapesRoot`, and absolute link
    /// targets are judged against the resolved directory's own
    /// canonical path.
    ///
    /// Authority comes from the already-held handle — there is no
    /// pathname re-open and no `NodeId`/strong-identity comparison,
    /// so this works on weak-identity filesystems and keeps working
    /// after the original root is renamed or its pathname replaced:
    /// the held handle still addresses the same directory, while a
    /// replacement planted at the old pathname is a different object
    /// the capability never reaches.
    ///
    /// The new lexical anchor is derived from the resolved handle
    /// (`canonicalize_handle`) only — it is never re-bound from a
    /// path string, and an anchor is never fabricated. Backends
    /// that cannot produce a handle-derived canonical path fail
    /// `Unsupported` rather than silently narrowing the
    /// absolute-link guarantee.
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// `NotDirectory` for non-directory leaves (link under
    /// `NoFollow` included), `Unsupported` where a handle-derived
    /// canonical anchor is unavailable, plus the usual resolve
    /// errors.
    pub fn open_scoped_root(
        &self,
        rel: &RelativePath,
        mode: FollowMode,
    ) -> Result<Self, FileError> {
        let dir = self.resolve_dir(rel, mode)?;
        // Any failure to derive the anchor — unsupported backend or a
        // transient handle-canonicalization error — fails the API as
        // `Unsupported` (source preserved): the anchor is the
        // absolute-link guarantee and must not silently degrade.
        let canonical = dir
            .canonicalize_handle()
            .map_err(|e| FileError::with_source(FileErrorKind::Unsupported, e))
            .and_then(|p| {
                if p.as_os_str().is_empty() {
                    // An empty anchor would silently defeat the
                    // absolute-link prefix check — never fabricate.
                    Err(kind_err(FileErrorKind::Unsupported))
                } else {
                    Ok(p)
                }
            })?;
        let canonical_comps = normalize_canonical(&canonical);
        Ok(Self {
            ops: self.ops.clone(),
            chain: vec![dir],
            shared: Arc::new(RootShared {
                canonical,
                canonical_comps,
                options: self.shared.options,
            }),
        })
    }

    // --------------------------------------------------------------
    // Resolve plumbing shared by the public methods.
    // --------------------------------------------------------------

    fn resolve(
        &self,
        rel: &RelativePath,
        mode: FollowMode,
    ) -> Result<resolve::Resolved, FileError> {
        resolve::resolve(&self.chain, rel, mode, &self.shared)
    }

    /// Resolves `rel` to a directory and returns its handle —
    /// `NotDirectory` for leaf non-directories.
    fn resolve_dir(
        &self,
        rel: &RelativePath,
        mode: FollowMode,
    ) -> Result<Arc<dyn VfsDir>, FileError> {
        match self.resolve(rel, mode)? {
            resolve::Resolved::Dir(d) => Ok(d.chain.last().expect("chain is never empty").clone()),
            resolve::Resolved::Leaf(..) => Err(kind_err(FileErrorKind::NotDirectory)),
        }
    }

    /// Resolves `rel` expecting a named regular file leaf: returns
    /// the parent handle, the resolved leaf name, and the named
    /// leaf's no-follow probe (for `leaf_kind` reporting).
    fn resolve_leaf_name(
        &self,
        rel: &RelativePath,
        mode: FollowMode,
    ) -> Result<(Arc<dyn VfsDir>, OsString, VfsProbe), FileError> {
        match self.resolve(rel, mode)? {
            resolve::Resolved::Leaf(l) => Ok((l.dir, l.name, l.leaf_probe)),
            resolve::Resolved::Dir { .. } => Err(kind_err(FileErrorKind::NotRegularFile)),
        }
    }
}

/// `Read + Seek` over an opened regular file plus its same-object
/// facts — the pair is coherent: `info` describes exactly the
/// object `read`/`seek` operate on.
#[derive(Debug)]
pub struct RootedFile {
    file: File,
    info: NodeInfo,
}

impl RootedFile {
    /// Facts of the opened object (same-handle probe at open time).
    #[must_use]
    pub fn info(&self) -> &NodeInfo {
        &self.info
    }

    /// Extracts the underlying `std::fs::File` (the capability's
    /// guarantees end at handoff — subsequent use is ordinary `File`
    /// semantics on the same opened object).
    #[must_use]
    pub fn into_file(self) -> File {
        self.file
    }
}

impl Read for RootedFile {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.file.read(buf)
    }
}

impl Seek for RootedFile {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.file.seek(pos)
    }
}

/// How link components are treated during resolution.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FollowMode {
    /// No link is ever followed: a leaf link reports
    /// `kind = Symlink` via `metadata`, `open_regular` fails
    /// `NotRegularFile`, `open_dir`/`read_dir` fail `NotDirectory`,
    /// and a link at any intermediate component fails `NotDirectory`.
    NoFollow,
    /// Links are resolved inside the capability with bounded hops —
    /// the canonicalize+containment semantics, preserved under the
    /// structural constraint that nothing outside the root is opened.
    /// Leaf and intermediate links follow the same rule; a target
    /// that would escape fails `EscapesRoot` (including the
    /// conservative rejection of targets that leave and return).
    /// Absolute targets are judged lexically against
    /// [`RootedDir::canonical_root`]'s component sequence — a target
    /// spelled through a non-canonical alias of the root kernel-
    /// resolves inside yet conservatively fails `EscapesRoot` (the
    /// documented narrowing: judging aliases would require the path
    /// re-resolution this capability exists to remove).
    WithinRoot,
}

/// Whether a reported identity can discriminate delete+recreate —
/// the `pin`/`open_pinned` floor. Per-object, never a global claim.
#[non_exhaustive]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum IdentityStrength {
    /// Kernel-exported handle or birth time lets the identity
    /// discriminate recreation — `pin`/`open_pinned` are valid.
    Strong,
    /// Only coarse slot/ctime identity — same-tick inode reuse is
    /// not discriminated. `pin`/`open_pinned` fail `Unsupported`.
    Weak,
}

/// Opaque identity key: (volume, index, kind tag, discriminator
/// [, kernel handle]). Equality/hash only — never interpreted.
/// Two `NodeId`s of the same object on the same filesystem compare
/// equal; ctime-only metadata changes (chmod/link/touch) may also
/// change the value — fail-closed by design.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct NodeId {
    inner: vfs::NodeIdentity,
}

/// Opaque identity token for `open_pinned`. Issued only when the
/// leaf reports `IdentityStrength::Strong` — never a swap-detection
/// promise a weak filesystem cannot keep.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct NodePin {
    id: NodeId,
}

/// Facts of a resolved object.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct NodeInfo {
    /// Kind of the resolved object (a followed link's target kind
    /// under `WithinRoot`; the leaf itself otherwise).
    pub kind: NodeKind,
    /// The leaf name's own no-follow kind — `Symlink` when the leaf
    /// was a link (the `via_symlink` report).
    pub leaf_kind: NodeKind,
    /// Byte length where the kind carries one (file bytes; link
    /// target length for `Symlink`).
    pub byte_len: Option<u64>,
    /// Modification time where the backend reports one.
    pub modified: Option<SystemTime>,
    /// Resolved-object identity (loop guards, pin comparison).
    pub identity: NodeId,
    /// Whether `identity` can discriminate delete+recreate.
    pub identity_strength: IdentityStrength,
}

fn node_info(probe: &VfsProbe, leaf_probe: Option<&VfsProbe>) -> NodeInfo {
    NodeInfo {
        kind: probe.kind.into(),
        leaf_kind: leaf_probe.map_or_else(|| probe.kind.into(), |p| p.kind.into()),
        byte_len: probe.byte_len,
        modified: probe.modified,
        identity: NodeId {
            inner: probe.identity,
        },
        identity_strength: probe.strength,
    }
}

/// Filesystem object kind, always determined no-follow.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeKind {
    /// A regular file.
    File,
    /// A real directory.
    Dir,
    /// A symlink / name-surrogate reparse point.
    Symlink,
    /// Anything else (fifo, socket, device, non-surrogate reparse).
    Other,
}

impl From<VfsKind> for NodeKind {
    fn from(kind: VfsKind) -> Self {
        match kind {
            VfsKind::File => Self::File,
            VfsKind::Dir => Self::Dir,
            VfsKind::Symlink => Self::Symlink,
            VfsKind::Other => Self::Other,
        }
    }
}

/// One directory-listing entry.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirEntry {
    /// Entry name as stored — may be non-UTF-8 (such names cannot
    /// become a [`RelativePath`]; callers skip them).
    pub name: OsString,
    /// The entry's no-follow kind — `Symlink` stays visible.
    pub kind: NodeKind,
}

/// Per-entry flow control returned by a
/// [`RootedDir::read_dir_each`] visitor.
///
/// `Stop` ends the enumeration early — after it, the backend makes
/// no further enumeration progress (no additional directory reads,
/// per-entry probes, or visitor calls). A consumer observes its own
/// cancellation or deadline between entries by checking it inside
/// the visitor and returning `Stop`.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryFlow {
    /// Deliver the next entry.
    Continue,
    /// Stop enumerating — the outcome is
    /// [`EnumerationOutcome::Stopped`].
    Stop,
}

/// Terminal state of a [`RootedDir::read_dir_each`] enumeration:
/// whether the directory was walked to exhaustion or the visitor
/// stopped it. OS failures are `Err(FileError)`, never a reported
/// outcome.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnumerationOutcome {
    /// Every entry was delivered.
    Complete,
    /// The visitor returned [`EntryFlow::Stop`].
    Stopped,
}

/// Resolve budgets. Defaults are conservative; `Default` is
/// implemented.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RootedOptions {
    /// Link expansions allowed per resolve (loop bound). Default 40.
    pub max_hops: u32,
    /// Locator component-count bound. Default 40.
    pub max_depth: u32,
    /// Total components processed including link expansions.
    /// Default 512.
    pub max_components_total: u32,
    /// Simultaneous open directory handles (resolved ancestors).
    /// Default 64.
    pub max_open_dirs: u32,
}

impl Default for RootedOptions {
    fn default() -> Self {
        Self {
            max_hops: 40,
            max_depth: 40,
            max_components_total: 512,
            max_open_dirs: 64,
        }
    }
}

/// A validated `rooted`-locator: `/`-separated, UTF-8, relative.
///
/// An **empty** locator is valid and means *this directory itself*
/// (0 components — the root-enumeration/self form). Components are
/// never `.`/`..`, absolute, prefixed, or rooted; `\`, `:`, NUL,
/// control characters, Windows-reserved names, trailing `.`/space
/// and components over 255 bytes are all `InvalidInput`.
///
/// **RT contract:** non-RT only — validation allocates.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RelativePath {
    components: Vec<String>,
}

impl RelativePath {
    /// Validates a `/`-separated locator. `""` → 0 components (this
    /// directory itself); `"a"` → one; `"a/b"` → two. A single
    /// trailing `/` is accepted as a directory marker and stripped.
    ///
    /// **RT contract:** non-RT only — allocates.
    ///
    /// # Errors
    ///
    /// `InvalidInput` on any grammar violation (absolute/prefixed/
    /// rooted forms, `.`/`..`/empty components, `\`/`:`/NUL/control
    /// characters, Windows reserved names, trailing `.`/space,
    /// >255-byte components, non-UTF-8 input).
    pub fn new(locator: &str) -> Result<Self, FileError> {
        if locator.is_empty() {
            return Ok(Self {
                components: Vec::new(),
            });
        }
        if locator.contains('\0') {
            return Err(invalid("locator contains a NUL byte"));
        }
        if locator.contains('\\') {
            return Err(invalid(
                "locator contains `\\` (a Windows separator); rejecting rather than reinterpreting",
            ));
        }
        if locator.contains(':') {
            return Err(invalid(
                "locator contains `:` (drive/ADS separator); rejecting rather than reinterpreting",
            ));
        }
        if locator.starts_with('/') {
            return Err(invalid("locator is absolute (starts with `/`)"));
        }
        let stripped = locator.strip_suffix('/').unwrap_or(locator);
        let mut components = Vec::new();
        for part in stripped.split('/') {
            if part.is_empty() {
                return Err(invalid(
                    "locator contains an empty component (`//`, leading, or repeated `/`)",
                ));
            }
            validate_component(part)?;
            components.push(part.to_owned());
        }
        Ok(Self { components })
    }

    /// Single-component form from a raw name (e.g. a `read_dir`
    /// entry). Non-UTF-8 names fail `InvalidInput` — the caller skips
    /// them.
    ///
    /// **RT contract:** non-RT only — allocates.
    ///
    /// # Errors
    ///
    /// `InvalidInput` for non-UTF-8 input or a component-grammar
    /// violation.
    pub fn from_component(name: &OsStr) -> Result<Self, FileError> {
        let text = name
            .to_str()
            .ok_or_else(|| invalid("component is not valid UTF-8; locators require UTF-8"))?;
        validate_component(text)?;
        if text.contains('/') {
            return Err(invalid("component contains `/`"));
        }
        Ok(Self {
            components: vec![text.to_owned()],
        })
    }

    /// The validated components in order (empty iterator for the
    /// self/`""` form).
    ///
    /// **RT contract:** non-RT only.
    pub fn components(&self) -> impl ExactSizeIterator<Item = &str> + '_ {
        self.components.iter().map(String::as_str)
    }

    /// Component count (0 for the self/`""` form).
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn depth(&self) -> usize {
        self.components.len()
    }
}

fn validate_component(part: &str) -> Result<(), FileError> {
    if part == "." || part == ".." {
        return Err(invalid("locator contains `.`/`..` traversal"));
    }
    if part.len() > MAX_COMPONENT_BYTES {
        return Err(invalid("locator component exceeds 255 bytes"));
    }
    if part.starts_with(char::is_whitespace) || part.ends_with(char::is_whitespace) {
        return Err(invalid(
            "locator component has leading/trailing whitespace (Windows strips it)",
        ));
    }
    if part.ends_with('.') {
        return Err(invalid(
            "locator component ends with `.` (Windows strips it)",
        ));
    }
    if part.chars().any(char::is_control) {
        return Err(invalid("locator contains control characters"));
    }
    if part.chars().any(|c| FORBIDDEN_CHARS.contains(&c)) {
        return Err(invalid(
            "locator contains a character forbidden on Windows (`< > | \" ? *`)",
        ));
    }
    if is_reserved_stem(part) {
        return Err(invalid(
            "locator component is a Windows reserved device name",
        ));
    }
    Ok(())
}

/// Whether `part`'s stem (before the first `.`) matches a Windows
/// device name. Comparison is ASCII-case-insensitive and normalizes
/// the superscript digits `¹²³` (which NTFS maps onto `123`).
fn is_reserved_stem(part: &str) -> bool {
    let stem = part.split('.').next().unwrap_or(part);
    let mut upper = String::with_capacity(stem.len());
    for c in stem.chars() {
        match c {
            '\u{00B9}' => upper.push('1'),
            '\u{00B2}' => upper.push('2'),
            '\u{00B3}' => upper.push('3'),
            other => upper.extend(other.to_uppercase()),
        }
    }
    if RESERVED_STEMS.contains(&upper.as_str()) {
        return true;
    }
    for prefix in ["COM", "LPT"] {
        if let Some(rest) = upper.strip_prefix(prefix)
            && rest.len() == 1
            && rest
                .chars()
                .next()
                .is_some_and(|d| ('1'..='9').contains(&d))
        {
            return true;
        }
    }
    false
}

/// Splits a canonical root path into the normalized component list
/// used by the resolve walk's textual phase: unix drops the leading
/// `/`; Windows strips `\\?\`/`\\.\`/`\\` prefixes and splits on
/// `\`/`/`.
#[cfg(unix)]
fn normalize_canonical(path: &Path) -> Vec<OsString> {
    path.components()
        .filter_map(|c| match c {
            std::path::Component::Normal(s) => Some(s.to_os_string()),
            _ => None,
        })
        .collect()
}

#[cfg(windows)]
fn normalize_canonical(path: &Path) -> Vec<OsString> {
    let s = path.as_os_str().to_string_lossy();
    let stripped = if let Some(rest) = s.strip_prefix(r"\\?\") {
        rest
    } else if let Some(rest) = s.strip_prefix(r"\\.\") {
        rest
    } else if let Some(rest) = s.strip_prefix(r"\\") {
        rest
    } else {
        s.as_ref()
    };
    stripped
        .split(['\\', '/'])
        .filter(|p| !p.is_empty())
        .map(OsString::from)
        .collect()
}

#[cfg(not(any(unix, windows)))]
fn normalize_canonical(path: &Path) -> Vec<OsString> {
    path.components()
        .filter_map(|c| match c {
            std::path::Component::Normal(s) => Some(s.to_os_string()),
            _ => None,
        })
        .collect()
}

/// `open_regular_leaf` errors: an `ELOOP`/`NotADirectory`/`IsADirectory`/
/// `InvalidData` failure at the leaf means the object is not an
/// openable regular file — `InvalidData` is the vfs contract's
/// "verified non-regular" signal (the backend probed the leaf,
/// found it non-regular, and refused the open).
fn map_leaf_open_error(e: io::Error) -> FileError {
    #[cfg(unix)]
    if e.raw_os_error() == Some(libc::ELOOP) {
        return kind_err(FileErrorKind::NotRegularFile);
    }
    match e.kind() {
        io::ErrorKind::NotADirectory | io::ErrorKind::IsADirectory | io::ErrorKind::InvalidData => {
            kind_err(FileErrorKind::NotRegularFile)
        }
        _ => ops_error(e),
    }
}

impl fmt::Display for RelativePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.components.join("/"))
    }
}
