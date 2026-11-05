//! Filesystem staging helpers for extraction tests.
//!
//! `TestDir` is an RAII temp directory (the extraction `parent`);
//! unique names come from pid + counter + time so parallel test
//! binaries never collide.
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
                "rvf-extract-test-{tag}-{}-{n}-{nanos:016x}",
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

    /// The only child entries inside this dir (for staging-root
    /// uniqueness assertions).
    #[must_use]
    pub fn children(&self) -> Vec<PathBuf> {
        let mut out: Vec<PathBuf> = std::fs::read_dir(&self.path)
            .expect("read test dir")
            .filter_map(|e| e.ok().map(|e| e.path()))
            .collect();
        out.sort();
        out
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// Reads a staged file's bytes; `None` when absent.
#[must_use]
pub fn read_staged(path: &Path) -> Option<Vec<u8>> {
    std::fs::read(path).ok()
}

/// Whether `path` exists (any kind, following links).
#[must_use]
pub fn exists(path: &Path) -> bool {
    path.try_exists().unwrap_or(false)
}
