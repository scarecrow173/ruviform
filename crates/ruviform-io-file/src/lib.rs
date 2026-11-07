//! `ruviform-io-file` — safe staged file publish (IO-01 / Issue #283 v0.4).
//!
//! Producing a completed destination file is a typed lifecycle:
//!
//! 1. [`StagedFile::create`] opens a *temporary sibling* of the
//!    destination — always on the same filesystem — and the caller
//!    writes through `StagedFile` itself (`std::io::Write + Seek`).
//! 2. [`StagedFile::seal`] consumes the writer and yields a
//!    [`SealedFile`]: the type-level proof that the write phase ended.
//!    `SealedFile` cannot be written; it can only be read back
//!    ([`SealedFile::open_readback`]), published, or aborted.
//! 3. [`SealedFile::publish`] performs the requested sync + atomic
//!    publish + directory-sync pipeline and reports either a
//!    [`PublishReceipt`] (the publish *and* every requested guarantee
//!    succeeded) or a [`PublishFailure`] classifying
//!    `NotPublished` / `Published` / `Indeterminate`, the failing
//!    [`PublishPhase`], the [`DurabilityEvidence`] actually achieved,
//!    and a [`RecoveryHandle`] for inspection/cleanup.
//!
//! Neither the internal OS `File`, nor any cloneable handle to it, is
//! ever exposed. All OS interaction goes through the private
//! `PlatformFileOps` boundary so platform backends are interchangeable
//! implementation details.
//!
//! # OS backend — IO-IMP-02 scope
//!
//! On unix and Windows targets [`StagedFile::create`] stages a real
//! temp sibling and `publish` performs the real atomic primitives:
//! `renameat2(RENAME_NOREPLACE)` / `renamex_np(RENAME_EXCL)` /
//! `link()`+`unlink()` on unix and `MoveFileExW` on Windows for
//! [`PublishMode::CreateNew`]; `fs::rename` for
//! [`PublishMode::Replace`]; `File::sync_all` for file sync; and a
//! directory-handle sync for
//! [`Durability::FileAndDirectorySync`]. A platform or filesystem that
//! cannot provide a requested guarantee reports `Unsupported`/failure
//! — `exists()`-then-`rename()` is never presented as an atomic
//! `CreateNew`, and no silent weaker fallback is used. See the crate
//! README for the OS/FS capability matrix and trust boundaries.
//!
//! # Guarantees vs. requests
//!
//! `Ok(PublishReceipt)` is returned **only** when the requested mode,
//! visibility and durability were all achieved. A directory-sync
//! failure after a successful publish is `Err` with
//! [`PublishEffect::Published`] — never a false `Ok`, never a false
//! "not published". An indeterminate publish result is reported as
//! [`PublishEffect::Indeterminate`], and its leftovers are **not**
//! automatically deleted by `Drop`; recovery goes through
//! [`RecoveryHandle::inspect`] / [`RecoveryHandle::cleanup`].
//!
//! `Write::flush`, file sync, directory sync, atomic visibility and
//! power-failure durability are distinct guarantees; this crate never
//! reports a requested level it did not achieve.
//!
//! # RT contract
//!
//! **Non-RT only** — every operation may block on filesystem I/O and
//! allocate. Never call this API from an RT audio callback.
//!
//! # Examples
//!
//! Typical usage (real OS backend on unix/Windows):
//!
//! ```no_run
//! use ruviform_io_file::{Durability, PublishMode, PublishOptions, StagedFile};
//! use std::io::Write;
//!
//! let mut staged = StagedFile::create("out.wav")?;
//! staged.write_all(b"payload")?;
//! let sealed = staged.seal();
//! let receipt = sealed.publish(PublishOptions::new(
//!     PublishMode::CreateNew,
//!     Durability::FileAndDirectorySync,
//! ))?;
//! println!("published {:?} (durability: {:?})", receipt.destination(), receipt.durability());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Format-verified publish (readback before publish — the consumer's
//! format layer decides whether the sealed bytes are valid):
//!
//! ```no_run
//! use ruviform_io_file::{Durability, PublishMode, PublishOptions, StagedFile};
//! use std::io::{Read, Write};
//!
//! let mut staged = StagedFile::create("out.wav")?;
//! staged.write_all(b"RIFF....")?;
//! let mut sealed = staged.seal();
//! let mut buf = [0u8; 4];
//! sealed.open_readback()?.read_exact(&mut buf)?;
//! assert_eq!(&buf, b"RIFF");
//! // The ReadbackFile borrow has ended; publish consumes `sealed`.
//! let receipt = sealed.publish(PublishOptions::new(
//!     PublishMode::Replace,
//!     Durability::FileSync,
//! ))?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
#![allow(clippy::module_name_repetitions)]

mod error;
#[cfg(test)]
mod mock;
mod options;
#[cfg(any(unix, windows))]
mod os;
mod outcome;
mod platform;
mod receipt;
pub mod rooted;
mod staged;
#[doc(hidden)]
pub mod vfs;

pub use error::{FileError, FileErrorKind};
pub use options::{Durability, PublishMode, PublishOptions, VisibilityRequirement};
pub use outcome::{
    DurabilityEvidence, OperationId, PublishEffect, PublishFailure, PublishFailureKind,
    PublishPhase, TempState,
};
pub use receipt::{
    CleanupFailure, CleanupReceipt, PublishReceipt, RecoveryHandle, RecoveryInfo,
    RecoveryObservation,
};
pub use staged::{ReadbackFile, SealedFile, StagedFile};
