//! Real OS backend for the shared vfs layer — `UnixVfsOps`.
//!
//! Every operation is dirfd-relative and no-follow:
//!
//! - The trusted directory is opened once (`O_RDONLY | O_DIRECTORY |
//!   O_CLOEXEC`; its own path resolution is the caller's trust
//!   boundary).
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
//! - Identity is `fstat`/`fstatat(AT_SYMLINK_NOFOLLOW)`/`statx` on the
//!   same handle — the values name the object the handle addresses,
//!   never the path string.
//! - Read-side ops: `readlinkat` for link targets, `fdopendir` on a
//!   fresh `openat(dirfd, ".")` descriptor for listings (a shared
//!   dirfd's offset is never disturbed), `fstatat
//!   (AT_SYMLINK_NOFOLLOW)` probes, and the per-OS
//!   `open_regular_leaf` leaf-finalization documented on the trait.
//!
//! **RT contract:** non-RT only.

#![cfg(unix)]

use std::ffi::{CStr, CString, OsStr, OsString};
use std::fs::File;
use std::io;
use std::os::fd::{AsRawFd, FromRawFd, RawFd};
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};

use super::{NodeHandle, NodeIdentity, NodeTypeTag, VfsDir, VfsEntry, VfsKind, VfsOps, VfsProbe};
use crate::rooted::IdentityStrength;

const DIR_MODE: libc::mode_t = 0o700;
const FILE_MODE: libc::c_uint = 0o600;

/// The unix [`VfsOps`] implementation.
#[derive(Debug)]
pub(crate) struct UnixVfsOps;

impl VfsOps for UnixVfsOps {
    fn open_dir_path(&self, path: &Path) -> io::Result<Box<dyn VfsDir>> {
        let fd = open_dir_at(path)?;
        Ok(Box::new(UnixVfsDir {
            fd,
            path: path.to_path_buf(),
        }))
    }

    fn stat_file(&self, file: &File) -> io::Result<VfsProbe> {
        probe_fd(file.as_raw_fd())
    }
}

/// An open directory (dirfd + diagnostic path).
#[derive(Debug)]
struct UnixVfsDir {
    fd: File,
    path: PathBuf,
}

impl VfsDir for UnixVfsDir {
    fn path(&self) -> &Path {
        &self.path
    }

    fn identity(&self) -> io::Result<NodeIdentity> {
        Ok(stat_fd(self.fd.as_raw_fd())?.identity)
    }

    fn stat_self(&self) -> io::Result<VfsProbe> {
        stat_fd(self.fd.as_raw_fd())
    }

    fn canonicalize_handle(&self) -> io::Result<PathBuf> {
        canonicalize_fd(self.fd.as_raw_fd())
    }

    fn probe(&self, name: &OsStr) -> io::Result<VfsProbe> {
        stat_child(self.fd.as_raw_fd(), name)
    }

    fn open_dir(&self, name: &OsStr) -> io::Result<Box<dyn VfsDir>> {
        let fd = open_component(self.fd.as_raw_fd(), name)?;
        Ok(Box::new(UnixVfsDir {
            fd,
            path: self.path.join(name),
        }))
    }

    fn open_regular_leaf(&self, name: &OsStr) -> io::Result<(File, VfsProbe)> {
        open_leaf(self.fd.as_raw_fd(), name)
    }

    fn read_link(&self, name: &OsStr) -> io::Result<OsString> {
        read_link_at(self.fd.as_raw_fd(), name)
    }

    fn list(&self) -> io::Result<Vec<VfsEntry>> {
        list_dir(self)
    }

    fn create_dir(&self, name: &OsStr) -> io::Result<Box<dyn VfsDir>> {
        mkdirat(self.fd.as_raw_fd(), name)?;
        let fd = open_component(self.fd.as_raw_fd(), name)?;
        Ok(Box::new(UnixVfsDir {
            fd,
            path: self.path.join(name),
        }))
    }

    fn create_file(&self, name: &OsStr) -> io::Result<File> {
        create_file(self.fd.as_raw_fd(), name)
    }

    fn remove_file(&self, name: &OsStr) -> io::Result<()> {
        unlinkat(self.fd.as_raw_fd(), name, 0)
    }

    fn remove_dir(&self, name: &OsStr) -> io::Result<()> {
        unlinkat(self.fd.as_raw_fd(), name, libc::AT_REMOVEDIR)
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
            "ruviform-io-file: component contains an interior NUL byte",
        )
    })
}

/// Opens a path-level directory (trusted path only — whether links
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
fn create_file(dirfd: RawFd, name: &OsStr) -> io::Result<File> {
    let c = cstring(name)?;
    loop {
        // SAFETY: `dirfd`/`c` as above; flags are the documented
        // exclusive-create contract for staging files.
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
fn mkdirat(dirfd: RawFd, name: &OsStr) -> io::Result<()> {
    let c = cstring(name)?;
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

fn unlinkat(dirfd: RawFd, name: &OsStr, flags: i32) -> io::Result<()> {
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

/// `readlinkat(dirfd, name)` — the raw target string stored in the
/// link object; the buffer doubles until the target fits (silent
/// truncation can never produce a different resolved object).
#[expect(unsafe_code, reason = "readlinkat(2) is libc FFI; see SAFETY comment")]
fn read_link_at(dirfd: RawFd, name: &OsStr) -> io::Result<OsString> {
    let c = cstring(name)?;
    let mut cap = 512usize;
    loop {
        let mut buf = vec![0u8; cap];
        // SAFETY: `dirfd`/`c` as above; `buf` is valid for `cap`
        // bytes of write.
        let n = unsafe { libc::readlinkat(dirfd, c.as_ptr(), buf.as_mut_ptr().cast(), cap) };
        if n < 0 {
            let e = io::Error::last_os_error();
            if e.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(e);
        }
        let n = usize::try_from(n).unwrap_or(0);
        if n < cap {
            buf.truncate(n);
            return Ok(OsString::from_vec(buf));
        }
        if cap >= 1 << 16 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "ruviform-io-file: link target exceeds 64 KiB",
            ));
        }
        cap *= 2;
    }
}

// ------------------------------------------------------------------
// `open_regular_leaf` — the per-OS "no nonregular reaches an ordinary
// blocking open" leaf finalization.
// ------------------------------------------------------------------

/// Linux/Android: `openat(O_PATH|O_NOFOLLOW|O_CLOEXEC)` pins the inode
/// without blocking or device-open side effects; `fstat`/`statx` on
/// that descriptor *is* the probe (same object — no probe→open
/// window). Regular files are reopened through
/// `/proc/self/fd/N` (`O_RDONLY`) and identity-verified against the
/// `O_PATH` probe — magic-link resolution references the inode, never
/// re-walks a path, and re-runs `MAY_READ`/LSM checks. `/proc`
/// absent or reopen impossible → `Unsupported` (no silent fallback).
#[cfg(any(target_os = "linux", target_os = "android"))]
fn open_leaf(dirfd: RawFd, name: &OsStr) -> io::Result<(File, VfsProbe)> {
    let opath = open_opath(dirfd, name)?;
    let probe = probe_fd(opath.as_raw_fd())?;
    if probe.kind != VfsKind::File {
        // Return the pinned-but-unreadable object plus its probe;
        // the caller classifies `kind` and drops the file.
        return Ok((opath, probe));
    }
    let link = PathBuf::from(format!("/proc/self/fd/{}", opath.as_raw_fd()));
    let c = cstring(link.as_os_str())?;
    let reopened = open_readonly(&c).map_err(|e| {
        io::Error::new(
            io::ErrorKind::Unsupported,
            format!("ruviform-io-file: /proc/self/fd reopen failed ({e})"),
        )
    })?;
    let verify = probe_fd(reopened.as_raw_fd())?;
    if verify.identity != probe.identity {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "ruviform-io-file: /proc/self/fd reopen identity mismatch",
        ));
    }
    Ok((reopened, verify))
}

/// `openat(dirfd, name, O_PATH | O_NOFOLLOW | O_CLOEXEC)` — pins the
/// inode; `ELOOP` on links, never blocks on devices.
#[cfg(any(target_os = "linux", target_os = "android"))]
#[expect(unsafe_code, reason = "openat(2) is libc FFI; see SAFETY comment")]
fn open_opath(dirfd: RawFd, name: &OsStr) -> io::Result<File> {
    let c = cstring(name)?;
    loop {
        // SAFETY: `dirfd`/`c` as above; O_PATH opens a reference to
        // the object without data access or device side effects.
        let fd = unsafe {
            libc::openat(
                dirfd,
                c.as_ptr(),
                libc::O_PATH | libc::O_NOFOLLOW | libc::O_CLOEXEC,
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

#[cfg(any(target_os = "linux", target_os = "android"))]
#[expect(unsafe_code, reason = "open(2) is libc FFI; see SAFETY comment")]
fn open_readonly(c: &CString) -> io::Result<File> {
    loop {
        // SAFETY: `c` is a valid NUL-terminated string that outlives
        // the call; flags are the documented open(2) contract.
        let fd = unsafe { libc::open(c.as_ptr(), libc::O_RDONLY | libc::O_CLOEXEC) };
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

/// Other unix (no `O_PATH`): `openat(O_RDONLY|O_NOFOLLOW|O_NONBLOCK|
/// O_CLOEXEC)` — `O_NONBLOCK` keeps FIFOs from blocking at open —
/// then `fstat` on the opened descriptor decides the kind (the
/// probe→open window can only deliver a *different in-root regular
/// file*, which is accepted, or a nonregular, which the caller
/// rejects). `O_NONBLOCK` is cleared before the file is returned so
/// reads keep blocking semantics. Residual: device-open side effects
/// (documented design limit).
#[cfg(not(any(target_os = "linux", target_os = "android")))]
fn open_leaf(dirfd: RawFd, name: &OsStr) -> io::Result<(File, VfsProbe)> {
    let file = open_nonblock(dirfd, name)?;
    let probe = probe_fd(file.as_raw_fd())?;
    if probe.kind == VfsKind::File {
        clear_nonblock(file.as_raw_fd())?;
    }
    Ok((file, probe))
}

#[cfg(not(any(target_os = "linux", target_os = "android")))]
#[expect(unsafe_code, reason = "openat(2) is libc FFI; see SAFETY comment")]
fn open_nonblock(dirfd: RawFd, name: &OsStr) -> io::Result<File> {
    let c = cstring(name)?;
    loop {
        // SAFETY: `dirfd`/`c` as above; flags are the documented
        // non-blocking no-follow open contract.
        let fd = unsafe {
            libc::openat(
                dirfd,
                c.as_ptr(),
                libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC,
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

#[cfg(not(any(target_os = "linux", target_os = "android")))]
#[expect(unsafe_code, reason = "fcntl(2) is libc FFI; see SAFETY comment")]
fn clear_nonblock(fd: RawFd) -> io::Result<()> {
    // SAFETY: `fd` is a valid open descriptor; F_GETFL/F_SETFL take
    // no pointer arguments.
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: as above; clearing O_NONBLOCK restores blocking reads.
    let rc = unsafe { libc::fcntl(fd, libc::F_SETFL, flags & !libc::O_NONBLOCK) };
    if rc < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

// ------------------------------------------------------------------
// Directory listing — fresh fd + fdopendir per call so the shared
// dirfd's enumeration offset is never disturbed.
// ------------------------------------------------------------------

#[expect(
    unsafe_code,
    reason = "openat/fdopendir/readdir/closedir are libc FFI; see SAFETY comments"
)]
fn list_dir(dir: &UnixVfsDir) -> io::Result<Vec<VfsEntry>> {
    let dot = cstring(OsStr::new("."))?;
    // SAFETY: `dir.fd` is a valid open directory descriptor and `dot`
    // a valid C string; flags are the openat(2) contract. A fresh fd
    // is required because fdopendir takes ownership of the offset.
    let fd = unsafe {
        libc::openat(
            dir.fd.as_raw_fd(),
            dot.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC,
            0,
        )
    };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: `fd` is a fresh descriptor transferred to fdopendir.
    let dirp = unsafe { libc::fdopendir(fd) };
    if dirp.is_null() {
        let e = io::Error::last_os_error();
        // SAFETY: `fd` is still ours (fdopendir failed).
        unsafe { libc::close(fd) };
        return Err(e);
    }
    let mut out = Vec::new();
    loop {
        // SAFETY: `dirp` is a valid DIR*; readdir returns entries
        // owned by `dirp` (not freed) until closedir.
        let ent = unsafe { libc::readdir(dirp) };
        if ent.is_null() {
            break;
        }
        // SAFETY: `ent` is a valid dirent until the next readdir.
        let ent = unsafe { &*ent };
        // SAFETY: d_name is NUL-terminated within the dirent.
        let name_c = unsafe { CStr::from_ptr(ent.d_name.as_ptr()) };
        let name_bytes = name_c.to_bytes();
        if name_bytes == b"." || name_bytes == b".." {
            continue;
        }
        let name = OsString::from_vec(name_bytes.to_vec());
        let kind = match dirent_kind(ent) {
            Some(kind) => kind,
            // DT_UNKNOWN (or no d_type on this platform): classify
            // per entry with the same no-follow probe.
            None => dir.probe(&name)?.kind,
        };
        out.push(VfsEntry { name, kind });
    }
    // SAFETY: `dirp` is a valid DIR*; closedir releases it and the fd.
    unsafe { libc::closedir(dirp) };
    Ok(out)
}

/// `dirent.d_type` → [`VfsKind`] where the platform exposes the
/// field; `DT_UNKNOWN`/`None` requests a per-entry probe fallback.
#[cfg(any(
    target_os = "linux",
    target_os = "android",
    target_vendor = "apple",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
))]
fn dirent_kind(ent: &libc::dirent) -> Option<VfsKind> {
    match ent.d_type {
        libc::DT_REG => Some(VfsKind::File),
        libc::DT_DIR => Some(VfsKind::Dir),
        libc::DT_LNK => Some(VfsKind::Symlink),
        libc::DT_UNKNOWN => None,
        _ => Some(VfsKind::Other),
    }
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "android",
    target_vendor = "apple",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
)))]
fn dirent_kind(_ent: &libc::dirent) -> Option<VfsKind> {
    None
}

// ------------------------------------------------------------------
// Handle-derived canonical path (the handle, not a path string, is
// resolved — RootedDir::open's trust boundary).
// ------------------------------------------------------------------

/// Linux/Android: `/proc/self/fd/N` readlink names the open object.
/// `/proc` absent or the link unreadable → error; the caller falls
/// back to `fs::canonicalize` (documented residual race).
#[cfg(any(target_os = "linux", target_os = "android"))]
fn canonicalize_fd(fd: RawFd) -> io::Result<PathBuf> {
    std::fs::read_link(format!("/proc/self/fd/{fd}"))
}

/// Apple: `fcntl(F_GETPATH)` writes the object's path into the
/// caller's buffer.
#[cfg(all(unix, target_vendor = "apple"))]
#[expect(
    unsafe_code,
    reason = "fcntl(F_GETPATH) is libc FFI; see SAFETY comment"
)]
fn canonicalize_fd(fd: RawFd) -> io::Result<PathBuf> {
    let mut buf = vec![0u8; libc::PATH_MAX as usize];
    // SAFETY: `fd` is a valid descriptor; `buf` is valid for
    // PATH_MAX bytes of write.
    let rc = unsafe { libc::fcntl(fd, libc::F_GETPATH, buf.as_mut_ptr()) };
    if rc != 0 {
        return Err(io::Error::last_os_error());
    }
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    buf.truncate(end);
    Ok(PathBuf::from(OsString::from_vec(buf)))
}

#[cfg(all(
    unix,
    not(any(target_os = "linux", target_os = "android", target_vendor = "apple"))
))]
fn canonicalize_fd(_fd: RawFd) -> io::Result<PathBuf> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "ruviform-io-file: no handle-derived canonical path on this platform",
    ))
}

// ------------------------------------------------------------------
// Identity / probe plumbing.
// ------------------------------------------------------------------

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

/// Birth-time (inode creation time) → ns discriminator where the
/// platform's `stat` reports it (Apple `st_birthtimespec`). A
/// delete+recreate at a recycled inode always gets a fresh birth
/// time, so it discriminates objects that even share (dev, ino).
/// `None` on unix targets whose `stat` carries no birth time —
/// those fall to `Weak` identity strength (honest, never claimed).
#[cfg(target_vendor = "apple")]
fn birth_ns(st: &libc::stat) -> u64 {
    secs_nanos(st.st_birthtime, st.st_birthtime_nsec)
}

#[cfg(not(target_vendor = "apple"))]
fn birth_ns(_st: &libc::stat) -> Option<u64> {
    None
}

/// Change-time → ns discriminator (`st_ctime`/`st_ctime_nsec`).
/// ctime cannot be set by a caller and mutates on every write —
/// usable for files only, and only when the record is refreshed
/// post-write.
fn ctime_ns(st: &libc::stat) -> u64 {
    secs_nanos(st.st_ctime, st.st_ctime_nsec)
}

/// `stat` fields normalized for probe construction — shared by the
/// `fstat`/`fstatat` and `statx` paths.
struct RawStat {
    dev: u64,
    ino: u128,
    mode: u64,
    size: i64,
    mtime_ns: u64,
    /// Birth time where reported — the strong discriminator.
    birth_ns: Option<u64>,
    /// Change time — the weak non-directory discriminator.
    ctime_ns: u64,
    handle: Option<NodeHandle>,
}

impl RawStat {
    /// The discriminator policy: directories can only use birth
    /// time (their ctime mutates under child writes); other objects
    /// fall back to change time.
    fn discriminator(&self, is_dir: bool) -> u64 {
        if is_dir {
            self.birth_ns.unwrap_or(0)
        } else {
            self.birth_ns.unwrap_or(self.ctime_ns)
        }
    }
}

fn kind_of(mode: u64) -> VfsKind {
    match mode & u64::from(libc::S_IFMT) {
        m if m == u64::from(libc::S_IFREG) => VfsKind::File,
        m if m == u64::from(libc::S_IFDIR) => VfsKind::Dir,
        m if m == u64::from(libc::S_IFLNK) => VfsKind::Symlink,
        _ => VfsKind::Other,
    }
}

fn tag_of(kind: VfsKind) -> NodeTypeTag {
    match kind {
        VfsKind::File => NodeTypeTag::File,
        VfsKind::Dir => NodeTypeTag::Dir,
        _ => NodeTypeTag::Other,
    }
}

fn system_time_ns(ns: u64) -> std::time::SystemTime {
    std::time::UNIX_EPOCH + std::time::Duration::from_nanos(ns)
}

fn probe_from(raw: &RawStat) -> VfsProbe {
    let kind = kind_of(raw.mode);
    let strong = raw.handle.is_some() || raw.birth_ns.is_some();
    VfsProbe {
        identity: NodeIdentity {
            volume: raw.dev,
            index: raw.ino,
            tag: tag_of(kind),
            discriminator: raw.discriminator(kind == VfsKind::Dir),
            handle: raw.handle,
        },
        kind,
        byte_len: match kind {
            VfsKind::File | VfsKind::Symlink => Some(u64::try_from(raw.size).unwrap_or(u64::MAX)),
            _ => None,
        },
        modified: Some(system_time_ns(raw.mtime_ns)),
        strength: if strong {
            IdentityStrength::Strong
        } else {
            IdentityStrength::Weak
        },
    }
}

fn raw_from_stat(st: &libc::stat) -> RawStat {
    RawStat {
        dev: dev_id(st.st_dev),
        ino: u128::from(st.st_ino),
        mode: u64::from(st.st_mode),
        size: st.st_size,
        mtime_ns: secs_nanos(st.st_mtime, st.st_mtime_nsec),
        #[cfg(target_vendor = "apple")]
        birth_ns: Some(birth_ns(st)),
        #[cfg(not(target_vendor = "apple"))]
        birth_ns: birth_ns(st),
        ctime_ns: ctime_ns(st),
        handle: None,
    }
}

/// `statx(2)`-backed probe (Linux/glibc). Reports `stx_btime`
/// birth time — the strongest discriminator — for every object type
/// when the filesystem provides `STATX_BTIME`; falls back to
/// `stx_ctime` for non-directories otherwise.
#[cfg(all(target_os = "linux", target_env = "gnu"))]
mod gnu_statx {
    use super::{NodeHandle, RawStat, cstring, io, secs_nanos};
    use std::ffi::OsStr;
    use std::os::fd::RawFd;

    pub(super) fn statx_fd(fd: RawFd) -> io::Result<RawStat> {
        let mut raw = statx_at(fd, c"".as_ptr(), libc::AT_EMPTY_PATH)?;
        raw.handle = handle_at(fd, c"".as_ptr(), libc::AT_EMPTY_PATH).ok();
        Ok(raw)
    }

    pub(super) fn statx_child(dirfd: RawFd, name: &OsStr) -> io::Result<RawStat> {
        let c = cstring(name)?;
        let mut raw = statx_at(dirfd, c.as_ptr(), libc::AT_SYMLINK_NOFOLLOW)?;
        // name_to_handle_at never follows the leaf link unless
        // AT_SYMLINK_FOLLOW is passed — flags=0 gives the leaf
        // object's own handle, link or otherwise.
        raw.handle = handle_at(dirfd, c.as_ptr(), 0).ok();
        Ok(raw)
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
    ) -> io::Result<RawStat> {
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
                return Ok(raw_from_statx(&stx));
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

    fn raw_from_statx(stx: &libc::statx) -> RawStat {
        RawStat {
            // Compose the device id from statx major/minor the same
            // way makedev() does — opaque equality key only.
            dev: (u64::from(stx.stx_dev_major) << 32) | u64::from(stx.stx_dev_minor),
            ino: u128::from(stx.stx_ino),
            mode: u64::from(stx.stx_mode),
            // statx reports `stx_size` as u64; `RawStat.size` follows
            // the `stat.st_size`/`off_t` contract (i64). Kernel inode
            // sizes are `loff_t` so >i64::MAX is unreachable — clamp
            // honestly rather than wrap if an exotic filesystem ever
            // reports it. Negative sizes cannot exist in u64 source.
            size: i64::try_from(stx.stx_size).unwrap_or(i64::MAX),
            mtime_ns: ts_ns(&stx.stx_mtime),
            birth_ns: (stx.stx_mask & libc::STATX_BTIME != 0).then(|| ts_ns(&stx.stx_btime)),
            ctime_ns: ts_ns(&stx.stx_ctime),
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

/// Probe of an already-open descriptor (`fstat`/`statx
/// AT_EMPTY_PATH` — same object the fd addresses).
fn probe_fd(fd: RawFd) -> io::Result<VfsProbe> {
    Ok(probe_from(&stat_fd_raw(fd)?))
}

fn stat_fd(fd: RawFd) -> io::Result<VfsProbe> {
    probe_fd(fd)
}

fn stat_fd_raw(fd: RawFd) -> io::Result<RawStat> {
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    match gnu_statx::statx_fd(fd) {
        Ok(raw) => return Ok(raw),
        // statx(2) requires Linux 4.11+; ENOSYS means an ancient
        // kernel where plain stat is the graceful fallback.
        Err(e) if e.raw_os_error() == Some(libc::ENOSYS) => {}
        Err(e) => return Err(e),
    }
    stat_fd_stat(fd)
}

#[expect(unsafe_code, reason = "fstat(2) is libc FFI; see SAFETY comment")]
fn stat_fd_stat(fd: RawFd) -> io::Result<RawStat> {
    let mut st = std::mem::MaybeUninit::<libc::stat>::uninit();
    loop {
        // SAFETY: `st` is valid for writes of `size_of::<stat>()` and
        // `fd` is a valid open descriptor.
        let rc = unsafe { libc::fstat(fd, st.as_mut_ptr()) };
        if rc == 0 {
            // SAFETY: `st` was fully initialized by the successful call.
            let st = unsafe { st.assume_init() };
            return Ok(raw_from_stat(&st));
        }
        let e = io::Error::last_os_error();
        if e.kind() != io::ErrorKind::Interrupted {
            return Err(e);
        }
    }
}

fn stat_child(dirfd: RawFd, name: &OsStr) -> io::Result<VfsProbe> {
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    match gnu_statx::statx_child(dirfd, name) {
        Ok(raw) => return Ok(probe_from(&raw)),
        Err(e) if e.raw_os_error() == Some(libc::ENOSYS) => {}
        Err(e) => return Err(e),
    }
    stat_child_stat(dirfd, name)
}

/// `fstatat(dirfd, name, AT_SYMLINK_NOFOLLOW)` — probe of the child
/// object at `name` *itself* (a link reports the link's identity).
#[expect(unsafe_code, reason = "fstatat(2) is libc FFI; see SAFETY comment")]
fn stat_child_stat(dirfd: RawFd, name: &OsStr) -> io::Result<VfsProbe> {
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
            return Ok(probe_from(&raw_from_stat(&st)));
        }
        let e = io::Error::last_os_error();
        if e.kind() != io::ErrorKind::Interrupted {
            return Err(e);
        }
    }
}
