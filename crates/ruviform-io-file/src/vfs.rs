//! Shared workspace-internal filesystem primitive layer — `vfs`
//! (rooted-read capability, IO-IMP-10 commonization).
//!
//! **Internal contract, not a public API.** This module is
//! `#[doc(hidden)]` and exists solely so `ruviform-io-file` (`rooted`)
//! and `ruviform-io-archive` (secure-extraction staging backend) share
//! one authoritative implementation of the OS rules: handle/fd-relative
//! traversal, no-follow opens, exclusive create, and stable node
//! identity. It is **not** semver-covered and **must not** be consumed
//! by anything else. `publish = false` means it never ships as a
//! supported surface.
//!
//! # Contract
//!
//! - **Handle/fd-relative traversal.** Every child operation takes a
//!   single component name and executes relative to an open directory
//!   handle (`openat`/`OBJECT_ATTRIBUTES.RootDirectory`) — path strings
//!   are never re-resolved below the trusted parent.
//! - **No-follow component ops.** `probe`/`open_dir`/`read_link`/
//!   `open_regular_leaf`/`create_*`/`remove_*` never follow a link at
//!   the named component (`O_NOFOLLOW`/`fstatat(AT_SYMLINK_NOFOLLOW)` on
//!   unix; `FILE_OPEN_REPARSE_POINT` + `OBJ_DONT_REPARSE` on Windows).
//!   Link *following* is never a vfs operation: the resolve layer reads
//!   targets explicitly via [`VfsDir::read_link`] and re-walks.
//! - **Exclusive create.** `create_dir`/`create_file` fail
//!   `AlreadyExists` when the name is taken — including by a link.
//! - **Identity.** [`NodeIdentity`] is a stable (volume, index, kind
//!   tag, discriminator [, kernel handle]) equality key naming the
//!   object the handle addresses — never the path string.
//!   [`IdentityStrength`] reports per-object whether that identity can
//!   discriminate delete+recreate (the `pin`/`open_pinned` floor).
//!
//! Errors are `io::Error`; the caller-facing typed classification is
//! the consumer's job (`rooted` maps to `FileErrorKind`; the archive
//! adapter maps to `ArchiveErrorKind`).
//!
//! **RT contract:** non-RT only — every operation may block on
//! filesystem I/O and allocate.
#![allow(dead_code)] // staging-only ops are used by ruviform-io-archive, not this crate

use std::ffi::{OsStr, OsString};
use std::fmt;
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

use crate::rooted::IdentityStrength;

#[cfg(unix)]
mod unix;
mod unsupported;
#[cfg(windows)]
mod windows;

/// The object-type the identity was captured for. Cross-type swaps
/// never compare equal — a link at a recorded file name mismatches
/// even if the underlying index were somehow reused.
#[doc(hidden)]
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeTypeTag {
    /// Unidentifiable — never matches a real object tag.
    Unknown = 0,
    /// A regular file.
    File,
    /// A real directory.
    Dir,
    /// Links, sockets, devices, reparse points — never equals a
    /// recorded File/Dir tag.
    Other,
}

/// Stable filesystem identity of a node.
///
/// unix: (`st_dev`/`stx_dev`, `st_ino`/`stx_ino`, `S_IFMT`-derived
/// tag, birth-time or change-time discriminator). Windows:
/// (`VolumeSerialNumber`, `FileId`, attribute-derived tag,
/// `FileBasicInfo` time discriminator).
///
/// The `handle` is the kernel-exported file handle
/// (`name_to_handle_at` on Linux): it embeds the inode generation,
/// so a delete+recreate that lands on the same inode in the same
/// timestamp tick still produces a different identity. `None` where
/// the platform or filesystem does not export handles.
///
/// The `discriminator` is a secondary timestamp key — birth time
/// where the platform reports it, else change time — closing most
/// of the index-reuse window even without handles. A directory's
/// change time mutates when the session adds children, so
/// directories without birth time carry `0`.
///
/// Compared for equality only — the values are opaque and never
/// interpreted.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeIdentity {
    pub volume: u64,
    pub index: u128,
    /// Object type at capture time.
    pub tag: NodeTypeTag,
    /// Timestamp discriminator; `0` for directories without birth
    /// time.
    pub discriminator: u64,
    /// Kernel file-handle discriminator where available.
    pub handle: Option<NodeHandle>,
}

/// Opaque kernel file handle — the `name_to_handle_at` result
/// (`handle_type` + bytes). Equality covers the full buffer; the
/// unused tail is always zeroed so same-length handles compare by
/// content.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeHandle {
    /// `handle_type` reported by the kernel (e.g. `FILEID_INO32_GEN`).
    pub kind: i32,
    /// Kernel-reported byte length of the handle.
    pub len: u8,
    /// Handle bytes, zero-padded to the fixed capacity.
    pub bytes: [u8; 128],
}

/// Filesystem object classification, always determined no-follow —
/// a link reports `Symlink`, never its target's kind.
#[doc(hidden)]
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VfsKind {
    /// A regular file.
    File,
    /// A real directory.
    Dir,
    /// A symlink (unix) or a name-surrogate reparse point
    /// (`IO_REPARSE_TAG_SYMLINK`/`IO_REPARSE_TAG_MOUNT_POINT`) on
    /// Windows.
    Symlink,
    /// Anything else: fifo, socket, device, non-surrogate reparse
    /// point, unknown.
    Other,
}

/// A no-follow probe of one filesystem object: kind, identity,
/// optional byte length and mtime, and the strength of that identity.
#[doc(hidden)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct VfsProbe {
    /// Identity of the probed object itself (a link's own identity,
    /// never a target's).
    pub identity: NodeIdentity,
    /// No-follow object kind.
    pub kind: VfsKind,
    /// Byte length where the object kind carries a meaningful one
    /// (regular file bytes; link target length). `None` for
    /// directories and other kinds.
    pub byte_len: Option<u64>,
    /// Modification time where the backend reports one.
    pub modified: Option<std::time::SystemTime>,
    /// Per-object identity strength — the authoritative gate for
    /// `pin`/`open_pinned`-style operations.
    pub strength: IdentityStrength,
}

/// One directory listing entry. `.`/`..` are never reported.
#[doc(hidden)]
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct VfsEntry {
    /// Entry name as stored (may be non-UTF-8).
    pub name: OsString,
    /// No-follow entry kind (links are visible as `Symlink`).
    pub kind: VfsKind,
}

/// Per-entry flow control returned by a [`VfsDir::list_each`]
/// visitor. `Stop` terminates the enumeration early — the backend
/// must then make no further enumeration progress (no additional OS
/// enumeration calls, per-entry probes, or visitor invocations;
/// already-fetched OS batch records are simply discarded).
#[doc(hidden)]
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisitFlow {
    /// Deliver the next entry.
    Continue,
    /// Stop enumerating — the outcome is [`ListingOutcome::Stopped`].
    Stop,
}

/// Terminal state of a [`VfsDir::list_each`] enumeration: whether
/// the directory was walked to exhaustion or the visitor stopped it.
/// Enumeration errors are reported as `Err`, never via this type.
#[doc(hidden)]
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListingOutcome {
    /// Every entry was delivered.
    Complete,
    /// The visitor returned [`VisitFlow::Stop`].
    Stopped,
}

/// The vfs backend factory surface.
#[doc(hidden)]
pub trait VfsOps: Send + Sync + fmt::Debug {
    /// Opens a directory by absolute path — the **only** path-level
    /// resolution point; whether links on that path may be resolved
    /// is the caller's trust boundary. unix
    /// `open(O_RDONLY|O_DIRECTORY|O_CLOEXEC)`; Windows
    /// `NtOpenFile` (`FILE_DIRECTORY_FILE`, path-level
    /// `OBJ_CASE_INSENSITIVE`). A non-directory fails.
    fn open_dir_path(&self, path: &Path) -> io::Result<Box<dyn VfsDir>>;

    /// Identity/probe of an already-open `File` (unix `fstat`;
    /// Windows `FileIdInfo`+`FileBasicInfo`+`FileStandardInfo`).
    /// Used for post-open regular/identity verification and the
    /// archive `StagingFile::identity` equivalent.
    fn stat_file(&self, file: &File) -> io::Result<VfsProbe>;
}

/// An open directory handle. All child operations are relative to
/// *this* handle, take a single component name, and never follow a
/// link at the named component.
#[doc(hidden)]
pub trait VfsDir: Send + Sync + fmt::Debug {
    /// Diagnostic path — never an identity proof.
    fn path(&self) -> &Path;

    /// Identity of this directory (its own handle; `tag` = `Dir`,
    /// `discriminator` = 0 where birth time is unavailable).
    fn identity(&self) -> io::Result<NodeIdentity>;

    /// Probe of this directory itself — used for `metadata("")` and
    /// a link target that resolved to the root/scoped dir.
    fn stat_self(&self) -> io::Result<VfsProbe>;

    /// The canonical path of this directory *derived from the open
    /// handle* where the platform offers one (Linux
    /// `/proc/self/fd/N` readlink; Windows
    /// `GetFinalPathNameByHandleW`; Apple `fcntl(F_GETPATH)`).
    /// Returns [`io::ErrorKind::Unsupported`] where the backend
    /// cannot produce a handle-derived path — the caller falls back
    /// to `fs::canonicalize` (documented residual trust boundary).
    fn canonicalize_handle(&self) -> io::Result<PathBuf>;

    /// No-follow probe of the child named `name`: reports the leaf
    /// object itself — a link's own kind/identity/length.
    fn probe(&self, name: &OsStr) -> io::Result<VfsProbe>;

    /// Opens the child *directory* `name` no-follow — a link or a
    /// non-directory fails rather than being followed.
    fn open_dir(&self, name: &OsStr) -> io::Result<Box<dyn VfsDir>>;

    /// Verified read-open of the child leaf `name` implementing the
    /// per-OS "never let a nonregular reach an ordinary blocking
    /// open" rule. Returns the opened [`File`] plus a probe of the
    /// *same opened object* — the probe, not a pre-open stat, is the
    /// authoritative kind/identity. Callers must re-check
    /// `probe.kind`.
    ///
    /// - Linux: `openat(O_PATH|O_NOFOLLOW)` pin → `fstat` →
    ///   `/proc/self/fd/N` reopen `O_RDONLY` → identity re-verify.
    ///   `/proc` absent or reopen impossible → `Unsupported`.
    /// - Other unix: `openat(O_RDONLY|O_NOFOLLOW|O_NONBLOCK)` →
    ///   `fstat` → `O_NONBLOCK` cleared before return.
    /// - Windows: `NtOpenFile(FILE_NON_DIRECTORY_FILE|
    ///   FILE_OPEN_REPARSE_POINT|FILE_SYNCHRONOUS_IO_NONALERT)` →
    ///   attribute/id/size query.
    fn open_regular_leaf(&self, name: &OsStr) -> io::Result<(File, VfsProbe)>;

    /// Reads the link target stored at `name` in OS-raw form
    /// (`\??\`-style NT names on Windows — namespace bridging is the
    /// resolve layer's job). A non-link `name` fails
    /// `InvalidInput`.
    fn read_link(&self, name: &OsStr) -> io::Result<OsString>;

    /// Enumerates this directory's entries (`.`/`..` excluded), each
    /// with a no-follow kind, delivering them one at a time to
    /// `visit`. Every call takes an independent enumeration state
    /// (unix: a fresh `openat(dirfd, ".")` + `fdopendir`; Windows: a
    /// fresh empty-name `NtOpenFile` handle — the scan position lives
    /// on the `FILE_OBJECT` — queried with `NtQueryDirectoryFile` and
    /// `RestartScan`). An entry that vanishes mid-enumeration is
    /// skipped (unix `NotFound` probe → skip; Windows delivers the
    /// fetched record); only real enumeration failures return `Err`.
    ///
    /// The backend must not materialize the whole directory before
    /// visiting: each entry is classified (probing only that entry
    /// where the OS record lacks a kind) and delivered in turn. When
    /// `visit` returns [`VisitFlow::Stop`] the backend returns
    /// [`ListingOutcome::Stopped`] without issuing another OS
    /// enumeration call, per-entry probe, or visitor invocation —
    /// records already fetched inside a fixed-size OS batch are
    /// discarded. A visitor panic unwinds with the enumeration
    /// resources released (unix `DIR*`/`fd` via drop guard).
    fn list_each(&self, visit: &mut dyn FnMut(VfsEntry) -> VisitFlow)
    -> io::Result<ListingOutcome>;

    /// Lists this directory's entries (`.`/`..` excluded) — the
    /// collect form of [`list_each`](Self::list_each), kept as the
    /// single source of truth so full-vector callers share the same
    /// enumeration rules.
    fn list(&self) -> io::Result<Vec<VfsEntry>> {
        let mut out = Vec::new();
        self.list_each(&mut |entry| {
            out.push(entry);
            VisitFlow::Continue
        })?;
        Ok(out)
    }

    // ---- archive staging ops (current IO-IMP-10 semantics) ----

    /// Exclusive-create a child directory (unix `mkdirat(0700)`;
    /// Windows `NtCreateFile(FILE_CREATE|FILE_DIRECTORY_FILE|
    /// FILE_OPEN_REPARSE_POINT)`). `AlreadyExists` refuses *every*
    /// existing name, including links.
    fn create_dir(&self, name: &OsStr) -> io::Result<Box<dyn VfsDir>>;

    /// Exclusive-create a child regular file (unix
    /// `openat(O_WRONLY|O_CREAT|O_EXCL|O_NOFOLLOW, 0600)`; Windows
    /// `NtCreateFile(FILE_CREATE|FILE_NON_DIRECTORY_FILE|
    /// FILE_OPEN_REPARSE_POINT)` + `FILE_GENERIC_WRITE|
    /// FILE_READ_ATTRIBUTES` + `ShareAccess=0`).
    fn create_file(&self, name: &OsStr) -> io::Result<File>;

    /// Deletes the leaf file `name` — never follows it (unix
    /// `unlinkat(0)` — a swapped directory fails `EISDIR`; Windows
    /// `NtDeleteFile` + `OBJ_DONT_REPARSE`). The `EISDIR`
    /// file-vs-dir distinction is unix-scoped; Windows kind checks
    /// are the session layer's probe+kind duty.
    fn remove_file(&self, name: &OsStr) -> io::Result<()>;

    /// Deletes the leaf *empty* directory `name` (unix
    /// `unlinkat(AT_REMOVEDIR)` — a swapped file fails `ENOTDIR`;
    /// Windows `NtDeleteFile` + `OBJ_DONT_REPARSE`).
    fn remove_dir(&self, name: &OsStr) -> io::Result<()>;
}

/// The platform backend for this target, shared via `Arc` so handles
/// derived from it stay on the same backend.
///
/// unix and Windows have real backends; every other target gets a
/// backend that reports `Unsupported` on every operation rather than
/// degrading to path-string traversal. On Windows older than
/// Windows 10 RS1 (build 14393 — where `OBJ_DONT_REPARSE` is silently
/// ignored) the Unsupported backend is installed for the same reason.
#[doc(hidden)]
#[must_use]
pub fn platform_vfs_ops() -> std::sync::Arc<dyn VfsOps> {
    os_vfs_ops()
}

#[cfg(unix)]
fn os_vfs_ops() -> std::sync::Arc<dyn VfsOps> {
    std::sync::Arc::new(unix::UnixVfsOps)
}

#[cfg(windows)]
fn os_vfs_ops() -> std::sync::Arc<dyn VfsOps> {
    if windows::supports_dont_reparse() {
        std::sync::Arc::new(windows::WindowsVfsOps)
    } else {
        // Pre-RS1 builds silently ignore OBJ_DONT_REPARSE — the
        // no-follow guarantee cannot be made, so no ops at all.
        unsupported::ops()
    }
}

#[cfg(not(any(unix, windows)))]
fn os_vfs_ops() -> std::sync::Arc<dyn VfsOps> {
    unsupported::ops()
}

/// Builds a [`VfsProbe`] from an identity plus kind classification —
/// the identity's `tag` must already agree with `kind` (they are
/// derived from the same object state).
pub(crate) fn probe_of(
    identity: NodeIdentity,
    kind: VfsKind,
    byte_len: Option<u64>,
    modified: Option<std::time::SystemTime>,
    strength: IdentityStrength,
) -> VfsProbe {
    VfsProbe {
        identity,
        kind,
        byte_len,
        modified,
        strength,
    }
}

#[cfg(test)]
pub(crate) mod mock;
