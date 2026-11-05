//! ZIP writer: explicit `finish` state separation between entry and
//! archive writers.
//!
//! `start_entry` writes a local file header with zeroed size/CRC fields,
//! streams entry data through [`ZipEntryWriter`] (Stored passthrough or
//! Deflate via the hidden backend), and `ZipEntryWriter::finish` seeks
//! back to patch the local header with measured values, then records a
//! central-directory record. `ZipArchiveWriter::finish` writes the
//! central directory + EOCD and returns the inner writer.
//!
//! Drop is never a completion proof: a [`ZipEntryWriter`] dropped
//! without `finish` **poisons** the archive — subsequent `start_entry`
//! and `finish` report `InvalidState` so an incomplete archive can never
//! be finalized.

use std::io::{Seek, SeekFrom, Write};

use flate2::Compression;
use flate2::write::DeflateEncoder;

use crate::error::{ArchiveError, ArchiveErrorKind};
use crate::zip::record::{
    CD_RECORD_LEN, CD_SIG, EOCD_LEN, EOCD_SIG, FLAG_UTF8, LFH_LEN, LFH_SIG, METHOD_DEFLATE,
    METHOD_STORED, put_u16le, put_u32le,
};

/// Compression method selectable per entry.
///
/// `#[non_exhaustive]` per the public-enum policy — additional methods
/// may be added without breaking downstream `match` arms.
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ZipCompressionMethod {
    /// Uncompressed storage (method 0).
    Stored,
    /// DEFLATE (method 8).
    #[default]
    Deflate,
}

impl ZipCompressionMethod {
    /// On-disk method id.
    pub(crate) const fn id(self) -> u16 {
        match self {
            Self::Stored => METHOD_STORED,
            Self::Deflate => METHOD_DEFLATE,
        }
    }
}

/// Per-entry write options.
///
/// `#[non_exhaustive]` — construct via [`ZipEntryOptions::new`] or
/// `Default` and mutate fields. `Default` is [`ZipCompressionMethod::Deflate`].
///
/// # Examples
///
/// ```
/// use ruviform_io_archive::zip::{ZipCompressionMethod, ZipEntryOptions};
///
/// let stored = ZipEntryOptions::new(ZipCompressionMethod::Stored);
/// assert_eq!(stored.method, ZipCompressionMethod::Stored);
/// ```
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ZipEntryOptions {
    /// Compression method for this entry.
    pub method: ZipCompressionMethod,
}

impl ZipEntryOptions {
    /// Options selecting `method`.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn new(method: ZipCompressionMethod) -> Self {
        Self { method }
    }
}

/// A finished entry's central-directory record (internal).
struct CdRecordOut {
    name: Vec<u8>,
    flags: u16,
    method: u16,
    crc32: u32,
    compressed_size: u32,
    uncompressed_size: u32,
    local_offset: u32,
    external_attrs: u32,
}

/// Counts bytes pushed through to the inner writer (compressed size for
/// deflate, identity size for stored).
struct CountWrite<'a, W: Write> {
    inner: &'a mut W,
    count: u64,
}

impl<W: Write> Write for CountWrite<'_, W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let n = self.inner.write(buf)?;
        self.count += n as u64;
        Ok(n)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

/// Entry byte sink: stored counts raw bytes through; deflate compresses
/// through the hidden backend while counting compressed output.
enum EntrySink<'a, W: Write + Seek> {
    Stored(CountWrite<'a, W>),
    Deflate(DeflateEncoder<CountWrite<'a, W>>),
}

/// Streaming ZIP archive writer for `Write + Seek` sinks.
///
/// Write sequence: [`start_entry`](Self::start_entry) → write payload →
/// [`ZipEntryWriter::finish`] → repeat → [`finish`](Self::finish).
/// Seek is required: `finish` on an entry patches its local header with
/// measured size/CRC (no data descriptors are emitted).
///
/// An entry writer dropped without `finish` poisons the archive —
/// `start_entry` and `finish` then return `InvalidState`, so an
/// incomplete archive can never look completed. Dropping the archive
/// writer itself leaves whatever partial ZIP was written downstream;
/// turning bytes into a *published* file is the caller's concern
/// (e.g. IO-01 `StagedFile`).
///
/// **RT contract:** non-RT only.
///
/// # Examples
///
/// ```
/// use ruviform_io_archive::zip::{ZipArchiveWriter, ZipCompressionMethod, ZipEntryOptions};
/// use std::io::{Cursor, Write};
///
/// let mut writer = ZipArchiveWriter::new(Cursor::new(Vec::new()));
/// {
///     let mut entry = writer.start_entry("hello.txt", &ZipEntryOptions::default())?;
///     entry.write_all(b"hello archive")?;
///     entry.finish()?;
/// }
/// let cursor = writer.finish()?;
/// assert!(!cursor.into_inner().is_empty());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct ZipArchiveWriter<W: Write + Seek> {
    inner: W,
    records: Vec<CdRecordOut>,
    poisoned: bool,
}

impl<W: Write + Seek> ZipArchiveWriter<W> {
    /// Starts a ZIP archive on `inner`, writing from its current
    /// position (recorded offsets are absolute stream positions).
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn new(inner: W) -> Self {
        Self {
            inner,
            records: Vec::new(),
            poisoned: false,
        }
    }

    /// Begins an entry named `name` (written as UTF-8; `/` is the
    /// separator). A trailing `/` marks the record as a directory.
    ///
    /// The returned writer holds an exclusive `&mut` borrow — the
    /// archive cannot start another entry or finish until it is
    /// finished or dropped.
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// `InvalidState` (archive poisoned by a dropped entry),
    /// `InvalidInput` (empty/NUL/too-long name), `LimitExceeded`
    /// (more than 65 535 entries), `Unsupported` (offset beyond
    /// classic-ZIP range), `Io`.
    pub fn start_entry(
        &mut self,
        name: &str,
        options: &ZipEntryOptions,
    ) -> Result<ZipEntryWriter<'_, W>, ArchiveError> {
        if self.poisoned {
            return Err(ArchiveError::new(
                ArchiveErrorKind::InvalidState,
                "archive is poisoned: a previous entry writer was dropped without finish",
            ));
        }
        if name.is_empty() || name.as_bytes().contains(&0) {
            return Err(ArchiveError::new(
                ArchiveErrorKind::InvalidInput,
                "entry name must be non-empty and contain no NUL bytes",
            ));
        }
        if name.len() > usize::from(u16::MAX) {
            return Err(ArchiveError::new(
                ArchiveErrorKind::InvalidInput,
                format!("entry name exceeds {} bytes", u16::MAX),
            ));
        }
        if self.records.len() >= usize::from(u16::MAX) {
            return Err(ArchiveError::new(
                ArchiveErrorKind::LimitExceeded,
                format!("classic ZIP cannot hold more than {} entries", u16::MAX),
            ));
        }
        let local_offset = self
            .inner
            .stream_position()
            .map_err(ArchiveError::from_io)?;
        if local_offset > u64::from(u32::MAX) {
            return Err(ArchiveError::new(
                ArchiveErrorKind::Unsupported,
                "archive offset exceeds classic ZIP range (ZIP64 output is not supported)",
            ));
        }
        let method = options.method.id();
        let mut header = [0_u8; LFH_LEN];
        put_u32le(&mut header[0..4], LFH_SIG);
        put_u16le(
            &mut header[4..6],
            if method == METHOD_DEFLATE { 20 } else { 10 },
        );
        put_u16le(&mut header[6..8], FLAG_UTF8);
        put_u16le(&mut header[8..10], method);
        // DOS date 1980-01-01, zeroed time/size/CRC — the size/CRC are
        // patched in on entry finish.
        put_u16le(&mut header[12..14], 0x21);
        put_u16le(
            &mut header[26..28],
            u16::try_from(name.len()).unwrap_or(u16::MAX),
        );
        // On failure the sink now holds partial header bytes: the
        // archive can never be a valid ZIP, so poison it.
        if let Err(error) = self.inner.write_all(&header) {
            self.poisoned = true;
            return Err(ArchiveError::from_io(error));
        }
        if let Err(error) = self.inner.write_all(name.as_bytes()) {
            self.poisoned = true;
            return Err(ArchiveError::from_io(error));
        }
        let counter = CountWrite {
            inner: &mut self.inner,
            count: 0,
        };
        let sink = if method == METHOD_DEFLATE {
            EntrySink::Deflate(DeflateEncoder::new(counter, Compression::default()))
        } else {
            EntrySink::Stored(counter)
        };
        Ok(ZipEntryWriter {
            sink: Some(sink),
            records: &mut self.records,
            poisoned: &mut self.poisoned,
            name: name.as_bytes().to_vec(),
            flags: FLAG_UTF8,
            method,
            local_offset,
            crc: crc32fast::Hasher::new(),
            uncompressed: 0,
            finished: false,
        })
    }

    /// Writes a directory marker (`name` gets a trailing `/` if missing)
    /// with an empty stored payload.
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// Same as [`start_entry`](Self::start_entry).
    pub fn add_directory(&mut self, name: &str) -> Result<(), ArchiveError> {
        let mut dir = String::from(name);
        if !dir.ends_with('/') {
            dir.push('/');
        }
        let entry = self.start_entry(&dir, &ZipEntryOptions::new(ZipCompressionMethod::Stored))?;
        entry.finish()
    }

    /// Number of finished entries so far.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn entry_count(&self) -> usize {
        self.records.len()
    }

    /// Writes the central directory and end-of-central-directory record,
    /// then returns the inner writer. This is the **only** proof that
    /// the archive completed — `Drop` never finishes anything.
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// `InvalidState` (a dropped unfinished entry poisoned the archive),
    /// `Unsupported` (offsets/sizes beyond the classic-ZIP range), `Io`.
    pub fn finish(mut self) -> Result<W, ArchiveError> {
        if self.poisoned {
            return Err(ArchiveError::new(
                ArchiveErrorKind::InvalidState,
                "archive is poisoned: a previous entry writer was dropped without finish",
            ));
        }
        let cd_offset = self
            .inner
            .stream_position()
            .map_err(ArchiveError::from_io)?;
        if cd_offset > u64::from(u32::MAX) {
            return Err(ArchiveError::new(
                ArchiveErrorKind::Unsupported,
                "central directory offset exceeds classic ZIP range",
            ));
        }
        for record in &self.records {
            let mut fixed = [0_u8; CD_RECORD_LEN];
            put_u32le(&mut fixed[0..4], CD_SIG);
            put_u16le(&mut fixed[4..6], 20);
            put_u16le(
                &mut fixed[6..8],
                if record.method == METHOD_DEFLATE {
                    20
                } else {
                    10
                },
            );
            put_u16le(&mut fixed[8..10], record.flags);
            put_u16le(&mut fixed[10..12], record.method);
            put_u16le(&mut fixed[12..14], 0);
            put_u16le(&mut fixed[14..16], 0x21);
            put_u32le(&mut fixed[16..20], record.crc32);
            put_u32le(&mut fixed[20..24], record.compressed_size);
            put_u32le(&mut fixed[24..28], record.uncompressed_size);
            put_u16le(
                &mut fixed[28..30],
                u16::try_from(record.name.len()).unwrap_or(u16::MAX),
            );
            put_u32le(&mut fixed[38..42], record.external_attrs);
            put_u32le(&mut fixed[42..46], record.local_offset);
            self.inner
                .write_all(&fixed)
                .map_err(ArchiveError::from_io)?;
            self.inner
                .write_all(&record.name)
                .map_err(ArchiveError::from_io)?;
        }
        let cd_end = self
            .inner
            .stream_position()
            .map_err(ArchiveError::from_io)?;
        let cd_size = cd_end - cd_offset;
        if cd_size > u64::from(u32::MAX) {
            return Err(ArchiveError::new(
                ArchiveErrorKind::Unsupported,
                "central directory size exceeds classic ZIP range",
            ));
        }
        let count = u16::try_from(self.records.len()).map_err(|_| {
            ArchiveError::new(
                ArchiveErrorKind::LimitExceeded,
                format!("classic ZIP cannot hold more than {} entries", u16::MAX),
            )
        })?;
        let mut eocd = [0_u8; EOCD_LEN];
        put_u32le(&mut eocd[0..4], EOCD_SIG);
        put_u16le(&mut eocd[8..10], count);
        put_u16le(&mut eocd[10..12], count);
        put_u32le(
            &mut eocd[12..16],
            u32::try_from(cd_size).unwrap_or(u32::MAX),
        );
        put_u32le(
            &mut eocd[16..20],
            u32::try_from(cd_offset).unwrap_or(u32::MAX),
        );
        self.inner.write_all(&eocd).map_err(ArchiveError::from_io)?;
        Ok(self.inner)
    }

    /// Returns the inner writer **without** writing the central
    /// directory — the downstream bytes are then an incomplete ZIP.
    /// Use only to abandon an archive before any publish step.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn into_inner(self) -> W {
        self.inner
    }
}

/// Exclusive-borrow writer for one entry's uncompressed payload.
///
/// Implements [`Write`]: bytes are CRC32-measured and (for
/// [`ZipCompressionMethod::Deflate`]) compressed on the way out.
/// [`finish`](Self::finish) completes the entry structure — patches the
/// local header with the measured size/CRC and records the entry for
/// the central directory. **Dropping without `finish` poisons the
/// archive** — the incomplete entry is never treated as completed.
///
/// **RT contract:** non-RT only.
pub struct ZipEntryWriter<'a, W: Write + Seek> {
    sink: Option<EntrySink<'a, W>>,
    records: &'a mut Vec<CdRecordOut>,
    poisoned: &'a mut bool,
    name: Vec<u8>,
    flags: u16,
    method: u16,
    local_offset: u64,
    crc: crc32fast::Hasher,
    uncompressed: u64,
    finished: bool,
}

impl<W: Write + Seek> ZipEntryWriter<'_, W> {
    /// Completes the entry: finishes the codec stream, patches the local
    /// header's CRC/size fields with measured values, and records the
    /// central-directory record. Consumes the writer — after this the
    /// archive may start the next entry or finish.
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// `Unsupported` (measured size beyond classic-ZIP range), `Io`.
    /// On error the entry is not recorded; since `self` is consumed the
    /// archive is poisoned via `Drop`.
    pub fn finish(mut self) -> Result<(), ArchiveError> {
        let Some(sink) = self.sink.take() else {
            return Err(ArchiveError::new(
                ArchiveErrorKind::InvalidState,
                "entry writer already finished",
            ));
        };
        let counted = match sink {
            EntrySink::Stored(counter) => counter,
            EntrySink::Deflate(encoder) => encoder.finish().map_err(ArchiveError::from_io)?,
        };
        let inner = counted.inner;
        let compressed = counted.count;
        if self.uncompressed > u64::from(u32::MAX) || compressed > u64::from(u32::MAX) {
            return Err(ArchiveError::new(
                ArchiveErrorKind::Unsupported,
                "entry size exceeds classic ZIP range (ZIP64 output is not supported)",
            ));
        }
        // Seek back and patch the local header's CRC/size fields
        // (offsets 14..26), then return to the entry-data end.
        let data_end = inner.stream_position().map_err(ArchiveError::from_io)?;
        inner
            .seek(SeekFrom::Start(self.local_offset + 14))
            .map_err(ArchiveError::from_io)?;
        let crc = self.crc.clone().finalize();
        let compressed_size = u32::try_from(compressed).unwrap_or(u32::MAX);
        let uncompressed_size = u32::try_from(self.uncompressed).unwrap_or(u32::MAX);
        let mut patch = [0_u8; 12];
        put_u32le(&mut patch[0..4], crc);
        put_u32le(&mut patch[4..8], compressed_size);
        put_u32le(&mut patch[8..12], uncompressed_size);
        inner.write_all(&patch).map_err(ArchiveError::from_io)?;
        inner
            .seek(SeekFrom::Start(data_end))
            .map_err(ArchiveError::from_io)?;
        let is_dir = self.name.last() == Some(&b'/');
        self.records.push(CdRecordOut {
            name: std::mem::take(&mut self.name),
            flags: self.flags,
            method: self.method,
            crc32: crc,
            compressed_size,
            uncompressed_size,
            local_offset: u32::try_from(self.local_offset).unwrap_or(u32::MAX),
            external_attrs: if is_dir {
                0x10 | (0o40_755 << 16)
            } else {
                0o100_644 << 16
            },
        });
        self.finished = true;
        Ok(())
    }
}

impl<W: Write + Seek> Write for ZipEntryWriter<'_, W> {
    /// Writes uncompressed payload bytes; CRC32 and the uncompressed
    /// counter cover exactly what was accepted.
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let Some(sink) = self.sink.as_mut() else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "entry writer is closed",
            ));
        };
        let n = match sink {
            EntrySink::Stored(counter) => counter.write(buf)?,
            EntrySink::Deflate(encoder) => encoder.write(buf)?,
        };
        self.crc.update(&buf[..n]);
        self.uncompressed += n as u64;
        Ok(n)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self.sink.as_mut() {
            Some(EntrySink::Stored(counter)) => counter.flush(),
            Some(EntrySink::Deflate(encoder)) => encoder.flush(),
            None => Ok(()),
        }
    }
}

impl<W: Write + Seek> Drop for ZipEntryWriter<'_, W> {
    /// A dropped-but-unfinished entry poisons the archive: the partial
    /// local header + payload can never be made consistent, so the
    /// archive must not be completed.
    fn drop(&mut self) {
        if !self.finished {
            *self.poisoned = true;
        }
    }
}
