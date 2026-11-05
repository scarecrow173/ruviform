//! TAR streaming reader: sequential entry parsing with the
//! `Ready`/`EntryOpen`/`Pending`/`Failed` state machine, explicit
//! discard recovery, budgets enforced before and during parsing, and
//! end-of-archive verification.
//!
//! TAR is a sequential format — there is no index and no random access.
//! [`TarArchiveReader::next_entry`] walks 512-byte header blocks;
//! PAX (`x`/`g`/`X`) and GNU (`L`/`K`) extension entries are consumed
//! internally and folded into the following real entry's effective
//! name/link target/size.
//!
//! Dropping a [`TarEntryReader`] mid-read moves the archive to
//! `Pending`: `next_entry` and [`TarArchiveReader::finish`] are then
//! rejected with `InvalidState` until [`TarArchiveReader::discard_pending`]
//! drains the remainder within budget. Structural/I/O failures move the
//! reader to `Failed` — recovery is impossible because the stream
//! position can no longer be trusted.

use std::io::Read;

use crate::entry::{ArchiveEntryInfo, ArchiveName, EntryKind};
use crate::error::{ArchiveError, ArchiveErrorKind};
use crate::limits::ArchiveReadLimits;
use crate::receipt::{ArchiveReadReceipt, EntryChecksum, EntryDisposition, EntryReadReceipt};
use crate::tar::header::{
    self, BLOCK, Header, HeaderError, TYPE_CONTIGUOUS, TYPE_DIRECTORY, TYPE_GNU_LONGLINK,
    TYPE_GNU_LONGNAME, TYPE_HARDLINK, TYPE_PAX, TYPE_PAX_GLOBAL, TYPE_PAX_SOLARIS, TYPE_REGULAR,
    TYPE_REGULAR_ALT, TYPE_SYMLINK,
};

/// Internal state of the streaming reader.
enum ReaderState {
    /// Positioned at the next header block.
    Ready,
    /// A [`TarEntryReader`] was dropped mid-read: `skip` payload bytes
    /// plus `pad` padding bytes must be drained by `discard_pending`
    /// before the stream can continue.
    Pending { skip: u64, pad: u64 },
    /// The first all-zero end-of-archive block was consumed.
    EndSeen,
    /// An unrecoverable structural/I/O failure occurred.
    Failed,
}

/// Extension attributes accumulated while walking `x`/`g`/`L`/`K`
/// records before a real entry header.
#[derive(Default)]
struct PendingExtensions {
    pax_path: Option<Vec<u8>>,
    pax_linkpath: Option<Vec<u8>>,
    pax_size: Option<u64>,
    gnu_name: Option<Vec<u8>>,
    gnu_link: Option<Vec<u8>>,
}

/// A surfaced entry: everything needed to hand a [`TarEntryReader`]
/// its bounds without holding a borrow on the archive.
struct ParsedEntry {
    info: ArchiveEntryInfo,
    declared: u64,
}

/// Streaming reader for a `Read` TAR byte stream.
///
/// [`next_entry`](Self::next_entry) parses the next header (folding any
/// preceding PAX/GNU extensions into the effective fields) and returns
/// an exclusive-borrow [`TarEntryReader`]; `Ok(None)` means the
/// end-of-archive marker was reached. [`finish`](Self::finish) drains
/// and structurally verifies every remaining entry, then checks the
/// end-of-archive marker and that the rest of the stream is all zeros —
/// it is the only archive-level completion proof.
///
/// **TAR payloads carry no checksum.** A finished entry proves the
/// declared end and padding were reached — never that the content is
/// uncorrupted (`EntryChecksum::NotPresent`). A declared size that lies
/// desynchronizes the stream and surfaces as a classified error at the
/// next header.
///
/// **`tar.gz`/`tar.zst` note:** when `R` is a decompression decoder,
/// `finish` verifies only the TAR-level end inside the decoded stream.
/// The compressed stream's own trailer/CRC/EOF is the compression
/// layer's separate `finish` contract (IO-03) — a successful TAR
/// `finish` never proves the compressed stream verified.
///
/// **RT contract:** non-RT only.
///
/// # Examples
///
/// ```no_run
/// use ruviform_io_archive::{ArchiveReadLimits, tar::TarArchiveReader};
/// use std::io::Read;
///
/// let file = std::fs::File::open("bundle.tar")?;
/// let mut archive = TarArchiveReader::new(file, ArchiveReadLimits::default());
/// while let Some(mut entry) = archive.next_entry()? {
///     let mut buf = Vec::new();
///     entry.read_to_end(&mut buf)?;
///     let receipt = entry.finish()?;
///     println!("{}: {} bytes", receipt.info().name(), receipt.uncompressed_bytes());
/// }
/// let (_file, receipt) = archive.finish()?;
/// assert_eq!(receipt.entries_unverified(), 0);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct TarArchiveReader<R: Read> {
    inner: R,
    limits: ArchiveReadLimits,
    state: ReaderState,
    pending_ext: PendingExtensions,
    entries_total: u32,
    entries_verified: u32,
    entries_unverified: u32,
    /// Measured uncompressed bytes consumed (read or discarded).
    payload_bytes: u64,
    /// Header blocks + extension payload bytes consumed.
    metadata_bytes: u64,
}

impl<R: Read> TarArchiveReader<R> {
    /// Wraps a byte stream. No I/O happens here — headers are parsed
    /// lazily by [`next_entry`](Self::next_entry) and
    /// [`finish`](Self::finish) under `limits`.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn new(inner: R, limits: ArchiveReadLimits) -> Self {
        Self {
            inner,
            limits,
            state: ReaderState::Ready,
            pending_ext: PendingExtensions::default(),
            entries_total: 0,
            entries_verified: 0,
            entries_unverified: 0,
            payload_bytes: 0,
            metadata_bytes: 0,
        }
    }

    /// Parses the next entry header and returns an exclusive-borrow
    /// reader for its payload. `Ok(None)` means the end-of-archive
    /// marker was reached (call [`finish`](Self::finish) to verify the
    /// remaining stream).
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// `InvalidState` (a dropped entry is pending recovery, or the
    /// reader previously failed), `Truncated`, `Malformed`,
    /// `ChecksumMismatch`, `LimitExceeded`, `Unsupported` (sparse,
    /// special, or unknown typeflags), `Io`. Any error other than
    /// `InvalidState` moves the reader to `Failed`.
    pub fn next_entry(&mut self) -> Result<Option<TarEntryReader<'_, R>>, ArchiveError> {
        match self.state {
            ReaderState::Ready => {}
            ReaderState::EndSeen => return Ok(None),
            ReaderState::Pending { .. } => {
                return Err(ArchiveError::new(
                    ArchiveErrorKind::InvalidState,
                    "an entry payload is pending; call discard_pending before the next entry",
                ));
            }
            ReaderState::Failed => {
                return Err(ArchiveError::new(
                    ArchiveErrorKind::InvalidState,
                    "reader failed on a previous entry",
                ));
            }
        }
        let Some(parsed) = self.advance()? else {
            return Ok(None);
        };
        Ok(Some(TarEntryReader {
            archive: self,
            info: parsed.info,
            declared: parsed.declared,
            produced: 0,
            status: EntryStatus::Active,
            resolved: false,
        }))
    }

    /// Number of entries surfaced so far.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn entry_count(&self) -> usize {
        usize::try_from(self.entries_total).unwrap_or(usize::MAX)
    }

    /// `true` while a dropped entry's payload is pending — recovery via
    /// [`discard_pending`](Self::discard_pending) is required before
    /// `next_entry`/`finish`.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn has_pending_entry(&self) -> bool {
        matches!(self.state, ReaderState::Pending { .. })
    }

    /// Drains the pending entry's remaining payload and padding within
    /// budget, then returns to `Ready`. This is the only recovery path
    /// out of `Pending`.
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// `InvalidState` (not in `Pending`), `LimitExceeded` (the drain
    /// exceeds the total budget — the reader then stays `Failed`),
    /// `Malformed` (non-zero padding; the reader still returns to
    /// `Ready`), `Truncated`/`Io` (drain failed — `Failed`).
    pub fn discard_pending(&mut self) -> Result<(), ArchiveError> {
        let ReaderState::Pending { skip, pad } = self.state else {
            return Err(ArchiveError::new(
                ArchiveErrorKind::InvalidState,
                "no pending entry to discard",
            ));
        };
        self.drain_bytes(skip)?;
        match self.verify_padding(pad) {
            // The padding region was consumed either way: position is
            // committed, so Malformed is reported with the reader back
            // at Ready. Io/Truncated already failed it via fail().
            Ok(()) => {
                self.state = ReaderState::Ready;
                Ok(())
            }
            Err(error) if error.kind() == ArchiveErrorKind::Malformed => {
                self.state = ReaderState::Ready;
                Err(error)
            }
            Err(error) => Err(error),
        }
    }

    /// Verifies the rest of the archive and returns the inner stream
    /// plus the verification receipt.
    ///
    /// Any entries not yet surfaced are drained and **structurally**
    /// verified (headers, padding, budgets) and counted
    /// `entries_unverified` — their content is never delivered to the
    /// caller. After the end-of-archive marker, every remaining byte
    /// must be zero. A lone zero block followed by EOF is accepted;
    /// non-zero trailing data is `Malformed`.
    ///
    /// Rejected while `Pending` or `Failed` — drain the pending entry
    /// with [`discard_pending`](Self::discard_pending) first.
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// `InvalidState` (pending/failed), `Truncated`, `Malformed`
    /// (trailing garbage or a bad remaining header/padding),
    /// `ChecksumMismatch`, `LimitExceeded`, `Unsupported`, `Io`.
    pub fn finish(mut self) -> Result<(R, ArchiveReadReceipt), ArchiveError> {
        match self.state {
            ReaderState::Pending { .. } => {
                return Err(ArchiveError::new(
                    ArchiveErrorKind::InvalidState,
                    "an entry payload is pending; call discard_pending before finish",
                ));
            }
            ReaderState::Failed => {
                return Err(ArchiveError::new(
                    ArchiveErrorKind::InvalidState,
                    "reader failed on a previous entry",
                ));
            }
            ReaderState::Ready | ReaderState::EndSeen => {}
        }
        while let Some(parsed) = self.advance()? {
            self.drain_bytes(parsed.declared)?;
            self.verify_padding(header::pad_len(parsed.declared))?;
            self.entries_unverified += 1;
        }
        // state == EndSeen: the rest of the stream must be all zeros
        // (the second end block and any record padding).
        let mut buf = [0_u8; 8192];
        loop {
            let n = self.inner.read(&mut buf).map_err(ArchiveError::from_io)?;
            if n == 0 {
                break;
            }
            if buf[..n].iter().any(|&b| b != 0) {
                return Err(ArchiveError::new(
                    ArchiveErrorKind::Malformed,
                    "non-zero trailing data after the end-of-archive marker",
                ));
            }
        }
        let receipt = ArchiveReadReceipt::new(
            self.entries_total,
            self.entries_verified,
            self.entries_unverified,
            self.payload_bytes,
        );
        Ok((self.inner, receipt))
    }

    /// Returns the inner stream without verifying the archive's end.
    /// Bytes consumed so far are simply abandoned mid-stream.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn into_inner(self) -> R {
        self.inner
    }

    /// Marks the reader `Failed` and returns the error — every
    /// structural failure inside `advance`/`drain` funnels through here
    /// so the stream position is never trusted again.
    fn fail<T>(&mut self, error: ArchiveError) -> Result<T, ArchiveError> {
        self.state = ReaderState::Failed;
        Err(error)
    }

    /// Walks header blocks until a real entry surfaces (`Ok(Some)`),
    /// the end-of-archive marker appears (`Ok(None)`, `EndSeen`), or a
    /// failure moves the reader to `Failed`. Extension entries
    /// (`x`/`g`/`X`/`L`/`K`) are consumed and folded into
    /// `pending_ext`.
    fn advance(&mut self) -> Result<Option<ParsedEntry>, ArchiveError> {
        if matches!(self.state, ReaderState::EndSeen) {
            return Ok(None);
        }
        loop {
            let mut block = [0_u8; BLOCK];
            if let Err(error) = self.read_block(&mut block) {
                return self.fail(error);
            }
            self.metadata_bytes += BLOCK as u64;
            if self.metadata_bytes > self.limits.max_metadata_bytes {
                return self.fail(limit_u64(
                    "max_metadata_bytes",
                    self.metadata_bytes,
                    self.limits.max_metadata_bytes,
                ));
            }
            if header::is_zero_block(&block) {
                self.state = ReaderState::EndSeen;
                return Ok(None);
            }
            let header = match Header::parse(&block) {
                Ok(header) => header,
                Err(HeaderError::Checksum) => {
                    return self.fail(ArchiveError::new(
                        ArchiveErrorKind::ChecksumMismatch,
                        "entry header checksum does not match the measured sum",
                    ));
                }
                Err(HeaderError::Malformed(detail)) => {
                    return self.fail(ArchiveError::new(
                        ArchiveErrorKind::Malformed,
                        format!("invalid tar header: {detail}"),
                    ));
                }
            };
            match header.typeflag {
                TYPE_PAX | TYPE_PAX_SOLARIS => {
                    let payload = match self.read_extension(&header) {
                        Ok(payload) => payload,
                        Err(error) => return self.fail(error),
                    };
                    if let Err(error) = self.apply_pax(&payload) {
                        return self.fail(error);
                    }
                }
                TYPE_PAX_GLOBAL => {
                    // Global records are grammar-validated and
                    // budgeted but never applied to effective fields —
                    // letting a global record rename/resize every entry
                    // is rejected by design (see crate docs).
                    let payload = match self.read_extension(&header) {
                        Ok(payload) => payload,
                        Err(error) => return self.fail(error),
                    };
                    if let Err(error) = validate_pax(&payload) {
                        return self.fail(error);
                    }
                }
                TYPE_GNU_LONGNAME => match self.read_extension(&header) {
                    Ok(payload) => self.pending_ext.gnu_name = Some(trim_nuls(&payload)),
                    Err(error) => return self.fail(error),
                },
                TYPE_GNU_LONGLINK => match self.read_extension(&header) {
                    Ok(payload) => self.pending_ext.gnu_link = Some(trim_nuls(&payload)),
                    Err(error) => return self.fail(error),
                },
                TYPE_REGULAR | TYPE_REGULAR_ALT | TYPE_CONTIGUOUS | TYPE_DIRECTORY
                | TYPE_HARDLINK | TYPE_SYMLINK => match self.surface(&header) {
                    Ok(parsed) => return Ok(Some(parsed)),
                    Err(error) => return self.fail(error),
                },
                other => {
                    return self.fail(ArchiveError::new(
                        ArchiveErrorKind::Unsupported,
                        format!("unsupported tar entry typeflag 0x{other:02x}"),
                    ));
                }
            }
        }
    }

    /// Reads one full 512-byte header block. Reaching EOF mid-block or
    /// with no block at all is `Truncated` (a TAR archive must end with
    /// the zero-block marker, never a bare EOF).
    fn read_block(&mut self, block: &mut [u8; BLOCK]) -> Result<(), ArchiveError> {
        let mut filled = 0_usize;
        while filled < BLOCK {
            match self.inner.read(&mut block[filled..]) {
                Ok(0) => break,
                Ok(n) => filled += n,
                Err(ref error) if error.kind() == std::io::ErrorKind::Interrupted => {}
                Err(error) => return Err(ArchiveError::from_io(error)),
            }
        }
        if filled == BLOCK {
            Ok(())
        } else if filled == 0 {
            Err(ArchiveError::new(
                ArchiveErrorKind::Truncated,
                "archive ended without an end-of-archive marker",
            ))
        } else {
            Err(ArchiveError::new(
                ArchiveErrorKind::Truncated,
                format!("entry header block cut after {filled} bytes"),
            ))
        }
    }

    /// Reads an extension entry's payload (PAX records or a GNU long
    /// name) plus its block padding. The declared size is checked
    /// against the remaining metadata budget **before** the payload is
    /// buffered.
    fn read_extension(&mut self, header: &Header<'_>) -> Result<Vec<u8>, ArchiveError> {
        let remaining = self
            .limits
            .max_metadata_bytes
            .saturating_sub(self.metadata_bytes);
        if header.size > remaining {
            return Err(limit_u64("max_metadata_bytes", header.size, remaining));
        }
        self.metadata_bytes += header.size;
        let len = usize::try_from(header.size).unwrap_or(usize::MAX);
        let mut payload = vec![0_u8; len];
        self.inner
            .read_exact(&mut payload)
            .map_err(|e| map_exact_error("extension payload", e))?;
        self.consume_padding(header::pad_len(header.size))?;
        Ok(payload)
    }

    /// Folds a parsed PAX `x`/`X` record set into `pending_ext`. Only
    /// `path`/`linkpath`/`size` affect the effective fields; other keys
    /// are parsed for grammar but not surfaced.
    fn apply_pax(&mut self, payload: &[u8]) -> Result<(), ArchiveError> {
        let records = parse_pax(payload)?;
        for record in records {
            match record.key {
                b"path" => self.pending_ext.pax_path = Some(record.value.to_vec()),
                b"linkpath" => self.pending_ext.pax_linkpath = Some(record.value.to_vec()),
                b"size" => {
                    let text = std::str::from_utf8(record.value).map_err(|_| {
                        ArchiveError::new(
                            ArchiveErrorKind::Malformed,
                            "pax size record is not utf-8 decimal",
                        )
                    })?;
                    self.pending_ext.pax_size = Some(text.trim().parse().map_err(|_| {
                        ArchiveError::new(
                            ArchiveErrorKind::Malformed,
                            "pax size record is not a decimal number",
                        )
                    })?);
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Resolves the effective name/link target/size for a real entry
    /// and enforces the per-entry budgets before handing it out.
    fn surface(&mut self, header: &Header<'_>) -> Result<ParsedEntry, ArchiveError> {
        let extensions = std::mem::take(&mut self.pending_ext);
        let name: Vec<u8> = if let Some(path) = extensions.pax_path {
            path
        } else if let Some(name) = extensions.gnu_name {
            name
        } else if header.prefix.is_empty() {
            header.name.to_vec()
        } else {
            let mut joined = Vec::with_capacity(header.prefix.len() + 1 + header.name.len());
            joined.extend_from_slice(header.prefix);
            joined.push(b'/');
            joined.extend_from_slice(header.name);
            joined
        };
        let link: Option<Vec<u8>> = match header.typeflag {
            TYPE_HARDLINK | TYPE_SYMLINK => {
                extensions.pax_linkpath.or(extensions.gnu_link).or_else(|| {
                    if header.linkname.is_empty() {
                        None
                    } else {
                        Some(header.linkname.to_vec())
                    }
                })
            }
            _ => None,
        };
        let declared = extensions.pax_size.unwrap_or(header.size);
        if name.is_empty() {
            return Err(ArchiveError::new(
                ArchiveErrorKind::Malformed,
                "entry has an empty effective name",
            ));
        }
        if name.len() > usize::try_from(self.limits.max_path_bytes).unwrap_or(usize::MAX) {
            return Err(limit_u64(
                "max_path_bytes",
                name.len() as u64,
                u64::from(self.limits.max_path_bytes),
            ));
        }
        if let Some(link) = &link
            && link.len() > usize::try_from(self.limits.max_path_bytes).unwrap_or(usize::MAX)
        {
            return Err(limit_u64(
                "max_path_bytes",
                link.len() as u64,
                u64::from(self.limits.max_path_bytes),
            ));
        }
        if declared > self.limits.max_entry_uncompressed_bytes {
            return Err(limit_u64(
                "max_entry_uncompressed_bytes",
                declared,
                self.limits.max_entry_uncompressed_bytes,
            ));
        }
        if self.entries_total >= self.limits.max_entries {
            return Err(limit_u64(
                "max_entries",
                u64::from(self.entries_total) + 1,
                u64::from(self.limits.max_entries),
            ));
        }
        self.entries_total += 1;
        let kind = match header.typeflag {
            TYPE_DIRECTORY => EntryKind::Directory,
            TYPE_HARDLINK => EntryKind::HardLink,
            TYPE_SYMLINK => EntryKind::SymbolicLink,
            _ => EntryKind::File,
        };
        let index = self.entries_total as usize - 1;
        Ok(ParsedEntry {
            info: ArchiveEntryInfo::new(
                index,
                ArchiveName::new(name),
                kind,
                link.map(ArchiveName::new),
                Some(declared),
                None,
            ),
            declared,
        })
    }

    /// Drains `count` payload bytes through a bounded scratch buffer,
    /// accumulating them into `payload_bytes` and enforcing
    /// `max_total_uncompressed_bytes` mid-drain. Failures are
    /// unrecoverable (the stream position can no longer be trusted).
    fn drain_bytes(&mut self, mut count: u64) -> Result<(), ArchiveError> {
        let mut scratch = [0_u8; 8192];
        while count > 0 {
            let remaining = self
                .limits
                .max_total_uncompressed_bytes
                .saturating_sub(self.payload_bytes);
            if remaining == 0 {
                return self.fail(limit_u64(
                    "max_total_uncompressed_bytes",
                    self.payload_bytes + count,
                    self.limits.max_total_uncompressed_bytes,
                ));
            }
            let want = usize::try_from(count.min(remaining).min(8192)).unwrap_or(8192);
            if let Err(error) = self
                .inner
                .read_exact(&mut scratch[..want])
                .map_err(|e| map_exact_error("entry payload", e))
            {
                return self.fail(error);
            }
            self.payload_bytes += want as u64;
            count -= want as u64;
        }
        Ok(())
    }

    /// Consumes `pad` bytes after a payload and verifies they are all
    /// zero. A non-zero pad is `Malformed` but leaves the stream
    /// correctly positioned — callers decide whether it fails the
    /// archive. I/O failures mid-read are also reported to the caller,
    /// which must then treat the archive as `Failed`.
    fn consume_padding(&mut self, pad: u64) -> Result<(), ArchiveError> {
        let mut pad = usize::try_from(pad).unwrap_or(usize::MAX);
        let mut scratch = [0_u8; BLOCK];
        while pad > 0 {
            let want = pad.min(BLOCK);
            self.inner
                .read_exact(&mut scratch[..want])
                .map_err(|e| map_exact_error("entry padding", e))?;
            if scratch[..want].iter().any(|&b| b != 0) {
                return Err(ArchiveError::new(
                    ArchiveErrorKind::Malformed,
                    "non-zero padding bytes after entry payload",
                ));
            }
            pad -= want;
        }
        Ok(())
    }

    /// Entry-close padding check: `Malformed` (non-zero pad) leaves the
    /// stream positioned at the next header and does not fail the
    /// archive; any other error kills it.
    fn verify_padding(&mut self, pad: u64) -> Result<(), ArchiveError> {
        match self.consume_padding(pad) {
            Err(error) if error.kind() != ArchiveErrorKind::Malformed => self.fail(error),
            other => other,
        }
    }
}

/// Parses a PAX record payload, classifying grammar failures.
fn parse_pax(payload: &[u8]) -> Result<Vec<header::PaxRecord<'_>>, ArchiveError> {
    header::parse_pax_records(payload).map_err(|detail| {
        ArchiveError::new(
            ArchiveErrorKind::Malformed,
            format!("invalid pax extension payload: {detail}"),
        )
    })
}

/// Validates a PAX `g` (global) payload's grammar without applying it.
fn validate_pax(payload: &[u8]) -> Result<(), ArchiveError> {
    parse_pax(payload).map(|_| ())
}

/// Strips trailing NUL bytes from a GNU long-name/long-link payload.
fn trim_nuls(payload: &[u8]) -> Vec<u8> {
    let end = payload
        .iter()
        .rposition(|&b| b != 0)
        .map_or(0, |pos| pos + 1);
    payload[..end].to_vec()
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

fn limit_u64(name: &'static str, actual: u64, max: u64) -> ArchiveError {
    ArchiveError::new(
        ArchiveErrorKind::LimitExceeded,
        format!("{name}: declared {actual} exceeds limit {max}"),
    )
}

/// Read state of an open entry.
#[derive(Clone, Copy)]
enum EntryStatus {
    Active,
    /// Declared payload fully produced.
    Exhausted,
    Failed(ArchiveErrorKind),
}

/// Exclusive-borrow reader for one TAR entry's payload.
///
/// Created by [`TarArchiveReader::next_entry`]. Implements [`Read`],
/// bounded at the declared effective size and the remaining total
/// budget — bytes beyond either cap are never yielded.
///
/// - [`finish`](Self::finish) succeeds **only** when the caller consumed
///   the full payload: it then consumes and verifies the block padding
///   and reports [`EntryDisposition::ReadToEnd`]. Calling it early is a
///   contract violation (`InvalidState`) and leaves the archive
///   `Pending`.
/// - [`discard_remaining`](Self::discard_remaining) drains the rest
///   within budget and reports [`EntryDisposition::Discarded`].
/// - Dropping without either moves the archive to `Pending`; recover
///   with [`TarArchiveReader::discard_pending`].
///
/// TAR payloads carry no checksum: `finish` proves the declared end
/// and padding — never content integrity.
///
/// **RT contract:** non-RT only.
pub struct TarEntryReader<'a, R: Read> {
    archive: &'a mut TarArchiveReader<R>,
    info: ArchiveEntryInfo,
    declared: u64,
    produced: u64,
    status: EntryStatus,
    /// Set once the stream position is committed past this entry's
    /// data+padding (or the archive is dead). Until then, `Drop` moves
    /// the archive to `Pending`.
    resolved: bool,
}

impl<R: Read> TarEntryReader<'_, R> {
    /// The entry's metadata (effective name, kind, declared size, and
    /// the link target for `SymbolicLink`/`HardLink` entries).
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn info(&self) -> &ArchiveEntryInfo {
        &self.info
    }

    /// Completes the entry **after the caller fully consumed the
    /// payload** (`produced == declared`): consumes the block padding
    /// and verifies it is zero, marks the archive `Ready`, and returns
    /// the `ReadToEnd` receipt.
    ///
    /// Calling this with payload remaining is `InvalidState` — use
    /// [`discard_remaining`](Self::discard_remaining) instead.
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// `InvalidState` (payload not fully consumed — the archive is left
    /// `Pending`), `Malformed` (non-zero padding; the archive stays
    /// usable), `Truncated`/`Io` (archive `Failed`), or the deferred
    /// `LimitExceeded` from a budget-cut read.
    pub fn finish(mut self) -> Result<EntryReadReceipt, ArchiveError> {
        if let EntryStatus::Failed(kind) = self.status {
            return Err(ArchiveError::new(
                kind,
                format!(
                    "entry {} ({}) failed while reading",
                    self.info.index(),
                    self.info.name().to_string_lossy()
                ),
            ));
        }
        if self.produced < self.declared {
            return Err(ArchiveError::new(
                ArchiveErrorKind::InvalidState,
                format!(
                    "entry {} payload not fully consumed ({} of {} bytes); use discard_remaining",
                    self.info.index(),
                    self.produced,
                    self.declared
                ),
            ));
        }
        self.resolved = true;
        match self.archive.verify_padding(header::pad_len(self.declared)) {
            Ok(()) => {
                self.archive.state = ReaderState::Ready;
                self.archive.entries_verified += 1;
                Ok(EntryReadReceipt::new(
                    self.info.clone(),
                    self.produced,
                    EntryChecksum::NotPresent,
                    EntryDisposition::ReadToEnd,
                ))
            }
            Err(error) => {
                // Malformed padding leaves the position correct and the
                // archive Ready; io failure left it Failed.
                self.archive.entries_unverified += 1;
                Err(error)
            }
        }
    }

    /// Drains the entry's remaining payload within budget, verifies
    /// the block padding, marks the archive `Ready`, and returns the
    /// `Discarded` receipt — structural verification only; the content
    /// was not consumed by the caller.
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// `LimitExceeded` (drain exceeds the total budget — archive
    /// `Failed`), `Malformed` (non-zero padding; archive usable),
    /// `Truncated`/`Io` (archive `Failed`).
    pub fn discard_remaining(mut self) -> Result<EntryReadReceipt, ArchiveError> {
        self.resolved = true;
        self.archive.drain_bytes(self.declared - self.produced)?;
        match self.archive.verify_padding(header::pad_len(self.declared)) {
            Ok(()) => {
                self.archive.state = ReaderState::Ready;
                self.archive.entries_unverified += 1;
                Ok(EntryReadReceipt::new(
                    self.info.clone(),
                    self.declared,
                    EntryChecksum::NotPresent,
                    EntryDisposition::Discarded,
                ))
            }
            Err(error) => {
                self.archive.entries_unverified += 1;
                Err(error)
            }
        }
    }
}

impl<R: Read> Read for TarEntryReader<'_, R> {
    /// Reads payload bytes, capped at the declared size and the
    /// remaining `max_total_uncompressed_bytes` budget. `Ok(0)` at the
    /// declared end is the clean end-of-entry. EOF before the declared
    /// end is `Truncated` — the archive is `Failed`.
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
        let remaining = self.declared - self.produced;
        if remaining == 0 {
            self.status = EntryStatus::Exhausted;
            return Ok(0);
        }
        let budget_left = self
            .archive
            .limits
            .max_total_uncompressed_bytes
            .saturating_sub(self.archive.payload_bytes);
        if budget_left == 0 {
            self.status = EntryStatus::Failed(ArchiveErrorKind::LimitExceeded);
            self.archive.state = ReaderState::Failed;
            return Err(std::io::Error::from(limit_u64(
                "max_total_uncompressed_bytes",
                self.archive.payload_bytes + remaining,
                self.archive.limits.max_total_uncompressed_bytes,
            )));
        }
        let want = usize::try_from(remaining.min(budget_left))
            .unwrap_or(usize::MAX)
            .min(buf.len());
        match self.archive.inner.read(&mut buf[..want]) {
            Ok(0) => {
                self.status = EntryStatus::Failed(ArchiveErrorKind::Truncated);
                self.archive.state = ReaderState::Failed;
                Err(std::io::Error::from(ArchiveError::new(
                    ArchiveErrorKind::Truncated,
                    format!(
                        "entry {} payload ended after {} of {} declared bytes",
                        self.info.index(),
                        self.produced,
                        self.declared
                    ),
                )))
            }
            Ok(n) => {
                self.produced += n as u64;
                self.archive.payload_bytes += n as u64;
                if self.produced == self.declared {
                    self.status = EntryStatus::Exhausted;
                }
                Ok(n)
            }
            Err(error) => {
                let kind = if error.kind() == std::io::ErrorKind::UnexpectedEof {
                    ArchiveErrorKind::Truncated
                } else {
                    ArchiveErrorKind::Io
                };
                self.status = EntryStatus::Failed(kind);
                self.archive.state = ReaderState::Failed;
                Err(std::io::Error::from(ArchiveError::with_source(
                    kind,
                    format!("entry {} payload read failed", self.info.index()),
                    error,
                )))
            }
        }
    }
}

impl<R: Read> Drop for TarEntryReader<'_, R> {
    /// A dropped-but-unresolved entry leaves the archive `Pending`
    /// (remaining payload + padding recorded for `discard_pending`).
    /// Nothing is claimed verified.
    fn drop(&mut self) {
        if self.resolved || matches!(self.archive.state, ReaderState::Failed) {
            return;
        }
        self.archive.entries_unverified += 1;
        self.archive.state = ReaderState::Pending {
            skip: self.declared - self.produced,
            pad: header::pad_len(self.declared),
        };
    }
}
