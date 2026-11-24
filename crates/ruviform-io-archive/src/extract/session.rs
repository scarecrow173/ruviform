//! Extraction lifecycle: [`ExtractOptions`] → [`ExtractionSession`] →
//! [`ExtractedTree`] (+ [`ExtractionRecoveryHandle`]) — IO-IMP-10.
//!
//! State machine:
//!
//! ```text
//! ExtractionSession::create(parent, options)
//!        │  (exclusive isolated staging root created)
//!        ▼
//! session.extract_zip(archive) / session.extract_tar(archive)
//!        │  plan-validate → stage → archive-end verify
//!        ├──► Ok(ExtractedTree)      — all checks passed; caller owns tree
//!        └──► Err(ExtractionFailure) — cleanup attempted; .handle() =
//!                                     Some when the staged tree could
//!                                     not be fully removed
//!
//! ExtractedTree::cleanup() ──► Ok(CleanupReceipt) | Err(RecoveryCleanupError + handle)
//! ExtractedTree::into_path() ──► PathBuf (caller assumes ownership)
//! Drop                        ──► best-effort verified cleanup
//! ```
//!
//! **RT contract:** non-RT only — filesystem I/O, allocation.

use std::borrow::Cow;
use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::hash::Hasher;
use std::io::{self, Read, Seek};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::entry::{ArchiveEntryInfo, EntryKind};
use crate::error::{ArchiveError, ArchiveErrorKind};
use crate::limits::ArchiveReadLimits;
use crate::tar::TarArchiveReader;
use crate::zip::ZipArchiveReader;

use super::fs::{
    NodeIdentity, NodeTypeTag, PlatformExtractOps, StagingDir, StagingFile, StagingRoot, VfsKind,
    VfsProbe, platform_extract_ops,
};
use super::path::SafeRelativePath;

/// Tunables for one extraction run. Defaults are the secure default —
/// every knob stays inside this type and never relaxes policy.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ExtractOptions {
    /// Budgets shared with the archive readers. `max_entries`,
    /// `max_metadata_bytes`, `max_path_bytes`,
    /// `max_entry_uncompressed_bytes`, and
    /// `max_total_uncompressed_bytes` bound the parse and the writes;
    /// `max_path_depth` bounds validated path depth.
    pub limits: ArchiveReadLimits,
    /// Payload bytes buffered between the archive reader and the
    /// staging file (copy granularity; clamped to [4 KiB, 1 MiB]).
    pub copy_buffer_bytes: usize,
}

impl ExtractOptions {
    /// Secure defaults: reader budget defaults + 64 KiB copy buffer.
    #[must_use]
    pub fn new() -> Self {
        Self {
            limits: ArchiveReadLimits::default(),
            copy_buffer_bytes: 64 * 1024,
        }
    }
}

impl Default for ExtractOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// What [`ExtractedTree::cleanup`]/`Drop` reported.
#[derive(Debug)]
#[non_exhaustive]
pub struct CleanupReceipt {
    /// Nodes whose removal was attempted (excluding the root probe).
    pub attempted: u32,
    /// Nodes successfully removed (or already absent).
    pub removed: u32,
    /// Nodes retained: identity mismatch, an identity match whose
    /// ownership could not be proven (foreign object at a recorded
    /// name — deliberately not deleted), or an irrecoverable FS error.
    pub retained: u32,
    /// Path the staging root occupied.
    pub root: PathBuf,
}

/// Cleanup could not fully remove the staged tree.
#[derive(Debug)]
#[non_exhaustive]
pub struct ExtractionCleanupFailure {
    /// Cleanup progress at the point of failure.
    pub receipt: CleanupReceipt,
    /// The first cleanup error (retained nodes without a hard error
    /// report `InvalidState`).
    pub error: ArchiveError,
}

impl std::fmt::Display for ExtractionCleanupFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "staging cleanup failed: {}", self.error)
    }
}

impl std::error::Error for ExtractionCleanupFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}

/// `cleanup`/`abort` failed: the [`ExtractionCleanupFailure`] plus a
/// live recovery handle for inspection or another attempt.
#[derive(Debug)]
#[non_exhaustive]
pub struct RecoveryCleanupError {
    /// What failed.
    pub failure: ExtractionCleanupFailure,
    /// Recovery handle for retry/diagnosis.
    pub handle: Box<ExtractionRecoveryHandle>,
}

impl std::fmt::Display for RecoveryCleanupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.failure)
    }
}

impl std::error::Error for RecoveryCleanupError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.failure.source()
    }
}

/// What a recovery `inspect` observes at the recorded staging path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RecoveryTreeState {
    /// Path exists, is a directory, and is proven to be the recorded
    /// staging root — a handle-matched identity, or a handle-less
    /// match whose tree holds only recorded names.
    Present,
    /// Path exists but identity differs (foreign swap), a
    /// handle-less match holds unrecorded names, or it is a reparse
    /// point — never touched by cleanup.
    Mismatch,
    /// Path no longer exists.
    Absent,
    /// State could not be determined (I/O error during probe).
    Unknown,
}

/// Snapshot returned by [`ExtractionRecoveryHandle::inspect`].
#[derive(Debug)]
#[non_exhaustive]
pub struct ExtractionRecoveryInfo {
    /// Recorded staging-root path.
    pub root: PathBuf,
    /// Observed state at that path.
    pub state: RecoveryTreeState,
    /// Number of nodes the session recorded (the cleanup worklist).
    pub recorded_nodes: u32,
    /// Cleanup error that produced this handle, if any.
    pub last_error: Option<String>,
}

/// Handle to a staging tree that survived a failed cleanup.
///
/// `inspect` probes the recorded *path* (a held handle would pin a
/// renamed inode and lie); `cleanup` re-opens the path no-follow,
/// re-verifies the recorded identity, and deletes only the objects
/// the session created — never a foreign tree at the same path.
/// Without a generation-bearing handle a reopened root's equality is
/// not proof by itself; both paths then demand the structural check
/// before trusting it.
#[derive(Debug)]
pub struct ExtractionRecoveryHandle {
    root_path: PathBuf,
    root_id: NodeIdentity,
    records: Vec<NodeRecord>,
    last_error: Option<String>,
    /// Backend the session ran under — recovery probes/retries go
    /// through the same guarantees.
    ops: Arc<dyn PlatformExtractOps>,
}

impl ExtractionRecoveryHandle {
    /// The recorded staging-root path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.root_path
    }

    /// Probes the recorded path and reports what is there now.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn inspect(&self) -> ExtractionRecoveryInfo {
        let state = match self.ops.open_staging_root(&self.root_path) {
            Ok(root) => match root.probe() {
                Ok(p) if p.identity == self.root_id => {
                    if reopened_root_proven(&*root, &p, &self.records) {
                        RecoveryTreeState::Present
                    } else {
                        RecoveryTreeState::Mismatch
                    }
                }
                Ok(_) => RecoveryTreeState::Mismatch,
                Err(_) => RecoveryTreeState::Unknown,
            },
            Err(e) if e.kind() == io::ErrorKind::NotFound => RecoveryTreeState::Absent,
            Err(_) => RecoveryTreeState::Unknown,
        };
        ExtractionRecoveryInfo {
            root: self.root_path.clone(),
            state,
            recorded_nodes: u32::try_from(self.records.len()).unwrap_or(u32::MAX),
            last_error: self.last_error.clone(),
        }
    }

    /// Re-runs verified cleanup of the recorded tree. On success the
    /// tree is gone.
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// [`RecoveryCleanupError`] carrying the failure plus an updated
    /// handle when the tree still cannot be fully removed —
    /// including the identity-mismatch case where cleanup refuses to
    /// delete a foreign tree.
    pub fn cleanup(self) -> Result<CleanupReceipt, RecoveryCleanupError> {
        let Self {
            root_path,
            root_id,
            records,
            last_error,
            ops,
        } = self;
        let make_handle = |last_error: Option<String>| ExtractionRecoveryHandle {
            root_path: root_path.clone(),
            root_id,
            records: records.clone(),
            last_error,
            ops: ops.clone(),
        };
        let root = match ops.open_staging_root(&root_path) {
            Ok(r) => r,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                // Root already gone — nothing left to clean.
                return Ok(empty_receipt(&root_path));
            }
            Err(e) => {
                return Err(RecoveryCleanupError {
                    failure: ExtractionCleanupFailure {
                        receipt: empty_receipt(&root_path),
                        error: map_backend_error(e, "re-opening staging root"),
                    },
                    handle: Box::new(make_handle(last_error)),
                });
            }
        };
        let root_ok = match root.probe() {
            Ok(p) => p.identity == root_id && reopened_root_proven(&*root, &p, &records),
            Err(e) => {
                return Err(RecoveryCleanupError {
                    failure: ExtractionCleanupFailure {
                        receipt: empty_receipt(&root_path),
                        error: map_backend_error(e, "reading staging root identity"),
                    },
                    handle: Box::new(make_handle(last_error)),
                });
            }
        };
        if !root_ok {
            return Err(RecoveryCleanupError {
                failure: ExtractionCleanupFailure {
                    receipt: CleanupReceipt {
                        attempted: 0,
                        removed: 0,
                        retained: u32::try_from(records.len()).unwrap_or(u32::MAX),
                        root: root_path.clone(),
                    },
                    error: ArchiveError::new(
                        ArchiveErrorKind::InvalidState,
                        "staging root identity mismatch or unprovable weak identity; refusing to delete a foreign tree",
                    ),
                },
                handle: Box::new(make_handle(last_error)),
            });
        }
        run_cleanup(root, &records, &root_path).map_err(|failure| {
            let last = failure.error.to_string();
            RecoveryCleanupError {
                failure,
                handle: Box::new(make_handle(Some(last))),
            }
        })
    }
}

/// An extraction run failed. Cleanup of the partial tree was
/// attempted; `handle` is `Some` whenever the staging root could not
/// be fully removed.
#[derive(Debug)]
#[non_exhaustive]
pub struct ExtractionFailure {
    /// Why extraction failed.
    pub error: ArchiveError,
    /// Staging-root path (diagnostic; contents may be partially
    /// cleaned).
    pub staging_root: PathBuf,
    /// Recovery handle when the staged tree could not be fully
    /// removed.
    pub handle: Option<Box<ExtractionRecoveryHandle>>,
}

impl ExtractionFailure {
    /// The underlying [`ArchiveError`].
    #[must_use]
    pub fn error(&self) -> &ArchiveError {
        &self.error
    }

    /// Recovery handle when the staged tree survived cleanup.
    #[must_use]
    pub fn handle(&self) -> Option<&ExtractionRecoveryHandle> {
        self.handle.as_deref()
    }

    /// Consumes the failure and returns the recovery handle, if any.
    #[must_use]
    pub fn into_handle(self) -> Option<ExtractionRecoveryHandle> {
        self.handle.map(|b| *b)
    }
}

impl std::fmt::Display for ExtractionFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "extraction failed: {}", self.error)
    }
}

impl std::error::Error for ExtractionFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}

/// What a successful extraction reports. Counts are measured, never
/// declared.
#[derive(Debug)]
#[non_exhaustive]
pub struct ExtractionReceipt {
    /// Entries surfaced by the archive and extracted.
    pub entries: u32,
    /// Regular files created.
    pub files: u32,
    /// Directories created (explicit directory entries only).
    pub directories: u32,
    /// Total payload bytes written to staging.
    pub payload_bytes: u64,
    /// Entries the archive reported but the run did not extract —
    /// always 0 on a successful extraction (the run stops at the
    /// first unverified entry).
    pub unverified_entries: u32,
}

/// A recorded staging node: the cleanup worklist's unit.
#[derive(Debug, Clone)]
struct NodeRecord {
    /// Real (non-canonical) components relative to the staging root.
    components: Vec<String>,
    kind: NodeKind,
    /// Identity captured at creation (post-write for files).
    identity: NodeIdentity,
    /// Content proof for files (`None` for dirs): byte length and
    /// content hash of what the session wrote.
    proof: Option<ContentProof>,
}

/// What the session wrote into a staged file. On a weak-identity
/// filesystem equality of `(byte_len, hash)` with the object now at
/// the recorded name is the ownership proof identity cannot give.
#[derive(Debug, Clone, Copy)]
struct ContentProof {
    byte_len: u64,
    hash: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NodeKind {
    File,
    Dir,
}

/// An in-progress extraction: exclusive isolated staging root plus
/// the run's [`ExtractOptions`].
///
/// Created by [`ExtractionSession::create`]; consumed by
/// [`extract_zip`](Self::extract_zip)/[`extract_tar`](Self::extract_tar)
/// or [`abort`](Self::abort). Dropping a live session performs a
/// best-effort cleanup of the (typically empty) staging root.
///
/// **RT contract:** non-RT only.
#[derive(Debug)]
pub struct ExtractionSession {
    root: Option<Box<dyn StagingRoot>>,
    options: ExtractOptions,
    records: Vec<NodeRecord>,
    /// Canonical dir-prefix → real components, for dirs already
    /// created (dedupe across entries and case variants).
    created_dirs: BTreeMap<Vec<String>, Vec<String>>,
    ops: Arc<dyn PlatformExtractOps>,
}

impl ExtractionSession {
    /// Creates an exclusive isolated staging root inside `parent`.
    ///
    /// `parent` is the caller's trust boundary: it must be a directory
    /// the caller controls (a process-private temp dir is ideal). The
    /// staging root gets a collision-resistant name and is created
    /// exclusively — the session never opens a pre-existing directory.
    ///
    /// **RT contract:** non-RT only — filesystem I/O.
    ///
    /// # Errors
    ///
    /// `Io` on filesystem failure; `Unsupported` on targets without a
    /// no-follow/handle-relative backend.
    pub fn create(parent: impl AsRef<Path>, options: ExtractOptions) -> Result<Self, ArchiveError> {
        Self::create_with_ops(parent, options, platform_extract_ops())
    }

    fn create_with_ops(
        parent: impl AsRef<Path>,
        options: ExtractOptions,
        ops: Arc<dyn PlatformExtractOps>,
    ) -> Result<Self, ArchiveError> {
        let root = ops
            .create_staging_root(parent.as_ref())
            .map_err(|e| map_backend_error(e, "creating isolated staging root"))?;
        Ok(Self {
            root: Some(root),
            options,
            records: Vec::new(),
            created_dirs: BTreeMap::new(),
            ops,
        })
    }

    /// The staging-root path (diagnostic — the tree is not yet
    /// verified).
    #[must_use]
    pub fn staging_root(&self) -> &Path {
        self.root
            .as_deref()
            .map_or_else(|| Path::new(""), StagingDir::path)
    }

    /// Extracts a fully indexed ZIP archive into staging.
    ///
    /// Plan before write: every stored name is validated to
    /// [`SafeRelativePath`], kinds are confined to File/Directory,
    /// canonical (NFC + case-folded) collisions are detected, and
    /// declared-size budgets are enforced **before** any filesystem
    /// write. On failure the partial tree is cleaned and the error
    /// surfaces as [`ExtractionFailure`].
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// [`ExtractionFailure`] carrying the [`ArchiveError`]:
    /// `InvalidInput` (bad name/kind/collision), `LimitExceeded`
    /// (budget), `Io` (archive read or staging write),
    /// `Malformed`/`ChecksumMismatch` (archive corruption),
    /// `Unsupported` (platform cannot provide required guarantees).
    pub fn extract_zip<R: Read + Seek>(
        mut self,
        archive: &mut ZipArchiveReader<R>,
    ) -> Result<ExtractedTree, ExtractionFailure> {
        match self.run_zip(archive) {
            Ok(tree) => Ok(tree),
            Err(error) => Err(self.fail(error)),
        }
    }

    /// Extracts a streaming TAR archive into staging. Same policy as
    /// [`extract_zip`](Self::extract_zip); planning is incremental
    /// (each header validated as it surfaces). The reader is consumed
    /// and [`TarArchiveReader::finish`] verifies the archive end
    /// before the tree is handed out.
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// [`ExtractionFailure`] — same contract as `extract_zip`. On
    /// failure the inner reader is dropped with the session.
    pub fn extract_tar<R: Read>(
        mut self,
        archive: TarArchiveReader<R>,
    ) -> Result<(R, ExtractedTree), ExtractionFailure> {
        match self.run_tar(archive) {
            Ok((inner, tree)) => Ok((inner, tree)),
            Err(error) => Err(self.fail(error)),
        }
    }

    /// Abandons the session and removes the staging tree.
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// [`RecoveryCleanupError`] when the tree cannot be fully removed.
    pub fn abort(self) -> Result<CleanupReceipt, RecoveryCleanupError> {
        self.cleanup_tree()
    }

    // --------------------------------------------------------------
    // Internals
    // --------------------------------------------------------------

    fn run_zip<R: Read + Seek>(
        &mut self,
        archive: &mut ZipArchiveReader<R>,
    ) -> Result<ExtractedTree, ArchiveError> {
        let count = archive.entry_count();
        if count > usize::try_from(self.options.limits.max_entries).unwrap_or(usize::MAX) {
            return Err(limit(
                "entry count",
                u64::try_from(count).unwrap_or(u64::MAX),
                u64::from(self.options.limits.max_entries),
            ));
        }
        // Plan phase: validate + collision-check every entry before a
        // single filesystem write. Cumulative *declared* bytes are
        // also bounded here so a zip bomb is refused up front.
        let mut plan: Vec<PlanEntry> = Vec::with_capacity(count);
        let mut canonical: BTreeMap<Vec<String>, (EntryKind, usize)> = BTreeMap::new();
        let mut declared_total = 0u64;
        for index in 0..count {
            let info = archive.entry_info(index).ok_or_else(|| {
                ArchiveError::new(
                    ArchiveErrorKind::InvalidState,
                    "entry index vanished between count and info lookup",
                )
            })?;
            let declared = info.uncompressed_size().unwrap_or(0);
            declared_total = declared_total.saturating_add(declared);
            if declared_total > self.options.limits.max_total_uncompressed_bytes {
                return Err(limit(
                    "total uncompressed bytes (declared)",
                    declared_total,
                    self.options.limits.max_total_uncompressed_bytes,
                ));
            }
            plan.push(self.plan_entry(&info, declared, index, &mut canonical)?);
        }
        // Write phase.
        let mut totals = Totals::default();
        for entry in &plan {
            match entry.kind {
                NodeKind::Dir => {
                    self.ensure_dirs(&entry.path, entry.path.depth())?;
                    let zr = archive
                        .open_entry(entry.index)
                        .map_err(|e| ctx(e, "opening ZIP directory entry"))?;
                    // `finish` drains any unexpected payload and
                    // verifies structure; directory entries never
                    // write bytes to the tree.
                    zr.finish()
                        .map_err(|e| ctx(e, "verifying ZIP directory entry"))?;
                    totals.dirs = totals.dirs.saturating_add(1);
                }
                NodeKind::File => {
                    if totals.bytes.saturating_add(entry.declared)
                        > self.options.limits.max_total_uncompressed_bytes
                    {
                        return Err(limit(
                            "total uncompressed bytes (declared)",
                            totals.bytes.saturating_add(entry.declared),
                            self.options.limits.max_total_uncompressed_bytes,
                        ));
                    }
                    let zr = archive
                        .open_entry(entry.index)
                        .map_err(|e| ctx(e, "opening ZIP entry"))?;
                    self.stage_file(&entry.path, zr, entry.declared, &mut totals)?;
                    totals.files = totals.files.saturating_add(1);
                }
            }
        }
        Ok(self.take_tree(ExtractionReceipt {
            entries: u32::try_from(plan.len()).unwrap_or(u32::MAX),
            files: totals.files,
            directories: totals.dirs,
            payload_bytes: totals.bytes,
            unverified_entries: 0,
        }))
    }

    fn run_tar<R: Read>(
        &mut self,
        mut archive: TarArchiveReader<R>,
    ) -> Result<(R, ExtractedTree), ArchiveError> {
        let mut canonical: BTreeMap<Vec<String>, (EntryKind, usize)> = BTreeMap::new();
        let mut totals = Totals::default();
        let mut index = 0usize;
        loop {
            let Some(entry) = archive
                .next_entry()
                .map_err(|e| ctx(e, "surfacing next TAR entry"))?
            else {
                break;
            };
            if index >= usize::try_from(self.options.limits.max_entries).unwrap_or(usize::MAX) {
                return Err(limit(
                    "entry count",
                    u64::try_from(index + 1).unwrap_or(u64::MAX),
                    u64::from(self.options.limits.max_entries),
                ));
            }
            let declared = entry.info().uncompressed_size().unwrap_or(0);
            let planned = self.plan_entry(entry.info(), declared, index, &mut canonical)?;
            match planned.kind {
                NodeKind::Dir => {
                    self.ensure_dirs(&planned.path, planned.path.depth())?;
                    if declared > 0 {
                        entry
                            .discard_remaining()
                            .map_err(|e| ctx(e, "draining directory-entry payload"))?;
                    } else {
                        entry
                            .finish()
                            .map_err(|e| ctx(e, "finishing directory entry"))?;
                    }
                    totals.dirs = totals.dirs.saturating_add(1);
                }
                NodeKind::File => {
                    if totals.bytes.saturating_add(declared)
                        > self.options.limits.max_total_uncompressed_bytes
                    {
                        return Err(limit(
                            "total uncompressed bytes (declared)",
                            totals.bytes.saturating_add(declared),
                            self.options.limits.max_total_uncompressed_bytes,
                        ));
                    }
                    self.stage_file(&planned.path, entry, declared, &mut totals)?;
                    totals.files = totals.files.saturating_add(1);
                }
            }
            index += 1;
        }
        let (inner, receipt) = archive
            .finish()
            .map_err(|e| ctx(e, "verifying TAR archive end"))?;
        let tree = self.take_tree(ExtractionReceipt {
            entries: u32::try_from(index).unwrap_or(u32::MAX),
            files: totals.files,
            directories: totals.dirs,
            payload_bytes: totals.bytes,
            unverified_entries: receipt.entries_unverified(),
        });
        Ok((inner, tree))
    }

    /// Validates one stored entry: name policy, kind confinement,
    /// depth/byte/declared-size budgets, canonical-collision
    /// detection. Records the canonical path → kind map; prefixes are
    /// remembered as implicit directories.
    fn plan_entry(
        &self,
        info: &ArchiveEntryInfo,
        declared: u64,
        index: usize,
        canonical: &mut BTreeMap<Vec<String>, (EntryKind, usize)>,
    ) -> Result<PlanEntry, ArchiveError> {
        let limits = &self.options.limits;
        let entry_kind = info.kind();
        let kind = match entry_kind {
            EntryKind::File => NodeKind::File,
            EntryKind::Directory => NodeKind::Dir,
            EntryKind::SymbolicLink | EntryKind::HardLink => {
                return Err(reject(format!(
                    "entry {index} (`{}`): links are never extracted under the secure default",
                    info.name().to_string_lossy()
                )));
            }
            _ => {
                return Err(reject(format!(
                    "entry {index} (`{}`): unsupported entry kind {entry_kind:?}; only regular files and directories are extracted",
                    info.name().to_string_lossy()
                )));
            }
        };
        let path =
            SafeRelativePath::new(info.name()).map_err(|e| ctx(e, format!("entry {index}")))?;
        if path.depth() > usize::try_from(limits.max_path_depth).unwrap_or(usize::MAX) {
            return Err(limit(
                "path depth",
                u64::try_from(path.depth()).unwrap_or(u64::MAX),
                u64::from(limits.max_path_depth),
            ));
        }
        if u64::try_from(path.byte_len()).unwrap_or(u64::MAX) > u64::from(limits.max_path_bytes) {
            return Err(limit(
                "path bytes",
                u64::try_from(path.byte_len()).unwrap_or(u64::MAX),
                u64::from(limits.max_path_bytes),
            ));
        }
        if declared > limits.max_entry_uncompressed_bytes {
            return Err(limit(
                "entry uncompressed bytes (declared)",
                declared,
                limits.max_entry_uncompressed_bytes,
            ));
        }
        // Canonical collision detection (NFC + case-folded key).
        let key = path.canonical_key();
        for depth in 1..key.len() {
            let prefix = &key[..depth];
            match canonical.get(prefix) {
                Some((EntryKind::File, other)) => {
                    return Err(reject(format!(
                        "entry {index} `{path}`: collides with file entry {other} — a path is used as both file and directory",
                    )));
                }
                Some(_) => {}
                None => {
                    canonical.insert(prefix.to_vec(), (EntryKind::Directory, usize::MAX));
                }
            }
        }
        match canonical.get(&key) {
            Some((prior_kind, other)) => {
                let both_dirs =
                    *prior_kind == EntryKind::Directory && entry_kind == EntryKind::Directory;
                if !both_dirs {
                    return Err(reject(format!(
                        "entry {index} `{path}`: canonical-path collision with entry {other} (kind {prior_kind:?} vs {entry_kind:?})"
                    )));
                }
            }
            None => {
                canonical.insert(key.clone(), (entry_kind, index));
            }
        }
        Ok(PlanEntry {
            path,
            kind,
            index,
            declared,
        })
    }

    /// Creates (or reopens) the leading `upto` components of `path`
    /// as directories; returns the handle at that depth, or `None`
    /// when `upto == 0` (the staging root itself).
    fn ensure_dirs(
        &mut self,
        path: &SafeRelativePath,
        upto: usize,
    ) -> Result<Option<Box<dyn StagingDir>>, ArchiveError> {
        let canonical = path.canonical_key();
        let real: Vec<String> = path.components().map(str::to_owned).collect();
        let root: &dyn StagingDir = self
            .root
            .as_deref()
            .map(|r| r as &dyn StagingRoot)
            .ok_or_else(|| {
                ArchiveError::new(ArchiveErrorKind::InvalidState, "staging root already taken")
            })?;
        let mut current: Option<Box<dyn StagingDir>> = None;
        for level in 0..upto.min(real.len()) {
            let canonical_prefix = canonical[..=level].to_vec();
            // Descend with the *recorded real name* when the dir was
            // already created — canonical dedupe may have seen a
            // different case/NFD spelling of the same directory.
            let name = self
                .created_dirs
                .get(&canonical_prefix)
                .and_then(|r| r.last().cloned())
                .unwrap_or_else(|| real[level].clone());
            let parent: &dyn StagingDir = match &current {
                Some(d) => &**d,
                None => root,
            };
            let next = if self.created_dirs.contains_key(&canonical_prefix) {
                parent
                    .open_dir(&name)
                    .map_err(|e| map_backend_error(e, format!("re-opening staged dir `{name}`")))?
            } else {
                let dir = parent
                    .create_dir(&name)
                    .map_err(|e| map_backend_error(e, format!("creating staged dir `{name}`")))?;
                let identity = match dir.probe() {
                    Ok(p) => p.identity,
                    Err(e) => {
                        // Unrecordable dir — remove best-effort (we
                        // exclusively created the name), then fail.
                        let _ = parent.remove_dir(&name);
                        return Err(map_backend_error(e, "reading staged dir identity"));
                    }
                };
                self.records.push(NodeRecord {
                    components: real[..=level].to_vec(),
                    kind: NodeKind::Dir,
                    identity,
                    proof: None,
                });
                self.created_dirs
                    .insert(canonical_prefix, real[..=level].to_vec());
                dir
            };
            current = Some(next);
        }
        Ok(current)
    }

    /// Stages one file entry: parent dirs → exclusive create →
    /// bounded copy → entry finish → record.
    fn stage_file<E: EntryFinish>(
        &mut self,
        path: &SafeRelativePath,
        mut entry: E,
        declared: u64,
        totals: &mut Totals,
    ) -> Result<(), ArchiveError> {
        let depth = path.depth();
        let parent = self.ensure_dirs(path, depth - 1)?;
        let leaf = path.component(depth - 1).ok_or_else(|| {
            ArchiveError::new(ArchiveErrorKind::InvalidState, "missing leaf component")
        })?;
        let root: &dyn StagingDir = self
            .root
            .as_deref()
            .map(|r| r as &dyn StagingRoot)
            .ok_or_else(|| {
                ArchiveError::new(ArchiveErrorKind::InvalidState, "staging root already taken")
            })?;
        let dir: &dyn StagingDir = parent.as_deref().unwrap_or(root);
        let mut file = dir
            .create_file(leaf)
            .map_err(|e| map_backend_error(e, format!("creating staged file `{leaf}`")))?;
        let create_id = match file.probe() {
            Ok(p) => p.identity,
            Err(e) => {
                // The created file cannot be recorded without an
                // identity — remove it best-effort (we exclusively
                // created the name) so the tree stays recoverable.
                let _ = dir.remove_file(leaf);
                return Err(map_backend_error(e, "reading staged file identity"));
            }
        };
        // Record *before* the payload copy: any later failure must
        // still let verified cleanup remove this file. Components
        // must be the *on-disk* names — canonical dedupe may have
        // spelled a parent dir differently than this entry does.
        let canonical = path.canonical_key();
        let mut components: Vec<String> = if depth > 1 {
            self.created_dirs
                .get(&canonical[..depth - 1])
                .cloned()
                .unwrap_or_else(|| {
                    path.components()
                        .take(depth - 1)
                        .map(str::to_owned)
                        .collect()
                })
        } else {
            Vec::new()
        };
        components.push(leaf.to_owned());
        self.records.push(NodeRecord {
            components,
            kind: NodeKind::File,
            identity: create_id,
            proof: None,
        });
        let mut proof = ContentProof {
            byte_len: 0,
            hash: empty_hash(),
        };
        let copy = self.copy_payload(&mut entry, &mut *file, declared, &mut proof);
        // Refresh the recorded probe post-write: backends whose
        // discriminator tracks change time (rather than immutable
        // birth time) must see the value the final write produced,
        // or cleanup would retain our own files as "foreign". The
        // content proof is recorded for whatever bytes completed —
        // on a weak filesystem it is the only way cleanup can prove
        // a partially staged file is ours.
        match file.probe() {
            Ok(p) => {
                if let Some(rec) = self.records.last_mut() {
                    rec.identity = p.identity;
                    rec.proof = Some(proof);
                }
            }
            Err(e) => {
                let _ = copy;
                return Err(map_backend_error(
                    e,
                    format!("re-reading staged file `{leaf}` identity"),
                ));
            }
        }
        let written = copy?;
        drop(file);
        entry
            .finish_entry()
            .map_err(|e| ctx(e, format!("verifying entry `{path}`")))?;
        totals.bytes = totals.bytes.saturating_add(written);
        Ok(())
    }

    /// Bounded payload copy. The reader-side caps (declared size,
    /// per-entry budget) are enforced by the entry reader itself;
    /// this loop additionally cuts off measured output beyond the
    /// per-entry budget so excess bytes never reach the file. The
    /// content proof is updated per completed `write_all`, so it
    /// always describes the exact prefix the file is known to hold —
    /// a partially staged file stays provable to cleanup.
    fn copy_payload<E: EntryFinish>(
        &self,
        entry: &mut E,
        file: &mut dyn StagingFile,
        declared: u64,
        proof: &mut ContentProof,
    ) -> Result<u64, ArchiveError> {
        let cap = declared.min(self.options.limits.max_entry_uncompressed_bytes);
        let mut buf = vec![0u8; self.options.copy_buffer_bytes.clamp(4 * 1024, 1 << 20)];
        let mut hasher = DefaultHasher::new();
        let mut written = 0u64;
        loop {
            let want = usize::try_from(cap - written)
                .unwrap_or(usize::MAX)
                .min(buf.len());
            if want == 0 {
                break;
            }
            let n = entry
                .read(&mut buf[..want])
                .map_err(unwrap_archive_error)
                .map_err(|e| ctx(e, "reading entry payload"))?;
            if n == 0 {
                break;
            }
            file.write_all(&buf[..n])
                .map_err(|e| map_backend_error(e, "writing staged file"))?;
            hasher.write(&buf[..n]);
            written = written.saturating_add(u64::try_from(n).unwrap_or(u64::MAX));
            proof.byte_len = written;
            proof.hash = hasher.clone().finish();
        }
        Ok(written)
    }

    /// Verified cleanup of the staged tree.
    fn cleanup_tree(mut self) -> Result<CleanupReceipt, RecoveryCleanupError> {
        let records = std::mem::take(&mut self.records);
        let Some(root) = self.root.take() else {
            return Ok(CleanupReceipt {
                attempted: 0,
                removed: 0,
                retained: 0,
                root: PathBuf::new(),
            });
        };
        let root_path = root.path().to_path_buf();
        let root_id = root_identity(&*root);
        let ops = self.ops.clone();
        run_cleanup(root, &records, &root_path).map_err(|failure| {
            let last = failure.error.to_string();
            RecoveryCleanupError {
                failure,
                handle: Box::new(ExtractionRecoveryHandle {
                    root_path: root_path.clone(),
                    root_id,
                    records,
                    last_error: Some(last),
                    ops,
                }),
            }
        })
    }

    /// Post-failure path: attempt cleanup of the partial tree, then
    /// attach a recovery handle whenever anything is retained.
    fn fail(mut self, error: ArchiveError) -> ExtractionFailure {
        let records = std::mem::take(&mut self.records);
        let Some(root) = self.root.take() else {
            return ExtractionFailure {
                error,
                staging_root: PathBuf::new(),
                handle: None,
            };
        };
        let root_path = root.path().to_path_buf();
        let root_id = root_identity(&*root);
        match run_cleanup(root, &records, &root_path) {
            Ok(receipt) if receipt.retained == 0 => ExtractionFailure {
                error,
                staging_root: root_path,
                handle: None,
            },
            outcome => {
                let last_error = match &outcome {
                    Ok(_) => None,
                    Err(f) => Some(f.error.to_string()),
                };
                ExtractionFailure {
                    error,
                    staging_root: root_path.clone(),
                    handle: Some(Box::new(ExtractionRecoveryHandle {
                        root_path,
                        root_id,
                        records,
                        last_error,
                        ops: self.ops.clone(),
                    })),
                }
            }
        }
    }

    fn take_tree(&mut self, receipt: ExtractionReceipt) -> ExtractedTree {
        ExtractedTree {
            root: self.root.take(),
            receipt,
            records: std::mem::take(&mut self.records),
            cleanup_on_drop: true,
            ops: self.ops.clone(),
        }
    }
}

impl Drop for ExtractionSession {
    fn drop(&mut self) {
        // Best-effort only — a consumed session has no root.
        let Some(root) = self.root.take() else {
            return;
        };
        let records = std::mem::take(&mut self.records);
        let root_path = root.path().to_path_buf();
        let _ = run_cleanup(root, &records, &root_path);
    }
}

/// A successfully extracted tree: every planned entry was created
/// inside the isolated staging root with exclusive/no-follow
/// semantics, and the archive structure verified end-to-end.
///
/// **Not an immutable snapshot** — the staged contents are ordinary
/// files that can be modified after handoff. Consumer-level
/// publication (commit/CAS/directory replace) is out of scope for
/// this layer.
///
/// **RT contract:** non-RT only.
#[derive(Debug)]
pub struct ExtractedTree {
    root: Option<Box<dyn StagingRoot>>,
    receipt: ExtractionReceipt,
    records: Vec<NodeRecord>,
    cleanup_on_drop: bool,
    ops: Arc<dyn PlatformExtractOps>,
}

impl ExtractedTree {
    /// The staging-root path holding the verified tree.
    #[must_use]
    pub fn path(&self) -> &Path {
        self.root
            .as_deref()
            .map_or_else(|| Path::new(""), StagingDir::path)
    }

    /// Measured extraction counts.
    #[must_use]
    pub const fn receipt(&self) -> &ExtractionReceipt {
        &self.receipt
    }

    /// Hands the staging path to the caller without cleanup.
    ///
    /// Ownership transfer: after this call the tree is the caller's
    /// responsibility — `ExtractedTree` no longer deletes anything.
    #[must_use]
    pub fn into_path(mut self) -> PathBuf {
        self.cleanup_on_drop = false;
        let path = self
            .root
            .as_deref()
            .map_or_else(PathBuf::new, |r| r.path().to_path_buf());
        let _ = self.root.take(); // close the handle; tree stays on disk
        path
    }

    /// Verified cleanup: removes the recorded objects deepest-first
    /// with per-node identity checks, then the root.
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// [`RecoveryCleanupError`] carrying the failure plus a recovery
    /// handle when the tree cannot be fully removed.
    pub fn cleanup(mut self) -> Result<CleanupReceipt, RecoveryCleanupError> {
        self.cleanup_on_drop = false;
        let records = std::mem::take(&mut self.records);
        let Some(root) = self.root.take() else {
            return Ok(empty_receipt(Path::new("")));
        };
        let root_path = root.path().to_path_buf();
        let root_id = root_identity(&*root);
        let ops = self.ops.clone();
        run_cleanup(root, &records, &root_path).map_err(|failure| {
            let last = failure.error.to_string();
            RecoveryCleanupError {
                failure,
                handle: Box::new(ExtractionRecoveryHandle {
                    root_path: root_path.clone(),
                    root_id,
                    records,
                    last_error: Some(last),
                    ops,
                }),
            }
        })
    }
}

impl Drop for ExtractedTree {
    fn drop(&mut self) {
        if !self.cleanup_on_drop {
            return;
        }
        let Some(root) = self.root.take() else {
            return;
        };
        let records = std::mem::take(&mut self.records);
        let root_path = root.path().to_path_buf();
        let _ = run_cleanup(root, &records, &root_path);
    }
}

// ------------------------------------------------------------------
// Internal plumbing
// ------------------------------------------------------------------

const EMPTY_ID: NodeIdentity = NodeIdentity {
    volume: 0,
    index: 0,
    tag: NodeTypeTag::Unknown,
    discriminator: 0,
    handle: None,
};

#[derive(Debug)]
struct PlanEntry {
    path: SafeRelativePath,
    kind: NodeKind,
    index: usize,
    declared: u64,
}

#[derive(Debug, Default)]
struct Totals {
    bytes: u64,
    files: u32,
    dirs: u32,
}

/// Unifies `ZipEntryReader`/`TarEntryReader` for the copy loop: a
/// `Read` plus a completion call that verifies the entry.
trait EntryFinish: Read {
    fn finish_entry(self) -> Result<(), ArchiveError>;
}

impl<R: Read + Seek> EntryFinish for crate::zip::ZipEntryReader<'_, R> {
    fn finish_entry(self) -> Result<(), ArchiveError> {
        self.finish().map(|_| ())
    }
}

impl<R: Read> EntryFinish for crate::tar::TarEntryReader<'_, R> {
    fn finish_entry(self) -> Result<(), ArchiveError> {
        self.finish().map(|_| ())
    }
}

/// Deepest-first verified cleanup: descend by handle from the root,
/// probe each recorded leaf, delete only the objects the session can
/// still *prove* it created. Foreign/missing objects are retained,
/// never deleted. The root is removed last after a final identity
/// check.
fn run_cleanup(
    root: Box<dyn StagingRoot>,
    records: &[NodeRecord],
    root_path: &Path,
) -> Result<CleanupReceipt, ExtractionCleanupFailure> {
    let mut receipt = CleanupReceipt {
        attempted: 0,
        removed: 0,
        retained: 0,
        root: root_path.to_path_buf(),
    };
    let mut first_error: Option<ArchiveError> = None;
    let root_id = root_identity(&*root);
    let mut order: Vec<&NodeRecord> = records.iter().collect();
    order.sort_by_key(|r| std::cmp::Reverse(r.components.len()));
    let by_path: BTreeMap<&[String], &NodeRecord> = records
        .iter()
        .map(|r| (r.components.as_slice(), r))
        .collect();
    // Children each recorded dir may legitimately hold — the weak
    // structural proof's whitelist.
    let mut children: BTreeMap<&[String], BTreeSet<OsString>> = BTreeMap::new();
    for record in records {
        if let Some((leaf, parent)) = record.components.split_last() {
            children
                .entry(parent)
                .or_default()
                .insert(OsString::from(leaf));
        }
    }
    for record in order {
        receipt.attempted = receipt.attempted.saturating_add(1);
        match cleanup_node(&*root, record, &by_path, &children) {
            Ok(true) => receipt.removed = receipt.removed.saturating_add(1),
            Ok(false) => receipt.retained = receipt.retained.saturating_add(1),
            Err(e) => {
                receipt.retained = receipt.retained.saturating_add(1);
                if first_error.is_none() {
                    first_error = Some(e);
                }
            }
        }
    }
    // Root last: re-verify identity, then remove. Held-open since
    // creation, the handle pins the inode — on a weak filesystem an
    // equal probe here can only be the same object (no foreign dir
    // could ever take the pinned inode), so equality stays sound.
    receipt.attempted = receipt.attempted.saturating_add(1);
    match root.probe() {
        Ok(p) if p.identity == root_id => match root.remove_root() {
            Ok(()) => receipt.removed = receipt.removed.saturating_add(1),
            Err(e) => {
                receipt.retained = receipt.retained.saturating_add(1);
                if first_error.is_none() {
                    first_error = Some(map_backend_error(e, "removing staging root"));
                }
            }
        },
        _ => {
            receipt.retained = receipt.retained.saturating_add(1);
        }
    }
    match first_error {
        Some(error) => Err(ExtractionCleanupFailure { receipt, error }),
        None if receipt.retained > 0 => Err(ExtractionCleanupFailure {
            receipt,
            error: ArchiveError::new(
                ArchiveErrorKind::InvalidState,
                "cleanup retained nodes (identity mismatch, unprovable identity, or missing objects)",
            ),
        }),
        None => Ok(receipt),
    }
}

/// Deletes one recorded node iff the object at its recorded path is
/// provably the object the session created. `Ok(false)` = retained
/// (foreign/absent parent/identity mismatch/unprovable identity);
/// `Ok(true)` = removed or already absent.
///
/// A matched generation-bearing file handle proves sameness on its
/// own — a recreated object cannot reproduce the recorded handle.
/// Everything weaker (birth/change-time discriminators, a reported
/// `Strong` strength) can still alias delete+recreate on
/// filesystems whose timestamps have coarse granularity, so the
/// kind-appropriate secondary proof ([`weak_proof`]) is required
/// before anything is deleted.
fn cleanup_node(
    root: &dyn StagingDir,
    record: &NodeRecord,
    by_path: &BTreeMap<&[String], &NodeRecord>,
    children: &BTreeMap<&[String], BTreeSet<OsString>>,
) -> Result<bool, ArchiveError> {
    let parents = record.components.len().saturating_sub(1);
    let mut owned: Vec<Box<dyn StagingDir>> = Vec::new();
    for (level, name) in record.components[..parents].iter().enumerate() {
        let parent: &dyn StagingDir = owned.last().map_or(root, |d| &**d);
        match parent.open_dir(name) {
            Ok(next) => {
                // Never descend into a foreign directory: verify each
                // ancestor's identity against its record.
                let expected = by_path
                    .get(&record.components[..=level])
                    .map(|r| r.identity);
                let id = next.probe().map_or(EMPTY_ID, |p| p.identity);
                if expected != Some(id) {
                    return Ok(false);
                }
                owned.push(next);
            }
            // Absent ancestor ⇒ the recorded node cannot exist.
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(true),
            Err(_) => return Ok(false),
        }
    }
    let dir: &dyn StagingDir = owned.last().map_or(root, |d| &**d);
    let leaf = &record.components[parents];
    let observed = match dir.probe_child(leaf) {
        Ok(p) => p,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(true),
        Err(_) => return Ok(false),
    };
    if observed.identity != record.identity {
        return Ok(false);
    }
    if observed.identity.handle.is_none() {
        // No generation-bearing handle in the match: the remaining
        // identity fields can alias a foreign object recreated at the
        // same name, whatever strength the probe reports.
        match weak_proof(dir, leaf, record, children) {
            WeakProof::Ours => {}
            WeakProof::Foreign => return Ok(false),
            WeakProof::Gone => return Ok(true),
        }
    }
    let removed = match record.kind {
        NodeKind::File => dir.remove_file(leaf),
        NodeKind::Dir => dir.remove_dir(leaf),
    };
    match removed {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(true),
        Err(e) => Err(map_backend_error(e, format!("removing staged `{leaf}`"))),
    }
}

/// Result of the weak-identity secondary ownership proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WeakProof {
    /// Proven to be the object the session created.
    Ours,
    /// Not provable (mismatch, absent record data, or I/O error) —
    /// retained, never deleted.
    Foreign,
    /// Vanished between the leaf probe and the proof — nothing to
    /// remove.
    Gone,
}

/// Secondary ownership proof for a handle-less identity match.
/// Identity equality has already been observed; this adds the
/// channel a weak or coarse-discriminator FS cannot alias:
///
/// - **file**: a verified no-follow `open_leaf` re-probe plus
///   `(byte_len, content hash)` equality against what the session
///   wrote. The open's own probe must still match the record — the
///   object we read is the object we judge.
/// - **dir**: every direct child name must be one the session
///   recorded (deepest-first order already removed our children, so
///   a session dir reads empty). A foreign object planted inside
///   makes the tree retained rather than deleted.
fn weak_proof(
    dir: &dyn StagingDir,
    leaf: &str,
    record: &NodeRecord,
    children: &BTreeMap<&[String], BTreeSet<OsString>>,
) -> WeakProof {
    match record.kind {
        NodeKind::File => {
            let Some(proof) = record.proof else {
                return WeakProof::Foreign;
            };
            let (mut reader, probe) = match dir.open_leaf(leaf) {
                Ok(v) => v,
                Err(e) if e.kind() == io::ErrorKind::NotFound => return WeakProof::Gone,
                Err(_) => return WeakProof::Foreign,
            };
            if probe.kind != VfsKind::File
                || probe.identity != record.identity
                || probe.byte_len != Some(proof.byte_len)
            {
                return WeakProof::Foreign;
            }
            match hash_reader(&mut *reader) {
                Ok((len, hash)) if len == proof.byte_len && hash == proof.hash => WeakProof::Ours,
                _ => WeakProof::Foreign,
            }
        }
        NodeKind::Dir => {
            let child = match dir.open_dir(leaf) {
                Ok(d) => d,
                Err(e) if e.kind() == io::ErrorKind::NotFound => return WeakProof::Gone,
                Err(_) => return WeakProof::Foreign,
            };
            let Ok(names) = child.list_names() else {
                return WeakProof::Foreign;
            };
            let allowed = children.get(record.components.as_slice());
            if names
                .iter()
                .all(|n| allowed.is_some_and(|s| s.contains(n.as_os_str())))
            {
                WeakProof::Ours
            } else {
                WeakProof::Foreign
            }
        }
    }
}

/// Hashes a reader to end: `(bytes read, content hash)` — the same
/// accumulator `copy_payload` uses, so session writes and cleanup
/// reads compare equal.
fn hash_reader(reader: &mut dyn Read) -> io::Result<(u64, u64)> {
    let mut hasher = DefaultHasher::new();
    let mut buf = [0u8; 8192];
    let mut total = 0u64;
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.write(&buf[..n]);
        total = total.saturating_add(u64::try_from(n).unwrap_or(u64::MAX));
    }
    Ok((total, hasher.finish()))
}

/// Hash of an empty stream — a staged file whose payload copy wrote
/// nothing hashes to this.
fn empty_hash() -> u64 {
    DefaultHasher::new().finish()
}

/// Identity of an open root handle, with the conservative `EMPTY_ID`
/// fallback when the probe fails.
fn root_identity(root: &dyn StagingDir) -> NodeIdentity {
    root.probe().map_or(EMPTY_ID, |p| p.identity)
}

/// Whether a *freshly reopened* staging root can be proven to be the
/// one the session recorded. A matched generation-bearing file
/// handle proves sameness on its own; without one, the remaining
/// identity fields can alias a foreign directory swapped in while
/// nothing held the inode — whatever strength the probes report — so
/// the root must additionally hold no names the session never
/// recorded: a foreign tree fails the check, while an empty or
/// record-only tree can be safely re-cleaned.
fn reopened_root_proven(root: &dyn StagingDir, probe: &VfsProbe, records: &[NodeRecord]) -> bool {
    if probe.identity.handle.is_some() {
        return true;
    }
    let recorded: BTreeSet<OsString> = records
        .iter()
        .map(|r| OsString::from(&r.components[0]))
        .collect();
    root.list_names()
        .is_ok_and(|names| names.iter().all(|n| recorded.contains(n.as_os_str())))
}

fn empty_receipt(root: &Path) -> CleanupReceipt {
    CleanupReceipt {
        attempted: 0,
        removed: 0,
        retained: 0,
        root: root.to_path_buf(),
    }
}

/// Adds a context message while preserving kind + source chain (the
/// original error becomes the `source`).
fn ctx(e: ArchiveError, context: impl Into<Cow<'static, str>>) -> ArchiveError {
    ArchiveError::with_source(e.kind(), context, e)
}

/// Recovers an [`ArchiveError`] that an entry reader wrapped into
/// `io::Error`; genuine I/O errors map to `Io` with the error as
/// source.
fn unwrap_archive_error(e: io::Error) -> ArchiveError {
    let is_archive = e
        .get_ref()
        .is_some_and(<dyn std::error::Error + Send + Sync>::is::<ArchiveError>);
    if is_archive {
        match e
            .into_inner()
            .and_then(|b| b.downcast::<ArchiveError>().ok())
        {
            Some(b) => *b,
            None => ArchiveError::from_kind(ArchiveErrorKind::Io),
        }
    } else {
        ArchiveError::from_io(e)
    }
}

fn map_backend_error(e: io::Error, context: impl Into<Cow<'static, str>>) -> ArchiveError {
    let kind = if e.kind() == io::ErrorKind::Unsupported {
        ArchiveErrorKind::Unsupported
    } else {
        ArchiveErrorKind::Io
    };
    ArchiveError::with_source(kind, context, e)
}

fn limit(what: &'static str, actual: u64, budget: u64) -> ArchiveError {
    ArchiveError::new(
        ArchiveErrorKind::LimitExceeded,
        format!("{what}: {actual} exceeds budget {budget}"),
    )
}

fn reject(message: impl Into<Cow<'static, str>>) -> ArchiveError {
    ArchiveError::new(ArchiveErrorKind::InvalidInput, message)
}

#[cfg(test)]
mod tests {
    //! Mock-backend fault-injection tests: the scripted
    //! `MockExtractOps` drives session/lifecycle branches the real FS
    //! cannot produce deterministically.

    use super::*;
    use crate::extract::fs::IdentityStrength;
    use crate::extract::fs::mock::{MockExtractOps, MockNode, MockStep, script};
    use crate::zip::{ZipArchiveReader, ZipArchiveWriter, ZipCompressionMethod, ZipEntryOptions};
    use std::io::{Cursor, Write};

    fn zip_bytes(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut writer = ZipArchiveWriter::new(Cursor::new(Vec::new()));
        for &(name, data) in entries {
            let mut entry = writer
                .start_entry(name, &ZipEntryOptions::new(ZipCompressionMethod::Stored))
                .expect("start_entry");
            entry.write_all(data).expect("write");
            entry.finish().expect("finish");
        }
        writer.finish().expect("archive finish").into_inner()
    }

    fn mock_session() -> (
        ExtractionSession,
        Arc<std::sync::Mutex<crate::extract::fs::mock::MockScript>>,
    ) {
        let (ops, state) = MockExtractOps::new();
        let session = ExtractionSession::create_with_ops(
            Path::new("parent"),
            ExtractOptions::new(),
            Arc::new(ops),
        )
        .expect("session");
        (session, state)
    }

    #[test]
    fn plan_phase_rejection_makes_no_tree_writes() {
        let (session, state) = mock_session();
        let mut archive = ZipArchiveReader::new(
            Cursor::new(zip_bytes(&[("../evil", b"x")])),
            ArchiveReadLimits::default(),
        )
        .expect("reader");
        let failure = session.extract_zip(&mut archive).expect_err("must fail");
        assert_eq!(failure.error().kind(), ArchiveErrorKind::InvalidInput);
        let calls = &state.lock().unwrap().calls;
        // Only the staging-root create + its cleanup ran — no dir/file
        // creation ever happened for the rejected entry.
        assert_eq!(calls.get("create_dir"), None);
        assert_eq!(calls.get("create_file"), None);
        assert_eq!(calls.get("remove_root"), Some(&1));
    }

    #[test]
    fn abort_failure_produces_a_recovery_handle() {
        let (session, state) = mock_session();
        script(
            &state,
            "remove_root",
            vec![MockStep::Fail(io::ErrorKind::PermissionDenied, "denied")],
        );
        let err = session.abort().expect_err("abort must fail");
        assert_eq!(err.handle.inspect().state, RecoveryTreeState::Present);
    }

    #[test]
    fn extraction_failure_attaches_handle_when_cleanup_retains() {
        let (session, state) = mock_session();
        // `a.txt` stages; `b.txt`'s create fails → write-phase failure
        // with one recorded node; cleanup then sees a foreign identity
        // at `a.txt` → retained → recovery handle attached.
        script(
            &state,
            "create_file",
            vec![
                MockStep::Ok,
                MockStep::Fail(io::ErrorKind::Other, "disk full"),
            ],
        );
        script(&state, "probe_child", vec![MockStep::ForeignIdentity]);
        let mut archive = ZipArchiveReader::new(
            Cursor::new(zip_bytes(&[("a.txt", b"1"), ("b.txt", b"2")])),
            ArchiveReadLimits::default(),
        )
        .expect("reader");
        let failure = session.extract_zip(&mut archive).expect_err("must fail");
        assert!(
            failure.handle().is_some(),
            "retained node must attach a handle"
        );
    }

    #[test]
    fn failed_cleanup_then_retry_uses_same_backend() {
        let (session, state) = mock_session();
        // Duplicate `a.txt` → plan fails before writes; the scripted
        // `remove_root` failure leaves the root in `nodes` → handle
        // attached; `inspect`/`cleanup` run through the same mock ops.
        script(
            &state,
            "remove_root",
            vec![MockStep::Fail(io::ErrorKind::Other, "denied")],
        );
        let mut archive = ZipArchiveReader::new(
            Cursor::new(zip_bytes(&[("a.txt", b"1"), ("a.txt", b"2")])),
            ArchiveReadLimits::default(),
        )
        .expect("reader");
        let failure = session.extract_zip(&mut archive).expect_err("must fail");
        let handle = failure.into_handle().expect("recovery handle");
        assert_eq!(handle.inspect().state, RecoveryTreeState::Present);
        // Script drained → retry succeeds and empties the mock tree.
        handle.cleanup().expect("retry cleanup");
        assert!(state.lock().unwrap().nodes.is_empty());
    }

    // ---- weak-identity filesystem scenarios (Issue #958) ----
    //
    // `default_strength: Weak` makes every probe report a weak
    // identity, like overlayfs in an unprivileged container (no btime,
    // no file handle). The mock's `nodes` map then lets a test plant
    // a foreign object that *aliases* a recorded identity — the exact
    // delete+recreate collision a strong FS would discriminate.

    fn weak_mock_session() -> (
        ExtractionSession,
        Arc<std::sync::Mutex<crate::extract::fs::mock::MockScript>>,
    ) {
        let (ops, state) = MockExtractOps::new();
        state.lock().unwrap().default_strength = IdentityStrength::Weak;
        let session = ExtractionSession::create_with_ops(
            Path::new("parent"),
            ExtractOptions::new(),
            Arc::new(ops),
        )
        .expect("session");
        (session, state)
    }

    fn staged_path(leaf: &str) -> PathBuf {
        Path::new("parent").join("rvf-staging-mock").join(leaf)
    }

    #[test]
    fn weak_fs_legitimate_cleanup_removes_the_tree() {
        let (session, state) = weak_mock_session();
        let mut archive = ZipArchiveReader::new(
            Cursor::new(zip_bytes(&[("d/f.txt", b"data"), ("a.txt", b"1")])),
            ArchiveReadLimits::default(),
        )
        .expect("reader");
        let tree = session.extract_zip(&mut archive).expect("extract");
        let receipt = tree.cleanup().expect("weak cleanup must still work");
        assert_eq!(receipt.retained, 0);
        assert!(state.lock().unwrap().nodes.is_empty());
    }

    #[test]
    fn weak_fs_aliased_foreign_file_is_retained() {
        let (session, state) = weak_mock_session();
        let mut archive = ZipArchiveReader::new(
            Cursor::new(zip_bytes(&[("victim.txt", b"ours")])),
            ArchiveReadLimits::default(),
        )
        .expect("reader");
        let tree = session.extract_zip(&mut archive).expect("extract");
        // Foreign file recreated at the recorded name with the SAME
        // weak identity (same-tick inode reuse) and the same length —
        // only its content differs.
        let path = staged_path("victim.txt");
        let (id, strength) = {
            let s = state.lock().unwrap();
            let node = s.nodes.get(&path).expect("staged node");
            (node.id, node.strength)
        };
        state.lock().unwrap().nodes.insert(
            path.clone(),
            MockNode {
                id,
                strength,
                content: b"FORE".to_vec(),
            },
        );
        let err = tree
            .cleanup()
            .expect_err("must refuse to delete the foreign file");
        assert_eq!(err.failure.receipt.retained, 1);
        let s = state.lock().unwrap();
        assert_eq!(
            s.nodes.get(&path).map(|n| n.content.as_slice()),
            Some(b"FORE".as_slice()),
            "the foreign object must survive cleanup",
        );
    }

    #[test]
    fn reported_strong_but_aliased_foreign_file_is_retained() {
        // The docker-lane regression: a filesystem that *reports*
        // `Strong` (btime exported) can still alias a recreated
        // object when its timestamp granularity is coarse — same
        // inode, same discriminator tick. Reported strength is not a
        // safe substitute for the secondary proof.
        let (session, state) = mock_session();
        let mut archive = ZipArchiveReader::new(
            Cursor::new(zip_bytes(&[("victim.txt", b"ours")])),
            ArchiveReadLimits::default(),
        )
        .expect("reader");
        let tree = session.extract_zip(&mut archive).expect("extract");
        let path = staged_path("victim.txt");
        let (id, strength) = {
            let s = state.lock().unwrap();
            let node = s.nodes.get(&path).expect("staged node");
            (node.id, node.strength)
        };
        assert_eq!(strength, IdentityStrength::Strong);
        state.lock().unwrap().nodes.insert(
            path.clone(),
            MockNode {
                id,
                strength,
                content: b"FORE".to_vec(),
            },
        );
        let err = tree
            .cleanup()
            .expect_err("must refuse to delete the foreign file");
        assert_eq!(err.failure.receipt.retained, 1);
        assert_eq!(
            state
                .lock()
                .unwrap()
                .nodes
                .get(&path)
                .map(|n| n.content.as_slice()),
            Some(b"FORE".as_slice()),
        );
    }

    #[test]
    fn weak_fs_dir_holding_a_planted_child_is_retained() {
        let (session, state) = weak_mock_session();
        let mut archive = ZipArchiveReader::new(
            Cursor::new(zip_bytes(&[("d/f.txt", b"data")])),
            ArchiveReadLimits::default(),
        )
        .expect("reader");
        let tree = session.extract_zip(&mut archive).expect("extract");
        // Plant an unrecorded object inside recorded dir `d`: even if
        // `d`'s weak identity aliases, a dir holding foreign objects
        // must not be rmdir'd into oblivion.
        let alien = staged_path("d/alien.txt");
        state.lock().unwrap().nodes.insert(
            alien.clone(),
            MockNode {
                id: NodeIdentity {
                    volume: 7,
                    index: 555,
                    tag: NodeTypeTag::File,
                    discriminator: 0,
                    handle: None,
                },
                strength: IdentityStrength::Weak,
                content: b"x".to_vec(),
            },
        );
        let err = tree
            .cleanup()
            .expect_err("must retain the dir holding a foreign child");
        assert_eq!(err.failure.receipt.retained, 1);
        let s = state.lock().unwrap();
        assert!(s.nodes.contains_key(&alien));
        assert!(s.nodes.contains_key(&staged_path("d")));
    }

    #[test]
    fn weak_fs_recovery_inspect_rejects_an_unrecorded_root() {
        let (session, state) = weak_mock_session();
        // `a.txt` stages; `b.txt`'s create fails → cleanup removes the
        // legit file but `remove_root` is denied → handle attached.
        script(
            &state,
            "create_file",
            vec![
                MockStep::Ok,
                MockStep::Fail(io::ErrorKind::Other, "disk full"),
            ],
        );
        script(
            &state,
            "remove_root",
            vec![MockStep::Fail(io::ErrorKind::Other, "denied")],
        );
        let mut archive = ZipArchiveReader::new(
            Cursor::new(zip_bytes(&[("a.txt", b"1"), ("b.txt", b"2")])),
            ArchiveReadLimits::default(),
        )
        .expect("reader");
        let failure = session.extract_zip(&mut archive).expect_err("must fail");
        let handle = failure.into_handle().expect("recovery handle");
        // A foreign object appears under the retained root path: on a
        // weak FS the reopened root's identity still equals the record,
        // so the structural check is what proves it foreign.
        state.lock().unwrap().nodes.insert(
            Path::new("parent")
                .join("rvf-staging-mock")
                .join("alien.txt"),
            MockNode {
                id: NodeIdentity {
                    volume: 7,
                    index: 777,
                    tag: NodeTypeTag::File,
                    discriminator: 0,
                    handle: None,
                },
                strength: IdentityStrength::Weak,
                content: b"!".to_vec(),
            },
        );
        assert_eq!(handle.inspect().state, RecoveryTreeState::Mismatch);
        handle
            .cleanup()
            .expect_err("refuse to clean a foreign tree");
    }

    #[test]
    fn weak_fs_recovery_retry_still_cleans_a_legit_tree() {
        let (session, state) = weak_mock_session();
        script(
            &state,
            "remove_root",
            vec![MockStep::Fail(io::ErrorKind::Other, "denied")],
        );
        let mut archive = ZipArchiveReader::new(
            Cursor::new(zip_bytes(&[("a.txt", b"1")])),
            ArchiveReadLimits::default(),
        )
        .expect("reader");
        let tree = session.extract_zip(&mut archive).expect("extract");
        let err = tree.cleanup().expect_err("scripted remove_root fails");
        let handle = err.handle;
        // Weak identity + structural proof → still recognised as ours.
        assert_eq!(handle.inspect().state, RecoveryTreeState::Present);
        handle.cleanup().expect("retry cleanup");
        assert!(state.lock().unwrap().nodes.is_empty());
    }
}
