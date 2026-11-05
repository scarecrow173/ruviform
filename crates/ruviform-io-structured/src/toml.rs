//! TOML document I/O (`feature = "toml"`).
//!
//! Single-document [`from_reader`] / [`to_writer`]. TOML has no
//! streaming reader in this adapter: the input is buffered under
//! [`TomlReadOptions::limits`], decoded as UTF-8
//! ([`DocumentErrorKind::InvalidEncoding`]) and parsed as one complete
//! finite document — trailing data is indistinguishable from a
//! malformed document at the grammar level and is reported as
//! [`DocumentErrorKind::Syntax`].
//!
//! Decoding runs in two phases so the classification stays honest:
//! first the document is parsed into the TOML DOM
//! ([`DocumentErrorKind::Syntax`]), then the DOM is deserialized into
//! `T` ([`DocumentErrorKind::DataModel`]).
//!
//! **RT contract:** non-RT only — may block on I/O and allocate.

use crate::input::read_to_string;
use crate::{DocumentError, DocumentErrorKind, DocumentFormat, DocumentPosition, InputLimits};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::io::{Read, Write};

const FORMAT: DocumentFormat = DocumentFormat::Toml;

/// Read options for [`from_reader`].
///
/// `#[non_exhaustive]` — new knobs may be added; construct with
/// [`TomlReadOptions::new`] or [`Default`], then adjust fields.
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Debug, Clone, Copy)]
pub struct TomlReadOptions {
    /// Hard input limits (default 16 MiB, see [`InputLimits`]).
    ///
    /// The limit is a hard ceiling on bytes pulled from the reader, not
    /// a bound on the resulting document's size after parse.
    pub limits: InputLimits,
}

impl TomlReadOptions {
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

impl Default for TomlReadOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// Write options for [`to_writer`].
///
/// `#[non_exhaustive]` — new knobs may be added; construct with
/// [`TomlWriteOptions::new`] or [`Default`], then adjust fields.
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Debug, Clone, Copy)]
pub struct TomlWriteOptions {
    /// `true` — human-readable pretty layout; `false` (default) —
    /// compact layout. TOML is inherently line-oriented, so both modes
    /// remain a single document.
    pub pretty: bool,
}

impl TomlWriteOptions {
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

impl Default for TomlWriteOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// Deserializes a single TOML document of type `T` from `reader`.
///
/// Reads at most `options.limits.max_bytes` bytes from `reader`
/// ([`DocumentErrorKind::LimitExceeded`] past the budget), requires
/// UTF-8 input ([`DocumentErrorKind::InvalidEncoding`]), then parses the
/// complete document into the TOML DOM
/// ([`DocumentErrorKind::Syntax`]) and deserializes it into `T`
/// ([`DocumentErrorKind::DataModel`]).
///
/// Unlike JSON, TOML's grammar has no separate "trailing data" state:
/// bytes after a would-be document are simply an invalid continuation
/// and are reported as `Syntax`, not
/// [`DocumentErrorKind::TrailingData`].
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
/// use ruviform_io_structured::toml;
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct Cfg { name: String }
///
/// let mut input = br#"name = "demo""#.as_slice();
/// let cfg: Cfg = toml::from_reader(&mut input, &toml::TomlReadOptions::default())?;
/// assert_eq!(cfg.name, "demo");
/// # Ok::<(), ruviform_io_structured::DocumentError>(())
/// ```
pub fn from_reader<T: DeserializeOwned>(
    reader: &mut impl Read,
    options: &TomlReadOptions,
) -> Result<T, DocumentError> {
    let input = read_to_string(reader, options.limits, FORMAT)?;
    // Phase 1 — grammar: parse into the TOML DOM. Any failure here is
    // a document-format violation.
    let dom = toml::from_str::<toml::Table>(&input)
        .map_err(|e| map_de_error(&e, DocumentErrorKind::Syntax))?;
    // Phase 2 — data model: grammar is proven, so a DOM → T failure is
    // always a structural/type mismatch.
    dom.try_into::<T>()
        .map_err(|e| map_de_error(&e, DocumentErrorKind::DataModel))
}

/// Serializes `value` as a single TOML document into `writer`.
///
/// Success means the serializer's output reached `writer` — not that it
/// was flushed, synced or published. Failure may leave a partial
/// document behind.
///
/// **RT contract:** non-RT only.
///
/// # Errors
///
/// `DataModel` for unserializable values (e.g. a top-level scalar, a
/// non-string map key, or `None` at a position TOML cannot express),
/// `Io` for writer failures.
///
/// # Examples
///
/// ```
/// use ruviform_io_structured::toml;
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct Cfg { name: String }
///
/// let mut out = Vec::new();
/// toml::to_writer(&mut out, &Cfg { name: "demo".into() }, &toml::TomlWriteOptions::default())?;
/// assert_eq!(out, b"name = \"demo\"\n");
/// # Ok::<(), ruviform_io_structured::DocumentError>(())
/// ```
pub fn to_writer<T: Serialize + ?Sized>(
    writer: &mut impl Write,
    value: &T,
    options: &TomlWriteOptions,
) -> Result<(), DocumentError> {
    let encoded = if options.pretty {
        toml::to_string_pretty(value)
    } else {
        toml::to_string(value)
    }
    .map_err(|e| {
        // Serialize failures are value/model problems.
        DocumentError::with_source(DocumentErrorKind::DataModel, FORMAT, e)
    })?;
    writer
        .write_all(encoded.as_bytes())
        .map_err(|e| DocumentError::with_source(DocumentErrorKind::Io, FORMAT, e))
}

/// Maps a `toml::de::Error` to the normalized model, attaching the
/// backend's byte span when reported.
fn map_de_error(e: &toml::de::Error, kind: DocumentErrorKind) -> DocumentError {
    let position = e.span().map(|span| DocumentPosition {
        byte: Some(span.start as u64),
        line: None,
        column: None,
        record: None,
    });
    // The backend `Display` renders the offending input line; keep the
    // source diagnostic to the bare message so snippets never travel
    // through the error chain unconditionally.
    DocumentError::with_parts(
        kind,
        FORMAT,
        position,
        None,
        Some(Box::new(BackendMessage(e.message().to_owned()))),
    )
}

/// Private diagnostic wrapper exposing only the backend's message
/// text — the real `toml::de::Error` `Display` embeds input lines.
#[derive(Debug)]
struct BackendMessage(String);

impl std::fmt::Display for BackendMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for BackendMessage {}

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
    options: &TomlReadOptions,
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
    options: &TomlWriteOptions,
    publish: ruviform_io_file::PublishOptions,
) -> Result<ruviform_io_file::PublishReceipt, crate::DocumentFileWriteError> {
    crate::file::write_document(path, publish, |writer| to_writer(writer, value, options))
}
