//! Safe staged output: [`StagedOutput`], the boundary's generic
//! write-to-temp-then-publish primitive.
//!
//! Producing a completed destination file is a two-step contract:
//!
//! 1. A muxer writes the container into a *temporary sibling file* —
//!    the temp lives in the destination's directory, so it is always
//!    on the same filesystem and the final rename is a same-volume
//!    rename (best-effort atomic on the platforms Ruviform targets;
//!    no cross-filesystem atomicity is claimed).
//! 2. [`StagedOutput::publish`] syncs the temp file to storage
//!    (`File::sync_all`) and renames it over the destination.
//!
//! Anything that drops the [`StagedOutput`] before `publish` —
//! encode failure, cancellation, a disk-full write error, a
//! destination conflict — removes the temp file and leaves an
//! existing destination **byte-for-byte untouched**. A partially
//! written temp is never reported as a completed output.
//!
//! Product-level policy — *whether* an export may overwrite — stays
//! with the consumer; this primitive exposes only the
//! [`PublishOptions::replace_existing`] flag.
//!
//! **RT contract:** non-RT only — every method may block on
//! filesystem I/O.

use crate::error::{Error, ErrorKind, Result};
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// How [`StagedOutput::publish`] treats an existing destination.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PublishOptions {
    /// When `true`, `publish` replaces an existing destination
    /// (same-volume rename over it). When `false`, an existing
    /// destination makes `publish` fail with
    /// [`ErrorKind::InvalidValue`] — the destination stays
    /// byte-for-byte untouched either way until the rename succeeds.
    pub replace_existing: bool,
    /// When `true` (default), `publish` calls [`File::sync_all`] on
    /// the temp file before renaming — the checked "close/sync" half
    /// of the completion contract. Set `false` only for sinks where
    /// durability is meaningless (tests on scratch files).
    pub sync_before_publish: bool,
}

impl PublishOptions {
    /// Options that fail when the destination already exists.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn no_overwrite() -> Self {
        Self {
            replace_existing: false,
            sync_before_publish: true,
        }
    }

    /// Options that replace an existing destination.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn replace() -> Self {
        Self {
            replace_existing: true,
            sync_before_publish: true,
        }
    }
}

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);
const TEMP_ATTEMPTS: usize = 8;
const TEMP_TAG: &str = ".rvf-tmp-";

/// A staged output file: writes go to a temp sibling of the
/// destination; nothing touches the destination until
/// [`publish`](Self::publish).
///
/// `StagedOutput` owns the temp [`File`] for the whole sequence, so
/// the staged file can be handed to a muxer as a [`MediaSink`] (via
/// [`sink`](Self::sink), a `&mut File`) while durability stays with
/// this type: `publish` still owns the file when it calls
/// `File::sync_all`.
///
/// Drop and [`abort`](Self::abort) both delete the temp file;
/// `abort` additionally exists so callers can *say* "cancelled"
/// instead of relying on an incidental drop order.
///
/// **RT contract:** non-RT only — filesystem I/O.
///
/// [`MediaSink`]: crate::MediaSink
///
/// # Examples
///
/// ```no_run
/// use ruviform_media_io::{PublishOptions, StagedOutput};
/// use std::io::Write;
///
/// let mut staged = StagedOutput::create("out.wav")?;
/// staged.sink()?.write_all(b"payload")?;
/// staged.publish(PublishOptions::no_overwrite())?;
/// # Ok::<(), ruviform_media_io::Error>(())
/// ```
#[derive(Debug)]
pub struct StagedOutput {
    /// Temp file path, always a sibling of `dest_path`.
    temp_path: PathBuf,
    /// Final destination path.
    dest_path: PathBuf,
    /// The staged file — `None` only inside `publish`, after it was
    /// consumed by the sync step.
    file: Option<File>,
    /// Whether the destination already existed at `create` time —
    /// captured up front so `publish` can distinguish "conflict" from
    /// "no destination" without a second stat race.
    dest_existed: bool,
}

impl StagedOutput {
    /// Creates a staged output for `dest` by opening a unique temp
    /// file in `dest`'s directory (`O_EXCL`; a handful of collision
    /// retries, then [`ErrorKind::Io`]).
    ///
    /// **RT contract:** non-RT only — filesystem I/O.
    ///
    /// # Errors
    ///
    /// [`ErrorKind::Io`] when the temp file cannot be created (missing
    /// directory, permissions, name collisions beyond the retry
    /// bound).
    pub fn create(dest: impl AsRef<Path>) -> Result<Self> {
        let dest_path = dest.as_ref().to_path_buf();
        let dest_existed = dest_path.exists();
        for attempt in 0..TEMP_ATTEMPTS {
            let temp_path = Self::temp_name(&dest_path, attempt);
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp_path)
            {
                Ok(file) => {
                    return Ok(Self {
                        temp_path,
                        dest_path,
                        file: Some(file),
                        dest_existed,
                    });
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(Error::with_source(ErrorKind::Io, error)),
            }
        }
        Err(Error::from_kind(ErrorKind::Io))
    }

    /// The staged file as a writable sink — `&mut File` satisfies
    /// [`MediaSink`](crate::MediaSink), so muxer constructors can take
    /// it directly.
    ///
    /// **RT contract:** non-RT only — returns a borrow.
    ///
    /// # Errors
    ///
    /// [`ErrorKind::InvalidValue`] if the file was already consumed —
    /// unreachable before `publish`/`abort`.
    pub fn sink(&mut self) -> Result<&mut File> {
        self.file
            .as_mut()
            .ok_or_else(|| Error::from_kind(ErrorKind::InvalidValue))
    }

    /// The temp file path (diagnostics/tests; treat as opaque).
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn temp_path(&self) -> &Path {
        &self.temp_path
    }

    /// The destination path.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn dest_path(&self) -> &Path {
        &self.dest_path
    }

    /// Whether the destination existed when this staged output was
    /// created.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn dest_existed(&self) -> bool {
        self.dest_existed
    }

    /// Publishes the staged file: optionally syncs it to storage, then
    /// renames it over the destination.
    ///
    /// On success the destination holds the staged bytes. On *any*
    /// failure the temp file is removed and an existing destination is
    /// left untouched.
    ///
    /// **RT contract:** non-RT only — filesystem I/O (fsync + rename).
    ///
    /// # Errors
    ///
    /// [`ErrorKind::InvalidValue`] when `options.replace_existing` is
    /// `false` and the destination already exists.
    /// [`ErrorKind::Io`] on fsync/rename failure.
    pub fn publish(mut self, options: PublishOptions) -> Result<()> {
        let result = self.publish_inner(options);
        if result.is_err() {
            self.cleanup_temp();
        }
        result
    }

    fn publish_inner(&mut self, options: PublishOptions) -> Result<()> {
        if !options.replace_existing && self.dest_path.exists() {
            return Err(Error::with_source(
                ErrorKind::InvalidValue,
                std::io::Error::new(
                    std::io::ErrorKind::AlreadyExists,
                    "destination exists and replace_existing is false",
                ),
            ));
        }
        if options.sync_before_publish
            && let Some(file) = &mut self.file
        {
            file.sync_all()
                .map_err(|error| Error::with_source(ErrorKind::Io, error))?;
        }
        // Close before rename — some filesystems refuse to rename an
        // open file.
        drop(self.file.take());
        fs::rename(&self.temp_path, &self.dest_path)
            .map_err(|error| Error::with_source(ErrorKind::Io, error))
    }

    /// Cancels the staged output: deletes the temp file and never
    /// touches the destination. Equivalent to dropping, but explicit.
    ///
    /// **RT contract:** non-RT only — filesystem I/O (best effort).
    pub fn abort(mut self) {
        self.cleanup_temp();
    }

    fn cleanup_temp(&mut self) {
        drop(self.file.take());
        let _ = fs::remove_file(&self.temp_path);
    }

    fn temp_name(dest: &Path, attempt: usize) -> PathBuf {
        let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let name = dest.file_name().map_or_else(
            || "output".to_string(),
            |name| name.to_string_lossy().into_owned(),
        );
        dest.with_file_name(format!(
            "{name}{TEMP_TAG}{}-{counter}-{attempt}",
            std::process::id()
        ))
    }
}

impl Drop for StagedOutput {
    /// Best-effort temp cleanup — never a completion path.
    fn drop(&mut self) {
        self.cleanup_temp();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    fn scratch_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "staged-test-{}-{}",
            std::process::id(),
            TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn read(path: &Path) -> Vec<u8> {
        let mut bytes = Vec::new();
        File::open(path).unwrap().read_to_end(&mut bytes).unwrap();
        bytes
    }

    #[test]
    fn publish_writes_destination() {
        let dir = scratch_dir();
        let dest = dir.join("out.wav");
        let mut staged = StagedOutput::create(&dest).unwrap();
        staged.sink().unwrap().write_all(b"payload").unwrap();
        let temp = staged.temp_path().to_path_buf();
        staged.publish(PublishOptions::no_overwrite()).unwrap();
        assert_eq!(read(&dest), b"payload");
        assert!(!temp.exists());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn abort_preserves_existing_destination() {
        let dir = scratch_dir();
        let dest = dir.join("out.wav");
        fs::write(&dest, b"old bytes").unwrap();
        let mut staged = StagedOutput::create(&dest).unwrap();
        staged.sink().unwrap().write_all(b"new").unwrap();
        assert!(staged.dest_existed());
        staged.abort();
        assert_eq!(read(&dest), b"old bytes");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn drop_without_publish_preserves_destination() {
        let dir = scratch_dir();
        let dest = dir.join("out.wav");
        fs::write(&dest, b"old bytes").unwrap();
        {
            let mut staged = StagedOutput::create(&dest).unwrap();
            staged.sink().unwrap().write_all(b"partial").unwrap();
        }
        assert_eq!(read(&dest), b"old bytes");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn no_overwrite_fails_and_preserves_destination() {
        let dir = scratch_dir();
        let dest = dir.join("out.wav");
        fs::write(&dest, b"old bytes").unwrap();
        let mut staged = StagedOutput::create(&dest).unwrap();
        staged.sink().unwrap().write_all(b"new").unwrap();
        let error = staged.publish(PublishOptions::no_overwrite()).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::InvalidValue);
        assert_eq!(read(&dest), b"old bytes");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn replace_overwrites_destination() {
        let dir = scratch_dir();
        let dest = dir.join("out.wav");
        fs::write(&dest, b"old bytes").unwrap();
        let mut staged = StagedOutput::create(&dest).unwrap();
        staged.sink().unwrap().write_all(b"new").unwrap();
        staged.publish(PublishOptions::replace()).unwrap();
        assert_eq!(read(&dest), b"new");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn temp_is_sibling_of_destination() {
        let dir = scratch_dir();
        let dest = dir.join("out.wav");
        let staged = StagedOutput::create(&dest).unwrap();
        assert_eq!(staged.temp_path().parent(), Some(dir.as_path()));
        assert_ne!(staged.temp_path(), dest.as_path());
        staged.abort();
        fs::remove_dir_all(&dir).unwrap();
    }
}
