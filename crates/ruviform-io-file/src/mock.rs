//! Fault-injection `PlatformFileOps` — compiled under `cfg(test)` for
//! the crate's own unit tests and under the opt-in `fault-injection`
//! feature for cross-crate contract tests (Issue #306 G07).
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
//!
//! The crate-private types (`MockFileOps`, `MockScript`, `MockPublish`)
//! are re-used verbatim; the `pub` surface at the bottom of this file
//! is a std-type-only facade ([`FaultScript`], [`PublishScript`],
//! [`staged_file`], [`MockFs`]) so external test crates never see the
//! `PlatformFileOps` boundary itself.

use crate::options::{PublishMode, PublishOptions};
use crate::outcome::{OperationId, TempState};
use crate::platform::{
    ApplyOutcome, CleanupAttempt, PlatformFileOps, PublishAttempt, ReadSeek, TempDisposition,
};
use crate::receipt::RecoveryInfo;
use crate::staged::StagedFile;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

/// The mock's observable filesystem: temp contents, destination
/// contents, and the stage call log.
///
/// `pub` only so the `fault-injection` facade can hand it to external
/// contract tests as observation evidence; the fields stay plain std
/// types (`Vec<u8>` buffers and a stage-name log).
#[derive(Debug, Default)]
pub struct MockFs {
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
            // `clone_from` cannot borrow `fs.temp` while `fs.dest` is
            // mutably borrowed through the guard — clone to a local.
            let published = fs.temp.clone();
            fs.dest = published;
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

// ---------------------------------------------------------------------------
// `fault-injection` facade — the std-type-only scripting surface external
// contract tests (e.g. `ruviform-io-e2e`) consume. Everything below maps
// onto the crate-private `MockScript`/`MockFileOps` above; no
// `PlatformFileOps` type appears in a `pub` signature, so the private
// boundary stays private.
// ---------------------------------------------------------------------------

/// A scripted `io::Error` for one mock stage.
fn mock_error(kind: io::ErrorKind, stage: &'static str) -> io::Error {
    io::Error::new(kind, format!("mock {stage} fault"))
}

/// What a scripted publish attempt reports to the pipeline — the
/// facade-side name for the adapter's `ApplyOutcome` classification.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublishReport {
    /// The publish applied (the temp is consumed per `temp`).
    Applied,
    /// The publish definitively did not apply.
    NotApplied,
    /// The publish outcome cannot be determined — the pipeline
    /// normalizes this to `PublishEffect::Indeterminate` and does not
    /// auto-delete the leftover.
    Unknown,
}

impl PublishReport {
    const fn outcome(self) -> ApplyOutcome {
        match self {
            Self::Applied => ApplyOutcome::Applied,
            Self::NotApplied => ApplyOutcome::NotApplied,
            Self::Unknown => ApplyOutcome::Unknown,
        }
    }
}

/// What a scripted stage reports about the staged temp — the
/// facade-side name for `TempDisposition`.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TempReport {
    /// The temp was consumed by the publish or removed.
    Removed,
    /// The temp is still present.
    Retained,
    /// The temp's presence could not be determined.
    Unknown,
}

impl TempReport {
    const fn disposition(self) -> TempDisposition {
        match self {
            Self::Removed => TempDisposition::Removed,
            Self::Retained => TempDisposition::Retained,
            Self::Unknown => TempDisposition::Unknown,
        }
    }
}

/// A scripted publish attempt: the reported outcome, the resulting
/// temp disposition, an optional OS-level diagnostic kind, and whether
/// the mock *physically* moved the temp bytes to the destination —
/// an axis independent of the report so `Unknown` + applied models
/// "applied but could not be told" honestly.
///
/// Build with [`PublishScript::new`] and the `with_*` methods. Unset
/// fields default to the happy path: `temp: Removed`, no error, and
/// `physically_applied` matching an `Applied` report.
#[derive(Debug, Clone)]
pub struct PublishScript {
    report: PublishReport,
    temp: TempReport,
    error: Option<io::ErrorKind>,
    physically_applied: bool,
}

impl PublishScript {
    /// A script reporting `report`; remaining fields take happy-path
    /// defaults.
    #[must_use]
    pub fn new(report: PublishReport) -> Self {
        Self {
            report,
            temp: TempReport::Removed,
            error: None,
            physically_applied: matches!(report, PublishReport::Applied),
        }
    }

    /// The temp disposition the attempt reports.
    #[must_use]
    pub fn with_temp(mut self, temp: TempReport) -> Self {
        self.temp = temp;
        self
    }

    /// The OS-level diagnostic the attempt reports — an `io::Error`
    /// with this kind. `AlreadyExists`/`Unsupported` carry semantic
    /// weight in the pipeline's `PublishFailureKind` classification.
    #[must_use]
    pub fn with_error(mut self, kind: io::ErrorKind) -> Self {
        self.error = Some(kind);
        self
    }

    /// Whether the mock physically applies the publish (moves the temp
    /// bytes to the destination) regardless of the reported outcome.
    #[must_use]
    pub fn with_physically_applied(mut self, applied: bool) -> Self {
        self.physically_applied = applied;
        self
    }

    fn into_mock(self) -> MockPublish {
        MockPublish {
            attempt: PublishAttempt {
                effect: self.report.outcome(),
                temp: self.temp.disposition(),
                error: self.error.map(|kind| mock_error(kind, "publish")),
            },
            physically_applied: self.physically_applied,
        }
    }
}

/// Per-stage fault script for [`staged_file`]. Every slot is optional:
/// an unset stage runs the happy path, and a consumed slot falls back
/// to it (the same once-only semantics as the internal `MockScript`).
///
/// Build with [`FaultScript::new`] and the stage methods — each names
/// one pipeline stage and the fault it injects.
#[derive(Debug, Default)]
pub struct FaultScript {
    validate: Option<io::ErrorKind>,
    sync_file: Option<io::ErrorKind>,
    prepare: Option<io::ErrorKind>,
    sync_directory: Option<io::ErrorKind>,
    readback: Option<io::ErrorKind>,
    publish: Option<PublishScript>,
    cleanup: Option<(TempReport, Option<io::ErrorKind>)>,
    inspect: Option<Result<TempReport, io::ErrorKind>>,
}

impl FaultScript {
    /// An empty script — every stage on the happy path.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// `validate_publish` fails with this error kind.
    #[must_use]
    pub fn validate_error(mut self, kind: io::ErrorKind) -> Self {
        self.validate = Some(kind);
        self
    }

    /// `sync_file` fails with this error kind.
    #[must_use]
    pub fn sync_file_error(mut self, kind: io::ErrorKind) -> Self {
        self.sync_file = Some(kind);
        self
    }

    /// `prepare_for_publish` (the close stage) fails with this error
    /// kind.
    #[must_use]
    pub fn prepare_error(mut self, kind: io::ErrorKind) -> Self {
        self.prepare = Some(kind);
        self
    }

    /// `sync_directory` fails with this error kind.
    #[must_use]
    pub fn sync_directory_error(mut self, kind: io::ErrorKind) -> Self {
        self.sync_directory = Some(kind);
        self
    }

    /// `open_readback` fails with this error kind.
    #[must_use]
    pub fn readback_error(mut self, kind: io::ErrorKind) -> Self {
        self.readback = Some(kind);
        self
    }

    /// The publish attempt reports `script`.
    #[must_use]
    pub fn publish(mut self, script: PublishScript) -> Self {
        self.publish = Some(script);
        self
    }

    /// `cleanup_temp` reports `temp` (plus an optional error kind).
    #[must_use]
    pub fn cleanup(mut self, temp: TempReport, error: Option<io::ErrorKind>) -> Self {
        self.cleanup = Some((temp, error));
        self
    }

    /// `inspect_temp` reports `result` once — `Ok` a temp disposition,
    /// `Err` an `io::Error` with the given kind.
    #[must_use]
    pub fn inspect(mut self, result: Result<TempReport, io::ErrorKind>) -> Self {
        self.inspect = Some(result);
        self
    }

    fn into_mock_script(self) -> MockScript {
        MockScript {
            validate: self.validate.map(|kind| Err(mock_error(kind, "validate"))),
            sync_file: self
                .sync_file
                .map(|kind| Err(mock_error(kind, "sync_file"))),
            prepare: self.prepare.map(|kind| Err(mock_error(kind, "prepare"))),
            publish: self.publish.map(PublishScript::into_mock),
            sync_directory: self
                .sync_directory
                .map(|kind| Err(mock_error(kind, "sync_directory"))),
            cleanup: self.cleanup.map(|(temp, error)| CleanupAttempt {
                temp: temp.disposition(),
                error: error.map(|kind| mock_error(kind, "cleanup")),
            }),
            inspect: self.inspect.map(|result| match result {
                Ok(temp) => Ok(temp.disposition()),
                Err(kind) => Err(mock_error(kind, "inspect")),
            }),
            readback: self.readback.map(|kind| Err(mock_error(kind, "readback"))),
        }
    }
}

/// Builds a [`StagedFile`] driven by [`MockFileOps`] under `script`,
/// with `initial_temp`/`existing_dest` as the mock filesystem's
/// starting contents. Returns the staged file plus the shared
/// [`MockFs`] observation handle — temp/destination bytes and the
/// stage call log stay inspectable after the pipeline moves the
/// adapter into a `RecoveryHandle`.
///
/// `destination` is diagnostic only (it lands in `RecoveryInfo`); the
/// mock performs no real filesystem I/O.
#[must_use]
pub fn staged_file(
    destination: &Path,
    script: FaultScript,
    initial_temp: Option<Vec<u8>>,
    existing_dest: Option<Vec<u8>>,
) -> (StagedFile, Arc<Mutex<MockFs>>) {
    let fs = Arc::new(Mutex::new(MockFs {
        temp: initial_temp,
        dest: existing_dest,
        calls: Vec::new(),
    }));
    let ops = MockFileOps::new(fs.clone(), script.into_mock_script());
    let temp_path = ops.temp_path().map(Path::to_path_buf);
    let info = RecoveryInfo::new(
        OperationId::next(),
        destination.to_path_buf(),
        temp_path,
        TempState::Retained,
    );
    (StagedFile::with_ops(Box::new(ops), info), fs)
}
