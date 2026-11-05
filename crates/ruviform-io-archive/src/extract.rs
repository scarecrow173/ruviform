//! Secure archive extraction (IO-IMP-10).
//!
//! Turns a *verified-parse* archive ([`ZipArchiveReader`],
//! [`TarArchiveReader`]) into a verified on-disk tree under an
//! exclusive isolated staging root — never into a caller-named
//! destination.
//!
//! **Secure default.** Only ordinary relative files and directories
//! are extracted: every stored name passes
//! [`ArchiveName`](crate::ArchiveName) → [`SafeRelativePath`]
//! validation (no absolute/drive/UNC paths, no `..`, no `\`/`:`,
//! no Windows-reserved names or trailing-dot/space components, no
//! control characters, no NUL, UTF-8 only), canonical collisions
//! (exact duplicates, file-vs-directory conflicts, Unicode
//! NFC/case-fold equivalents) are rejected, and symlink/hardlink/
//! special entries are refused. Writes never overwrite: every file
//! and directory is created exclusively, and the staging backend
//! traverses by handle with no-follow semantics so a concurrently
//! planted link or swapped directory is an error, not a redirect.
//!
//! **Budgets.** `ExtractOptions::limits` applies the shared
//! [`ArchiveReadLimits`] contract: entry count, metadata bytes, path
//! bytes/depth, per-entry declared bytes, and total declared
//! bytes are all bounded *before* a byte is written, and measured
//! output is capped during the copy (excess never reaches disk).
//!
//! **Lifecycle.** `ExtractionSession::create` makes the isolated
//! staging root; `extract_zip`/`extract_tar` stage + verify and hand
//! out an [`ExtractedTree`]. The tree is a *verified tree*, not an
//! immutable snapshot — contents are ordinary files. `cleanup()`
//! removes it with per-node identity checks (foreign objects are
//! retained, never deleted); failure yields
//! [`ExtractionRecoveryHandle`] for `inspect`/retry. `into_path()`
//! transfers ownership without cleanup; `Drop` cleans best-effort.
//!
//! **Honest guarantees.** On targets without a no-follow,
//! handle-relative backend (`unix`, `windows` have one), session
//! creation reports `Unsupported` rather than degrading to unsafe
//! path-string traversal. Consumer-level publication — project
//! commit, CAS, directory replace — is out of scope for this layer.
//!
//! **RT contract:** non-RT only — filesystem I/O, allocation.

mod fs;
mod path;
mod session;
#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

pub use path::SafeRelativePath;
pub use session::{
    CleanupReceipt, ExtractOptions, ExtractedTree, ExtractionCleanupFailure, ExtractionFailure,
    ExtractionReceipt, ExtractionRecoveryHandle, ExtractionRecoveryInfo, ExtractionSession,
    RecoveryCleanupError, RecoveryTreeState,
};
