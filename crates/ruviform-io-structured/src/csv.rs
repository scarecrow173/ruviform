//! CSV record streaming (`feature = "csv"`) — IO-IMP-04 / Issue #298.
//!
//! Unlike the single-document formats, CSV is record-oriented:
//! [`CsvReader`] borrows the caller's `Read` and yields one [`CsvRecord`]
//! at a time; [`CsvWriter`] borrows a `Write` and emits records until
//! [`CsvWriter::finish`]. The convenience pair [`from_reader`] /
//! [`to_writer`] collects / emits whole documents of `T` with the same
//! borrowed-stream contract as the other format modules.
//!
//! ## Limits enforced *before* allocation
//!
//! Parsing runs on `csv-core`'s pull-based NFA over caller-owned
//! buffers, so every growth point is checked first: `max_bytes`
//! ([`InputLimits`] byte budget), `max_record_bytes`, `max_field_bytes`,
//! `max_fields` and `max_records` all produce
//! [`DocumentErrorKind::LimitExceeded`] — `DocumentError::limit` names
//! the exceeded option. Field/record buffers are reused across records
//! ([`CsvRecord`] keeps its allocation), so steady-state streaming
//! allocates a fixed 8 KiB input chunk plus the configured record cap.
//!
//! ## Encoding, BOM and `field_str`
//!
//! Records are byte-oriented: [`CsvRecord::field_bytes`] never decodes.
//! [`CsvRecord::field_str`] and [`CsvReader::read_typed`] require valid
//! UTF-8 per field — violations are
//! [`DocumentErrorKind::InvalidEncoding`]. A UTF-8 BOM is stripped when
//! [`CsvReadOptions::bom`] is [`CsvBomPolicy::StripUtf8`] (the default);
//! UTF-16-family BOM input is rejected as `InvalidEncoding` — no
//! transcoding is performed.
//!
//! ## Dialect & leniency
//!
//! The backend intentionally follows the permissive CSV grammar: an
//! unclosed quote at EOF still yields a final record, and a `"` outside
//! quoting context is data. There is therefore no
//! [`DocumentErrorKind::Syntax`] case and no
//! [`DocumentErrorKind::TrailingData`] — the reader consumes input until
//! `read_record` reports `Ok(false)`. Rectangularity is a *data-model*
//! contract (`flexible` / `has_headers`), reported as
//! [`DocumentErrorKind::DataModel`].
//!
//! ## Formula injection boundary
//!
//! Fields starting with `=`, `+`, `-` or `@` are written and read
//! verbatim — sanitizing them changes user data and is an export-policy
//! decision that belongs to the consumer, not to this module.
//!
//! **RT contract:** non-RT only — may block on I/O and allocate.

use crate::input::BoundedRead;
use crate::{
    DocumentError, DocumentErrorKind, DocumentFormat, DocumentLimit, DocumentPosition, InputLimits,
};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::io::{Read, Write};

const FORMAT: DocumentFormat = DocumentFormat::Csv;
/// Fixed size of the reader's input chunk.
const INPUT_CHUNK: usize = 8 * 1024;
/// Default ceiling on the bytes a single record may occupy (1 MiB).
const DEFAULT_MAX_RECORD_BYTES: usize = 1024 * 1024;
/// Default ceiling on the bytes a single field may occupy (256 KiB).
const DEFAULT_MAX_FIELD_BYTES: usize = 256 * 1024;
/// Default ceiling on the number of fields in a single record.
const DEFAULT_MAX_FIELDS: usize = 4096;
/// Default ceiling on the number of data records per input.
const DEFAULT_MAX_RECORDS: u64 = 1_000_000;

/// Record terminator convention for CSV dialects.
///
/// `CrLf` is the RFC 4180 terminator: on read it accepts `\r`, `\n` and
/// `\r\n`; on write it emits `\r\n`. `Any(byte)` treats exactly that
/// byte as the terminator in both directions.
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CsvTerminator {
    /// `\r`, `\n` or `\r\n` (read) / emits `\r\n` (write). Default.
    #[default]
    CrLf,
    /// A specific single byte used verbatim.
    Any(u8),
}

/// Whitespace-trimming policy applied to parsed record fields.
///
/// Trimming removes ASCII whitespace at field edges after parsing —
/// the same semantics as the backend's record-level trim.
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CsvTrim {
    /// No trimming (default).
    #[default]
    None,
    /// Trim the header record only.
    Headers,
    /// Trim data records only.
    Fields,
    /// Trim headers and data records.
    All,
}

/// Quoting style used when writing fields.
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CsvQuoteStyle {
    /// Quote only when needed to preserve the field (default).
    #[default]
    Necessary,
    /// Quote every field.
    Always,
    /// Quote only non-numeric fields.
    NonNumeric,
    /// Never quote — fields containing structural bytes are written
    /// verbatim (may produce ambiguous output).
    Never,
}

/// Handling of a leading UTF-8 byte-order mark on input.
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CsvBomPolicy {
    /// Strip a leading UTF-8 BOM (`EF BB BF`) before parsing (default).
    /// Other BOMs (e.g. UTF-16 `FF FE`/`FE FF`) are rejected with
    /// [`DocumentErrorKind::InvalidEncoding`].
    #[default]
    StripUtf8,
    /// Do not strip: a leading UTF-8 BOM becomes part of the first
    /// field. Other BOMs are still rejected.
    Preserve,
}

/// Shared byte-level dialect: delimiter, quote, terminator and the
/// escaping conventions common to reading and writing.
///
/// `#[non_exhaustive]` — new knobs may be added; construct with
/// [`CsvDialect::new`] or [`Default`], then adjust fields.
///
/// `quoting` is parse-side only: on write, quoting behavior is
/// controlled by [`CsvWriteOptions::quote_style`]. A `delimiter` equal
/// to `quote` is reported as [`DocumentErrorKind::Unsupported`].
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CsvDialect {
    /// Field delimiter byte (default `b','`).
    pub delimiter: u8,
    /// Quote byte (default `b'"'`).
    pub quote: u8,
    /// Record terminator convention (default [`CsvTerminator::CrLf`]).
    pub terminator: CsvTerminator,
    /// Comment byte: a record starting with it is skipped on read;
    /// on write it only informs quoting of matching leading bytes.
    pub comment: Option<u8>,
    /// Interpret a doubled quote inside a quoted field as an escaped
    /// quote (default `true`).
    pub double_quote: bool,
    /// Escape character used instead of/in addition to doubled quotes
    /// (e.g. `b'\\'` for backslash-escaped dialects).
    pub escape: Option<u8>,
    /// Whether quote bytes start quoted fields on read (default
    /// `true`). When `false`, quotes are ordinary data.
    pub quoting: bool,
}

impl CsvDialect {
    /// RFC-4180-flavored default dialect.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            delimiter: b',',
            quote: b'"',
            terminator: CsvTerminator::CrLf,
            comment: None,
            double_quote: true,
            escape: None,
            quoting: true,
        }
    }

    /// Overrides `delimiter`.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn with_delimiter(mut self, delimiter: u8) -> Self {
        self.delimiter = delimiter;
        self
    }

    /// Overrides `quote`.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn with_quote(mut self, quote: u8) -> Self {
        self.quote = quote;
        self
    }

    /// Overrides `terminator`.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn with_terminator(mut self, terminator: CsvTerminator) -> Self {
        self.terminator = terminator;
        self
    }

    /// Overrides `comment`.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn with_comment(mut self, comment: Option<u8>) -> Self {
        self.comment = comment;
        self
    }

    /// Overrides `double_quote`.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn with_double_quote(mut self, double_quote: bool) -> Self {
        self.double_quote = double_quote;
        self
    }

    /// Overrides `escape`.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn with_escape(mut self, escape: Option<u8>) -> Self {
        self.escape = escape;
        self
    }

    /// Overrides `quoting`.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn with_quoting(mut self, quoting: bool) -> Self {
        self.quoting = quoting;
        self
    }
}

impl Default for CsvDialect {
    fn default() -> Self {
        Self::new()
    }
}

/// Read options for [`CsvReader`] and [`from_reader`].
///
/// `#[non_exhaustive]` — new knobs may be added; construct with
/// [`CsvReadOptions::new`] or [`Default`], then adjust fields.
///
/// The `max_*` fields are hard limits enforced *before* the affected
/// buffer may grow, all reported as
/// [`DocumentErrorKind::LimitExceeded`] with [`DocumentError::limit`]
/// naming the option.
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Debug, Clone, Copy)]
pub struct CsvReadOptions {
    /// Dialect bytes and conventions (default [`CsvDialect::new`]).
    pub dialect: CsvDialect,
    /// Whether the first record is a header row (default `true`).
    ///
    /// When `true` the first record is consumed but never yielded by
    /// `read_record`; [`CsvReader::headers`] exposes it, and
    /// `read_typed` maps fields to `T`'s field names. When `false`
    /// every record is yielded and `read_typed` binds positionally.
    pub has_headers: bool,
    /// Whether records may have varying field counts (default `false`).
    /// When `false`, a record whose field count differs from the first
    /// record's is [`DocumentErrorKind::DataModel`].
    pub flexible: bool,
    /// Edge-whitespace trimming (default [`CsvTrim::None`]).
    pub trim: CsvTrim,
    /// UTF-8 BOM handling (default [`CsvBomPolicy::StripUtf8`]).
    pub bom: CsvBomPolicy,
    /// Hard input byte budget (default 16 MiB, see [`InputLimits`]).
    pub limits: InputLimits,
    /// Maximum bytes a single record's field data may occupy
    /// (default 1 MiB).
    pub max_record_bytes: usize,
    /// Maximum bytes a single field may occupy (default 256 KiB).
    pub max_field_bytes: usize,
    /// Maximum fields a single record may contain (default 4096).
    pub max_fields: usize,
    /// Maximum data records readable from one input
    /// (default `1_000_000`). Headers do not count.
    pub max_records: u64,
}

impl CsvReadOptions {
    /// Options with all defaults.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            dialect: CsvDialect::new(),
            has_headers: true,
            flexible: false,
            trim: CsvTrim::None,
            bom: CsvBomPolicy::StripUtf8,
            limits: InputLimits::new(crate::DEFAULT_MAX_INPUT_BYTES),
            max_record_bytes: DEFAULT_MAX_RECORD_BYTES,
            max_field_bytes: DEFAULT_MAX_FIELD_BYTES,
            max_fields: DEFAULT_MAX_FIELDS,
            max_records: DEFAULT_MAX_RECORDS,
        }
    }

    /// Overrides `dialect`.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn with_dialect(mut self, dialect: CsvDialect) -> Self {
        self.dialect = dialect;
        self
    }

    /// Overrides `has_headers`.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn with_has_headers(mut self, has_headers: bool) -> Self {
        self.has_headers = has_headers;
        self
    }

    /// Overrides `flexible`.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn with_flexible(mut self, flexible: bool) -> Self {
        self.flexible = flexible;
        self
    }

    /// Overrides `trim`.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn with_trim(mut self, trim: CsvTrim) -> Self {
        self.trim = trim;
        self
    }

    /// Overrides `bom`.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn with_bom(mut self, bom: CsvBomPolicy) -> Self {
        self.bom = bom;
        self
    }

    /// Overrides `limits`.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn with_limits(mut self, limits: InputLimits) -> Self {
        self.limits = limits;
        self
    }

    /// Overrides `max_record_bytes`.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn with_max_record_bytes(mut self, max_record_bytes: usize) -> Self {
        self.max_record_bytes = max_record_bytes;
        self
    }

    /// Overrides `max_field_bytes`.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn with_max_field_bytes(mut self, max_field_bytes: usize) -> Self {
        self.max_field_bytes = max_field_bytes;
        self
    }

    /// Overrides `max_fields`.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn with_max_fields(mut self, max_fields: usize) -> Self {
        self.max_fields = max_fields;
        self
    }

    /// Overrides `max_records`.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn with_max_records(mut self, max_records: u64) -> Self {
        self.max_records = max_records;
        self
    }
}

impl Default for CsvReadOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// Write options for [`CsvWriter`] and [`to_writer`].
///
/// `#[non_exhaustive]` — new knobs may be added; construct with
/// [`CsvWriteOptions::new`] or [`Default`], then adjust fields.
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Debug, Clone, Copy)]
pub struct CsvWriteOptions {
    /// Dialect bytes and conventions (default [`CsvDialect::new`]).
    /// `dialect.quoting` is ignored on write — use `quote_style`.
    pub dialect: CsvDialect,
    /// Field quoting policy (default [`CsvQuoteStyle::Necessary`]).
    pub quote_style: CsvQuoteStyle,
    /// Whether `write_typed`/`to_writer` emit a header row from the
    /// serialized struct's field names (default `true`). Only affects
    /// Serde serialization — `write_record` never emits headers.
    pub has_headers: bool,
    /// Whether records may vary in field count (default `false`).
    /// When `false`, a mismatching record is
    /// [`DocumentErrorKind::DataModel`].
    pub flexible: bool,
}

impl CsvWriteOptions {
    /// Options with all defaults.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            dialect: CsvDialect::new(),
            quote_style: CsvQuoteStyle::Necessary,
            has_headers: true,
            flexible: false,
        }
    }

    /// Overrides `dialect`.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn with_dialect(mut self, dialect: CsvDialect) -> Self {
        self.dialect = dialect;
        self
    }

    /// Overrides `quote_style`.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn with_quote_style(mut self, quote_style: CsvQuoteStyle) -> Self {
        self.quote_style = quote_style;
        self
    }

    /// Overrides `has_headers`.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn with_has_headers(mut self, has_headers: bool) -> Self {
        self.has_headers = has_headers;
        self
    }

    /// Overrides `flexible`.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn with_flexible(mut self, flexible: bool) -> Self {
        self.flexible = flexible;
        self
    }
}

impl Default for CsvWriteOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// A reusable CSV record buffer: the field bytes of one record plus
/// their boundaries and the record's input position.
///
/// Pass the same `CsvRecord` to successive [`CsvReader::read_record`]
/// calls to reuse its allocation — the buffer grows only up to the
/// configured limits and is then held steady.
///
/// `#[non_exhaustive]` — construct with [`CsvRecord::new`].
///
/// **RT contract:** non-RT only — may allocate up to the configured
/// record limit.
#[non_exhaustive]
#[derive(Debug, Default)]
pub struct CsvRecord {
    /// Concatenated field bytes; `data_len` marks the valid prefix.
    /// The Vec's length is its *capacity* — the parser writes into
    /// `data[outlen..]` slices.
    data: Vec<u8>,
    /// Number of valid bytes in `data`.
    data_len: usize,
    /// Per-field end offsets into `data`; `field_count` valid entries.
    ends: Vec<usize>,
    /// Number of fields in the record.
    field_count: usize,
    /// Input position where the record began.
    position: Option<DocumentPosition>,
}

impl CsvRecord {
    /// An empty record buffer.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            data: Vec::new(),
            data_len: 0,
            ends: Vec::new(),
            field_count: 0,
            position: None,
        }
    }

    /// Number of fields in the record.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.field_count
    }

    /// Whether the record has no fields.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.field_count == 0
    }

    /// The position where this record began, when read through
    /// [`CsvReader`].
    ///
    /// `byte` is the 0-based offset where record parsing started (after
    /// a stripped BOM it may be > 0), `line` the 1-based parser line
    /// and `record` the 0-based data-record index (headers excluded).
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn position(&self) -> Option<DocumentPosition> {
        self.position
    }

    /// Raw bytes of field `index` (`None` when out of range).
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn field_bytes(&self, index: usize) -> Option<&[u8]> {
        if index >= self.field_count {
            return None;
        }
        let start = if index == 0 { 0 } else { self.ends[index - 1] };
        Some(&self.data[start..self.ends[index]])
    }

    /// UTF-8 view of field `index`.
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// [`DocumentErrorKind::InvalidEncoding`] when the field is not
    /// valid UTF-8; position points at the first invalid byte.
    pub fn field_str(&self, index: usize) -> Result<Option<&str>, DocumentError> {
        let Some(bytes) = self.field_bytes(index) else {
            return Ok(None);
        };
        std::str::from_utf8(bytes).map(Some).map_err(|e| {
            let start = if index == 0 { 0 } else { self.ends[index - 1] };
            let field_start = u64::try_from(start).unwrap_or(u64::MAX);
            let invalid_at = u64::try_from(e.valid_up_to()).unwrap_or(u64::MAX);
            let byte = self
                .position
                .and_then(|p| p.byte)
                .map(|b| b + field_start + invalid_at);
            DocumentError::with_parts(
                DocumentErrorKind::InvalidEncoding,
                FORMAT,
                Some(DocumentPosition {
                    byte,
                    line: self.position.and_then(|p| p.line),
                    column: None,
                    record: self.position.and_then(|p| p.record),
                }),
                None,
                Some(Box::new(e)),
            )
        })
    }

    /// Iterates the fields as byte slices.
    ///
    /// **RT contract:** non-RT only.
    pub fn iter(&self) -> impl Iterator<Item = &[u8]> + '_ {
        (0..self.field_count).filter_map(|i| self.field_bytes(i))
    }

    /// The concatenated field bytes of the record (after trimming, when
    /// applied). Mostly useful for size inspection.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.data[..self.data_len]
    }

    /// Clears the record, keeping the allocation for reuse.
    ///
    /// **RT contract:** non-RT only.
    pub fn clear(&mut self) {
        self.data_len = 0;
        self.field_count = 0;
        self.position = None;
    }

    /// Finalizes the record after a parse: `field_count` ends and
    /// `data_len` bytes of field data.
    fn finish(&mut self, field_count: usize, data_len: usize) {
        self.field_count = field_count;
        self.data_len = data_len;
    }

    /// Trims ASCII whitespace at every field's edges in place.
    fn trim(&mut self) {
        let mut write_pos = 0usize;
        let mut read_start = 0usize;
        for i in 0..self.field_count {
            let read_end = self.ends[i];
            let field = &self.data[read_start..read_end];
            let mut s = 0usize;
            let mut e = field.len();
            while s < e && field[s].is_ascii_whitespace() {
                s += 1;
            }
            while e > s && field[e - 1].is_ascii_whitespace() {
                e -= 1;
            }
            self.data
                .copy_within(read_start + s..read_start + e, write_pos);
            self.ends[i] = write_pos + (e - s);
            write_pos += e - s;
            read_start = read_end;
        }
        self.data_len = write_pos;
    }
}

/// Header record lifecycle inside [`CsvReader`]: pending parse,
/// resolved (`None` when the input had no records at all), or disabled
/// by `has_headers == false`.
#[derive(Debug)]
enum HeaderState {
    /// `has_headers` enabled, header not yet parsed.
    Pending,
    /// Header resolved (`None` when input had no records).
    Parsed(Option<CsvRecord>),
    /// `has_headers == false`.
    Disabled,
}

/// Sequential CSV reader over a borrowed input stream.
///
/// Holds `&'a mut R` for its lifetime — it never seeks, rewinds or
/// reads past what parsing requires, so the only "lookahead" is the
/// fixed 8 KiB input chunk plus the one-byte over-budget probe inside
/// the shared [`BoundedRead`] contract. After the reader is dropped the
/// caller's reader may continue from wherever parsing stopped.
///
/// Records are produced in input order; `read_record` returns
/// `Ok(false)` at end of input (strict EOF: every byte is consumed by
/// parsing — malformed trailing content simply yields malformed
/// records, which surface as [`DocumentErrorKind::DataModel`] or are
/// accepted per the permissive grammar, never silently dropped).
///
/// **RT contract:** non-RT only — may block on I/O and allocate up to
/// the configured limits.
#[derive(Debug)]
pub struct CsvReader<'a, R: Read> {
    /// Budgeted source over the borrowed reader.
    src: BoundedRead<&'a mut R>,
    /// Leading bytes pulled during the BOM probe, fed before `src`.
    prefix: Vec<u8>,
    /// Consumed prefix bytes.
    prefix_pos: usize,
    /// Fixed input chunk handed to the parser.
    inbuf: Vec<u8>,
    /// `inbuf` bookkeeping: bytes consumed / valid bytes.
    in_pos: usize,
    in_len: usize,
    /// The underlying reader reached EOF (or the budget probe found it).
    exhausted: bool,
    /// Pull-based parse engine with caller-owned buffers.
    core: csv_core::Reader,
    /// Read options (copied — `CsvReadOptions: Copy`).
    options: CsvReadOptions,
    /// Header record lifecycle — parsed lazily on first access.
    header_state: HeaderState,
    /// Field count the rigid contract expects (`flexible == false`).
    expected_fields: Option<usize>,
    /// Data records yielded so far (headers excluded).
    record_no: u64,
    /// Parser-consumed input bytes (includes a stripped BOM).
    byte_pos: u64,
    /// Scratch record for `read_typed` — kept out of the caller's
    /// record so `read_record` reuse is unaffected.
    scratch: CsvRecord,
    /// Backend header container for typed deserialize.
    typed_headers: Option<csv::ByteRecord>,
    /// `End` already observed: every later call returns `Ok(false)`.
    done: bool,
    /// Preserve-BOM guard: the first `read_record` call must see fewer
    /// than 3 input bytes so `csv_core`'s own first-read BOM strip
    /// cannot fire before its `has_read` flag latches.
    bom_guard: bool,
}

impl<'a, R: Read> CsvReader<'a, R> {
    /// Builds a reader over `reader` under `options`.
    ///
    /// Reads up to a few leading bytes eagerly for BOM handling: a
    /// UTF-8 BOM is stripped or preserved per `options.bom`, and a
    /// UTF-16-family BOM fails immediately.
    ///
    /// **RT contract:** non-RT only — may block on I/O and allocate.
    ///
    /// # Errors
    ///
    /// [`DocumentErrorKind::Unsupported`] for a contradictory dialect
    /// (`delimiter == quote`), [`DocumentErrorKind::InvalidEncoding`]
    /// for a UTF-16 BOM, [`DocumentErrorKind::Io`] /
    /// `LimitExceeded` for probe-read failures.
    pub fn new(reader: &'a mut R, options: &CsvReadOptions) -> Result<Self, DocumentError> {
        let options = *options;
        validate_dialect(&options.dialect)?;
        let mut src = BoundedRead::new(reader, options.limits.max_bytes);
        // Probe up to 4 leading bytes for BOM classification.
        let mut prefix = Vec::with_capacity(4);
        let mut exhausted = false;
        while prefix.len() < 4 && !exhausted {
            let mut tmp = [0u8; 4];
            match src.read(&mut tmp[..4 - prefix.len()]) {
                Ok(0) => exhausted = true,
                Ok(n) => prefix.extend_from_slice(&tmp[..n]),
                Err(e) => {
                    return Err(if src.was_limit_exceeded() {
                        limit_error("max_bytes", options.limits.max_bytes, None)
                    } else {
                        DocumentError::with_source(DocumentErrorKind::Io, FORMAT, e)
                    });
                }
            }
        }
        let mut byte_pos = 0u64;
        if prefix.len() >= 2
            && (prefix[0] == 0xFF && prefix[1] == 0xFE || prefix[0] == 0xFE && prefix[1] == 0xFF)
        {
            return Err(DocumentError::with_parts(
                DocumentErrorKind::InvalidEncoding,
                FORMAT,
                Some(DocumentPosition {
                    byte: Some(0),
                    line: Some(1),
                    column: None,
                    record: None,
                }),
                None,
                Some(Box::new(BackendMessage(
                    "UTF-16 byte-order mark is not valid CSV input".into(),
                ))),
            ));
        }
        let utf8_bom = prefix.len() >= 3 && prefix[..3] == [0xEF, 0xBB, 0xBF];
        if utf8_bom && options.bom == CsvBomPolicy::StripUtf8 {
            prefix.drain(..3);
            byte_pos = 3;
        }
        let core = core_reader(&options.dialect);
        Ok(Self {
            src,
            prefix,
            prefix_pos: 0,
            inbuf: vec![0u8; INPUT_CHUNK],
            in_pos: 0,
            in_len: 0,
            exhausted,
            core,
            options,
            header_state: if options.has_headers {
                HeaderState::Pending
            } else {
                HeaderState::Disabled
            },
            expected_fields: None,
            record_no: 0,
            byte_pos,
            scratch: CsvRecord::new(),
            typed_headers: None,
            done: false,
            bom_guard: utf8_bom && options.bom == CsvBomPolicy::Preserve,
        })
    }

    /// Reads the next record into `record`, reusing its buffers.
    ///
    /// Returns `Ok(false)` once every input byte is consumed. On error
    /// the contents of `record` are unspecified; call `clear` or reuse
    /// it on the next read — its allocation is retained either way.
    ///
    /// **RT contract:** non-RT only — may block on I/O and allocate up
    /// to the configured record/field limits.
    ///
    /// # Errors
    ///
    /// `LimitExceeded` (byte budget, record/field sizes, counts),
    /// `DataModel` (non-rectangular data when `flexible` is `false`),
    /// `Io` for source failures.
    pub fn read_record(&mut self, record: &mut CsvRecord) -> Result<bool, DocumentError> {
        self.ensure_headers()?;
        self.parse_next(record, true, self.trim_fields())
    }

    /// Reads the next record and deserializes it as `T`.
    ///
    /// With `has_headers`, `T`'s field names bind to the header row;
    /// otherwise fields bind positionally. A deserialization failure is
    /// reported for the whole record — it is never truncated to a
    /// partial value.
    ///
    /// `read_typed` shares the cursor with [`read_record`]: every call
    /// consumes one record.
    ///
    /// [`read_record`]: Self::read_record
    ///
    /// **RT contract:** non-RT only — may block on I/O and allocate.
    ///
    /// # Errors
    ///
    /// `DataModel` / `InvalidEncoding` for record→`T` failures, plus
    /// the [`read_record`](Self::read_record) contract.
    pub fn read_typed<T: DeserializeOwned>(&mut self) -> Result<Option<T>, DocumentError> {
        self.ensure_headers()?;
        let apply_trim = self.trim_fields();
        // Move the scratch record out so `parse_next` can take `&mut
        // self`; it is restored before returning.
        let mut scratch = std::mem::take(&mut self.scratch);
        let parsed = self.parse_next(&mut scratch, true, apply_trim);
        let position = scratch.position();
        let mut backend = csv::ByteRecord::new();
        if matches!(parsed, Ok(true)) {
            for field in scratch.iter() {
                backend.push_field(field);
            }
        }
        self.scratch = scratch;
        if !parsed? {
            return Ok(None);
        }
        backend
            .deserialize(self.typed_headers.as_ref())
            .map(Some)
            .map_err(|e| map_csv_error(e, position))
    }

    /// The header record, when `has_headers` is enabled.
    ///
    /// Parses it lazily on first access; `None` either means
    /// `has_headers == false` or the input had no records.
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// Same contract as [`read_record`](Self::read_record).
    pub fn headers(&mut self) -> Result<Option<&CsvRecord>, DocumentError> {
        self.ensure_headers()?;
        Ok(match &self.header_state {
            HeaderState::Parsed(record) => record.as_ref(),
            HeaderState::Pending | HeaderState::Disabled => None,
        })
    }

    /// Current parse cursor: bytes consumed, 1-based line and the
    /// 0-based index of the next data record.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn position(&self) -> DocumentPosition {
        DocumentPosition {
            byte: Some(self.byte_pos),
            line: Some(self.core.line()),
            column: None,
            record: Some(self.record_no),
        }
    }

    /// Number of data records yielded so far (headers excluded).
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn record_index(&self) -> u64 {
        self.record_no
    }

    /// Whether trimming applies to data records.
    fn trim_fields(&self) -> bool {
        matches!(self.options.trim, CsvTrim::Fields | CsvTrim::All)
    }

    /// Whether trimming applies to the header record.
    fn trim_headers(&self) -> bool {
        matches!(self.options.trim, CsvTrim::Headers | CsvTrim::All)
    }

    /// Parses and stashes the header record when `has_headers`.
    fn ensure_headers(&mut self) -> Result<(), DocumentError> {
        if !matches!(self.header_state, HeaderState::Pending) {
            return Ok(());
        }
        let mut record = CsvRecord::new();
        let parsed = if self.parse_next(&mut record, false, self.trim_headers())? {
            let mut typed = csv::ByteRecord::new();
            for field in record.iter() {
                typed.push_field(field);
            }
            self.typed_headers = Some(typed);
            Some(record)
        } else {
            None
        };
        self.header_state = HeaderState::Parsed(parsed);
        Ok(())
    }

    /// Core parse loop: feeds `csv_core` chunks and grows `record`'s
    /// buffers strictly under the configured limits.
    ///
    /// `count_record` distinguishes data records (counted toward
    /// `max_records` and `record_no`) from the header record.
    fn parse_next(
        &mut self,
        record: &mut CsvRecord,
        count_record: bool,
        apply_trim: bool,
    ) -> Result<bool, DocumentError> {
        record.clear();
        if self.done {
            return Ok(false);
        }
        let position = DocumentPosition {
            byte: Some(self.byte_pos),
            line: Some(self.core.line()),
            column: None,
            record: Some(self.record_no),
        };
        record.position = Some(position);
        let mut outlen = 0usize;
        let mut endlen = 0usize;
        let mut checked_endlen = 0usize;
        if record.data.is_empty() {
            record
                .data
                .resize(INPUT_CHUNK.min(self.options.max_record_bytes), 0);
        }
        if record.ends.is_empty() {
            record.ends.resize(16usize.min(self.options.max_fields), 0);
        }
        loop {
            if self.in_pos == self.in_len
                && (!self.exhausted || self.prefix_pos < self.prefix.len())
            {
                self.refill()?;
            }
            // `csv_core` unconditionally strips a leading UTF-8 BOM on
            // its first read when handed >= 3 bytes; under `Preserve`
            // the first call is capped at 2 bytes so its `has_read`
            // flag latches before a full BOM is ever visible.
            let mut end = self.in_len;
            if self.bom_guard {
                self.bom_guard = false;
                end = end.min(self.in_pos + 2);
            }
            let input = &self.inbuf[self.in_pos..end];
            let (result, nin, nout, nend) = self.core.read_record(
                input,
                &mut record.data[outlen..],
                &mut record.ends[endlen..],
            );
            self.in_pos += nin;
            self.byte_pos += u64::try_from(nin).unwrap_or(u64::MAX);
            outlen += nout;
            endlen += nend;
            check_field_sizes(
                record,
                endlen,
                outlen,
                &mut checked_endlen,
                self.options.max_field_bytes,
                position,
            )?;
            match result {
                csv_core::ReadRecordResult::InputEmpty => {}
                csv_core::ReadRecordResult::OutputFull
                | csv_core::ReadRecordResult::OutputEndsFull => {
                    grow_record_buffers(record, &result, &self.options, position)?;
                }
                csv_core::ReadRecordResult::Record => {
                    if !self.options.flexible {
                        let expected = *self.expected_fields.get_or_insert(endlen);
                        if expected != endlen {
                            return Err(DocumentError::with_parts(
                                DocumentErrorKind::DataModel,
                                FORMAT,
                                Some(position),
                                None,
                                Some(Box::new(BackendMessage(format!(
                                    "record has {endlen} fields, expected {expected}",
                                )))),
                            ));
                        }
                    }
                    if count_record {
                        if self.record_no >= self.options.max_records {
                            return Err(limit_error(
                                "max_records",
                                self.options.max_records,
                                Some(position),
                            ));
                        }
                        self.record_no += 1;
                    }
                    record.finish(endlen, outlen);
                    if apply_trim {
                        record.trim();
                    }
                    return Ok(true);
                }
                csv_core::ReadRecordResult::End => {
                    self.done = true;
                    return Ok(false);
                }
            }
        }
    }

    /// Compacts and refills the input chunk: leftover BOM-probe bytes
    /// first, then the budgeted source.
    fn refill(&mut self) -> Result<(), DocumentError> {
        if self.in_pos > 0 {
            self.inbuf.copy_within(self.in_pos..self.in_len, 0);
            self.in_len -= self.in_pos;
            self.in_pos = 0;
        }
        if self.prefix_pos < self.prefix.len() {
            let n = (self.prefix.len() - self.prefix_pos).min(INPUT_CHUNK - self.in_len);
            self.inbuf[self.in_len..self.in_len + n]
                .copy_from_slice(&self.prefix[self.prefix_pos..self.prefix_pos + n]);
            self.in_len += n;
            self.prefix_pos += n;
        }
        if self.in_len < INPUT_CHUNK && !self.exhausted {
            match self.src.read(&mut self.inbuf[self.in_len..]) {
                Ok(0) => self.exhausted = true,
                Ok(n) => self.in_len += n,
                Err(e) => {
                    return Err(if self.src.was_limit_exceeded() {
                        limit_error("max_bytes", self.options.limits.max_bytes, None)
                    } else {
                        DocumentError::with_source(DocumentErrorKind::Io, FORMAT, e)
                    });
                }
            }
        }
        Ok(())
    }
}

/// Rejects fields over `max_field_bytes`: audits every completed
/// field boundary written so far (`checked_endlen` advances across
/// calls) plus the in-flight tail of the field under construction.
fn check_field_sizes(
    record: &CsvRecord,
    endlen: usize,
    outlen: usize,
    checked_endlen: &mut usize,
    max_field_bytes: usize,
    position: DocumentPosition,
) -> Result<(), DocumentError> {
    while *checked_endlen < endlen {
        let end = record.ends[*checked_endlen];
        let start = if *checked_endlen == 0 {
            0
        } else {
            record.ends[*checked_endlen - 1]
        };
        if end - start > max_field_bytes {
            return Err(limit_error(
                "max_field_bytes",
                u64::try_from(max_field_bytes).unwrap_or(u64::MAX),
                Some(position),
            ));
        }
        *checked_endlen += 1;
    }
    let last_end = if endlen == 0 {
        0
    } else {
        record.ends[endlen - 1]
    };
    if outlen - last_end > max_field_bytes {
        return Err(limit_error(
            "max_field_bytes",
            u64::try_from(max_field_bytes).unwrap_or(u64::MAX),
            Some(position),
        ));
    }
    Ok(())
}

/// Doubles `record`'s buffers when the parser reports them full —
/// never beyond `max_record_bytes` / `max_fields`, which fail with
/// `LimitExceeded` instead of growing.
fn grow_record_buffers(
    record: &mut CsvRecord,
    result: &csv_core::ReadRecordResult,
    options: &CsvReadOptions,
    position: DocumentPosition,
) -> Result<(), DocumentError> {
    match result {
        csv_core::ReadRecordResult::OutputFull => {
            if record.data.len() >= options.max_record_bytes {
                return Err(limit_error(
                    "max_record_bytes",
                    u64::try_from(options.max_record_bytes).unwrap_or(u64::MAX),
                    Some(position),
                ));
            }
            let cap = record
                .data
                .len()
                .saturating_mul(2)
                .min(options.max_record_bytes);
            record.data.resize(cap, 0);
        }
        csv_core::ReadRecordResult::OutputEndsFull => {
            if record.ends.len() >= options.max_fields {
                return Err(limit_error(
                    "max_fields",
                    u64::try_from(options.max_fields).unwrap_or(u64::MAX),
                    Some(position),
                ));
            }
            let cap = record.ends.len().saturating_mul(2).min(options.max_fields);
            record.ends.resize(cap, 0);
        }
        _ => {}
    }
    Ok(())
}

/// Sequential CSV writer over a borrowed output stream.
///
/// Records are buffered internally; [`finish`](Self::finish) flushes
/// the remainder to the writer. Success of `finish` means the buffered
/// output reached the writer — file sync/publish are separate
/// guarantees (see the `file` feature's staged publish).
///
/// **RT contract:** non-RT only — may block on I/O and allocate.
#[derive(Debug)]
pub struct CsvWriter<'a, W: Write> {
    /// Buffered backend writer over the borrowed stream.
    inner: csv::Writer<&'a mut W>,
}

impl<'a, W: Write> CsvWriter<'a, W> {
    /// Builds a writer over `writer` under `options`.
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// [`DocumentErrorKind::Unsupported`] for a contradictory dialect
    /// (`delimiter == quote`).
    pub fn new(writer: &'a mut W, options: &CsvWriteOptions) -> Result<Self, DocumentError> {
        let options = *options;
        validate_dialect(&options.dialect)?;
        let mut builder = csv::WriterBuilder::new();
        builder
            .delimiter(options.dialect.delimiter)
            .quote(options.dialect.quote)
            .terminator(writer_terminator(options.dialect.terminator))
            .quote_style(writer_quote_style(options.quote_style))
            .double_quote(options.dialect.double_quote)
            .comment(options.dialect.comment)
            .has_headers(options.has_headers)
            .flexible(options.flexible);
        if let Some(escape) = options.dialect.escape {
            builder.escape(escape);
        }
        Ok(Self {
            inner: builder.from_writer(writer),
        })
    }

    /// Writes one record from an iterator of byte-viewable fields —
    /// e.g. `record.iter()` or `["a", "b"]`.
    ///
    /// **RT contract:** non-RT only — may allocate.
    ///
    /// # Errors
    ///
    /// `DataModel` for a field-count mismatch when `flexible` is
    /// `false`, `Io` for writer failures.
    pub fn write_record<I, S>(&mut self, record: I) -> Result<(), DocumentError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<[u8]>,
    {
        self.inner
            .write_record(record)
            .map_err(|e| map_csv_error(e, None))
    }

    /// Serializes `value` as one record; emits the header row on the
    /// first call when `has_headers` is enabled and `T` carries field
    /// names.
    ///
    /// **RT contract:** non-RT only — may allocate.
    ///
    /// # Errors
    ///
    /// `DataModel` for serialize failures, `Io` for writer failures.
    pub fn write_typed<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), DocumentError> {
        self.inner
            .serialize(value)
            .map_err(|e| map_csv_error(e, None))
    }

    /// Flushes all buffered output to the writer.
    ///
    /// Success means the backend's buffered bytes were handed to the
    /// writer — not flushed to OS buffers, synced or published.
    ///
    /// **RT contract:** non-RT only — may block on I/O.
    ///
    /// # Errors
    ///
    /// `Io` when the final flush fails.
    pub fn finish(mut self) -> Result<(), DocumentError> {
        self.inner
            .flush()
            .map_err(|e| DocumentError::with_source(DocumentErrorKind::Io, FORMAT, e))
    }
}

/// Deserializes every record of the input as `T`, collecting them into
/// a `Vec`.
///
/// Equivalent to driving a [`CsvReader`] with
/// [`read_typed`](CsvReader::read_typed) until `Ok(false)`; all limits
/// apply — the output `Vec` is bounded by `max_records`.
///
/// **RT contract:** non-RT only — may block on I/O and allocate.
///
/// # Errors
///
/// The [`CsvReader`] contract.
///
/// # Examples
///
/// ```
/// use ruviform_io_structured::csv;
/// use serde::Deserialize;
///
/// #[derive(Debug, PartialEq, Deserialize)]
/// struct Row { city: String, pop: u64 }
///
/// let mut input = b"city,pop\nBoston,4628910\n".as_slice();
/// let rows: Vec<Row> = csv::from_reader(&mut input, &csv::CsvReadOptions::default())?;
/// assert_eq!(rows, vec![Row { city: "Boston".into(), pop: 4628910 }]);
/// # Ok::<(), ruviform_io_structured::DocumentError>(())
/// ```
pub fn from_reader<T: DeserializeOwned>(
    reader: &mut impl Read,
    options: &CsvReadOptions,
) -> Result<Vec<T>, DocumentError> {
    let mut reader = CsvReader::new(reader, options)?;
    let mut rows = Vec::new();
    while let Some(row) = reader.read_typed()? {
        rows.push(row);
    }
    Ok(rows)
}

/// Serializes `records` as one CSV document into `writer`.
///
/// Equivalent to driving a [`CsvWriter`] over `records` and finishing
/// it: on success the writer holds the complete document.
///
/// **RT contract:** non-RT only — may block on I/O and allocate.
///
/// # Errors
///
/// The [`CsvWriter`] contract.
///
/// # Examples
///
/// ```
/// use ruviform_io_structured::csv;
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct Row { city: &'static str, pop: u64 }
///
/// let mut out = Vec::new();
/// let rows = [Row { city: "Boston", pop: 4628910 }];
/// csv::to_writer(&mut out, &rows, &csv::CsvWriteOptions::default())?;
/// assert_eq!(out, b"city,pop\r\nBoston,4628910\r\n");
/// # Ok::<(), ruviform_io_structured::DocumentError>(())
/// ```
pub fn to_writer<T: Serialize>(
    writer: &mut impl Write,
    records: &[T],
    options: &CsvWriteOptions,
) -> Result<(), DocumentError> {
    let mut wtr = CsvWriter::new(writer, options)?;
    for record in records {
        wtr.write_typed(record)?;
    }
    wtr.finish()
}

/// `feature = "file"` convenience: opens `path` and collects records
/// through [`from_reader`] via a buffered reader.
///
/// **RT contract:** non-RT only.
///
/// # Errors
///
/// [`DocumentFileReadError::Open`](crate::DocumentFileReadError) on
/// open failure, `Decode` on document errors.
#[cfg(feature = "file")]
pub fn read_file<T: DeserializeOwned>(
    path: impl AsRef<std::path::Path>,
    options: &CsvReadOptions,
) -> Result<Vec<T>, crate::DocumentFileReadError> {
    crate::file::open_document(path, |reader| from_reader(reader, options))
}

/// `feature = "file"` convenience: serializes `records` into a
/// [`ruviform_io_file::StagedFile`], seals and publishes it.
///
/// Encode failure is `Encode` (the staged temp is dropped for
/// best-effort cleanup and the destination is never touched); publish
/// failure is `Publish` with the full
/// [`ruviform_io_file::PublishFailure`] preserved.
///
/// **RT contract:** non-RT only.
///
/// # Errors
///
/// [`DocumentFileWriteError`](crate::DocumentFileWriteError).
#[cfg(feature = "file")]
pub fn write_file<T: Serialize>(
    path: impl AsRef<std::path::Path>,
    records: &[T],
    options: &CsvWriteOptions,
    publish: ruviform_io_file::PublishOptions,
) -> Result<ruviform_io_file::PublishReceipt, crate::DocumentFileWriteError> {
    crate::file::write_document(path, publish, |writer| to_writer(writer, records, options))
}

/// Builds the `csv-core` engine for `dialect`.
fn core_reader(dialect: &CsvDialect) -> csv_core::Reader {
    let mut builder = csv_core::ReaderBuilder::new();
    builder
        .delimiter(dialect.delimiter)
        .quote(dialect.quote)
        .terminator(core_terminator(dialect.terminator))
        .double_quote(dialect.double_quote)
        .quoting(dialect.quoting)
        .comment(dialect.comment)
        .escape(dialect.escape);
    builder.build()
}

/// Maps the public terminator onto `csv_core`.
fn core_terminator(terminator: CsvTerminator) -> csv_core::Terminator {
    match terminator {
        CsvTerminator::CrLf => csv_core::Terminator::CRLF,
        CsvTerminator::Any(byte) => csv_core::Terminator::Any(byte),
    }
}

/// Maps the public terminator onto `csv`'s writer.
fn writer_terminator(terminator: CsvTerminator) -> csv::Terminator {
    match terminator {
        CsvTerminator::CrLf => csv::Terminator::CRLF,
        CsvTerminator::Any(byte) => csv::Terminator::Any(byte),
    }
}

/// Maps the public quote style onto `csv`'s writer.
fn writer_quote_style(style: CsvQuoteStyle) -> csv::QuoteStyle {
    match style {
        CsvQuoteStyle::Necessary => csv::QuoteStyle::Necessary,
        CsvQuoteStyle::Always => csv::QuoteStyle::Always,
        CsvQuoteStyle::NonNumeric => csv::QuoteStyle::NonNumeric,
        CsvQuoteStyle::Never => csv::QuoteStyle::Never,
    }
}

/// Rejects dialects the backend cannot honor honestly.
fn validate_dialect(dialect: &CsvDialect) -> Result<(), DocumentError> {
    if dialect.delimiter == dialect.quote {
        return Err(DocumentError::with_source(
            DocumentErrorKind::Unsupported,
            FORMAT,
            BackendMessage("delimiter and quote must differ".into()),
        ));
    }
    Ok(())
}

/// `LimitExceeded` carrying the exceeded option name, ceiling and
/// record position.
fn limit_error(name: &'static str, max: u64, position: Option<DocumentPosition>) -> DocumentError {
    DocumentError::with_parts(
        DocumentErrorKind::LimitExceeded,
        FORMAT,
        position,
        Some(DocumentLimit { name, max }),
        None,
    )
}

/// Maps a backend `csv::Error` onto the normalized model: I/O stays
/// `Io`, UTF-8 failures `InvalidEncoding`, record↔`T` shape failures
/// `DataModel`. The backend diagnostic is attached as `source`.
fn map_csv_error(e: csv::Error, fallback: Option<DocumentPosition>) -> DocumentError {
    let (kind, position) = match e.kind() {
        csv::ErrorKind::Io(_) => (DocumentErrorKind::Io, None),
        csv::ErrorKind::Utf8 { pos, .. } => (DocumentErrorKind::InvalidEncoding, pos.as_ref()),
        csv::ErrorKind::UnequalLengths { pos, .. } => (DocumentErrorKind::DataModel, pos.as_ref()),
        csv::ErrorKind::Deserialize { pos, err } => {
            if matches!(err.kind(), csv::DeserializeErrorKind::InvalidUtf8(_)) {
                (DocumentErrorKind::InvalidEncoding, pos.as_ref())
            } else {
                (DocumentErrorKind::DataModel, pos.as_ref())
            }
        }
        _ => (DocumentErrorKind::DataModel, None),
    };
    DocumentError::with_parts(
        kind,
        FORMAT,
        position.map(backend_position).or(fallback),
        None,
        Some(Box::new(e)),
    )
}

/// Converts a backend position (byte/line/record) to the shared type.
fn backend_position(pos: &csv::Position) -> DocumentPosition {
    DocumentPosition {
        byte: Some(pos.byte()),
        line: Some(pos.line()),
        column: None,
        record: Some(pos.record()),
    }
}

/// Private diagnostic carrying a plain message — used when no backend
/// error object exists (option validation, BOM rejection).
#[derive(Debug)]
struct BackendMessage(String);

impl std::fmt::Display for BackendMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for BackendMessage {}
