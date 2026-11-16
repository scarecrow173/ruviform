//! C3 value model: the closed [`ScriptValue`] variant set, source and
//! artifact provenance types, and script error/diagnostic vocabulary.
//!
//! Invariants preserved from the RF-SCRIPT-00 contract:
//!
//! - [`ScriptValue::Absent`] and [`ScriptValue::Null`] are distinct.
//! - [`ScriptValue::Integer`] is a strict `i64`; values that cannot
//!   cross a vendor boundary safely are tagged by the codec.
//! - [`ScriptValue::Time`] keeps its domain, rational timebase, and
//!   count — no implicit unit conversion or rounding.
//! - [`ScriptValue::ResourceId`] keeps opaque domain/kind/value.

use std::collections::BTreeMap;
use std::fmt;

use crate::canonical::{definition_digest, source_digest};
use crate::id::{ResourceDomainId, ScriptBackendId, ScriptLanguageId, TimeDomainId};

/// Schema version for [`ScriptSource`].
pub const SOURCE_SCHEMA_VERSION: u32 = 1;

/// Schema version for [`ScriptArtifact`].
pub const ARTIFACT_SCHEMA_VERSION: u32 = 1;

/// Rational number used as a [`ScriptValue::Time`] timebase.
///
/// `denominator` must be non-zero; the contract does not reduce the
/// fraction — `6/4` and `3/2` are distinct timebases.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Rational {
    /// Numerator.
    pub numerator: i64,
    /// Denominator (non-zero).
    pub denominator: i64,
}

impl Rational {
    /// Creates a rational timebase.
    #[must_use]
    pub fn new(numerator: i64, denominator: i64) -> Self {
        Self {
            numerator,
            denominator,
        }
    }
}

/// The closed neutral value model exchanged across the contract.
///
/// `Absent` ("no value / field omitted") and `Null` ("explicit null")
/// are distinct. `Integer` is a strict `i64`; `Number` is an `f64`
/// whose canonical form normalizes integral values within `2^53`.
/// `Time` and `ResourceId` carry domain-qualified opaque references —
/// the contract performs no unit conversion or dereference.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum ScriptValue {
    /// No value — distinct from [`ScriptValue::Null`]. Omitted from
    /// canonical records; has no top-level canonical form.
    Absent,
    /// Explicit null value.
    Null,
    /// Boolean.
    Bool(bool),
    /// Strict 64-bit integer.
    Integer(i64),
    /// IEEE-754 double.
    Number(f64),
    /// UTF-8 string.
    String(String),
    /// Raw bytes.
    Bytes(Vec<u8>),
    /// Ordered array.
    Array(Vec<ScriptValue>),
    /// String-keyed record. `Absent` fields are preserved in the value
    /// model but omitted from the canonical form.
    Record(BTreeMap<String, ScriptValue>),
    /// Domain-qualified rational time: `count` ticks of `timebase` in
    /// `domain`. No implicit conversion or rounding.
    Time {
        /// Time domain identifier.
        domain: TimeDomainId,
        /// Rational timebase (ticks per unit, uninterpreted).
        timebase: Rational,
        /// Tick count.
        count: i64,
    },
    /// Opaque domain-qualified resource reference.
    ResourceId {
        /// Resource domain identifier.
        domain: ResourceDomainId,
        /// Resource kind within the domain.
        kind: String,
        /// Opaque resource value.
        value: String,
    },
}

impl ScriptValue {
    /// Returns `true` for [`ScriptValue::Absent`].
    #[must_use]
    pub fn is_absent(&self) -> bool {
        matches!(self, Self::Absent)
    }
}

/// A position in a [`ScriptSource`]: absolute UTF-8 byte `offset`,
/// zero-based `line`, and zero-based `col` measured in UTF-8 bytes
/// within the line (the Luau `Position` convention).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct SourcePosition {
    /// Absolute UTF-8 byte offset from the start of the source.
    pub offset: u64,
    /// Zero-based line number.
    pub line: u32,
    /// Zero-based column in UTF-8 bytes within the line.
    pub col: u32,
}

/// A half-open source range `[start, end)` in position space.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct SourceRange {
    /// Range start (inclusive).
    pub start: SourcePosition,
    /// Range end (exclusive).
    pub end: SourcePosition,
}

/// Content digest of a script source or typed payload.
///
/// Always formatted `"sha256:v1:<64 lowercase hex>"`. Produced by
/// [`source_digest`] and [`definition_digest`]; compare by equality —
/// the digest commits to the canonical form, not the surface text.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceDigest(String);

impl SourceDigest {
    /// Returns the digest string (`"sha256:v1:<hex>"`).
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Builds a digest from a bare 64-char lowercase hex SHA-256.
    ///
    /// Intended for the canonical/digest module; consumers should use
    /// [`source_digest`] or [`definition_digest`] instead.
    #[must_use]
    pub(crate) fn from_sha256_hex(hex: &str) -> Self {
        Self(format!("sha256:v1:{hex}"))
    }
}

impl fmt::Display for SourceDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Digest of a typed artifact definition payload.
///
/// Same `"sha256:v1:<hex>"` format and canonical commitment rules as
/// [`SourceDigest`]; a distinct type so source and definition digests
/// cannot be interchanged accidentally.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DefinitionDigest(String);

impl DefinitionDigest {
    /// Returns the digest string (`"sha256:v1:<hex>"`).
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Builds a digest from a bare 64-char lowercase hex SHA-256.
    #[must_use]
    pub(crate) fn from_sha256_hex(hex: &str) -> Self {
        Self(format!("sha256:v1:{hex}"))
    }
}

impl fmt::Display for DefinitionDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Monotonic source revision assigned by the consumer or backend.
///
/// Analyzer results carry the [`ScriptSource::revision`] they were
/// computed against; consumers reject stale results by comparing
/// revisions (see [`crate::verify_result_revision`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceRevision(u64);

impl SourceRevision {
    /// Creates a revision counter value.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the raw counter value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl fmt::Display for SourceRevision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "rev:{}", self.0)
    }
}

/// A script source presented to a backend session or analyzer.
///
/// `digest` commits to `text` via [`source_digest`]; `revision`
/// identifies the source version for stale-result detection.
/// Construct with [`ScriptSource::new`] so the digest stays consistent
/// with the text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptSource {
    /// UTF-8 source text.
    pub text: String,
    /// Language the source is written in.
    pub language: ScriptLanguageId,
    /// Optional backend hint for registry resolution.
    pub backend_hint: Option<ScriptBackendId>,
    /// Schema version of this source envelope.
    pub schema_version: u32,
    /// `"sha256:v1:<hex>"` digest of `text`.
    pub digest: SourceDigest,
    /// Source revision for stale-result detection.
    pub revision: SourceRevision,
}

impl ScriptSource {
    /// Creates a source, computing `digest` from `text` and stamping
    /// [`SOURCE_SCHEMA_VERSION`].
    #[must_use]
    pub fn new(
        text: impl Into<String>,
        language: ScriptLanguageId,
        backend_hint: Option<ScriptBackendId>,
        revision: SourceRevision,
    ) -> Self {
        let text = text.into();
        Self {
            digest: source_digest(&text),
            text,
            language,
            backend_hint,
            schema_version: SOURCE_SCHEMA_VERSION,
            revision,
        }
    }
}

/// Kind of typed artifact a script evaluation may produce.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum ArtifactKind {
    /// A consumer action recipe.
    ActionRecipe,
    /// A consumer workflow definition.
    WorkflowDefinition,
    /// A consumer-defined extension artifact in `domain`.
    ConsumerExtension {
        /// Extension domain identifier.
        domain: String,
    },
}

/// Provenance pointer recording which source produced an artifact.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactSource {
    /// Digest of the producing [`ScriptSource`].
    pub source_digest: SourceDigest,
    /// Revision of the producing [`ScriptSource`].
    pub source_revision: SourceRevision,
}

/// Producer metadata recorded on an artifact.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProducedBy {
    /// Backend that produced the artifact.
    pub backend: ScriptBackendId,
    /// Backend implementation/contract version pair.
    pub version: crate::id::ScriptBackendVersion,
}

/// A typed artifact produced by script evaluation.
///
/// `digest` commits to the canonical form of `payload` — the record
/// form of the typed definition. [`ScriptArtifact::new`] computes it;
/// consumers verify `definition_digest(payload) == digest` before
/// trusting the artifact (fail-closed, F-07 family).
#[derive(Clone, Debug, PartialEq)]
pub struct ScriptArtifact {
    /// Schema version of this artifact envelope.
    pub schema_version: u32,
    /// Artifact kind.
    pub kind: ArtifactKind,
    /// `"sha256:v1:<hex>"` digest of the canonical payload.
    pub digest: DefinitionDigest,
    /// Typed payload record.
    pub payload: BTreeMap<String, ScriptValue>,
    /// Producing source provenance.
    pub source: ArtifactSource,
    /// Producer metadata.
    pub produced_by: ProducedBy,
}

impl ScriptArtifact {
    /// Creates an artifact, computing `digest` over the canonical form
    /// of `payload` and stamping [`ARTIFACT_SCHEMA_VERSION`].
    ///
    /// # Errors
    ///
    /// Returns [`crate::canonical::CanonicalError`] if the payload cannot be
    /// canonicalized (e.g. a top-level `Absent` leaf is fine inside a
    /// record only when the record omits it — nested `Absent` record
    /// fields are omitted by the canonical form).
    pub fn new(
        kind: ArtifactKind,
        payload: BTreeMap<String, ScriptValue>,
        source: ArtifactSource,
        produced_by: ProducedBy,
    ) -> Result<Self, crate::canonical::CanonicalError> {
        let digest = definition_digest(&ScriptValue::Record(payload.clone()))?;
        Ok(Self {
            schema_version: ARTIFACT_SCHEMA_VERSION,
            kind,
            digest,
            payload,
            source,
            produced_by,
        })
    }
}

/// Classification of a [`ScriptError`]; carries no vendor error type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ScriptErrorKind {
    /// Source could not be parsed.
    Parse,
    /// Source parsed but could not be compiled.
    Compile,
    /// Static type error.
    Type,
    /// Error during evaluation.
    Runtime,
    /// An execution budget limit was exceeded.
    BudgetExceeded,
    /// Evaluation was cancelled via the session [`crate::CancelToken`].
    Cancelled,
    /// The requested operation is not supported by this backend.
    Unsupported,
    /// A host binding call failed or was rejected.
    Host,
    /// The backend itself failed.
    Backend,
    /// Internal contract violation.
    Internal,
}

/// A neutral script error. `range` locates the offending source span
/// when the backend can provide one.
#[derive(Clone, Debug, PartialEq)]
pub struct ScriptError {
    /// Error classification.
    pub kind: ScriptErrorKind,
    /// Human-readable message (not a stable contract surface).
    pub message: String,
    /// Source span, when known.
    pub range: Option<SourceRange>,
}

impl ScriptError {
    /// Creates an error without a source range.
    #[must_use]
    pub fn new(kind: ScriptErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            range: None,
        }
    }

    /// Attaches a source range.
    #[must_use]
    pub fn with_range(mut self, range: SourceRange) -> Self {
        self.range = Some(range);
        self
    }
}

impl fmt::Display for ScriptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)
    }
}

impl std::error::Error for ScriptError {}

/// Severity of a [`ScriptDiagnostic`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum DiagnosticSeverity {
    /// Informational note.
    Info,
    /// Hint (e.g. completion-style suggestion).
    Hint,
    /// Warning — evaluation may proceed.
    #[default]
    Warning,
    /// Error — evaluation should be considered failed.
    Error,
}

/// A neutral diagnostic emitted during evaluation.
#[derive(Clone, Debug, PartialEq)]
pub struct ScriptDiagnostic {
    /// Severity.
    pub severity: DiagnosticSeverity,
    /// Human-readable message.
    pub message: String,
    /// Source span, when known.
    pub range: Option<SourceRange>,
    /// Optional stable diagnostic code (backend vocabulary).
    pub code: Option<String>,
}

/// Result of a successful [`crate::ScriptSession::evaluate`] call.
///
/// An analyzer or evaluation succeeding never grants execution
/// permission by itself — artifact trust decisions belong to the
/// consumer (fail-closed F-07 family).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ScriptResult {
    /// Typed artifact, when the evaluation produced one. `None` for
    /// evaluations that yield only a return value (e.g. round-trip
    /// value models).
    pub artifact: Option<ScriptArtifact>,
    /// Optional return value.
    pub return_value: Option<ScriptValue>,
    /// Diagnostics emitted during evaluation.
    pub diagnostics: Vec<ScriptDiagnostic>,
}
