//! ZIP archive reading and writing.
//!
//! - [`ZipArchiveReader`]: indexed reader for `Read + Seek` sources —
//!   EOCD/central-directory parsing under [`crate::ArchiveReadLimits`],
//!   exclusive-borrow [`ZipEntryReader`], and [`ZipArchiveReader::verify_all`].
//! - [`ZipArchiveWriter`]: streaming writer for `Write + Seek` sinks —
//!   `start_entry` → [`ZipEntryWriter`] → entry `finish` → archive
//!   `finish`, with explicit finish as the only completion proof.
//!
//! Supported surface: Stored/Deflate methods, classic (non-ZIP64)
//! archives. Encryption, unknown methods, multi-disk, and ZIP64-required
//! structures report [`crate::ArchiveErrorKind::Unsupported`].
//!
//! **RT contract:** non-RT only.

mod read;
mod record;
mod write;

pub use read::{ZipArchiveReader, ZipEntryReader};
pub use write::{ZipArchiveWriter, ZipCompressionMethod, ZipEntryOptions, ZipEntryWriter};
