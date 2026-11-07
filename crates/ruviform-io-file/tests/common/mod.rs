//! Filesystem helpers for rooted-capability integration tests.
//!
//! `TestDir` is an RAII temp directory; unique names come from pid +
//! counter + time so parallel test binaries never collide. Link
//! creation is per-OS: unix symlinks, Windows junctions via
//! `mklink /J` (unprivileged) and `symlink_file`/`symlink_dir`
//! best-effort (skipped by tests when the token lacks
//! `SeCreateSymbolicLinkPrivilege`).
#![allow(dead_code)]

use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// A temporary directory removed on drop.
pub struct TestDir {
    path: PathBuf,
}

impl TestDir {
    /// Creates a uniquely named empty directory under the system temp dir.
    #[must_use]
    pub fn new(tag: &str) -> Self {
        let base = std::env::temp_dir();
        for _ in 0..64 {
            let n = COUNTER.fetch_add(1, Ordering::Relaxed);
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| u64::try_from(d.as_nanos()).unwrap_or(u64::MAX));
            let path = base.join(format!(
                "rvf-rooted-test-{tag}-{}-{n}-{nanos:016x}",
                std::process::id()
            ));
            if std::fs::create_dir(&path).is_ok() {
                return Self { path };
            }
        }
        panic!(
            "could not create a unique temp dir under {}",
            base.display()
        );
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// Creates a directory link at `link` → `target` (unix symlink;
/// Windows junction via `mklink /J` — both are name-surrogate
/// reparse/symlink objects the capability must classify `Symlink`).
/// `Err` means the platform cannot create links for this test —
/// callers skip.
pub fn dir_link(target: &Path, link: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link)
    }
    #[cfg(windows)]
    {
        junction(target, link)
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (target, link);
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "no link primitive",
        ))
    }
}

/// Creates a file link at `link` → `target` (unix symlink; Windows
/// `symlink_file`, which typically needs
/// `SeCreateSymbolicLinkPrivilege` — tests must skip on `Err`).
pub fn file_link(target: &Path, link: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link)
    }
    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_file(target, link)
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (target, link);
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "no link primitive",
        ))
    }
}

/// `mklink /J` — the unprivileged directory-junction primitive.
#[cfg(windows)]
fn junction(target: &Path, link: &Path) -> io::Result<()> {
    let status = std::process::Command::new("cmd")
        .args([
            "/c",
            "mklink",
            "/J",
            link.to_str()
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "non-utf8 link path"))?,
            target.to_str().ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "non-utf8 target path")
            })?,
        ])
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!("mklink /J failed with {status}")))
    }
}

/// Writes a regular file with bytes.
pub fn write_file(path: &Path, data: &[u8]) {
    std::fs::write(path, data).expect("write test file");
}

/// Creates a fifo at `path` (unix only).
#[cfg(unix)]
#[expect(unsafe_code, reason = "mkfifo(3) is libc FFI; see SAFETY comment")]
pub fn make_fifo(path: &Path) -> io::Result<()> {
    use std::os::unix::ffi::OsStrExt;
    let c = std::ffi::CString::new(path.as_os_str().as_bytes())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "path contains NUL"))?;
    // SAFETY: `c` is a valid NUL-terminated string that outlives the
    // call; mode 0600 confines the fifo to this test.
    let rc = unsafe { libc::mkfifo(c.as_ptr(), 0o600) };
    if rc == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}
