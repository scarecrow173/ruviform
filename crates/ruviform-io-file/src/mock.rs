//! `#[cfg(test)]` fault-injection `PlatformFileOps`.
//!
//! `MockFileOps` simulates a one-file virtual filesystem (temp +
//! destination byte buffers plus a stage call log) behind `Arc<Mutex>`
//! so tests can inspect the physical outcome after the pipeline has
//! moved the adapter into a `RecoveryHandle`. The script sits in its
//! own `Mutex` because `validate_publish`/`inspect_temp` are `&self`
//! on the trait. Every pipeline stage is scriptable: pre-set outcomes
//! are consumed on use, and the defaults after exhaustion are the
//! happy path (validate/sync/prepare → Ok, publish → `Applied` +
//! `Removed`, dir sync → Ok, cleanup → `Removed`, inspect → actual
//! state, readback → Ok).

use crate::options::{PublishMode, PublishOptions};
use crate::platform::{
    ApplyOutcome, CleanupAttempt, PlatformFileOps, PublishAttempt, ReadSeek, TempDisposition,
};
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

/// The mock's observable filesystem: temp contents, destination
/// contents, and the stage call log.
#[derive(Debug, Default)]
pub(crate) struct MockFs {
    /// Temp file contents — `None` means the temp is absent.
    pub temp: Option<Vec<u8>>,
    /// Destination contents — `None` means nothing published/existing.
    pub dest: Option<Vec<u8>>,
    /// Pipeline stage calls, in order.
    pub calls: Vec<&'static str>,
}

/// A scripted publish attempt plus what *physically* happened in the
/// mock filesystem — the pair exists so `Unknown` reports can model
/// "applied but could not be told" honestly.
#[derive(Debug)]
pub(crate) struct MockPublish {
    /// What the adapter reports to the pipeline.
    pub attempt: PublishAttempt,
    /// Whether the mock FS moves temp contents to the destination.
    pub physically_applied: bool,
}

impl MockPublish {
    /// A clean applied publish (temp consumed).
    pub(crate) fn applied() -> Self {
        Self {
            attempt: PublishAttempt {
                effect: ApplyOutcome::Applied,
                temp: TempDisposition::Removed,
                error: None,
            },
            physically_applied: true,
        }
    }
}

/// Per-stage scripted outcomes. `None`/consumed slots fall back to
/// the happy path.
#[derive(Debug, Default)]
pub(crate) struct MockScript {
    pub validate: Option<io::Result<()>>,
    pub sync_file: Option<io::Result<()>>,
    pub prepare: Option<io::Result<()>>,
    pub publish: Option<MockPublish>,
    pub sync_directory: Option<io::Result<()>>,
    pub cleanup: Option<CleanupAttempt>,
    pub inspect: Option<io::Result<TempDisposition>>,
    /// `Some(Err(..))` makes `open_readback` fail (e.g. `Unsupported`).
    pub readback: Option<io::Result<()>>,
}

/// The fault-injection adapter.
pub(crate) struct MockFileOps {
    fs: Arc<Mutex<MockFs>>,
    /// Write cursor (the `PlatformFileOps` write/seek surface).
    wpos: u64,
    /// Read cursor (the `ReadSeek` readback surface).
    rpos: u64,
    /// Diagnostic temp path reported through `PlatformFileOps::temp_path`.
    temp_path: PathBuf,
    script: Mutex<MockScript>,
}

impl MockFileOps {
    pub(crate) fn new(fs: Arc<Mutex<MockFs>>, script: MockScript) -> Self {
        Self {
            fs,
            wpos: 0,
            rpos: 0,
            temp_path: PathBuf::from("dest.bin.rvf-tmp"),
            script: Mutex::new(script),
        }
    }

    fn fs(&self) -> MutexGuard<'_, MockFs> {
        self.fs.lock().expect("mock fs poisoned")
    }

    fn script(&self) -> MutexGuard<'_, MockScript> {
        self.script.lock().expect("mock script poisoned")
    }

    /// Shared cursor-step logic for the write (`wpos`) and read
    /// (`rpos`) surfaces.
    fn seek_cursor(cursor: &mut u64, len: u64, pos: SeekFrom) -> io::Result<u64> {
        let next = match pos {
            SeekFrom::Start(n) => i128::from(n),
            SeekFrom::End(n) => i128::from(len) + i128::from(n),
            SeekFrom::Current(n) => i128::from(*cursor) + i128::from(n),
        };
        *cursor = u64::try_from(next)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "mock: seek out of range"))?;
        Ok(*cursor)
    }
}

impl Read for MockFileOps {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let available = {
            let fs = self.fs();
            let Some(temp) = &fs.temp else {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "mock: temp file is absent",
                ));
            };
            let pos = usize::try_from(self.rpos).map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidInput, "mock: read position overflow")
            })?;
            let n = temp.len().saturating_sub(pos).min(buf.len());
            buf[..n].copy_from_slice(&temp[pos..pos + n]);
            n
        };
        self.rpos += available as u64;
        Ok(available)
    }
}

impl Seek for MockFileOps {
    /// The readback cursor (`rpos`).
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let len = self.fs().temp.as_ref().map_or(0, Vec::len) as u64;
        Self::seek_cursor(&mut self.rpos, len, pos)
    }
}

impl PlatformFileOps for MockFileOps {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        {
            let mut fs = self.fs();
            let temp = fs.temp.get_or_insert_with(Vec::new);
            let pos = usize::try_from(self.wpos).map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidInput, "mock: write position overflow")
            })?;
            if pos > temp.len() {
                temp.resize(pos, 0);
            }
            let overwrite = temp.len() - pos;
            if buf.len() <= overwrite {
                temp[pos..pos + buf.len()].copy_from_slice(buf);
            } else {
                temp[pos..].copy_from_slice(&buf[..overwrite]);
                temp.extend_from_slice(&buf[overwrite..]);
            }
        }
        self.wpos += buf.len() as u64;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.fs().calls.push("flush");
        Ok(())
    }

    /// The write cursor (`wpos`) — `PlatformFileOps` seeks move the
    /// write position; the readback surface seeks `rpos`.
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let len = self.fs().temp.as_ref().map_or(0, Vec::len) as u64;
        Self::seek_cursor(&mut self.wpos, len, pos)
    }

    fn open_readback(&mut self) -> io::Result<&mut dyn ReadSeek> {
        if let Some(result) = self.script().readback.take() {
            result?;
        }
        self.fs().calls.push("open_readback");
        self.rpos = 0;
        Ok(self)
    }

    fn validate_publish(&self, _options: PublishOptions) -> io::Result<()> {
        self.fs().calls.push("validate");
        self.script().validate.take().unwrap_or(Ok(()))
    }

    fn sync_file(&mut self) -> io::Result<()> {
        self.fs().calls.push("sync_file");
        self.script().sync_file.take().unwrap_or(Ok(()))
    }

    fn prepare_for_publish(&mut self) -> io::Result<()> {
        self.fs().calls.push("prepare");
        self.script().prepare.take().unwrap_or(Ok(()))
    }

    fn publish(&mut self, _mode: PublishMode) -> PublishAttempt {
        self.fs().calls.push("publish");
        let scripted = self
            .script()
            .publish
            .take()
            .unwrap_or_else(MockPublish::applied);
        let mut fs = self.fs();
        if scripted.physically_applied {
            fs.dest = fs.temp.clone();
        }
        if scripted.attempt.temp == TempDisposition::Removed {
            fs.temp = None;
        }
        scripted.attempt
    }

    fn sync_directory(&mut self) -> io::Result<()> {
        self.fs().calls.push("sync_directory");
        self.script().sync_directory.take().unwrap_or(Ok(()))
    }

    fn cleanup_temp(&mut self) -> CleanupAttempt {
        self.fs().calls.push("cleanup");
        let attempt = self.script().cleanup.take().unwrap_or(CleanupAttempt {
            temp: TempDisposition::Removed,
            error: None,
        });
        if attempt.temp == TempDisposition::Removed {
            self.fs().temp = None;
        }
        attempt
    }

    fn inspect_temp(&self) -> io::Result<TempDisposition> {
        self.fs().calls.push("inspect");
        if let Some(result) = self.script().inspect.take() {
            return result;
        }
        let retained = self.fs().temp.is_some();
        Ok(if retained {
            TempDisposition::Retained
        } else {
            TempDisposition::Removed
        })
    }

    fn temp_path(&self) -> Option<&Path> {
        Some(&self.temp_path)
    }
}
