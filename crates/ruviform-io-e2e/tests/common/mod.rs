//! Filesystem helpers shared by the composition test binaries.
//!
//! Same RAII shape as `ruviform-io-archive/tests/common`: unique
//! names come from pid + counter + time so parallel test binaries
//! never collide; no `tempfile` dependency.
#![allow(dead_code)]

pub mod wav;

use std::io;
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

/// Creates a directory link at `link` → `target` (unix symlink;
/// Windows junction via `mklink /J` — both are name-surrogate
/// reparse/symlink objects the rooted capability must classify
/// `Symlink`). `Err` means the platform cannot create links for this
/// test — callers skip honestly.
///
/// Same primitives as `ruviform-io-file/tests/common`.
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

/// Patches the `external_attrs` field (fixed-part offset +38..42) of
/// every central-directory record in a ZIP byte stream, returning
/// the number of records patched.
///
/// The ZIP writer cannot emit link/special entries — `external_attrs`
/// is the only channel through which unix mode bits (e.g. `S_IFLNK`)
/// reach `EntryKind` classification on read. CRC32 covers payload
/// bytes only, so patching central-directory metadata leaves every
/// integrity check intact: the archive stays *valid*, and the hostile
/// kind is a classification/policy matter for the extraction layer.
///
/// The walk is layout-verified: the EOCD record locates the central
/// directory and each record's name/extra/comment lengths are used to
/// step to the next. A writer layout change that breaks this walk
/// fails the assertion instead of silently patching the wrong bytes.
pub fn patch_zip_external_attrs(bytes: &mut [u8], attrs: u32) -> usize {
    fn u16le(b: &[u8]) -> u16 {
        u16::from_le_bytes([b[0], b[1]])
    }
    fn u32le(b: &[u8]) -> u32 {
        u32::from_le_bytes([b[0], b[1], b[2], b[3]])
    }

    // Locate the EOCD record by scanning the bounded tail backwards
    // for its signature (mirrors the reader's own search window).
    const EOCD_SIG: u32 = 0x0605_4b50;
    const EOCD_LEN: usize = 22;
    const CD_SIG: u32 = 0x0201_4b50;
    let len = bytes.len();
    assert!(len >= EOCD_LEN, "zip too small for EOCD");
    let span = len.min(EOCD_LEN + u16::MAX as usize);
    let tail_start = len - span;
    let mut eocd_at = None;
    for pos in (tail_start..=len - EOCD_LEN).rev() {
        if u32le(&bytes[pos..pos + 4]) == EOCD_SIG {
            let comment_len = usize::from(u16le(&bytes[pos + 20..pos + 22]));
            if pos + EOCD_LEN + comment_len == len {
                eocd_at = Some(pos);
                break;
            }
        }
    }
    let eocd_at = eocd_at.expect("no end-of-central-directory record found");
    let cd_count = usize::from(u16le(&bytes[eocd_at + 10..eocd_at + 12]));
    let mut cursor =
        usize::try_from(u32le(&bytes[eocd_at + 16..eocd_at + 20])).expect("cd offset fits usize");

    let mut patched = 0;
    for _ in 0..cd_count {
        assert!(
            cursor + 46 <= len && u32le(&bytes[cursor..cursor + 4]) == CD_SIG,
            "central-directory record {patched} missing or misplaced at {cursor}"
        );
        let name_len = usize::from(u16le(&bytes[cursor + 28..cursor + 30]));
        let extra_len = usize::from(u16le(&bytes[cursor + 30..cursor + 32]));
        let comment_len = usize::from(u16le(&bytes[cursor + 32..cursor + 34]));
        bytes[cursor + 38..cursor + 42].copy_from_slice(&attrs.to_le_bytes());
        patched += 1;
        cursor += 46 + name_len + extra_len + comment_len;
    }
    assert!(patched > 0, "zip has no central-directory records");
    patched
}
