//! Format-agnostic archive entry metadata.
//!
//! [`ArchiveName`] preserves the entry's stored name **byte-for-byte**
//! (including non-UTF-8 names), and [`ArchiveEntryInfo`] records only
//! metadata that actually exists in the container format — no
//! normalization and no filesystem-path semantics are applied here.
//!
//! **RT contract:** non-RT only.

use std::borrow::Cow;
use std::fmt;

/// The stored name of an archive entry, kept as raw bytes.
///
/// Archive formats do not guarantee UTF-8 names: `ArchiveName` keeps the
/// original bytes so no information is lost. [`to_str`](Self::to_str)
/// returns `Some` only when the bytes are valid UTF-8;
/// [`to_string_lossy`](Self::to_string_lossy) is a diagnostic aid, not a
/// faithful representation.
///
/// **This type performs no path safety checks.** `..`, absolute paths,
/// drive/UNC prefixes, reserved device names, and separator ambiguity are
/// preserved untouched — rejecting them is the extraction layer's job
/// (IO-IMP-10 `SafeRelativePath`), not this type's.
///
/// **RT contract:** non-RT only — `to_string_lossy` may allocate.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ArchiveName {
    raw: Vec<u8>,
}

impl ArchiveName {
    /// Wraps raw stored bytes. No validation or normalization is applied.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn new(raw: Vec<u8>) -> Self {
        Self { raw }
    }

    /// The stored name bytes, unchanged.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.raw
    }

    /// The name as `&str`, or `None` when the bytes are not valid UTF-8.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn to_str(&self) -> Option<&str> {
        std::str::from_utf8(&self.raw).ok()
    }

    /// Lossy UTF-8 rendering for diagnostics. Never use this as a
    /// filesystem name — non-UTF-8 bytes become `U+FFFD`.
    ///
    /// **RT contract:** non-RT only — may allocate.
    #[must_use]
    pub fn to_string_lossy(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(&self.raw)
    }

    /// Byte length of the stored name.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn len(&self) -> usize {
        self.raw.len()
    }

    /// Whether the stored name is empty.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.raw.is_empty()
    }
}

impl fmt::Debug for ArchiveName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ArchiveName({:?})", self.to_string_lossy())
    }
}

impl fmt::Display for ArchiveName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_string_lossy())
    }
}

/// The kind of payload an archive entry represents.
///
/// `#[non_exhaustive]` per the public-enum policy — formats may report
/// additional kinds in the future.
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntryKind {
    /// A regular file payload.
    File,
    /// A directory marker (e.g. a ZIP name ending in `/` or a directory
    /// attribute bit).
    Directory,
    /// A symbolic link (payload is the link target).
    SymbolicLink,
    /// A hard link to another entry.
    HardLink,
    /// A special file (device, FIFO, socket, ...).
    Special,
    /// The entry's kind is declared but not understood by this crate.
    Unsupported,
}

/// Metadata describing one archive entry.
///
/// Only facts that genuinely exist in the container format are recorded.
/// Sizes are the format's **declared** values — early-rejection hints for
/// budgets, never a substitute for measuring the actual bytes produced
/// while reading.
///
/// **RT contract:** non-RT only — cloning allocates (name bytes).
#[derive(Debug, Clone)]
pub struct ArchiveEntryInfo {
    /// Position of the entry in the format's index (ZIP central
    /// directory order; TAR stream order).
    index: usize,
    /// Stored entry name (raw bytes).
    name: ArchiveName,
    /// Entry kind as declared by the format.
    kind: EntryKind,
    /// Declared uncompressed size in bytes, when the format records one.
    uncompressed_size: Option<u64>,
    /// Declared compressed size in bytes, when the format records one
    /// (ZIP does; TAR does not).
    compressed_size: Option<u64>,
}

impl ArchiveEntryInfo {
    /// Builds an info record. Crate-internal — formats fill in only the
    /// fields that exist.
    pub(crate) const fn new(
        index: usize,
        name: ArchiveName,
        kind: EntryKind,
        uncompressed_size: Option<u64>,
        compressed_size: Option<u64>,
    ) -> Self {
        Self {
            index,
            name,
            kind,
            uncompressed_size,
            compressed_size,
        }
    }

    /// Position of the entry in the format's index.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn index(&self) -> usize {
        self.index
    }

    /// The stored entry name (raw bytes, unmodified).
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn name(&self) -> &ArchiveName {
        &self.name
    }

    /// Entry kind as declared by the format.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn kind(&self) -> EntryKind {
        self.kind
    }

    /// Declared uncompressed size in bytes, if the format records one.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn uncompressed_size(&self) -> Option<u64> {
        self.uncompressed_size
    }

    /// Declared compressed size in bytes, if the format records one.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn compressed_size(&self) -> Option<u64> {
        self.compressed_size
    }
}
