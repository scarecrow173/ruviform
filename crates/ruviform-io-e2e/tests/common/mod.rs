//! Filesystem helpers shared by the composition test binaries.
//!
//! Same RAII shape as `ruviform-io-archive/tests/common`: unique
//! names come from pid + counter + time so parallel test binaries
//! never collide; no `tempfile` dependency.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// A temporary parent directory removed on drop.
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
                "rvf-io-e2e-{tag}-{}-{n}-{nanos:016x}",
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

    /// Destination path for a staged file inside this dir.
    #[must_use]
    pub fn dest(&self, name: &str) -> PathBuf {
        self.path.join(name)
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// Reads a file's bytes; `None` when absent.
#[must_use]
pub fn read(path: &Path) -> Option<Vec<u8>> {
    std::fs::read(path).ok()
}

/// Whether `path` exists (any kind, following links).
#[must_use]
pub fn exists(path: &Path) -> bool {
    path.try_exists().unwrap_or(false)
}

/// Non-directory entries directly inside `dir` (temp-leftover scans).
#[must_use]
pub fn children(dir: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = std::fs::read_dir(dir)
        .expect("read test dir")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    out.sort();
    out
}

/// Builds a single-entry ustar TAR body for hostile fixture crafting.
///
/// The writer API refuses nothing about names beyond empty/NUL, but
/// cannot emit link/special entries at all — so negative extraction
/// fixtures (traversal, symlink, absolute paths) are built byte-exact
/// here instead of through `TarArchiveWriter`.
///
/// `name` must fit the 100-byte field; `typeflag` is the raw ustar
/// type byte (`b'0'` file, `b'5'` dir, `b'2'` symlink).
pub fn crafted_tar_entry(name: &str, typeflag: u8, link_name: &str, payload: &[u8]) -> Vec<u8> {
    assert!(name.len() <= 100, "crafted name fits ustar name field");
    assert!(
        link_name.len() <= 100,
        "crafted link target fits linkname field"
    );
    let mut header = [0u8; 512];
    let put = |dst: &mut [u8], value: &[u8]| {
        let n = value.len().min(dst.len() - 1);
        dst[..n].copy_from_slice(&value[..n]);
    };
    put(&mut header[0..100], name.as_bytes());
    put(&mut header[100..108], b"0000644");
    put(&mut header[108..116], b"0000000");
    put(&mut header[116..124], b"0000000");
    put(
        &mut header[124..136],
        format!("{payload_len:011o}", payload_len = payload.len()).as_bytes(),
    );
    put(&mut header[136..148], b"00000000000");
    // chksum field counts as spaces during the sum.
    header[148..156].fill(b' ');
    header[156] = typeflag;
    put(&mut header[157..257], link_name.as_bytes());
    put(&mut header[257..263], b"ustar\0");
    // POSIX ustar version field is ASCII "00" at offset 263.
    header[263..265].copy_from_slice(b"00");
    let sum: u32 = header.iter().map(|b| u32::from(*b)).sum();
    put(&mut header[148..156], format!("{sum:06o}\0 ").as_bytes());

    let mut out = header.to_vec();
    out.extend_from_slice(payload);
    let pad = (512 - payload.len() % 512) % 512;
    out.resize(out.len() + pad, 0);
    // Two zero blocks terminate the archive.
    out.resize(out.len() + 1024, 0);
    out
}
