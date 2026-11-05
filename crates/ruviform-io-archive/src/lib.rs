//! `ruviform-io-archive` — shared archive entry contract + ZIP/TAR
//! reader/writer (IO-04 / Issue #286, IO-IMP-08 / Issue #302,
//! IO-IMP-09 / Issue #303).
//!
//! This crate is the **scaffold owner** for Ruviform's archive
//! foundation. It establishes the format-agnostic public types that
//! IO-IMP-10 (secure extraction) builds on:
//!
//! - [`ArchiveError`] / [`ArchiveErrorKind`] — normalized failure
//!   classification (structure, truncation, local/central-directory
//!   inconsistency, checksum/size mismatch, budget limits, overlap,
//!   unsupported features, I/O).
//! - [`ArchiveName`] / [`EntryKind`] / [`ArchiveEntryInfo`] — entry
//!   names preserved as raw bytes (non-UTF-8 safe) plus the metadata a
//!   format genuinely records (including link targets via
//!   [`ArchiveEntryInfo::link_name`]).
//! - [`ArchiveReadLimits`] — budgets enforced **before and during**
//!   parsing/decompression (entry count, metadata bytes, path bytes,
//!   per-entry and total uncompressed bytes).
//! - [`ArchiveReadReceipt`] / [`EntryReadReceipt`] — typed verification
//!   summaries, never a single `verified: bool` rollup;
//!   [`EntryDisposition`] records whether an entry was read to its end
//!   or discarded structurally.
//!
//! The [`zip`] module adds `Read + Seek` indexed reading and
//! `Write + Seek` writing for ZIP Stored/Deflate entries. CRC32 and
//! DEFLATE are handled by hidden backends (`crc32fast`, `flate2`
//! `rust_backend`) — no backend type, error, or feature name appears in
//! the public API.
//!
//! The [`tar`] module adds `Read`-only streaming reading and
//! `Write`-only writing for ustar/GNU/PAX TAR. TAR is sequential, so
//! entries are walked one header at a time; a mid-read dropped entry
//! leaves the archive `Pending` until `discard_pending` drains it.
//! TAR payloads carry no checksum — entry receipts always report
//! `EntryChecksum::NotPresent` (never a corruption/authenticity proof).
//!
//! # Threat boundary (contract — implementation is IO-IMP-10)
//!
//! - *Zip-slip paths:* [`ArchiveName`] hands raw stored bytes to the
//!   caller. `..`/absolute/drive/UNC/reserved-name rejection is the
//!   extraction layer's contract, not this crate's — nothing here
//!   claims a name is a safe filesystem path.
//! - *Expansion bombs:* declared sizes are early-rejection hints only.
//!   Actual output is measured while it streams and cut off at the
//!   configured caps, so excess bytes never reach the caller.
//! - *Central-directory inconsistency:* local headers are checked
//!   against the CD at entry open and exhaustively at `verify_all`;
//!   entry/CD region overlap and truncation are classified errors.
//! - *Authenticity:* CRC32 detects corruption; it is not a
//!   cryptographic integrity or authenticity proof — and TAR has no
//!   payload checksum at all.
//!
//! # Completion proof
//!
//! `ZipEntryReader::finish`, `ZipEntryWriter::finish`,
//! `ZipArchiveWriter::finish`, `TarEntryReader::finish` /
//! `discard_remaining`, `TarEntryWriter::finish`, and
//! `TarArchiveWriter::finish` are the only completion proofs — `Drop`
//! never counts as success, and an unfinished dropped entry writer
//! poisons its archive (`InvalidState`).
//!
//! # RT contract
//!
//! **Non-RT only** — every operation may block on I/O and allocate.
//! Never call this API from an RT audio callback.
//!
//! # Examples
//!
//! Write then verify a ZIP in memory:
//!
//! ```
//! use ruviform_io_archive::zip::{
//!     ZipArchiveReader, ZipArchiveWriter, ZipCompressionMethod, ZipEntryOptions,
//! };
//! use ruviform_io_archive::ArchiveReadLimits;
//! use std::io::{Cursor, Read, Write};
//!
//! let mut writer = ZipArchiveWriter::new(Cursor::new(Vec::new()));
//! {
//!     let mut entry = writer
//!         .start_entry("note.txt", &ZipEntryOptions::new(ZipCompressionMethod::Stored))?;
//!     entry.write_all(b"hello")?;
//!     entry.finish()?;
//! }
//! let bytes = writer.finish()?.into_inner();
//!
//! let mut archive = ZipArchiveReader::new(Cursor::new(bytes), ArchiveReadLimits::default())?;
//! let receipt = archive.verify_all()?;
//! assert_eq!(receipt.entries_verified(), 1);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
#![allow(clippy::module_name_repetitions)]

mod entry;
mod error;
mod limits;
mod receipt;
pub mod tar;
pub mod zip;

pub use entry::{ArchiveEntryInfo, ArchiveName, EntryKind};
pub use error::{ArchiveError, ArchiveErrorKind};
pub use limits::ArchiveReadLimits;
pub use receipt::{ArchiveReadReceipt, EntryChecksum, EntryDisposition, EntryReadReceipt};

// Compile-time API assertions: the public handles and errors stay
// `Send` so consumers may move them across threads.
const _: () = {
    const fn assert_send<T: Send>() {}
    assert_send::<ArchiveError>();
    assert_send::<ArchiveEntryInfo>();
    assert_send::<ArchiveReadLimits>();
    assert_send::<ArchiveReadReceipt>();
    assert_send::<zip::ZipArchiveReader<std::io::Cursor<Vec<u8>>>>();
    assert_send::<zip::ZipArchiveWriter<std::io::Cursor<Vec<u8>>>>();
    assert_send::<tar::TarArchiveReader<std::io::Cursor<Vec<u8>>>>();
    assert_send::<tar::TarArchiveWriter<std::io::Cursor<Vec<u8>>>>();
};
