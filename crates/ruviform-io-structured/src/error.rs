//! Normalized [`DocumentError`] / [`DocumentErrorKind`] model shared by
//! every structured-format module (`json`, `yaml`, `toml`, and the
//! future `csv`/`xml` modules of IO-IMP-04/05).
//!
//! The enum is the stable, matchable surface; backend parser errors are
//! kept only as a type-erased diagnostic reachable through
//! [`std::error::Error::source`]. No backend error type ever appears in
//! the public API — `serde` traits are the sole allowed exception.
//!
//! [`DocumentError::fmt`] deliberately stays minimal (kind, format,
//! position, limit) and never echoes the backend-rendered message or an
//! input snippet. Backend diagnostics may embed input context — reach
//! them explicitly through `source()` when needed; they are diagnostics,
//! not the matchable contract.

use std::fmt;

/// Structured document format — identifies which parser produced an
/// error or consumed an options struct.
///
/// `Csv`/`Xml` variants are added by IO-IMP-04 (#298) / IO-IMP-05
/// (#299); this enum is the shared scaffold they extend.
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DocumentFormat {
    /// JSON (`feature = "json"`, backend: `serde_json`).
    Json,
    /// YAML (`feature = "yaml"`, backend: `serde-saphyr`).
    Yaml,
    /// TOML (`feature = "toml"`, backend: `toml`).
    Toml,
    /// CSV (`feature = "csv"`, backends: `csv-core` + `csv`).
    Csv,
}

impl fmt::Display for DocumentFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::Json => "json",
            Self::Yaml => "yaml",
            Self::Toml => "toml",
            Self::Csv => "csv",
        };
        f.write_str(message)
    }
}

/// Normalized failure classification for structured document I/O.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy — new variants may be added without breaking downstream
/// `match` arms.
///
/// **RT contract:** non-RT only.
///
/// # Examples
///
/// ```
/// use ruviform_io_structured::DocumentErrorKind;
///
/// assert_eq!(DocumentErrorKind::TrailingData.to_string(), "trailing data");
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DocumentErrorKind {
    /// Underlying I/O failed (read or write side).
    Io,
    /// The input bytes are not valid in the format's required encoding
    /// (e.g. non-UTF-8 input for a `&str`-driven backend).
    InvalidEncoding,
    /// The input violates the format grammar (malformed JSON/YAML/TOML,
    /// premature end of input, or a backend-internal structural limit
    /// the format does not expose as an option).
    Syntax,
    /// The document is grammatically valid but does not fit the target
    /// type or the format's data model (type mismatch, missing field,
    /// value out of range, unrepresentable value on serialize).
    DataModel,
    /// Non-whitespace data follows the end of a single document —
    /// including a second YAML document in a single-document API.
    TrailingData,
    /// An enforced resource limit was exceeded (input byte budget,
    /// YAML depth/alias budget, …). [`DocumentError::limit`] names the
    /// exceeded limit when known.
    LimitExceeded,
    /// The requested capability or format feature is not supported by
    /// this adapter (e.g. a disabled format feature) — reported, never
    /// silently ignored.
    Unsupported,
}

impl fmt::Display for DocumentErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::Io => "I/O failure",
            Self::InvalidEncoding => "invalid encoding",
            Self::Syntax => "syntax error",
            Self::DataModel => "data model mismatch",
            Self::TrailingData => "trailing data",
            Self::LimitExceeded => "limit exceeded",
            Self::Unsupported => "unsupported",
        };
        f.write_str(message)
    }
}

/// Which enforced limit was exceeded — the matchable half of a
/// [`DocumentErrorKind::LimitExceeded`] error.
///
/// `name` is a stable identifier (`"max_bytes"`, `"max_depth"`,
/// `"max_aliases"`, …) naming the public option that was exceeded;
/// `max` is the enforced ceiling. Limits the backend enforces but
/// cannot attribute precisely are reported as `None` on
/// [`DocumentError::limit`].
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DocumentLimit {
    /// Stable identifier of the exceeded limit (a public option name).
    pub name: &'static str,
    /// The enforced maximum that was exceeded.
    pub max: u64,
}

impl fmt::Display for DocumentLimit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (max {})", self.name, self.max)
    }
}

/// Position of a failure inside the input document.
///
/// Every component is optional: the position is filled only with what
/// the backend actually reports (or what is exactly derivable, e.g. a
/// UTF-8 boundary offset) — never guessed.
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DocumentPosition {
    /// Byte offset into the input (0-based), when known.
    pub byte: Option<u64>,
    /// 1-based line number, when known.
    pub line: Option<u64>,
    /// 1-based column number, when known.
    pub column: Option<u64>,
    /// 0-based record index, when the format is record-oriented (CSV).
    pub record: Option<u64>,
}

impl fmt::Display for DocumentPosition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut sep = "";
        for (label, value) in [
            ("line", self.line),
            ("column", self.column),
            ("byte", self.byte),
            ("record", self.record),
        ] {
            if let Some(v) = value {
                write!(f, "{sep}{label} {v}")?;
                sep = ", ";
            }
        }
        if sep.is_empty() {
            f.write_str("unknown position")?;
        }
        Ok(())
    }
}

/// Structured-document error: normalized [`DocumentErrorKind`]
/// classification, producing [`DocumentFormat`], optional position and
/// limit detail, and a type-erased backend diagnostic.
///
/// `DocumentError` is `Send`: it may cross thread boundaries like any
/// other data type.
///
/// **RT contract:** non-RT only — may allocate (boxed diagnostic).
///
/// # Examples
///
/// ```
/// use ruviform_io_structured::{DocumentError, DocumentErrorKind, DocumentFormat};
///
/// let error = DocumentError::from_kind(DocumentErrorKind::Unsupported, DocumentFormat::Yaml);
/// assert_eq!(error.kind(), DocumentErrorKind::Unsupported);
/// assert_eq!(error.format(), DocumentFormat::Yaml);
/// ```
#[derive(Debug)]
pub struct DocumentError {
    /// Normalized classification — the stable, matchable surface.
    kind: DocumentErrorKind,
    /// The document format that produced the error.
    format: DocumentFormat,
    /// Input position, when the backend reports one.
    position: Option<DocumentPosition>,
    /// The exceeded limit, when known.
    limit: Option<DocumentLimit>,
    /// Type-erased backend diagnostic, exposed via
    /// [`std::error::Error::source`]. Backend types stay private;
    /// the diagnostic's own `Display` may embed input context and is
    /// never folded into this type's `Display`.
    source: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
}

impl DocumentError {
    /// Builds an error carrying only a normalized classification.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn from_kind(kind: DocumentErrorKind, format: DocumentFormat) -> Self {
        Self {
            kind,
            format,
            position: None,
            limit: None,
            source: None,
        }
    }

    /// Builds an error with a type-erased backend diagnostic attached.
    ///
    /// **RT contract:** non-RT only.
    pub fn with_source(
        kind: DocumentErrorKind,
        format: DocumentFormat,
        source: impl Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
    ) -> Self {
        Self {
            kind,
            format,
            position: None,
            limit: None,
            source: Some(source.into()),
        }
    }

    /// Full-detail constructor used by the format modules.
    #[cfg(any(feature = "json", feature = "yaml", feature = "toml", feature = "csv"))]
    pub(crate) fn with_parts(
        kind: DocumentErrorKind,
        format: DocumentFormat,
        position: Option<DocumentPosition>,
        limit: Option<DocumentLimit>,
        source: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
    ) -> Self {
        Self {
            kind,
            format,
            position,
            limit,
            source,
        }
    }

    /// The normalized classification.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn kind(&self) -> DocumentErrorKind {
        self.kind
    }

    /// The document format that produced the error.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn format(&self) -> DocumentFormat {
        self.format
    }

    /// Input position, when known.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn position(&self) -> Option<DocumentPosition> {
        self.position
    }

    /// The exceeded limit, when known. Present only on
    /// [`DocumentErrorKind::LimitExceeded`] errors that can be
    /// attributed to a named option.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn limit(&self) -> Option<DocumentLimit> {
        self.limit
    }

    /// Attaches an exceeded-limit detail (crate-internal).
    #[cfg(any(feature = "json", feature = "yaml", feature = "toml"))]
    pub(crate) fn with_limit(mut self, limit: DocumentLimit) -> Self {
        self.limit = Some(limit);
        self
    }
}

impl fmt::Display for DocumentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ruviform-io-structured: {}: {}", self.format, self.kind)?;
        if let Some(limit) = &self.limit {
            write!(f, " ({limit})")?;
        }
        if let Some(position) = &self.position {
            write!(f, " at {position}")?;
        }
        Ok(())
    }
}

impl std::error::Error for DocumentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source
            .as_deref()
            .map(|source| source as &(dyn std::error::Error + 'static))
    }
}
