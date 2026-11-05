//! TAR archive reading and writing.
//!
//! - [`TarArchiveReader`]: streaming reader for `Read` sources —
//!   sequential header/extension parsing under
//!   [`crate::ArchiveReadLimits`], the `Ready`/`EntryOpen`/`Pending`/
//!   `Failed` state machine, exclusive-borrow [`TarEntryReader`], and an
//!   archive `finish` that verifies the end-of-archive marker and a
//!   zero tail.
//! - [`TarArchiveWriter`]: streaming writer for `Write` sinks —
//!   `start_entry` with a declared size → [`TarEntryWriter`] → entry
//!   `finish` → archive `finish` (two zero blocks), with explicit finish
//!   as the only completion proof.
//!
//! Supported surface: POSIX ustar (name + prefix), GNU tar magic,
//! GNU `L`/`K` long-name extensions, PAX `x`/`g`/`X` extended headers
//! (`path`/`linkpath`/`size` applied; `g` grammar-validated only), and
//! base-256 numeric fields on the read side. Sparse (`S`), special
//! (`3`/`4`/`6` char/block/fifo), multi-volume, and unknown typeflags
//! report [`crate::ArchiveErrorKind::Unsupported`]. Symbolic/hard link
//! entries are exposed as metadata only — their targets are handed to
//! the caller untouched; safe handling is the extraction layer's
//! contract (IO-IMP-10).
//!
//! **RT contract:** non-RT only.

mod header;
mod read;
mod write;

pub use read::{TarArchiveReader, TarEntryReader};
pub use write::{TarArchiveWriter, TarEntryOptions, TarEntryWriter};
