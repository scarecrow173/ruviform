//! Read budgets for archive parsing and extraction measurement.
//!
//! [`ArchiveReadLimits`] bounds are enforced **before and during**
//! parsing and decompression — never only after the fact. Declared header
//! values (entry counts, directory sizes, uncompressed sizes) are used
//! for early rejection, while the actual bytes produced are measured as
//! they stream out and cut off mid-read when a budget is exceeded. A
//! compression ratio is never trusted in place of a byte budget.
//!
//! The struct is `#[non_exhaustive]` with public fields: construct it via
//! [`ArchiveReadLimits::default`] and mutate the fields you need.
//!
//! **RT contract:** non-RT only.

/// Budget limits applied while opening and reading an archive.
///
/// Defaults are conservative but generous enough for ordinary archives;
/// untrusted inputs should typically be read with *tighter* values chosen
/// by the caller. CPU/heap/filesystem costs cannot be modeled exactly by
/// byte budgets alone — these limits bound archive-controlled memory and
/// output growth, not every resource.
///
/// # Examples
///
/// ```
/// use ruviform_io_archive::ArchiveReadLimits;
///
/// let mut limits = ArchiveReadLimits::default();
/// limits.max_entries = 1_000;
/// limits.max_total_uncompressed_bytes = 256 * 1024 * 1024;
/// assert_eq!(limits.max_entries, 1_000);
/// ```
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveReadLimits {
    /// Maximum number of index entries (e.g. ZIP central directory
    /// records) the reader may accept. Checked against the declared
    /// count *before* any per-record allocation.
    pub max_entries: u32,
    /// Maximum byte size of index/metadata regions (e.g. the ZIP
    /// central directory). Checked before the region is buffered.
    pub max_metadata_bytes: u64,
    /// Maximum byte length of a single stored entry name.
    pub max_path_bytes: u32,
    /// Maximum uncompressed bytes a single entry may produce. Declared
    /// sizes above this are rejected at entry-open time; measured output
    /// is also cut off mid-read.
    pub max_entry_uncompressed_bytes: u64,
    /// Maximum total uncompressed bytes across an archive-wide
    /// verification or extraction pass (e.g. `verify_all`), accumulated
    /// and enforced while entries stream out.
    pub max_total_uncompressed_bytes: u64,
}

impl Default for ArchiveReadLimits {
    fn default() -> Self {
        Self {
            max_entries: 10_000,
            max_metadata_bytes: 16 * 1024 * 1024,
            max_path_bytes: 4 * 1024,
            max_entry_uncompressed_bytes: 4 * 1024 * 1024 * 1024,
            max_total_uncompressed_bytes: 16 * 1024 * 1024 * 1024,
        }
    }
}
