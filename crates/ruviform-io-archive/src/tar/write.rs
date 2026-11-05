//! TAR writer: declared-size entries and explicit `finish` state
//! separation between entry and archive writers.
//!
//! `start_entry` writes a ustar header declaring the payload size up
//! front — a TAR header must carry the size before the data, so
//! [`TarEntryWriter`] caps writes at the declared length and
//! [`TarEntryWriter::finish`] requires the measured count to match.
//! Long names that cannot be split into ustar `prefix`/`name`, and
//! sizes beyond the octal field's range, are emitted as a preceding
//! PAX `x` extended entry instead.
//!
//! Drop is never a completion proof: a [`TarEntryWriter`] dropped
//! without `finish` **poisons** the archive — subsequent `start_entry`
//! and `finish` report `InvalidState` so an incomplete archive can
//! never be finalized.

use std::io::Write;

use crate::error::{ArchiveError, ArchiveErrorKind};
use crate::tar::header::{self, BLOCK, OCTAL_MAX, TYPE_PAX, TYPE_REGULAR};

/// Per-entry write options.
///
/// `#[non_exhaustive]` — construct via [`TarEntryOptions::new`] or
/// `Default` and mutate fields. `Default` is mode `0o644`, mtime `0`
/// (a deterministic timestamp: archives written with defaults are
/// byte-reproducible).
///
/// # Examples
///
/// ```
/// use ruviform_io_archive::tar::TarEntryOptions;
///
/// let options = TarEntryOptions::new(0o600, 1_700_000_000);
/// assert_eq!(options.mode, 0o600);
/// ```
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TarEntryOptions {
    /// Unix permission bits recorded in the mode field.
    pub mode: u32,
    /// Modification time (seconds since the epoch) recorded in the
    /// mtime field.
    pub mtime: u64,
}

impl Default for TarEntryOptions {
    fn default() -> Self {
        Self {
            mode: 0o644,
            mtime: 0,
        }
    }
}

impl TarEntryOptions {
    /// Options selecting `mode` and `mtime`.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn new(mode: u32, mtime: u64) -> Self {
        Self { mode, mtime }
    }
}

/// How an entry name is represented on disk.
enum NameLayout<'a> {
    /// Whole name fits the 100-byte field.
    Plain,
    /// Split into ustar prefix (≤155) + name (≤100) at a `/`.
    Ustar { prefix: &'a [u8], name: &'a [u8] },
    /// Too long for either: a PAX `x` entry carries `path` and the
    /// header stores a ≤100-byte UTF-8-boundary truncation.
    Pax,
}

/// Splits a name into its ustar representation, or reports `Pax` when
/// no legal split exists.
fn name_layout(name: &[u8]) -> NameLayout<'_> {
    if name.len() <= 100 {
        return NameLayout::Plain;
    }
    // Scan for a `/` where prefix ≤155 and suffix ≤100.
    for (pos, &byte) in name.iter().enumerate() {
        if byte == b'/' && pos <= 155 && name.len() - pos - 1 <= 100 && name.len() - pos - 1 > 0 {
            return NameLayout::Ustar {
                prefix: &name[..pos],
                name: &name[pos + 1..],
            };
        }
    }
    NameLayout::Pax
}

/// Truncates a UTF-8 name to at most `max` bytes at a char boundary.
fn truncate_utf8(name: &str, max: usize) -> &str {
    if name.len() <= max {
        return name;
    }
    let mut end = max;
    while !name.is_char_boundary(end) {
        end -= 1;
    }
    &name[..end]
}

/// Streaming TAR archive writer for `Write` sinks.
///
/// Write sequence: [`start_entry`](Self::start_entry) → write exactly
/// the declared payload → [`TarEntryWriter::finish`] → repeat →
/// [`finish`](Self::finish). No `Seek` is required.
///
/// An entry writer dropped without `finish` (or finished short of its
/// declared size) poisons the archive — `start_entry` and `finish` then
/// return `InvalidState`, so an incomplete archive can never look
/// completed. Dropping the archive writer itself leaves whatever
/// partial TAR was written downstream; turning bytes into a *published*
/// file is the caller's concern (e.g. IO-01 `StagedFile`).
///
/// **Composing from a `Read` source:** `TarEntryWriter` accepts at most
/// the declared size — source bytes beyond it are never pulled. To
/// copy exactly `declared` bytes from a source, cap the source
/// (`source.take(declared)`) and check `io::copy`'s count; surplus
/// source bytes stay in the source (the writer never read-aheads into
/// the next entry's region). Verifying the *source's own* EOF is the
/// caller's contract, not this writer's.
///
/// **RT contract:** non-RT only.
///
/// # Examples
///
/// ```
/// use ruviform_io_archive::tar::{TarArchiveWriter, TarEntryOptions};
/// use std::io::Write;
///
/// let mut writer = TarArchiveWriter::new(Vec::new());
/// {
///     let mut entry = writer.start_entry("hello.txt", 5, &TarEntryOptions::default())?;
///     entry.write_all(b"hello")?;
///     entry.finish()?;
/// }
/// let bytes = writer.finish()?;
/// assert_eq!(bytes.len() % 512, 0);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct TarArchiveWriter<W: Write> {
    inner: W,
    entry_count: usize,
    poisoned: bool,
}

impl<W: Write> TarArchiveWriter<W> {
    /// Starts a TAR archive on `inner`, writing from its current
    /// position.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn new(inner: W) -> Self {
        Self {
            inner,
            entry_count: 0,
            poisoned: false,
        }
    }

    /// Begins a file entry named `name` (written as UTF-8; `/` is the
    /// separator) declaring exactly `declared_size` payload bytes.
    ///
    /// Names ≤100 bytes use the plain field; longer names that split
    /// under the ustar 155/100 prefix rule use it; anything longer is
    /// carried by a preceding PAX `x` entry (`path=`). `declared_size`
    /// beyond the 11-digit octal range is likewise carried by PAX
    /// (`size=`).
    ///
    /// The returned writer holds an exclusive `&mut` borrow — the
    /// archive cannot start another entry or finish until it is
    /// finished or dropped.
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// `InvalidState` (archive poisoned by a dropped/unfinished entry),
    /// `InvalidInput` (empty/NUL name, mtime beyond the octal range),
    /// `Io`. An `Io` failure while writing headers poisons the archive.
    pub fn start_entry(
        &mut self,
        name: &str,
        declared_size: u64,
        options: &TarEntryOptions,
    ) -> Result<TarEntryWriter<'_, W>, ArchiveError> {
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
        if options.mtime > OCTAL_MAX {
            return Err(ArchiveError::new(
                ArchiveErrorKind::InvalidInput,
                "mtime exceeds the octal field range",
            ));
        }
        // On failure the sink may already hold partial header bytes:
        // the archive can never be a valid TAR, so poison it.
        self.write_headers(name, declared_size, TYPE_REGULAR, options)
            .inspect_err(|_| self.poisoned = true)?;
        Ok(TarEntryWriter {
            inner: &mut self.inner,
            poisoned: &mut self.poisoned,
            entry_count: &mut self.entry_count,
            written: 0,
            declared: declared_size,
            finished: false,
        })
    }

    /// Emits the PAX `x` entry (when the name needs it or `size`
    /// exceeds the octal range) and then the `typeflag` header for
    /// `name`.
    fn write_headers(
        &mut self,
        name: &str,
        size: u64,
        typeflag: u8,
        options: &TarEntryOptions,
    ) -> Result<(), ArchiveError> {
        let layout = name_layout(name.as_bytes());
        let needs_pax_size = typeflag == TYPE_REGULAR && size > OCTAL_MAX;
        if matches!(layout, NameLayout::Pax) || needs_pax_size {
            let mut payload = Vec::new();
            if matches!(layout, NameLayout::Pax) {
                payload.extend_from_slice(&header::pax_record(b"path", name.as_bytes()));
            }
            if needs_pax_size {
                payload
                    .extend_from_slice(&header::pax_record(b"size", size.to_string().as_bytes()));
            }
            self.write_pax_entry(name, &payload, options)?;
        }
        let (header_name, prefix) = match layout {
            NameLayout::Plain => (name.as_bytes(), &b""[..]),
            NameLayout::Ustar { prefix, name } => (name, prefix),
            NameLayout::Pax => (truncate_utf8(name, 100).as_bytes(), &b""[..]),
        };
        let block = header::build_header(
            header_name,
            prefix,
            options.mode,
            size.min(OCTAL_MAX),
            options.mtime,
            typeflag,
            b"",
        );
        self.inner
            .write_all(&block)
            .map_err(ArchiveError::from_io)?;
        Ok(())
    }

    /// Writes one PAX `x` extension entry (header + records payload +
    /// block padding) before a real entry.
    fn write_pax_entry(
        &mut self,
        name: &str,
        payload: &[u8],
        options: &TarEntryOptions,
    ) -> Result<(), ArchiveError> {
        let block = header::build_header(
            truncate_utf8(name, 100).as_bytes(),
            b"",
            options.mode,
            payload.len() as u64,
            options.mtime,
            TYPE_PAX,
            b"",
        );
        self.inner
            .write_all(&block)
            .map_err(ArchiveError::from_io)?;
        self.inner
            .write_all(payload)
            .map_err(ArchiveError::from_io)?;
        let pad = vec![0_u8; usize::try_from(header::pad_len(payload.len() as u64)).unwrap_or(0)];
        self.inner.write_all(&pad).map_err(ArchiveError::from_io)?;
        Ok(())
    }

    /// Writes a directory marker (`name` gets a trailing `/` if
    /// missing) with a zero-length payload.
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// Same as [`start_entry`](Self::start_entry).
    pub fn add_directory(&mut self, name: &str) -> Result<(), ArchiveError> {
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
        let mut dir = String::from(name);
        if !dir.ends_with('/') {
            dir.push('/');
        }
        let options = TarEntryOptions::new(0o755, 0);
        self.write_headers(&dir, 0, header::TYPE_DIRECTORY, &options)
            .inspect_err(|_| self.poisoned = true)?;
        self.entry_count += 1;
        Ok(())
    }

    /// Number of finished entries so far.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn entry_count(&self) -> usize {
        self.entry_count
    }

    /// Writes the two zero end-of-archive blocks and returns the inner
    /// writer. This is the **only** proof that the archive completed —
    /// `Drop` never finishes anything. No record-size padding is
    /// emitted beyond the two mandated blocks.
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// `InvalidState` (a dropped/short entry poisoned the archive),
    /// `Io`.
    pub fn finish(mut self) -> Result<W, ArchiveError> {
        if self.poisoned {
            return Err(ArchiveError::new(
                ArchiveErrorKind::InvalidState,
                "archive is poisoned: a previous entry writer was dropped without finish",
            ));
        }
        self.inner
            .write_all(&[0_u8; BLOCK * 2])
            .map_err(ArchiveError::from_io)?;
        Ok(self.inner)
    }

    /// Returns the inner writer **without** writing the end-of-archive
    /// blocks — the downstream bytes are then an incomplete TAR.
    /// Use only to abandon an archive before any publish step.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn into_inner(self) -> W {
        self.inner
    }
}

/// Exclusive-borrow writer for one entry's payload.
///
/// Implements [`Write`]: accepts at most the declared size — calls
/// beyond it report a write error rather than letting surplus bytes
/// leak into the next entry's region. [`finish`](Self::finish) requires
/// the measured count to equal the declaration, then emits the block
/// padding. **Dropping without `finish` poisons the archive** — the
/// committed header can never be un-declared.
///
/// **RT contract:** non-RT only.
pub struct TarEntryWriter<'a, W: Write> {
    inner: &'a mut W,
    poisoned: &'a mut bool,
    entry_count: &'a mut usize,
    written: u64,
    declared: u64,
    finished: bool,
}

impl<W: Write> TarEntryWriter<'_, W> {
    /// Completes the entry: requires `written == declared` and emits
    /// the block padding. A short entry reports `SizeMismatch` and
    /// poisons the archive (the header's declared size is already
    /// committed downstream).
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// `SizeMismatch` (fewer bytes written than declared), `Io`.
    pub fn finish(mut self) -> Result<(), ArchiveError> {
        if self.written < self.declared {
            return Err(ArchiveError::new(
                ArchiveErrorKind::SizeMismatch,
                format!(
                    "entry payload wrote {} bytes, declared {}",
                    self.written, self.declared
                ),
            ));
        }
        let pad = vec![0_u8; usize::try_from(header::pad_len(self.declared)).unwrap_or(0)];
        self.inner.write_all(&pad).map_err(ArchiveError::from_io)?;
        self.finished = true;
        *self.entry_count += 1;
        Ok(())
    }
}

impl<W: Write> Write for TarEntryWriter<'_, W> {
    /// Writes payload bytes, accepting at most the declared size —
    /// partial writes leave the remainder for the next call; calls made
    /// with the declaration already exhausted fail.
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let remaining = self.declared - self.written;
        if remaining == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::WriteZero,
                "entry declared size is exhausted",
            ));
        }
        let want = usize::try_from(remaining)
            .unwrap_or(usize::MAX)
            .min(buf.len());
        let n = self.inner.write(&buf[..want])?;
        self.written += n as u64;
        Ok(n)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

impl<W: Write> Drop for TarEntryWriter<'_, W> {
    /// A dropped-but-unfinished entry poisons the archive: the header
    /// already declared a size the stream no longer satisfies.
    fn drop(&mut self) {
        if !self.finished {
            *self.poisoned = true;
        }
    }
}
