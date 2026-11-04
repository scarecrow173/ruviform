//! ZIP indexed reader: EOCD/central-directory parsing with
//! pre-allocation budgets, exclusive-borrow entry readers, and
//! archive-wide `verify_all`.
//!
//! Parsing pipeline (`ZipArchiveReader::new`):
//!
//! 1. measure the input, bound the EOCD tail search to a fixed
//!    ~64 KiB window and require the record to end exactly at EOF;
//! 2. reject multi-disk and ZIP64-required EOCDs as `Unsupported`;
//! 3. check `max_entries` / `max_metadata_bytes` **before** buffering
//!    the central directory;
//! 4. parse CD records one at a time, enforcing `max_path_bytes` before
//!    each name allocation and `max_entries` mid-parse;
//! 5. audit lower-bound entry regions for overlap with each other and
//!    the CD region.
//!
//! Local-header consistency and per-entry data/CRC checks happen lazily
//! at `open_entry` and exhaustively in `verify_all` — a successful
//! `new` means "the index parsed within budget", nothing more.

use std::io::{Read, Seek, SeekFrom, Take};

use flate2::read::DeflateDecoder;

use crate::entry::{ArchiveEntryInfo, ArchiveName, EntryKind};
use crate::error::{ArchiveError, ArchiveErrorKind};
use crate::limits::ArchiveReadLimits;
use crate::receipt::{ArchiveReadReceipt, EntryChecksum, EntryReadReceipt};
use crate::zip::record::{
    CD_RECORD_LEN, CD_SIG, CdRecord, EOCD_LEN, EOCD_SEARCH_SPAN, EOCD_SIG, Eocd,
    FLAG_DATA_DESCRIPTOR, FLAG_ENCRYPTED, LFH_LEN, LFH_SIG, METHOD_DEFLATE, METHOD_STORED, u16le,
    u32le,
};

/// Indexed reader for a `Read + Seek` ZIP byte stream.
///
/// Constructing the reader locates the end-of-central-directory record
/// and parses the central directory under [`ArchiveReadLimits`]. It does
/// **not** verify entry payloads: use [`open_entry`](Self::open_entry) +
/// [`ZipEntryReader::finish`] for per-entry verification, or
/// [`verify_all`](Self::verify_all) to re-read and check every entry
/// (CRC, measured size, local-header vs. central-directory consistency).
///
/// The reader re-seeks on every entry access; if the underlying input is
/// concurrently modified, verification failures surface as
/// `Inconsistent`/`ChecksumMismatch`-class errors rather than a defined
/// snapshot guarantee.
///
/// **RT contract:** non-RT only.
///
/// # Examples
///
/// ```no_run
/// use ruviform_io_archive::{ArchiveReadLimits, zip::ZipArchiveReader};
/// use std::io::Read;
///
/// let file = std::fs::File::open("bundle.zip")?;
/// let mut archive = ZipArchiveReader::new(file, ArchiveReadLimits::default())?;
/// for i in 0..archive.entry_count() {
///     let info = archive.entry_info(i).expect("index in range");
///     let mut entry = archive.open_entry(i)?;
///     let mut buf = Vec::new();
///     entry.read_to_end(&mut buf)?;
///     let receipt = entry.finish()?;
///     println!("{}: {} bytes verified", info.name(), receipt.uncompressed_bytes());
/// }
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct ZipArchiveReader<R: Read + Seek> {
    inner: R,
    limits: ArchiveReadLimits,
    entries: Vec<CdRecord>,
    /// Per-entry "fully read + verified" record. Dropping an entry
    /// reader without `finish` leaves its slot `false`.
    verified: Vec<bool>,
    file_len: u64,
    /// Absolute offset where the central directory begins.
    cd_offset: u64,
}

impl<R: Read + Seek> ZipArchiveReader<R> {
    /// Opens a ZIP stream: locates and parses EOCD + central directory
    /// under `limits`. See the module docs for the pipeline.
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// `Truncated` (too short / cut records / no EOCD at end),
    /// `Malformed` (bad signatures, out-of-range offsets, count
    /// mismatch), `LimitExceeded` (budget exceeded before/during
    /// parse), `Overlap` (entry regions colliding with each other or
    /// the CD), `Unsupported` (multi-disk / ZIP64-required), `Io`.
    pub fn new(mut inner: R, limits: ArchiveReadLimits) -> Result<Self, ArchiveError> {
        let file_len = inner
            .seek(SeekFrom::End(0))
            .map_err(ArchiveError::from_io)?;
        if file_len < EOCD_LEN as u64 {
            return Err(ArchiveError::new(
                ArchiveErrorKind::Truncated,
                "input shorter than a minimal end-of-central-directory record",
            ));
        }
        let eocd = locate_eocd(&mut inner, file_len)?;
        if eocd.is_multi_disk() {
            return Err(ArchiveError::new(
                ArchiveErrorKind::Unsupported,
                "multi-disk archives are not supported",
            ));
        }
        if eocd.requires_zip64() {
            return Err(ArchiveError::new(
                ArchiveErrorKind::Unsupported,
                "archive requires ZIP64 (saturated EOCD fields); ZIP64 is not supported yet",
            ));
        }
        let cd_offset = u64::from(eocd.cd_offset);
        let cd_size = u64::from(eocd.cd_size);
        if cd_offset + cd_size > eocd.position {
            return Err(ArchiveError::new(
                ArchiveErrorKind::Malformed,
                "central directory region extends past the end-of-central-directory record",
            ));
        }
        // Budget gates BEFORE the CD region is buffered or records are
        // allocated.
        if u64::from(eocd.cd_count) > u64::from(limits.max_entries) {
            return Err(limit("max_entries", eocd.cd_count, limits.max_entries));
        }
        if cd_size > limits.max_metadata_bytes {
            return Err(limit_u64(
                "max_metadata_bytes",
                cd_size,
                limits.max_metadata_bytes,
            ));
        }
        let entries = parse_central_directory(&mut inner, cd_offset, cd_size, &limits)?;
        if entries.len() != usize::from(eocd.cd_count) {
            return Err(ArchiveError::new(
                ArchiveErrorKind::Malformed,
                format!(
                    "central directory declared {} entries but contains {}",
                    eocd.cd_count,
                    entries.len()
                ),
            ));
        }
        audit_index_overlap(&entries, cd_offset)?;
        let verified = vec![false; entries.len()];
        Ok(Self {
            inner,
            limits,
            entries,
            verified,
            file_len,
            cd_offset,
        })
    }

    /// Number of entries in the parsed index.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Metadata of entry `index`, or `None` when out of range.
    ///
    /// **RT contract:** non-RT only — clones the stored name.
    #[must_use]
    pub fn entry_info(&self, index: usize) -> Option<ArchiveEntryInfo> {
        self.entries.get(index).map(|r| entry_info(index, r))
    }

    /// `true` once entry `index` was fully read and verified via
    /// [`ZipEntryReader::finish`] or [`verify_all`](Self::verify_all).
    /// Entries whose reader was dropped mid-read stay unverified.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn entry_is_verified(&self, index: usize) -> bool {
        self.verified.get(index).copied().unwrap_or(false)
    }

    /// Opens entry `index` for reading. The returned reader holds an
    /// exclusive `&mut` borrow — no other entry may be opened until it
    /// is dropped or finished.
    ///
    /// Validates the local header against the central directory and
    /// early-rejects entries whose declared uncompressed size exceeds
    /// `max_entry_uncompressed_bytes`. Dropping the reader without
    /// [`finish`](ZipEntryReader::finish) leaves the entry unverified
    /// and allows opening another (or the same) entry.
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// `InvalidInput` (index out of range), `LimitExceeded`,
    /// `Unsupported` (encrypted / unknown method / ZIP64 sizes),
    /// `Truncated`, `Malformed` (bad local signature),
    /// `Inconsistent` (local header contradicts the CD), `Overlap`
    /// (entry data reaching into the CD region), `Io`.
    pub fn open_entry(&mut self, index: usize) -> Result<ZipEntryReader<'_, R>, ArchiveError> {
        self.open_entry_inner(index, None)
    }

    /// Re-reads and verifies **every** entry: local-header/CD
    /// consistency, measured uncompressed size, and CRC32, accumulating
    /// `max_total_uncompressed_bytes` across all entries. Marks all
    /// entries verified on success.
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// The first entry failing verification aborts the pass with that
    /// entry's error kind (`ChecksumMismatch`, `SizeMismatch`,
    /// `Inconsistent`, `Unsupported`, `LimitExceeded`, `Overlap`, ...).
    /// No partial receipt is produced.
    pub fn verify_all(&mut self) -> Result<ArchiveReadReceipt, ArchiveError> {
        let mut payload_bytes = 0_u64;
        let mut regions: Vec<(u64, u64)> = Vec::with_capacity(self.entries.len());
        for index in 0..self.entries.len() {
            let remaining = self
                .limits
                .max_total_uncompressed_bytes
                .saturating_sub(payload_bytes);
            let entry = self.open_entry_inner(index, Some(remaining))?;
            regions.push(entry.data_range());
            let receipt = entry.finish()?;
            payload_bytes = payload_bytes.saturating_add(receipt.uncompressed_bytes());
        }
        // With every local header now parsed, audit the real data
        // regions pairwise (the open-time audit only had lower bounds).
        regions.sort_unstable();
        for pair in regions.windows(2) {
            if pair[0].1 > pair[1].0 {
                return Err(ArchiveError::new(
                    ArchiveErrorKind::Overlap,
                    format!(
                        "entry data regions overlap ({}..{} vs {}..{})",
                        pair[0].0, pair[0].1, pair[1].0, pair[1].1
                    ),
                ));
            }
        }
        let total = u32::try_from(self.entries.len()).unwrap_or(u32::MAX);
        Ok(ArchiveReadReceipt::new(total, total, 0, payload_bytes))
    }

    /// Returns the inner reader. Index/entry verification state is
    /// discarded.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn into_inner(self) -> R {
        self.inner
    }

    /// Shared open path. `total_budget` carries the remaining
    /// `max_total_uncompressed_bytes` for `verify_all`; `None` caps the
    /// entry only at its declared size.
    fn open_entry_inner(
        &mut self,
        index: usize,
        total_budget: Option<u64>,
    ) -> Result<ZipEntryReader<'_, R>, ArchiveError> {
        let Some(record) = self.entries.get(index) else {
            return Err(ArchiveError::new(
                ArchiveErrorKind::InvalidInput,
                format!(
                    "entry index {index} out of range ({} entries)",
                    self.entries.len()
                ),
            ));
        };
        if record.flags & FLAG_ENCRYPTED != 0 {
            return Err(ArchiveError::new(
                ArchiveErrorKind::Unsupported,
                format!("entry {index} is encrypted"),
            ));
        }
        if record.method != METHOD_STORED && record.method != METHOD_DEFLATE {
            return Err(ArchiveError::new(
                ArchiveErrorKind::Unsupported,
                format!("entry {index} uses compression method {}", record.method),
            ));
        }
        let declared = u64::from(record.uncompressed_size);
        if declared > self.limits.max_entry_uncompressed_bytes {
            return Err(limit_u64(
                "max_entry_uncompressed_bytes",
                declared,
                self.limits.max_entry_uncompressed_bytes,
            ));
        }
        let data_start =
            read_local_header(&mut self.inner, index, record, self.limits.max_path_bytes)?;
        let data_end = data_start.saturating_add(u64::from(record.compressed_size));
        if data_end > self.file_len {
            return Err(ArchiveError::new(
                ArchiveErrorKind::Truncated,
                format!("entry {index} data ends past end of input"),
            ));
        }
        if data_end > self.cd_offset {
            return Err(ArchiveError::new(
                ArchiveErrorKind::Overlap,
                format!("entry {index} data region reaches into the central directory"),
            ));
        }
        self.inner
            .seek(SeekFrom::Start(data_start))
            .map_err(ArchiveError::from_io)?;
        let source = if record.method == METHOD_STORED {
            let take = (&mut self.inner).take(u64::from(record.compressed_size));
            EntrySource::Stored(take)
        } else {
            let take = (&mut self.inner).take(u64::from(record.compressed_size));
            EntrySource::Deflate(DeflateDecoder::new(take))
        };
        let mut hard_cap = declared;
        if let Some(remaining) = total_budget {
            hard_cap = hard_cap.min(remaining);
        }
        Ok(ZipEntryReader {
            info: entry_info(index, record),
            source,
            verified: &mut self.verified[index],
            produced: 0,
            hard_cap,
            declared,
            data_start,
            data_end,
            expected_crc: record.crc32,
            crc: crc32fast::Hasher::new(),
            status: EntryStatus::Active,
        })
    }
}

/// Reads and validates the local file header of entry `index` against
/// its central-directory record, returning the absolute offset where
/// the entry's compressed data begins.
fn read_local_header<R: Read + Seek>(
    inner: &mut R,
    index: usize,
    record: &CdRecord,
    max_path_bytes: u32,
) -> Result<u64, ArchiveError> {
    inner
        .seek(SeekFrom::Start(u64::from(record.local_offset)))
        .map_err(ArchiveError::from_io)?;
    let mut fixed = [0_u8; LFH_LEN];
    inner
        .read_exact(&mut fixed)
        .map_err(|e| map_exact_error("local file header", e))?;
    if u32le(&fixed[0..4]) != LFH_SIG {
        return Err(ArchiveError::new(
            ArchiveErrorKind::Malformed,
            format!(
                "entry {index} local header at offset {} has no local signature",
                record.local_offset
            ),
        ));
    }
    let flags = u16le(&fixed[6..8]);
    let method = u16le(&fixed[8..10]);
    let crc32 = u32le(&fixed[14..18]);
    let compressed_size = u32le(&fixed[18..22]);
    let uncompressed_size = u32le(&fixed[22..26]);
    let name_len = u16le(&fixed[26..28]);
    let extra_len = u16le(&fixed[28..30]);
    if flags & FLAG_ENCRYPTED != 0 {
        return Err(ArchiveError::new(
            ArchiveErrorKind::Unsupported,
            format!("entry {index} local header declares encryption"),
        ));
    }
    if compressed_size == u32::MAX || uncompressed_size == u32::MAX {
        return Err(ArchiveError::new(
            ArchiveErrorKind::Unsupported,
            format!("entry {index} local header requires ZIP64 sizes"),
        ));
    }
    if u32::from(name_len) > max_path_bytes {
        return Err(limit("max_path_bytes", name_len, max_path_bytes));
    }
    if (flags & FLAG_DATA_DESCRIPTOR) != (record.flags & FLAG_DATA_DESCRIPTOR) {
        return Err(ArchiveError::new(
            ArchiveErrorKind::Inconsistent,
            format!("entry {index} data-descriptor flag differs between local header and CD"),
        ));
    }
    if method != record.method {
        return Err(ArchiveError::new(
            ArchiveErrorKind::Inconsistent,
            format!(
                "entry {index} method differs: local {method} vs CD {}",
                record.method
            ),
        ));
    }
    let mut variable = vec![0_u8; usize::from(name_len) + usize::from(extra_len)];
    inner
        .read_exact(&mut variable)
        .map_err(|e| map_exact_error("local header variable fields", e))?;
    let name = &variable[..usize::from(name_len)];
    if name != record.name.as_slice() {
        return Err(ArchiveError::new(
            ArchiveErrorKind::Inconsistent,
            format!("entry {index} name differs between local header and CD"),
        ));
    }
    // With the data-descriptor flag clear, local sizes/CRC must
    // match the CD exactly; with it set they may be zero (the real
    // values follow the data), but non-zero values must agree.
    let descriptor = flags & FLAG_DATA_DESCRIPTOR != 0;
    let local_values = [
        (crc32, record.crc32),
        (compressed_size, record.compressed_size),
        (uncompressed_size, record.uncompressed_size),
    ];
    for (local, cd) in local_values {
        let mismatch = if descriptor {
            local != 0 && local != cd
        } else {
            local != cd
        };
        if mismatch {
            return Err(ArchiveError::new(
                ArchiveErrorKind::Inconsistent,
                format!("entry {index} local size/CRC fields contradict the CD"),
            ));
        }
    }
    Ok(
        u64::from(record.local_offset)
            + LFH_LEN as u64
            + u64::from(name_len)
            + u64::from(extra_len),
    )
}

/// Builds the public [`ArchiveEntryInfo`] view of a CD record.
fn entry_info(index: usize, record: &CdRecord) -> ArchiveEntryInfo {
    let kind = classify_kind(record);
    ArchiveEntryInfo::new(
        index,
        ArchiveName::new(record.name.clone()),
        kind,
        Some(u64::from(record.uncompressed_size)),
        Some(u64::from(record.compressed_size)),
    )
}

/// ZIP entry-kind classification: unix `S_IFLNK`/`S_IFDIR` in the high
/// external attrs → symlink/directory; trailing `/` or DOS dir bit →
/// directory; a non-regular unix file type → `Special`; otherwise a
/// regular file.
fn classify_kind(record: &CdRecord) -> EntryKind {
    const S_IFMT: u32 = 0o170_000;
    const S_IFLNK: u32 = 0o120_000;
    const S_IFDIR: u32 = 0o040_000;
    const S_IFREG: u32 = 0o100_000;
    let file_type = (record.external_attrs >> 16) & S_IFMT;
    if file_type == S_IFLNK {
        return EntryKind::SymbolicLink;
    }
    if file_type == S_IFDIR
        || record.name.last() == Some(&b'/')
        || record.external_attrs & 0x10 != 0
    {
        return EntryKind::Directory;
    }
    if record.external_attrs >> 16 != 0 && file_type != S_IFREG {
        return EntryKind::Special;
    }
    EntryKind::File
}

/// Finds the end-of-central-directory record: reads the bounded tail
/// window (≤ ~64 KiB) and scans backwards for a signature whose record
/// length lands exactly on EOF.
fn locate_eocd<R: Read + Seek>(inner: &mut R, file_len: u64) -> Result<Eocd, ArchiveError> {
    let span = file_len.min(EOCD_SEARCH_SPAN);
    let tail_start = file_len - span;
    inner
        .seek(SeekFrom::Start(tail_start))
        .map_err(ArchiveError::from_io)?;
    let mut tail = vec![0_u8; usize::try_from(span).unwrap_or(usize::MAX)];
    inner.read_exact(&mut tail).map_err(ArchiveError::from_io)?;
    let mut pos = tail.len().saturating_sub(EOCD_LEN);
    loop {
        if u32le(&tail[pos..pos + 4]) == EOCD_SIG {
            let comment_len = usize::from(u16le(&tail[pos + 20..pos + 22]));
            if pos + EOCD_LEN + comment_len == tail.len() {
                return Ok(Eocd::parse(
                    &tail[pos..pos + EOCD_LEN],
                    tail_start + pos as u64,
                ));
            }
        }
        if pos == 0 {
            break;
        }
        pos -= 1;
    }
    Err(ArchiveError::new(
        ArchiveErrorKind::Truncated,
        "no end-of-central-directory record found at the end of input",
    ))
}

/// Parses every central-directory record inside the (already
/// budget-bounded) `cd_size` region. Per-record name lengths are
/// checked against `max_path_bytes` *before* each name allocation and
/// `max_entries` is re-enforced mid-parse.
fn parse_central_directory<R: Read + Seek>(
    inner: &mut R,
    cd_offset: u64,
    cd_size: u64,
    limits: &ArchiveReadLimits,
) -> Result<Vec<CdRecord>, ArchiveError> {
    inner
        .seek(SeekFrom::Start(cd_offset))
        .map_err(ArchiveError::from_io)?;
    let mut cd = vec![0_u8; usize::try_from(cd_size).unwrap_or(usize::MAX)];
    inner
        .read_exact(&mut cd)
        .map_err(|e| map_exact_error("central directory", e))?;
    let mut records = Vec::new();
    let mut pos = 0_usize;
    let cd_len = cd.len();
    while pos < cd_len {
        if pos + CD_RECORD_LEN > cd_len {
            return Err(ArchiveError::new(
                ArchiveErrorKind::Truncated,
                "central directory record extends past the declared directory size",
            ));
        }
        let fixed = &cd[pos..pos + CD_RECORD_LEN];
        if u32le(&fixed[0..4]) != CD_SIG {
            return Err(ArchiveError::new(
                ArchiveErrorKind::Malformed,
                format!("central directory record at offset {pos} has no CD signature"),
            ));
        }
        let flags = u16le(&fixed[8..10]);
        let method = u16le(&fixed[10..12]);
        let crc32 = u32le(&fixed[16..20]);
        let compressed_size = u32le(&fixed[20..24]);
        let uncompressed_size = u32le(&fixed[24..28]);
        let name_len = u16le(&fixed[28..30]);
        let extra_len = u16le(&fixed[30..32]);
        let comment_len = u16le(&fixed[32..34]);
        let disk_start = u16le(&fixed[34..36]);
        let external_attrs = u32le(&fixed[38..42]);
        let local_offset = u32le(&fixed[42..46]);
        if disk_start != 0 {
            return Err(ArchiveError::new(
                ArchiveErrorKind::Unsupported,
                "entry starts on a non-zero disk (multi-disk archive)",
            ));
        }
        if compressed_size == u32::MAX || uncompressed_size == u32::MAX || local_offset == u32::MAX
        {
            return Err(ArchiveError::new(
                ArchiveErrorKind::Unsupported,
                "central directory record requires ZIP64 fields",
            ));
        }
        if u32::from(name_len) > limits.max_path_bytes {
            return Err(limit("max_path_bytes", name_len, limits.max_path_bytes));
        }
        let rec_len = CD_RECORD_LEN
            .saturating_add(usize::from(name_len))
            .saturating_add(usize::from(extra_len))
            .saturating_add(usize::from(comment_len));
        if pos + rec_len > cd_len {
            return Err(ArchiveError::new(
                ArchiveErrorKind::Truncated,
                "central directory record's variable fields extend past the declared size",
            ));
        }
        records.push(CdRecord {
            flags,
            method,
            crc32,
            compressed_size,
            uncompressed_size,
            local_offset,
            external_attrs,
            name: cd[pos + CD_RECORD_LEN..pos + CD_RECORD_LEN + usize::from(name_len)].to_vec(),
        });
        if records.len() > usize::try_from(limits.max_entries).unwrap_or(usize::MAX) {
            return Err(ArchiveError::new(
                ArchiveErrorKind::LimitExceeded,
                format!(
                    "central directory contains more than {} entries",
                    limits.max_entries
                ),
            ));
        }
        pos += rec_len;
    }
    Ok(records)
}

/// Open-time structural audit: each entry's lower-bound region
/// `[local_offset, local_offset + LFH_LEN + compressed_size)` must end
/// at or before the CD region, and regions must be pairwise disjoint.
/// The true data end also depends on the local variable fields, so the
/// precise check repeats at `open_entry`/`verify_all`.
fn audit_index_overlap(entries: &[CdRecord], cd_offset: u64) -> Result<(), ArchiveError> {
    let mut regions: Vec<(u64, u64)> = entries
        .iter()
        .map(|r| {
            (
                u64::from(r.local_offset),
                u64::from(r.local_offset) + LFH_LEN as u64 + u64::from(r.compressed_size),
            )
        })
        .collect();
    for &(start, end) in &regions {
        if end > cd_offset {
            return Err(ArchiveError::new(
                ArchiveErrorKind::Overlap,
                format!(
                    "entry region {start}..{end} reaches into the central directory at {cd_offset}"
                ),
            ));
        }
    }
    regions.sort_unstable();
    for pair in regions.windows(2) {
        if pair[0].1 > pair[1].0 {
            return Err(ArchiveError::new(
                ArchiveErrorKind::Overlap,
                format!(
                    "entry regions overlap ({}..{} vs {}..{})",
                    pair[0].0, pair[0].1, pair[1].0, pair[1].1
                ),
            ));
        }
    }
    Ok(())
}

fn map_exact_error(context: &'static str, error: std::io::Error) -> ArchiveError {
    if error.kind() == std::io::ErrorKind::UnexpectedEof {
        ArchiveError::with_source(
            ArchiveErrorKind::Truncated,
            format!("input ended while reading {context}"),
            error,
        )
    } else {
        ArchiveError::with_source(
            ArchiveErrorKind::Io,
            format!("I/O failure while reading {context}"),
            error,
        )
    }
}

fn limit(name: &'static str, actual: u16, max: u32) -> ArchiveError {
    ArchiveError::new(
        ArchiveErrorKind::LimitExceeded,
        format!("{name}: declared {actual} exceeds limit {max}"),
    )
}

fn limit_u64(name: &'static str, actual: u64, max: u64) -> ArchiveError {
    ArchiveError::new(
        ArchiveErrorKind::LimitExceeded,
        format!("{name}: declared {actual} exceeds limit {max}"),
    )
}

/// Which bound capped the entry's uncompressed output — decides whether
/// an overflow at the cap is a `SizeMismatch` (declared size) or a
/// `LimitExceeded` (budget).
#[derive(Clone, Copy)]
enum EntryStatus {
    Active,
    /// Underlying source ended cleanly at or below the cap.
    Exhausted,
    Failed(ArchiveErrorKind),
}

/// The entry's byte source: raw `Take` for stored, inflate for deflate.
/// Both are bounded to the CD-declared compressed size.
enum EntrySource<'a, R: Read + Seek> {
    Stored(Take<&'a mut R>),
    Deflate(DeflateDecoder<Take<&'a mut R>>),
}

impl<R: Read + Seek> Read for EntrySource<'_, R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::Stored(take) => take.read(buf),
            Self::Deflate(decoder) => decoder.read(buf),
        }
    }
}

/// Exclusive-borrow reader for one ZIP entry's uncompressed payload.
///
/// Created by [`ZipArchiveReader::open_entry`]. Implements [`Read`];
/// output is capped at the declared uncompressed size (and any outer
/// budget) — bytes beyond the cap are never yielded. `read` returning
/// `Ok(0)` does **not** prove verification: only
/// [`finish`](Self::finish) confirms the measured size and CRC32 and
/// marks the entry verified. Dropping without `finish` leaves the entry
/// unverified.
///
/// **RT contract:** non-RT only.
pub struct ZipEntryReader<'a, R: Read + Seek> {
    info: ArchiveEntryInfo,
    source: EntrySource<'a, R>,
    /// Slot inside `ZipArchiveReader::verified` — set on `finish`.
    verified: &'a mut bool,
    produced: u64,
    /// Output cap: `min(declared, remaining budget)`.
    hard_cap: u64,
    declared: u64,
    data_start: u64,
    data_end: u64,
    expected_crc: u32,
    crc: crc32fast::Hasher,
    status: EntryStatus,
}

impl<R: Read + Seek> ZipEntryReader<'_, R> {
    /// The entry's metadata (declared sizes as stored in the CD).
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn info(&self) -> &ArchiveEntryInfo {
        &self.info
    }

    /// Entry's data region in the input (`verify_all` overlap audit).
    pub(crate) fn data_range(&self) -> (u64, u64) {
        (self.data_start, self.data_end)
    }

    /// Reads any remaining payload to its verified end and confirms the
    /// measured size and CRC32 against the central directory. Consumes
    /// the reader and marks the entry verified on success.
    ///
    /// Draining from a partially-read position is allowed: `finish`
    /// still verifies the whole entry (the CRC covers bytes the caller
    /// did not individually consume).
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// `ChecksumMismatch`, `SizeMismatch`, `LimitExceeded`, or the read
    /// failure already reported (`Malformed`/`Io`/`Truncated`).
    pub fn finish(mut self) -> Result<EntryReadReceipt, ArchiveError> {
        let mut scratch = [0_u8; 8192];
        while let Ok(n) = self.read(&mut scratch) {
            if n == 0 {
                break;
            }
        }
        if let EntryStatus::Failed(kind) = self.status {
            return Err(ArchiveError::new(
                kind,
                format!(
                    "entry {} ({}) failed verification",
                    self.info.index(),
                    self.info.name().to_string_lossy()
                ),
            ));
        }
        if self.produced < self.declared {
            return Err(ArchiveError::new(
                ArchiveErrorKind::SizeMismatch,
                format!(
                    "entry {} produced {} bytes, declared {}",
                    self.info.index(),
                    self.produced,
                    self.declared
                ),
            ));
        }
        if self.crc.clone().finalize() != self.expected_crc {
            return Err(ArchiveError::new(
                ArchiveErrorKind::ChecksumMismatch,
                format!(
                    "entry {} ({}) CRC32 does not match the declared value",
                    self.info.index(),
                    self.info.name().to_string_lossy()
                ),
            ));
        }
        *self.verified = true;
        Ok(EntryReadReceipt::new(
            self.info.clone(),
            self.produced,
            EntryChecksum::Verified,
        ))
    }
}

impl<R: Read + Seek> Read for ZipEntryReader<'_, R> {
    /// Reads uncompressed payload bytes, capped at the declared size /
    /// remaining budget. Excess output is never yielded — reaching the
    /// cap probes the source once and reports the classified error on
    /// overflow.
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        match self.status {
            EntryStatus::Exhausted => return Ok(0),
            EntryStatus::Failed(kind) => {
                return Err(std::io::Error::from(ArchiveError::new(
                    kind,
                    format!("entry {} read failure", self.info.index()),
                )));
            }
            EntryStatus::Active => {}
        }
        if self.produced >= self.hard_cap {
            // At the cap: a well-formed entry ends exactly here. Probe
            // once for excess output (over-declaration or budget
            // breach); bytes beyond the cap are never yielded.
            match self.probe() {
                Ok(()) => {
                    self.status = EntryStatus::Exhausted;
                    return Ok(0);
                }
                Err(kind) => {
                    self.status = EntryStatus::Failed(kind);
                    return Err(std::io::Error::from(ArchiveError::new(
                        kind,
                        format!("entry {} output exceeds its bound", self.info.index()),
                    )));
                }
            }
        }
        let remaining = self.hard_cap - self.produced;
        let want = usize::try_from(remaining)
            .unwrap_or(usize::MAX)
            .min(buf.len());
        match self.source.read(&mut buf[..want]) {
            Ok(0) => {
                // Source ended at/below the cap — could be a clean end
                // (produced == declared) or a short entry
                // (produced < declared; `finish` reports SizeMismatch).
                self.status = EntryStatus::Exhausted;
                Ok(0)
            }
            Ok(n) => {
                self.produced += n as u64;
                self.crc.update(&buf[..n]);
                Ok(n)
            }
            Err(error) => {
                let kind = classify_source_error(&self.source, &error);
                self.status = EntryStatus::Failed(kind);
                Err(std::io::Error::from(ArchiveError::with_source(
                    kind,
                    format!("entry {} payload read failed", self.info.index()),
                    error,
                )))
            }
        }
    }
}

impl<R: Read + Seek> ZipEntryReader<'_, R> {
    /// One-byte probe at the output cap: `Ok` means the source is also
    /// exhausted (clean end); `Err(kind)` reports the overflow's
    /// classification.
    fn probe(&mut self) -> Result<(), ArchiveErrorKind> {
        let mut scratch = [0_u8; 1];
        match self.source.read(&mut scratch) {
            Ok(0) => Ok(()),
            Ok(_) => {
                if self.hard_cap < self.declared {
                    Err(ArchiveErrorKind::LimitExceeded)
                } else {
                    Err(ArchiveErrorKind::SizeMismatch)
                }
            }
            Err(error) => Err(classify_source_error(&self.source, &error)),
        }
    }
}

/// Maps a source-stream error to a normalized kind: unexpected EOF is
/// truncation, deflate stream corruption is malformed input, anything
/// else is an underlying I/O failure.
fn classify_source_error<R: Read + Seek>(
    source: &EntrySource<'_, R>,
    error: &std::io::Error,
) -> ArchiveErrorKind {
    use std::io::ErrorKind;
    match error.kind() {
        ErrorKind::UnexpectedEof => ArchiveErrorKind::Truncated,
        ErrorKind::InvalidData | ErrorKind::InvalidInput => match source {
            EntrySource::Deflate(_) => ArchiveErrorKind::Malformed,
            EntrySource::Stored(_) => ArchiveErrorKind::Io,
        },
        _ => ArchiveErrorKind::Io,
    }
}
