//! JSON document I/O (`feature = "json"`).
//!
//! Single-document [`from_reader`] / [`to_writer`] over borrowed
//! `std::io` streams. The deserializer validates end-of-input:
//! non-whitespace bytes after the document are
//! [`DocumentErrorKind::TrailingData`], and input is hard-capped by
//! [`JsonReadOptions::limits`].
//!
//! **RT contract:** non-RT only — may block on I/O and allocate.

use crate::input::BoundedRead;
use crate::{
    DocumentError, DocumentErrorKind, DocumentFormat, DocumentLimit, DocumentPosition, InputLimits,
};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::error::Category;
use std::io::{Read, Write};

const FORMAT: DocumentFormat = DocumentFormat::Json;

/// Read options for [`from_reader`].
///
/// `#[non_exhaustive]` — new knobs may be added; construct with
/// [`JsonReadOptions::new`] or [`Default`], then adjust fields.
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Debug, Clone, Copy)]
pub struct JsonReadOptions {
    /// Hard input limits (default 16 MiB, see [`InputLimits`]).
    pub limits: InputLimits,
}

impl JsonReadOptions {
    /// Options with [`InputLimits::default`].
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            limits: InputLimits::new(crate::DEFAULT_MAX_INPUT_BYTES),
        }
    }

    /// Overrides `limits`.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn with_limits(mut self, limits: InputLimits) -> Self {
        self.limits = limits;
        self
    }
}

impl Default for JsonReadOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// Write options for [`to_writer`].
///
/// `#[non_exhaustive]` — new knobs may be added; construct with
/// [`JsonWriteOptions::new`] or [`Default`], then adjust fields.
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Debug, Clone, Copy)]
pub struct JsonWriteOptions {
    /// `true` — human-readable indented layout; `false` (default) —
    /// compact single-line output.
    pub pretty: bool,
}

impl JsonWriteOptions {
    /// Options with `pretty = false`.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn new() -> Self {
        Self { pretty: false }
    }

    /// Overrides `pretty`.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn with_pretty(mut self, pretty: bool) -> Self {
        self.pretty = pretty;
        self
    }
}

impl Default for JsonWriteOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// Deserializes a single JSON document of type `T` from `reader`.
///
/// Reads at most `options.limits.max_bytes` bytes from `reader`
/// ([`DocumentErrorKind::LimitExceeded`] past the budget). The document
/// must end the input: non-whitespace trailing data is
/// [`DocumentErrorKind::TrailingData`]. A prematurely ended document is
/// [`DocumentErrorKind::Syntax`]; grammatically valid JSON that does
/// not fit `T` is [`DocumentErrorKind::DataModel`].
///
/// Note: JSON parse depth is bounded by the backend's fixed internal
/// limit; deeper-than-supported input is reported as `Syntax` — it is
/// not a configurable [`InputLimits`] budget.
///
/// **RT contract:** non-RT only.
///
/// # Errors
///
/// Returns [`DocumentError`] classified per the contract above.
///
/// # Examples
///
/// ```
/// use ruviform_io_structured::json;
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct Cfg { name: String }
///
/// let mut input = br#"{"name":"demo"}"#.as_slice();
/// let cfg: Cfg = json::from_reader(&mut input, &json::JsonReadOptions::default())?;
/// assert_eq!(cfg.name, "demo");
/// # Ok::<(), ruviform_io_structured::DocumentError>(())
/// ```
pub fn from_reader<T: DeserializeOwned>(
    reader: &mut impl Read,
    options: &JsonReadOptions,
) -> Result<T, DocumentError> {
    let mut bounded = BoundedRead::new(reader, options.limits.max_bytes);
    let mut de = serde_json::Deserializer::from_reader(&mut bounded);
    let result = T::deserialize(&mut de);
    // `Deserializer::end()` only fails on `TrailingCharacters` (or a
    // wrapped I/O error — the budget trip is checked first).
    let end_result = match &result {
        Ok(_) => Some(de.end()),
        Err(_) => None,
    };
    // `de`'s borrow of `bounded` ended above — the limit flag is
    // readable again.
    let limit_tripped = bounded.was_limit_exceeded();
    match (result, end_result) {
        (Ok(value), Some(Ok(()))) => Ok(value),
        (Ok(_), Some(Err(e))) => Err(map_error(e, limit_tripped, options.limits.max_bytes, true)),
        (Ok(_), None) => unreachable!("end() runs whenever deserialize succeeds"),
        (Err(e), _) => Err(map_error(e, limit_tripped, options.limits.max_bytes, false)),
    }
}

/// Serializes `value` as a JSON document into `writer`.
///
/// Success means the serializer's output reached `writer` — not that it
/// was flushed, synced or published. Failure may leave a partial
/// document behind.
///
/// **RT contract:** non-RT only.
///
/// # Errors
///
/// `Io` for writer failures, `DataModel` for unserializable values
/// (e.g. a map with a non-string key type).
///
/// # Examples
///
/// ```
/// use ruviform_io_structured::json;
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct Cfg { name: String }
///
/// let mut out = Vec::new();
/// json::to_writer(&mut out, &Cfg { name: "demo".into() }, &json::JsonWriteOptions::default())?;
/// assert_eq!(out, br#"{"name":"demo"}"#);
/// # Ok::<(), ruviform_io_structured::DocumentError>(())
/// ```
pub fn to_writer<T: Serialize + ?Sized>(
    writer: &mut impl Write,
    value: &T,
    options: &JsonWriteOptions,
) -> Result<(), DocumentError> {
    let result = if options.pretty {
        serde_json::to_writer_pretty(&mut *writer, value)
    } else {
        serde_json::to_writer(&mut *writer, value)
    };
    result.map_err(|e| {
        let kind = if e.is_io() {
            DocumentErrorKind::Io
        } else {
            // Serialize failures are value/model problems, never
            // "syntax": the input side does not exist here.
            DocumentErrorKind::DataModel
        };
        DocumentError::with_parts(
            kind,
            FORMAT,
            Some(position(e.line(), e.column())),
            None,
            Some(Box::new(e)),
        )
    })
}

/// Maps a `serde_json` failure to the normalized error model.
///
/// `end_phase` marks the post-document `Deserializer::end()` check: any
/// non-I/O failure there is deterministically `TrailingCharacters`
/// → [`DocumentErrorKind::TrailingData`]. The byte-budget trip flag is
/// authoritative over the backend's I/O classification.
fn map_error(
    e: serde_json::Error,
    limit_tripped: bool,
    max_bytes: u64,
    end_phase: bool,
) -> DocumentError {
    if limit_tripped {
        return DocumentError::from_kind(DocumentErrorKind::LimitExceeded, FORMAT).with_limit(
            DocumentLimit {
                name: "max_bytes",
                max: max_bytes,
            },
        );
    }
    let kind = if end_phase {
        match e.classify() {
            Category::Io => DocumentErrorKind::Io,
            _ => DocumentErrorKind::TrailingData,
        }
    } else {
        match e.classify() {
            Category::Io => DocumentErrorKind::Io,
            // `Eof` = prematurely ended document; `Syntax` covers both
            // malformed input and the backend's fixed recursion limit.
            Category::Syntax | Category::Eof => DocumentErrorKind::Syntax,
            Category::Data => DocumentErrorKind::DataModel,
        }
    };
    DocumentError::with_parts(
        kind,
        FORMAT,
        Some(position(e.line(), e.column())),
        None,
        Some(Box::new(e)),
    )
}

fn position(line: usize, column: usize) -> DocumentPosition {
    DocumentPosition {
        byte: None,
        line: Some(line as u64),
        column: Some(column as u64),
    }
}

/// `feature = "file"` convenience: opens `path` and runs
/// [`from_reader`] through a buffered reader.
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
    options: &JsonReadOptions,
) -> Result<T, crate::DocumentFileReadError> {
    crate::file::open_document(path, |reader| from_reader(reader, options))
}

/// `feature = "file"` convenience: serializes `value` into a
/// [`ruviform_io_file::StagedFile`], seals and publishes it.
///
/// Encode failure is `Encode` (the staged temp is dropped for
/// best-effort cleanup and the destination is never touched); publish
/// failure is `Publish` with the full
/// [`ruviform_io_file::PublishFailure`] (effect, phase, recovery
/// handle) preserved.
///
/// **RT contract:** non-RT only.
///
/// # Errors
///
/// [`DocumentFileWriteError`](crate::DocumentFileWriteError).
#[cfg(feature = "file")]
pub fn write_file<T: Serialize + ?Sized>(
    path: impl AsRef<std::path::Path>,
    value: &T,
    options: &JsonWriteOptions,
    publish: ruviform_io_file::PublishOptions,
) -> Result<ruviform_io_file::PublishReceipt, crate::DocumentFileWriteError> {
    crate::file::write_document(path, publish, |writer| to_writer(writer, value, options))
}
