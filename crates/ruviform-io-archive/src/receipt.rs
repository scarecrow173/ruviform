//! Verification receipts for archive reads.
//!
//! Receipts report **what was actually verified** — never a single
//! `verified: bool` rollup. A checksum result only means integrity was
//! checked against the stored value; CRC-style checksums detect
//! corruption, they do not prove authenticity.
//!
//! **RT contract:** non-RT only.

use crate::entry::ArchiveEntryInfo;

/// Outcome of checking an entry's stored checksum against the measured
/// payload.
///
/// `#[non_exhaustive]` per the public-enum policy — formats may report
/// additional outcomes (e.g. a cryptographic hash) in the future.
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntryChecksum {
    /// The measured payload's checksum equals the stored one.
    /// Integrity was verified; authenticity is not implied.
    Verified,
    /// The format stores no checksum for this entry — reading to the
    /// declared end says nothing about corruption (e.g. plain TAR
    /// payloads).
    NotPresent,
}

/// Result of finishing one entry reader via `finish`.
///
/// `uncompressed_bytes` is the **measured** output (verified against the
/// declared size as part of `finish`), not the header's claim.
///
/// **RT contract:** non-RT only.
#[derive(Debug, Clone)]
pub struct EntryReadReceipt {
    info: ArchiveEntryInfo,
    uncompressed_bytes: u64,
    checksum: EntryChecksum,
}

impl EntryReadReceipt {
    /// Builds a receipt. Crate-internal.
    pub(crate) const fn new(
        info: ArchiveEntryInfo,
        uncompressed_bytes: u64,
        checksum: EntryChecksum,
    ) -> Self {
        Self {
            info,
            uncompressed_bytes,
            checksum,
        }
    }

    /// The verified entry's metadata.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn info(&self) -> &ArchiveEntryInfo {
        &self.info
    }

    /// Measured uncompressed bytes produced for this entry.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn uncompressed_bytes(&self) -> u64 {
        self.uncompressed_bytes
    }

    /// Checksum verification outcome.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn checksum(&self) -> EntryChecksum {
        self.checksum
    }
}

/// Result of an archive-wide verification pass (`verify_all`).
///
/// A successful receipt means every entry in the index was opened,
/// decompressed to its declared end, and checked (checksum where the
/// format stores one, measured size always). `payload_bytes` is the
/// measured uncompressed total actually produced.
///
/// **RT contract:** non-RT only.
#[derive(Debug, Clone)]
pub struct ArchiveReadReceipt {
    entries_total: u32,
    entries_verified: u32,
    entries_unverified: u32,
    payload_bytes: u64,
}

impl ArchiveReadReceipt {
    /// Builds a receipt. Crate-internal.
    pub(crate) const fn new(
        entries_total: u32,
        entries_verified: u32,
        entries_unverified: u32,
        payload_bytes: u64,
    ) -> Self {
        Self {
            entries_total,
            entries_verified,
            entries_unverified,
            payload_bytes,
        }
    }

    /// Number of entries in the archive index.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn entries_total(&self) -> u32 {
        self.entries_total
    }

    /// Number of entries fully read and verified.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn entries_verified(&self) -> u32 {
        self.entries_verified
    }

    /// Number of index entries that were not verified (e.g. entries
    /// dropped mid-read earlier and not re-verified). After a successful
    /// `verify_all` this is `0`.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn entries_unverified(&self) -> u32 {
        self.entries_unverified
    }

    /// Total measured uncompressed bytes produced while verifying.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn payload_bytes(&self) -> u64 {
        self.payload_bytes
    }
}
