//! Real OS backend for `PlatformFileOps` — `OsFileOps` (IO-IMP-02).
//!
//! `OsFileOps` owns a temp sibling of the destination plus the
//! destination/directory paths it will publish into. Every atomicity
//! and durability guarantee comes from a real OS primitive — never
//! from an `exists()`-then-`rename()` pair:
//!
//! - `CreateNew`: `renameat2(RENAME_NOREPLACE)` on Linux/Android,
//!   `renamex_np(RENAME_EXCL)` on Apple targets, `MoveFileExW`
//!   without `MOVEFILE_REPLACE_EXISTING` on Windows, and a
//!   `link()`+`unlink()` hardlink fallback on other Unix targets —
//!   taken only where the primary primitive is absent (see
//!   [`noreplace`]).
//! - `Replace`: `fs::rename` (POSIX `rename` / `MoveFileExW` with
//!   replace plus std's `FileRenameInfoEx` fallback on Windows).
//! - file sync: `File::sync_all` (`fsync` / `FlushFileBuffers`).
//! - directory sync: open the destination directory and `sync_all`;
//!   on Windows the handle is opened read+write with
//!   `FILE_FLAG_BACKUP_SEMANTICS`, and a filesystem that cannot
//!   provide it reports an honest `DirectorySync` failure.
//!
//! Publish errors are classified by *effect*, not just by error value:
//! definitively refused operations (`EEXIST`, `EACCES`, `ENOENT`,
//! `EISDIR`, …) report `NotApplied` with the temp `Retained`; error
//! codes from which the operation may still have applied remotely
//! (`EIO`, `ETIMEDOUT`, `ESTALE`, `ENOLINK`, network-family codes —
//! the "ambiguous" set) report `Unknown`/`Unknown`, which the pipeline
//! normalizes to `PublishEffect::Indeterminate`.

use crate::options::{PublishMode, PublishOptions};
use crate::platform::{
    ApplyOutcome, CleanupAttempt, PlatformFileOps, PublishAttempt, ReadSeek, TempDisposition,
};
use std::fs::{self, File, OpenOptions};
use std::io::{self, ErrorKind, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// Upper bound on exclusive temp-name attempts before `create` gives
/// up — astronomically unlikely to reach with a random suffix.
const MAX_TEMP_ATTEMPTS: u32 = 64;

/// The real-OS [`PlatformFileOps`] implementation.
///
/// `file` is `Some` while the temp handle is open; it is taken
/// (closed) by `prepare_for_publish`/`publish`/`cleanup_temp` before
/// rename or removal so Windows does not see a sharing violation and
/// so the temp handle cannot outlive the operation that consumed it.
#[derive(Debug)]
pub(crate) struct OsFileOps {
    /// Where the publish must land — resolved to absolute at create.
    dest: PathBuf,
    /// The staged temp sibling (`dest` may be a *diagnostic* target;
    /// this is the concrete file the ops act on).
    temp_path: PathBuf,
    /// The directory holding `dest`/`temp_path` — synced for
    /// `FileAndDirectorySync`.
    dir_path: PathBuf,
    /// The open temp handle.
    file: Option<File>,
    /// Readback cursor for the `Read + Seek` borrow — independent of
    /// the file's own cursor so readback never disturbs write
    /// position.
    rpos: u64,
}

impl OsFileOps {
    /// Opens a fresh temp sibling of `destination` exclusively
    /// (`create_new`), retrying on name collision.
    ///
    /// `destination` is resolved to an absolute path up front so a
    /// later `cwd` change cannot split the temp and the destination
    /// into different directories.
    pub(crate) fn create(destination: &Path) -> io::Result<Self> {
        let name = destination.file_name().ok_or_else(|| {
            io::Error::new(
                ErrorKind::InvalidInput,
                "ruviform-io-file: destination has no file name",
            )
        })?;
        let dest = std::path::absolute(destination)?;
        let dir_path = dest
            .parent()
            .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
        for _ in 0..MAX_TEMP_ATTEMPTS {
            let temp_path = dir_path.join(temp_name(name));
            match OpenOptions::new()
                .read(true)
                .write(true)
                .create_new(true)
                .open(&temp_path)
            {
                Ok(file) => {
                    return Ok(Self {
                        dest,
                        temp_path,
                        dir_path,
                        file: Some(file),
                        rpos: 0,
                    });
                }
                Err(e) if e.kind() != ErrorKind::AlreadyExists => return Err(e),
                Err(_) => {}
            }
        }
        Err(io::Error::new(
            ErrorKind::AlreadyExists,
            "ruviform-io-file: temp name space exhausted",
        ))
    }

    /// The open temp handle, or an error once it has been released.
    fn file(&self) -> io::Result<&File> {
        self.file.as_ref().ok_or_else(closed_error)
    }

    fn file_mut(&mut self) -> io::Result<&mut File> {
        self.file.as_mut().ok_or_else(closed_error)
    }

    /// Releases the temp handle — the honest meaning of "close" here.
    /// Rust `File` drop cannot report `close(2)` errors; durability
    /// errors are observed at `sync_file` instead (documented limit).
    fn release_handle(&mut self) {
        self.file.take();
    }
}

fn closed_error() -> io::Error {
    io::Error::other("ruviform-io-file: staged temp handle is closed")
}

/// Builds `<name>.rvf-tmp-<token>` — a diagnostic, collision-resistant
/// sibling name. The token is not cryptographic; it mixes time, pid
/// and a process counter, and `create_new` retries cover collisions.
fn temp_name(name: &std::ffi::OsStr) -> std::ffi::OsString {
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
    let mut out = std::ffi::OsString::from(name);
    out.push(format!(".rvf-tmp-{x:016x}"));
    out
}

/// Shared `Seek` arithmetic: resolves `pos` against `cursor`/`len`
/// with `i128` headroom so `End(-n)` and negative `Current` offsets
/// cannot wrap.
fn resolve_seek(cursor: &mut u64, len: u64, pos: SeekFrom) -> io::Result<u64> {
    let next = match pos {
        SeekFrom::Start(n) => i128::from(n),
        SeekFrom::End(n) => i128::from(len) + i128::from(n),
        SeekFrom::Current(n) => i128::from(*cursor) + i128::from(n),
    };
    *cursor = u64::try_from(next).map_err(|_| {
        io::Error::new(
            ErrorKind::InvalidInput,
            "ruviform-io-file: seek out of range",
        )
    })?;
    Ok(*cursor)
}

// ------------------------------------------------------------------
// Readback surface (`Read + Seek` over the same staged object).
// ------------------------------------------------------------------

impl Read for OsFileOps {
    /// Positioned read at `rpos` — the same underlying object, never a
    /// path re-open.
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = positioned_read(self.file()?, buf, self.rpos)?;
        self.rpos += n as u64;
        Ok(n)
    }
}

impl Seek for OsFileOps {
    /// Moves the *readback* cursor (`rpos`); `End` resolves against
    /// current file metadata length, not a cached size.
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let len = self.file()?.metadata()?.len();
        resolve_seek(&mut self.rpos, len, pos)
    }
}

#[cfg(unix)]
fn positioned_read(file: &File, buf: &mut [u8], pos: u64) -> io::Result<usize> {
    use std::os::unix::fs::FileExt;
    file.read_at(buf, pos)
}

#[cfg(windows)]
fn positioned_read(file: &File, buf: &mut [u8], pos: u64) -> io::Result<usize> {
    use std::os::windows::fs::FileExt;
    file.seek_read(buf, pos)
}

// ------------------------------------------------------------------
// Publish classification helpers.
// ------------------------------------------------------------------

fn applied(temp: TempDisposition) -> PublishAttempt {
    PublishAttempt {
        effect: ApplyOutcome::Applied,
        temp,
        error: None,
    }
}

/// A publish error is `Unknown`/`Unknown` only when the OS error
/// cannot prove the operation did not apply (the "ambiguous" set);
/// every other failure is a definite `NotApplied` with the temp
/// `Retained` on disk.
fn failed(error: io::Error) -> PublishAttempt {
    if is_ambiguous_error(&error) {
        PublishAttempt {
            effect: ApplyOutcome::Unknown,
            temp: TempDisposition::Unknown,
            error: Some(error),
        }
    } else {
        PublishAttempt {
            effect: ApplyOutcome::NotApplied,
            temp: TempDisposition::Retained,
            error: Some(error),
        }
    }
}

/// `rename(temp, dest)` — atomic create-or-replace on both families.
fn replace_publish(temp: &Path, dest: &Path) -> PublishAttempt {
    match fs::rename(temp, dest) {
        Ok(()) => applied(TempDisposition::Removed),
        Err(e) => failed(e),
    }
}

// ------------------------------------------------------------------
// `CreateNew` — the atomic no-replace primitive, per OS.
// ------------------------------------------------------------------

/// Internal tri-state for the unix no-replace primitive: applied
/// cleanly, applied but the temp could not be unlinked (hardlink
/// fallback leaves the staged path behind), or
/// definitively/ambiguously failed.
#[cfg(unix)]
enum Noreplace {
    Applied,
    AppliedTempRetained(io::Error),
    Failed(io::Error),
}

#[cfg(unix)]
fn create_new_publish(temp: &Path, dest: &Path) -> PublishAttempt {
    match noreplace(temp, dest) {
        Noreplace::Applied => applied(TempDisposition::Removed),
        Noreplace::AppliedTempRetained(e) => {
            // The destination got the content; the temp path still
            // exists (hardlink fallback where `unlink` failed). The
            // publish itself applied — report the unlink error
            // alongside `Retained` for diagnostics.
            PublishAttempt {
                effect: ApplyOutcome::Applied,
                temp: TempDisposition::Retained,
                error: Some(e),
            }
        }
        Noreplace::Failed(e) => failed(e),
    }
}

#[cfg(unix)]
fn cstring(path: &Path) -> io::Result<std::ffi::CString> {
    use std::os::unix::ffi::OsStrExt;
    std::ffi::CString::new(path.as_os_str().as_bytes()).map_err(|_| {
        io::Error::new(
            ErrorKind::InvalidInput,
            "ruviform-io-file: path contains an interior NUL byte",
        )
    })
}

/// Linux/Android: `renameat2(RENAME_NOREPLACE)` via raw `libc::syscall`
/// — the libc wrapper symbol is not declared for every unix target
/// (std itself calls the raw syscall for the same reason). On `ENOSYS`
/// (kernel predates the syscall) fall back to `link`+`unlink`; on
/// `EINVAL` retry the fallback — and, matching std's rule, report the
/// original `EINVAL` unless the fallback applied or proved the
/// destination exists.
#[cfg(any(target_os = "linux", target_os = "android"))]
#[expect(
    unsafe_code,
    reason = "raw renameat2 syscall: libc does not expose a portable wrapper on all unix targets; see SAFETY comment"
)]
fn noreplace(temp: &Path, dest: &Path) -> Noreplace {
    let (Ok(t), Ok(d)) = (cstring(temp), cstring(dest)) else {
        return Noreplace::Failed(io::Error::new(
            ErrorKind::InvalidInput,
            "ruviform-io-file: path contains an interior NUL byte",
        ));
    };
    loop {
        // SAFETY: `t`/`d` are valid NUL-terminated C strings that
        // outlive the call; AT_FDCWD/RENAME_NOREPLACE are the kernel
        // ABI constants for the relative-path no-replace rename.
        let rc = unsafe {
            libc::syscall(
                libc::SYS_renameat2,
                libc::AT_FDCWD,
                t.as_ptr(),
                libc::AT_FDCWD,
                d.as_ptr(),
                libc::RENAME_NOREPLACE,
            )
        };
        if rc >= 0 {
            return Noreplace::Applied;
        }
        let e = io::Error::last_os_error();
        if e.kind() != ErrorKind::Interrupted {
            return match e.raw_os_error() {
                Some(libc::ENOSYS) => link_then_unlink(temp, dest),
                Some(libc::EINVAL) => match link_then_unlink(temp, dest) {
                    Noreplace::Failed(f) if f.kind() != ErrorKind::AlreadyExists => {
                        Noreplace::Failed(e)
                    }
                    other => other,
                },
                _ => Noreplace::Failed(e),
            };
        }
    }
}

/// Apple: `renamex_np(RENAME_EXCL)` is the atomic no-replace rename;
/// on `ENOTSUP`/`ENOSYS` fall back to `link`+`unlink`.
#[cfg(all(unix, target_vendor = "apple"))]
#[expect(unsafe_code, reason = "renamex_np is libc FFI; see SAFETY comment")]
fn noreplace(temp: &Path, dest: &Path) -> Noreplace {
    let (Ok(t), Ok(d)) = (cstring(temp), cstring(dest)) else {
        return Noreplace::Failed(io::Error::new(
            ErrorKind::InvalidInput,
            "ruviform-io-file: path contains an interior NUL byte",
        ));
    };
    loop {
        // SAFETY: `t`/`d` are valid NUL-terminated C strings that
        // outlive the call; RENAME_EXCL is the documented exclusive
        // flag of `renamex_np`.
        let rc = unsafe { libc::renamex_np(t.as_ptr(), d.as_ptr(), libc::RENAME_EXCL) };
        if rc == 0 {
            return Noreplace::Applied;
        }
        let e = io::Error::last_os_error();
        if e.kind() == ErrorKind::Interrupted {
            continue;
        }
        return match e.raw_os_error() {
            Some(libc::ENOTSUP) | Some(libc::ENOSYS) => link_then_unlink(temp, dest),
            _ => Noreplace::Failed(e),
        };
    }
}

/// Other Unix targets have no guaranteed atomic no-replace rename, so
/// the verified fallback is the primary path.
#[cfg(all(
    unix,
    not(any(target_os = "linux", target_os = "android", target_vendor = "apple"))
))]
fn noreplace(temp: &Path, dest: &Path) -> Noreplace {
    link_then_unlink(temp, dest)
}

/// The verified hardlink fallback: `link(temp, dest)` creates the
/// destination atomically only if absent (atomic on POSIX filesystems
/// — the same fallback `std::fs` uses for `rename_noreplace`). The
/// temp is then unlinked; unlink failure reports
/// `Applied`+`Retained`, never a false "published". `EEXIST` maps to
/// `AlreadyExists` (→ `DestinationExists`); `EPERM`/`EOPNOTSUPP`/
/// `ENOSYS`/`ENOTSUP` here mean the filesystem cannot hardlink at all
/// — reported honestly as `Unsupported`.
#[cfg(unix)]
fn link_then_unlink(temp: &Path, dest: &Path) -> Noreplace {
    match fs::hard_link(temp, dest) {
        Ok(()) => match fs::remove_file(temp) {
            Ok(()) => Noreplace::Applied,
            Err(e) => Noreplace::AppliedTempRetained(e),
        },
        Err(e) => {
            if is_unsupported_errno(&e) {
                Noreplace::Failed(io::Error::new(ErrorKind::Unsupported, e))
            } else {
                Noreplace::Failed(e)
            }
        }
    }
}

/// At the `link()` call site these errnos mean the filesystem cannot
/// provide the operation (`protected_hardlinks`-style policy,
/// filesystems without link support, missing syscall) — capability
/// failures, not destination conflicts.
#[cfg(unix)]
fn is_unsupported_errno(e: &io::Error) -> bool {
    let Some(code) = e.raw_os_error() else {
        return false;
    };
    code == libc::EPERM
        || code == libc::EOPNOTSUPP
        || code == libc::ENOSYS
        || unsupported_extra(code)
}

/// Darwin keeps `ENOTSUP` distinct from `EOPNOTSUPP`; on Linux they
/// alias, so a literal second match arm would warn on duplication.
#[cfg(all(unix, target_vendor = "apple"))]
const fn unsupported_extra(code: i32) -> bool {
    code == libc::ENOTSUP
}

#[cfg(all(unix, not(target_vendor = "apple")))]
const fn unsupported_extra(code: i32) -> bool {
    let _ = code;
    false
}

/// Errnos from which a rename/link may have applied remotely but the
/// result was lost (storage/network transport) — classified
/// `Unknown`, never as a false `NotApplied`.
#[cfg(unix)]
fn is_ambiguous_error(e: &io::Error) -> bool {
    let Some(code) = e.raw_os_error() else {
        return false;
    };
    code == libc::EIO
        || code == libc::ETIMEDOUT
        || code == libc::ESTALE
        || code == libc::ENOLINK
        || code == libc::ENETDOWN
        || code == libc::ENETUNREACH
        || code == libc::ENETRESET
        || code == libc::ECONNABORTED
        || code == libc::ECONNRESET
        || ambiguous_extra(code)
}

#[cfg(any(target_os = "linux", target_os = "android"))]
const fn ambiguous_extra(code: i32) -> bool {
    code == libc::EREMOTEIO || code == libc::ECOMM || code == libc::ESHUTDOWN
}

#[cfg(all(unix, target_vendor = "apple"))]
const fn ambiguous_extra(code: i32) -> bool {
    code == libc::ESHUTDOWN
}

#[cfg(all(
    unix,
    not(any(target_os = "linux", target_os = "android", target_vendor = "apple"))
))]
const fn ambiguous_extra(code: i32) -> bool {
    let _ = code;
    false
}

/// Windows: `MoveFileExW(temp, dest, 0)` — no `MOVEFILE_*` flags —
/// performs the kernel rename and refuses an existing destination.
/// `ERROR_ACCESS_DENIED` is ambiguous on Windows (it may hide an
/// existing destination), so a single *diagnostic* `symlink_metadata`
/// probe distinguishes "exists" from a real access failure — the
/// atomicity mechanism remains the rename itself.
#[cfg(windows)]
fn create_new_publish(temp: &Path, dest: &Path) -> PublishAttempt {
    use windows_sys::Win32::Foundation;
    match move_file_ex(temp, dest, 0) {
        Ok(()) => applied(TempDisposition::Removed),
        Err(e) => match e.raw_os_error().map(i32::cast_unsigned) {
            Some(Foundation::ERROR_ALREADY_EXISTS | Foundation::ERROR_FILE_EXISTS) => {
                failed(io::Error::new(ErrorKind::AlreadyExists, e))
            }
            Some(Foundation::ERROR_ACCESS_DENIED) => {
                // Diagnostic only — decide whether the denial actually
                // meant "destination exists". The rename was the
                // atomic check; nothing here re-checks-then-acts.
                match fs::symlink_metadata(dest) {
                    Ok(_) => failed(io::Error::new(ErrorKind::AlreadyExists, e)),
                    Err(_) => failed(e),
                }
            }
            _ => failed(e),
        },
    }
}

#[cfg(windows)]
#[expect(
    unsafe_code,
    reason = "MoveFileExW is windows-sys FFI; see SAFETY comment"
)]
fn move_file_ex(temp: &Path, dest: &Path, flags: u32) -> io::Result<()> {
    let t = wide_path(temp)?;
    let d = wide_path(dest)?;
    // SAFETY: both are valid NUL-terminated UTF-16 buffers that
    // outlive the call; `flags` is the documented MOVEFILE_* mask.
    let ok = unsafe {
        windows_sys::Win32::Storage::FileSystem::MoveFileExW(t.as_ptr(), d.as_ptr(), flags)
    };
    if ok == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(windows)]
fn wide_path(path: &Path) -> io::Result<Vec<u16>> {
    use std::os::windows::ffi::OsStrExt;
    let mut v: Vec<u16> = path.as_os_str().encode_wide().collect();
    if v.contains(&0) {
        return Err(io::Error::new(
            ErrorKind::InvalidInput,
            "ruviform-io-file: path contains an interior NUL",
        ));
    }
    v.push(0);
    Ok(v)
}

/// Win32 error codes from which the operation may have applied
/// remotely — network-transport and device-level indeterminacy.
#[cfg(windows)]
fn is_ambiguous_error(e: &io::Error) -> bool {
    use windows_sys::Win32::Foundation as F;
    let Some(code) = e.raw_os_error().map(i32::cast_unsigned) else {
        return false;
    };
    matches!(
        code,
        F::ERROR_BAD_NET_RESP
            | F::ERROR_UNEXP_NET_ERR
            | F::ERROR_NETNAME_DELETED
            | F::ERROR_SEM_TIMEOUT
            | F::ERROR_NETWORK_BUSY
            | F::ERROR_REM_NOT_LIST
            | F::ERROR_BAD_NET_NAME
            | F::ERROR_NETWORK_UNREACHABLE
            | F::ERROR_HOST_UNREACHABLE
            | F::ERROR_PROTOCOL_UNREACHABLE
            | F::ERROR_PORT_UNREACHABLE
            | F::ERROR_CONNECTION_ABORTED
            | F::ERROR_CONNECTION_INVALID
            | F::ERROR_CONNECTION_UNAVAIL
            | F::ERROR_IO_DEVICE
            | F::ERROR_DEVICE_NOT_CONNECTED
    )
}

// ------------------------------------------------------------------
// Directory sync.
// ------------------------------------------------------------------

#[cfg(unix)]
fn sync_dir(dir: &Path) -> io::Result<()> {
    File::open(dir)?.sync_all()
}

/// Windows directory sync: the directory handle needs
/// `FILE_FLAG_BACKUP_SEMANTICS` to be openable at all, and
/// `FlushFileBuffers` requires write access — so the handle is opened
/// read+write. A filesystem that refuses the handle or the flush
/// reports an honest `DirectorySync` failure; nothing here pretends
/// the entry is durable.
#[cfg(windows)]
fn sync_dir(dir: &Path) -> io::Result<()> {
    use std::os::windows::fs::OpenOptionsExt;
    OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(windows_sys::Win32::Storage::FileSystem::FILE_FLAG_BACKUP_SEMANTICS)
        .open(dir)?
        .sync_all()
}

// ------------------------------------------------------------------
// `PlatformFileOps` implementation.
// ------------------------------------------------------------------

impl PlatformFileOps for OsFileOps {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.file_mut()?.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file_mut()?.flush()
    }

    /// The *write* cursor — the file's own `lseek` position.
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.file_mut()?.seek(pos)
    }

    /// Borrows `self` as the readback surface: positioned reads on the
    /// same open handle, never an anonymous path re-open (TOCTOU).
    fn open_readback(&mut self) -> io::Result<&mut dyn ReadSeek> {
        self.file()?;
        self.rpos = 0;
        Ok(self)
    }

    /// Every option axis is attempted for real: atomicity is
    /// established by the publish primitive itself and durability by
    /// the sync calls — there is nothing to pre-verify cheaply.
    fn validate_publish(&self, _options: PublishOptions) -> io::Result<()> {
        Ok(())
    }

    fn sync_file(&mut self) -> io::Result<()> {
        self.file()?.sync_all()
    }

    /// Releases the temp handle. `close(2)` errors are unreportable
    /// through `Drop`; `sync_file` is the honest durability
    /// observation point (documented).
    fn prepare_for_publish(&mut self) -> io::Result<()> {
        self.release_handle();
        Ok(())
    }

    fn publish(&mut self, mode: PublishMode) -> PublishAttempt {
        // Defensive: never rename while our own handle is still open.
        self.release_handle();
        match mode {
            PublishMode::CreateNew => create_new_publish(&self.temp_path, &self.dest),
            PublishMode::Replace => replace_publish(&self.temp_path, &self.dest),
        }
    }

    fn sync_directory(&mut self) -> io::Result<()> {
        sync_dir(&self.dir_path)
    }

    /// Closes our handle, then removes the temp path. A missing temp
    /// already satisfies the goal (`Removed`); an ambiguous error is
    /// reported `Unknown` rather than a false removal.
    fn cleanup_temp(&mut self) -> CleanupAttempt {
        self.release_handle();
        match fs::remove_file(&self.temp_path) {
            Ok(()) => CleanupAttempt {
                temp: TempDisposition::Removed,
                error: None,
            },
            Err(e) if e.kind() == ErrorKind::NotFound => CleanupAttempt {
                temp: TempDisposition::Removed,
                error: None,
            },
            Err(e) if is_ambiguous_error(&e) => CleanupAttempt {
                temp: TempDisposition::Unknown,
                error: Some(e),
            },
            Err(e) => CleanupAttempt {
                temp: TempDisposition::Retained,
                error: Some(e),
            },
        }
    }

    /// `symlink_metadata` on the recorded temp path — a state report,
    /// not an identity proof. Lookup failures are `Unknown` (the
    /// state is undetermined), not a false "absent".
    fn inspect_temp(&self) -> io::Result<TempDisposition> {
        match fs::symlink_metadata(&self.temp_path) {
            Ok(_) => Ok(TempDisposition::Retained),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(TempDisposition::Removed),
            Err(_) => Ok(TempDisposition::Unknown),
        }
    }

    fn temp_path(&self) -> Option<&Path> {
        Some(&self.temp_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn workdir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "rvf-os-ops-{tag}-{}-{:x}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos())
        ));
        fs::create_dir_all(&dir).expect("workdir");
        dir
    }

    struct DirGuard(PathBuf);
    impl Drop for DirGuard {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn temp_names_are_unique_siblings() {
        let a = temp_name(std::ffi::OsStr::new("f.bin"));
        let b = temp_name(std::ffi::OsStr::new("f.bin"));
        assert_ne!(a, b);
        assert!(a.to_string_lossy().starts_with("f.bin.rvf-tmp-"));
    }

    #[test]
    fn resolve_seek_bounds() {
        let mut cursor = 2u64;
        assert_eq!(resolve_seek(&mut cursor, 10, SeekFrom::End(-1)).unwrap(), 9);
        assert_eq!(
            resolve_seek(&mut cursor, 10, SeekFrom::Current(-3)).unwrap(),
            6
        );
        assert!(resolve_seek(&mut cursor, 10, SeekFrom::Current(-7)).is_err());
        assert!(resolve_seek(&mut cursor, 10, SeekFrom::End(-11)).is_err());
    }

    #[test]
    fn create_stages_exclusive_temp_sibling() {
        let dir = workdir("create");
        let _g = DirGuard(dir.clone());
        let dest = dir.join("a.bin");
        let mut ops = OsFileOps::create(&dest).expect("create");
        assert!(ops.temp_path().expect("temp").starts_with(&dir));
        assert!(ops.temp_path().unwrap().exists());
        PlatformFileOps::write(&mut ops, b"x").expect("write");
        ops.cleanup_temp();
        assert!(!ops.temp_path().unwrap().exists());
    }

    #[test]
    fn create_new_publish_applies_then_refuses() {
        let dir = workdir("noreplace");
        let _g = DirGuard(dir.clone());
        let dest = dir.join("b.bin");
        let temp1 = dir.join("t1");
        fs::write(&temp1, b"one").expect("temp1");
        match create_new_publish(&temp1, &dest).effect {
            ApplyOutcome::Applied => {}
            other => panic!("first publish: {other:?}"),
        }
        assert_eq!(fs::read(&dest).unwrap(), b"one");
        let temp2 = dir.join("t2");
        fs::write(&temp2, b"two").expect("temp2");
        let attempt = create_new_publish(&temp2, &dest);
        assert_eq!(attempt.effect, ApplyOutcome::NotApplied);
        assert_eq!(attempt.temp, TempDisposition::Retained);
        assert_eq!(
            attempt.error.expect("error").kind(),
            ErrorKind::AlreadyExists
        );
        assert_eq!(fs::read(&dest).unwrap(), b"one");
        assert!(temp2.exists(), "losers' temp must be retained");
    }

    #[test]
    fn replace_publish_overwrites() {
        let dir = workdir("replace");
        let _g = DirGuard(dir.clone());
        let dest = dir.join("c.bin");
        fs::write(&dest, b"old").expect("dest");
        let temp = dir.join("t3");
        fs::write(&temp, b"new").expect("temp");
        assert_eq!(replace_publish(&temp, &dest).effect, ApplyOutcome::Applied);
        assert_eq!(fs::read(&dest).unwrap(), b"new");
        assert!(!temp.exists());
    }

    #[test]
    fn replace_publish_into_directory_is_definite_failure() {
        let dir = workdir("replace-dir");
        let _g = DirGuard(dir.clone());
        let dest = dir.join("d-dir");
        fs::create_dir(&dest).expect("dir");
        let temp = dir.join("t4");
        fs::write(&temp, b"x").expect("temp");
        let attempt = replace_publish(&temp, &dest);
        assert_eq!(attempt.effect, ApplyOutcome::NotApplied);
        assert_eq!(attempt.temp, TempDisposition::Retained);
        assert!(temp.exists());
    }

    #[test]
    fn inspect_reports_absent_and_retained() {
        let dir = workdir("inspect");
        let _g = DirGuard(dir.clone());
        let ops = OsFileOps::create(&dir.join("e.bin")).expect("create");
        assert_eq!(ops.inspect_temp().unwrap(), TempDisposition::Retained);
    }
}
