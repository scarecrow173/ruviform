//! Real OS backend for staging trees — `UnixExtractOps` (IO-IMP-10).
//!
//! Every operation is dirfd-relative and no-follow:
//!
//! - The trusted parent is opened once (`O_RDONLY | O_DIRECTORY |
//!   O_CLOEXEC`; its own path resolution is the caller's trust
//!   boundary).
//! - The staging root is `mkdirat`'d under that dirfd with a unique
//!   name, then re-opened `O_NOFOLLOW | O_DIRECTORY` so a planted link
//!   at the same name is never followed.
//! - Each traversal step opens **one** component via
//!   `openat(O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC)` — a
//!   swapped-in symlink or non-directory at any level fails the open
//!   (`ELOOP`/`ENOTDIR`) instead of being followed.
//! - Files are created `O_WRONLY | O_CREAT | O_EXCL | O_NOFOLLOW` —
//!   `O_EXCL` is atomic and refuses every existing name (including a
//!   planted symlink); `O_NOFOLLOW` guarantees the leaf is never a
//!   link target.
//! - Deletion is `unlinkat` relative to a verified dirfd —
//!   `unlinkat` never follows links; `AT_REMOVEDIR` removes only real
//!   directories (a swapped link at a dir position is refused, not
//!   followed).
//! - Identity is `fstat`/`fstatat(AT_SYMLINK_NOFOLLOW)` on the same
//!   handle — the values name the object the handle addresses, never
//!   the path string.
//!
//! **RT contract:** non-RT only.

#![cfg(unix)]

use std::ffi::{CString, OsStr, OsString};
use std::fs::File;
use std::io::{self, Write};
use std::os::fd::{AsRawFd, FromRawFd, RawFd};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

#[cfg(all(target_os = "linux", target_env = "gnu"))]
use super::fs::NodeHandle;
use super::fs::{
    MAX_NAME_ATTEMPTS, NodeIdentity, NodeTypeTag, PlatformExtractOps, StagingDir, StagingFile,
    StagingRoot, staging_name,
};

const DIR_MODE: libc::mode_t = 0o700;
const FILE_MODE: libc::c_uint = 0o600;

/// The unix [`PlatformExtractOps`] implementation.
#[derive(Debug)]
pub(crate) struct UnixExtractOps;

impl PlatformExtractOps for UnixExtractOps {
    /// Creates the staging root: open trusted parent → `mkdirat`
    /// unique name → open the root `O_NOFOLLOW | O_DIRECTORY`.
    fn create_staging_root(&self, parent: &Path) -> io::Result<Box<dyn StagingRoot>> {
        let parent = std::path::absolute(parent)?;
        let parent_fd = open_dir_at(&parent)?;
        for _ in 0..MAX_NAME_ATTEMPTS {
            let name = staging_name("rvf-staging");
            match mkdirat(parent_fd.as_raw_fd(), &name) {
                Ok(()) => {
                    let fd = open_component(parent_fd.as_raw_fd(), &name)?;
                    return Ok(Box::new(UnixDir {
                        fd,
                        path: parent.join(&name),
                        parent: Some(ParentRef {
                            fd: parent_fd,
                            name: OsString::from(name),
                        }),
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
    /// `O_NOFOLLOW`) for recovery handle `inspect`/`cleanup`.
    fn open_staging_root(&self, path: &Path) -> io::Result<Box<dyn StagingRoot>> {
        let path = std::path::absolute(path)?;
        let name = path
            .file_name()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no file name"))?
            .to_os_string();
        let parent_path = path
            .parent()
            .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
        let parent_fd = open_dir_at(&parent_path)?;
        let fd = open_component(parent_fd.as_raw_fd(), &name)?;
        Ok(Box::new(UnixDir {
            fd,
            path,
            parent: Some(ParentRef {
                fd: parent_fd,
                name,
            }),
        }))
    }
}

/// An open staging directory (dirfd + diagnostic path). `parent` is
/// set only on the root so it can remove itself.
#[derive(Debug)]
struct UnixDir {
    fd: File,
    path: PathBuf,
    parent: Option<ParentRef>,
}

#[derive(Debug)]
struct ParentRef {
    fd: File,
    name: OsString,
}

impl StagingDir for UnixDir {
    fn path(&self) -> &Path {
        &self.path
    }

    fn identity(&self) -> io::Result<NodeIdentity> {
        stat_fd(self.fd.as_raw_fd())
    }

    fn probe_identity(&self, name: &str) -> io::Result<NodeIdentity> {
        stat_child(self.fd.as_raw_fd(), name)
    }

    fn create_dir(&self, name: &str) -> io::Result<Box<dyn StagingDir>> {
        mkdirat(self.fd.as_raw_fd(), name)?;
        let fd = open_component(self.fd.as_raw_fd(), name)?;
        Ok(Box::new(UnixDir {
            fd,
            path: self.path.join(name),
            parent: None,
        }))
    }

    fn open_dir(&self, name: &str) -> io::Result<Box<dyn StagingDir>> {
        let fd = open_component(self.fd.as_raw_fd(), name)?;
        Ok(Box::new(UnixDir {
            fd,
            path: self.path.join(name),
            parent: None,
        }))
    }

    fn create_file(&self, name: &str) -> io::Result<Box<dyn StagingFile>> {
        let file = create_file(self.fd.as_raw_fd(), name)?;
        Ok(Box::new(UnixFile(file)))
    }

    fn remove_file(&self, name: &str) -> io::Result<()> {
        unlinkat(self.fd.as_raw_fd(), name, 0)
    }

    fn remove_dir(&self, name: &str) -> io::Result<()> {
        unlinkat(self.fd.as_raw_fd(), name, libc::AT_REMOVEDIR)
    }
}

impl StagingRoot for UnixDir {
    /// Deletes the staging root via its parent dirfd. Re-checks the
    /// recorded name's identity against this handle's own `fstat`
    /// first: a swapped-in link or foreign directory at the name
    /// reports an error instead of being deleted.
    fn remove_root(self: Box<Self>) -> io::Result<()> {
        let Some(parent) = &self.parent else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "ruviform-io-archive: remove_root on a non-root directory",
            ));
        };
        let expected = stat_fd(self.fd.as_raw_fd())?;
        let observed = stat_child_os(parent.fd.as_raw_fd(), &parent.name)?;
        if observed != expected {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "ruviform-io-archive: staging root identity changed; refusing removal",
            ));
        }
        unlinkat_os(parent.fd.as_raw_fd(), &parent.name, libc::AT_REMOVEDIR)
    }
}

/// An open staging file.
#[derive(Debug)]
struct UnixFile(File);

impl Write for UnixFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

impl StagingFile for UnixFile {
    fn identity(&self) -> io::Result<NodeIdentity> {
        stat_fd(self.0.as_raw_fd())
    }
}

// ------------------------------------------------------------------
// libc helpers — each wraps one syscall with EINTR retry and the
// documented flag contract.
// ------------------------------------------------------------------

fn cstring(name: &OsStr) -> io::Result<CString> {
    CString::new(name.as_bytes()).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "ruviform-io-archive: component contains an interior NUL byte",
        )
    })
}

/// Opens a path-level directory (trusted parent only — whether links
/// may be resolved on this path is the caller's trust boundary).
#[expect(unsafe_code, reason = "open(2) is libc FFI; see SAFETY comment")]
fn open_dir_at(path: &Path) -> io::Result<File> {
    let c = cstring(path.as_os_str())?;
    loop {
        // SAFETY: `c` is a valid NUL-terminated C string that outlives
        // the call; the flags are the documented open(2) contract.
        let fd = unsafe {
            libc::open(
                c.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC,
            )
        };
        if fd >= 0 {
            // SAFETY: `fd` is a freshly opened descriptor owned by us.
            return Ok(unsafe { File::from_raw_fd(fd) });
        }
        let e = io::Error::last_os_error();
        if e.kind() != io::ErrorKind::Interrupted {
            return Err(e);
        }
    }
}

/// `openat(dirfd, name, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC)`
/// — the single-component traversal open. Fails (`ELOOP`/`ENOTDIR`)
/// on links and non-directories rather than following them.
#[expect(unsafe_code, reason = "openat(2) is libc FFI; see SAFETY comment")]
fn open_component(dirfd: RawFd, name: impl AsRef<OsStr>) -> io::Result<File> {
    let c = cstring(name.as_ref())?;
    loop {
        // SAFETY: `dirfd` is a valid open directory descriptor, `c` a
        // valid NUL-terminated string; flags are the documented
        // openat(2) contract for a no-follow directory open.
        let fd = unsafe {
            libc::openat(
                dirfd,
                c.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                0,
            )
        };
        if fd >= 0 {
            // SAFETY: `fd` is a freshly opened descriptor owned by us.
            return Ok(unsafe { File::from_raw_fd(fd) });
        }
        let e = io::Error::last_os_error();
        if e.kind() != io::ErrorKind::Interrupted {
            return Err(e);
        }
    }
}

/// `openat(dirfd, name, O_WRONLY | O_CREAT | O_EXCL | O_NOFOLLOW |
/// O_CLOEXEC, 0600)` — atomic exclusive create: refuses any existing
/// name (including links) and never follows.
#[expect(unsafe_code, reason = "openat(2) is libc FFI; see SAFETY comment")]
fn create_file(dirfd: RawFd, name: &str) -> io::Result<File> {
    let c = cstring(OsStr::new(name))?;
    loop {
        // SAFETY: `dirfd`/`c` as above; flags are the documented
        // exclusive-create contract for extraction staging files.
        let fd = unsafe {
            libc::openat(
                dirfd,
                c.as_ptr(),
                libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                FILE_MODE,
            )
        };
        if fd >= 0 {
            // SAFETY: `fd` is a freshly opened descriptor owned by us.
            return Ok(unsafe { File::from_raw_fd(fd) });
        }
        let e = io::Error::last_os_error();
        if e.kind() != io::ErrorKind::Interrupted {
            return Err(e);
        }
    }
}

#[expect(unsafe_code, reason = "mkdirat(2) is libc FFI; see SAFETY comment")]
fn mkdirat(dirfd: RawFd, name: &str) -> io::Result<()> {
    let c = cstring(OsStr::new(name))?;
    loop {
        // SAFETY: `dirfd`/`c` as above; mode is the staging directory
        // permission.
        let rc = unsafe { libc::mkdirat(dirfd, c.as_ptr(), DIR_MODE) };
        if rc == 0 {
            return Ok(());
        }
        let e = io::Error::last_os_error();
        if e.kind() != io::ErrorKind::Interrupted {
            return Err(e);
        }
    }
}

fn unlinkat(dirfd: RawFd, name: &str, flags: i32) -> io::Result<()> {
    let c = cstring(OsStr::new(name))?;
    unlinkat_raw(dirfd, &c, flags)
}

fn unlinkat_os(dirfd: RawFd, name: &OsStr, flags: i32) -> io::Result<()> {
    let c = cstring(name)?;
    unlinkat_raw(dirfd, &c, flags)
}

#[expect(unsafe_code, reason = "unlinkat(2) is libc FFI; see SAFETY comment")]
fn unlinkat_raw(dirfd: RawFd, c: &CString, flags: i32) -> io::Result<()> {
    loop {
        // SAFETY: `dirfd` is a valid open directory descriptor, `c` a
        // valid NUL-terminated string; `flags` is AT_REMOVEDIR or 0 —
        // unlinkat never follows the leaf link.
        let rc = unsafe { libc::unlinkat(dirfd, c.as_ptr(), flags) };
        if rc == 0 {
            return Ok(());
        }
        let e = io::Error::last_os_error();
        if e.kind() != io::ErrorKind::Interrupted {
            return Err(e);
        }
    }
}

/// `dev_t` → `u64`: identity where `dev_t` is already `u64` (Linux,
/// Android); a checked conversion elsewhere (e.g. macOS `dev_t` is
/// `i32`).
#[cfg(any(target_os = "linux", target_os = "android"))]
fn dev_id(dev: libc::dev_t) -> u64 {
    dev
}

#[cfg(not(any(target_os = "linux", target_os = "android")))]
fn dev_id(dev: libc::dev_t) -> u64 {
    u64::try_from(dev).unwrap_or(u64::MAX)
}

fn secs_nanos(sec: i64, nsec: i64) -> u64 {
    u64::try_from(sec)
        .unwrap_or(0)
        .saturating_mul(1_000_000_000)
        .saturating_add(u64::try_from(nsec).unwrap_or(0))
}

/// Birth-time (inode creation time) → ns discriminator. A
/// delete+recreate at a recycled inode always gets a fresh birth
/// time, so it discriminates objects that even share (dev, ino).
/// Apple targets expose `st_birthtimespec` in `stat`.
#[cfg(target_vendor = "apple")]
fn birth_ns(st: &libc::stat) -> u64 {
    secs_nanos(st.st_birthtime, st.st_birthtime_nsec)
}

/// Change-time → ns discriminator (`st_ctime`/`st_ctime_nsec`).
/// ctime cannot be set by a caller and mutates on every write —
/// usable for files only, and only when the record is refreshed
/// post-write. Unused on Apple, where birth time covers all
/// discriminators.
#[cfg(not(target_vendor = "apple"))]
fn ctime_ns(st: &libc::stat) -> u64 {
    secs_nanos(st.st_ctime, st.st_ctime_nsec)
}

/// Discriminator for directories: birth time where the platform
/// reports it through `stat` (Apple), else `0`. A directory's ctime
/// mutates under child writes so it can never discriminate.
#[cfg(target_vendor = "apple")]
fn dir_discriminator(st: &libc::stat) -> u64 {
    birth_ns(st)
}

#[cfg(not(target_vendor = "apple"))]
fn dir_discriminator(_st: &libc::stat) -> u64 {
    0
}

/// Discriminator for non-directory objects: birth time on Apple,
/// change time elsewhere. The session refreshes file identities
/// post-write so a ctime-based discriminator still reflects the
/// post-write state.
#[cfg(target_vendor = "apple")]
fn object_discriminator(st: &libc::stat) -> u64 {
    birth_ns(st)
}

#[cfg(not(target_vendor = "apple"))]
fn object_discriminator(st: &libc::stat) -> u64 {
    ctime_ns(st)
}

/// Builds the [`NodeIdentity`] for a `stat` result: type tag from
/// `S_IFMT`; discriminator per the platform rules above.
fn node_identity(st: &libc::stat) -> NodeIdentity {
    let mode = st.st_mode & libc::S_IFMT;
    let (tag, discriminator) = if mode == libc::S_IFDIR {
        (NodeTypeTag::Dir, dir_discriminator(st))
    } else if mode == libc::S_IFREG {
        (NodeTypeTag::File, object_discriminator(st))
    } else {
        (NodeTypeTag::Other, object_discriminator(st))
    };
    NodeIdentity {
        volume: dev_id(st.st_dev),
        index: u128::from(st.st_ino),
        tag,
        discriminator,
        handle: None,
    }
}

/// `statx(2)`-backed identity (Linux/glibc). Reports `stx_btime`
/// birth time — the strongest discriminator — for every object type
/// when the filesystem provides `STATX_BTIME`; falls back to
/// `stx_ctime` for non-directories otherwise.
#[cfg(all(target_os = "linux", target_env = "gnu"))]
mod gnu_statx {
    use super::{NodeHandle, NodeIdentity, NodeTypeTag, OsStr, RawFd, cstring, io, secs_nanos};

    pub(super) fn statx_fd(fd: RawFd) -> io::Result<NodeIdentity> {
        let mut id = statx_at(fd, c"".as_ptr(), libc::AT_EMPTY_PATH)?;
        id.handle = handle_at(fd, c"".as_ptr(), libc::AT_EMPTY_PATH).ok();
        Ok(id)
    }

    pub(super) fn statx_child(dirfd: RawFd, name: &OsStr) -> io::Result<NodeIdentity> {
        let c = cstring(name)?;
        let mut id = statx_at(dirfd, c.as_ptr(), libc::AT_SYMLINK_NOFOLLOW)?;
        // name_to_handle_at never follows the leaf link unless
        // AT_SYMLINK_FOLLOW is passed — flags=0 gives the leaf
        // object's own handle, link or otherwise.
        id.handle = handle_at(dirfd, c.as_ptr(), 0).ok();
        Ok(id)
    }

    /// `statx(dirfd, pathname, flags, STATX_ALL)` with EINTR retry.
    /// `AT_EMPTY_PATH` + `""` describes `dirfd` itself;
    /// `AT_SYMLINK_NOFOLLOW` describes the leaf object, never a link
    /// target.
    #[expect(unsafe_code, reason = "statx(2) is libc FFI; see SAFETY comment")]
    fn statx_at(
        dirfd: RawFd,
        pathname: *const std::ffi::c_char,
        flags: std::ffi::c_int,
    ) -> io::Result<NodeIdentity> {
        let mut stx = std::mem::MaybeUninit::<libc::statx>::uninit();
        loop {
            // SAFETY: `dirfd` is a valid open descriptor; `pathname`
            // is a valid NUL-terminated C string that outlives the
            // call; `stx` is valid for `size_of::<statx>()` bytes of
            // write.
            let rc =
                unsafe { libc::statx(dirfd, pathname, flags, libc::STATX_ALL, stx.as_mut_ptr()) };
            if rc == 0 {
                // SAFETY: `stx` was fully initialized by the call.
                let stx = unsafe { stx.assume_init() };
                return Ok(identity_from_statx(&stx));
            }
            let e = io::Error::last_os_error();
            if e.kind() != io::ErrorKind::Interrupted {
                return Err(e);
            }
        }
    }

    fn ts_ns(ts: &libc::statx_timestamp) -> u64 {
        secs_nanos(ts.tv_sec, i64::from(ts.tv_nsec))
    }

    fn identity_from_statx(stx: &libc::statx) -> NodeIdentity {
        let mode = u32::from(stx.stx_mode) & libc::S_IFMT;
        let birth = (stx.stx_mask & libc::STATX_BTIME != 0).then(|| ts_ns(&stx.stx_btime));
        let (tag, discriminator) = if mode == libc::S_IFDIR {
            // Directory ctime mutates under child writes — birth
            // time only, else none.
            (NodeTypeTag::Dir, birth.unwrap_or(0))
        } else if mode == libc::S_IFREG {
            (
                NodeTypeTag::File,
                birth.unwrap_or_else(|| ts_ns(&stx.stx_ctime)),
            )
        } else {
            (
                NodeTypeTag::Other,
                birth.unwrap_or_else(|| ts_ns(&stx.stx_ctime)),
            )
        };
        NodeIdentity {
            // Compose the device id from statx major/minor the same
            // way makedev() does — opaque equality key only.
            volume: (u64::from(stx.stx_dev_major) << 32) | u64::from(stx.stx_dev_minor),
            index: u128::from(stx.stx_ino),
            tag,
            discriminator,
            handle: None,
        }
    }

    const MAX_HANDLE_SZ: usize = 128;

    /// Buffer for `name_to_handle_at(2)`: the libc crate does not
    /// export `struct file_handle` for gnu targets, so the fixed
    /// layout (header + `MAX_HANDLE_SZ` payload) is spelled out here.
    #[repr(C)]
    struct HandleBuf {
        handle_bytes: libc::c_uint,
        handle_type: libc::c_int,
        f_handle: [u8; MAX_HANDLE_SZ],
    }

    /// `name_to_handle_at(dirfd, pathname, flags)` — the exported
    /// file handle embeds the inode generation, discriminating a
    /// delete+recreate that reuses the inode within one timestamp
    /// tick. No-follow is the default leaf behavior; valid flags
    /// are `AT_EMPTY_PATH` (describe `dirfd` itself) and
    /// `AT_SYMLINK_FOLLOW` (never passed — a link reports its own
    /// handle). Best-effort: callers degrade to `handle: None` on
    /// EOPNOTSUPP/ENOSYS (filesystems without export support).
    #[expect(
        unsafe_code,
        reason = "name_to_handle_at(2) via libc::syscall; see SAFETY comment"
    )]
    fn handle_at(
        dirfd: RawFd,
        pathname: *const std::ffi::c_char,
        flags: std::ffi::c_int,
    ) -> io::Result<NodeHandle> {
        let mut buf = HandleBuf {
            handle_bytes: u32::try_from(MAX_HANDLE_SZ).unwrap_or(u32::MAX),
            handle_type: 0,
            f_handle: [0; MAX_HANDLE_SZ],
        };
        let mut mnt_id: libc::c_int = 0;
        loop {
            // SAFETY: `dirfd` is a valid open descriptor; `pathname`
            // is a valid NUL-terminated C string that outlives the
            // call; `buf` is a valid `file_handle` buffer of
            // MAX_HANDLE_SZ bytes capacity; `mnt_id` is valid for
            // write.
            let rc = unsafe {
                libc::syscall(
                    libc::SYS_name_to_handle_at,
                    dirfd,
                    pathname,
                    &raw mut buf,
                    &raw mut mnt_id,
                    flags,
                )
            };
            if rc == 0 {
                let cap = u32::try_from(MAX_HANDLE_SZ).unwrap_or(u32::MAX);
                let len = u8::try_from(buf.handle_bytes.min(cap)).unwrap_or(u8::MAX);
                return Ok(NodeHandle {
                    kind: buf.handle_type,
                    len,
                    bytes: buf.f_handle,
                });
            }
            let e = io::Error::last_os_error();
            if e.kind() != io::ErrorKind::Interrupted {
                return Err(e);
            }
        }
    }
}
fn stat_fd(fd: RawFd) -> io::Result<NodeIdentity> {
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    match gnu_statx::statx_fd(fd) {
        Ok(id) => return Ok(id),
        // statx(2) requires Linux 4.11+; ENOSYS means an ancient
        // kernel where plain stat is the graceful fallback.
        Err(e) if e.raw_os_error() == Some(libc::ENOSYS) => {}
        Err(e) => return Err(e),
    }
    stat_fd_stat(fd)
}

#[expect(unsafe_code, reason = "fstat(2) is libc FFI; see SAFETY comment")]
fn stat_fd_stat(fd: RawFd) -> io::Result<NodeIdentity> {
    let mut st = std::mem::MaybeUninit::<libc::stat>::uninit();
    loop {
        // SAFETY: `st` is valid for writes of `size_of::<stat>()` and
        // `fd` is a valid open descriptor.
        let rc = unsafe { libc::fstat(fd, st.as_mut_ptr()) };
        if rc == 0 {
            // SAFETY: `st` was fully initialized by the successful call.
            let st = unsafe { st.assume_init() };
            return Ok(node_identity(&st));
        }
        let e = io::Error::last_os_error();
        if e.kind() != io::ErrorKind::Interrupted {
            return Err(e);
        }
    }
}

fn stat_child(dirfd: RawFd, name: &str) -> io::Result<NodeIdentity> {
    stat_child_os(dirfd, OsStr::new(name))
}

fn stat_child_os(dirfd: RawFd, name: &OsStr) -> io::Result<NodeIdentity> {
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    match gnu_statx::statx_child(dirfd, name) {
        Ok(id) => return Ok(id),
        Err(e) if e.raw_os_error() == Some(libc::ENOSYS) => {}
        Err(e) => return Err(e),
    }
    stat_child_stat(dirfd, name)
}

/// `fstatat(dirfd, name, AT_SYMLINK_NOFOLLOW)` — identity of the child
/// object at `name` *itself* (a link reports the link's identity).
#[expect(unsafe_code, reason = "fstatat(2) is libc FFI; see SAFETY comment")]
fn stat_child_stat(dirfd: RawFd, name: &OsStr) -> io::Result<NodeIdentity> {
    let c = cstring(name)?;
    let mut st = std::mem::MaybeUninit::<libc::stat>::uninit();
    loop {
        // SAFETY: `dirfd`/`c`/`st` as above; AT_SYMLINK_NOFOLLOW makes
        // the result describe the leaf object, never a link target.
        let rc = unsafe {
            libc::fstatat(
                dirfd,
                c.as_ptr(),
                st.as_mut_ptr(),
                libc::AT_SYMLINK_NOFOLLOW,
            )
        };
        if rc == 0 {
            // SAFETY: `st` was fully initialized by the successful call.
            let st = unsafe { st.assume_init() };
            return Ok(node_identity(&st));
        }
        let e = io::Error::last_os_error();
        if e.kind() != io::ErrorKind::Interrupted {
            return Err(e);
        }
    }
}
