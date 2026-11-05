//! YAML document I/O (`feature = "yaml"`).
//!
//! Single-document [`from_reader`] / [`to_writer`] over borrowed
//! `std::io` streams.
//!
//! # What is enforced vs. what is honest
//!
//! The backend ([`serde-saphyr`]) enforces a real resource budget, so
//! this module's limits are actual hard limits, not documentation
//! wishes:
//!
//! - **Input bytes** — [`InputLimits::max_bytes`] is enforced twice:
//!   by the adapter's own bounded reader (rejects before the parser can
//!   consume past it) and by the backend's reader budget
//!   ([`DocumentErrorKind::LimitExceeded`] either way).
//! - **Nesting depth** — [`YamlReadOptions::max_depth`] maps to the
//!   backend's structural-depth budget.
//! - **Aliases** — [`YamlReadOptions::max_aliases`] maps to the
//!   backend's alias-event budget; alias *replay* limits (expansions
//!   per anchor, replay stack depth, total replayed events) keep the
//!   backend's finite defaults and report as `LimitExceeded` when
//!   tripped.
//! - **Other backend budgets** — events, nodes, anchors, documents,
//!   scalar bytes and merge keys keep finite backend defaults; all
//!   report `LimitExceeded` with the exceeded limit named when
//!   attributable.
//!
//! Honest limits: a byte budget does **not** bound CPU time or the
//! heap a caller's `Deserialize` impl allocates. Explicitly tagged
//! values the backend cannot represent report
//! [`DocumentErrorKind::Unsupported`].
//!
//! Snippet rendering is disabled on the backend so its diagnostics do
//! not embed input text; [`DocumentError`] `Display` stays minimal.
//!
//! **RT contract:** non-RT only — may block on I/O and allocate.

use crate::input::BoundedRead;
use crate::{
    DocumentError, DocumentErrorKind, DocumentFormat, DocumentLimit, DocumentPosition, InputLimits,
};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_saphyr::budget::BudgetBreach;
use std::io::{Read, Write};

const FORMAT: DocumentFormat = DocumentFormat::Yaml;

/// Read options for [`from_reader`].
///
/// `#[non_exhaustive]` — new knobs may be added; construct with
/// [`YamlReadOptions::new`] or [`Default`], then adjust fields.
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Debug, Clone, Copy)]
pub struct YamlReadOptions {
    /// Hard input limits (default 16 MiB, see [`InputLimits`]).
    pub limits: InputLimits,
    /// Maximum YAML nesting depth (sequences + mappings). Enforced by
    /// the backend's structural-depth budget; default `64`.
    pub max_depth: usize,
    /// Maximum number of alias (`*ref`) events. Enforced by the
    /// backend's alias budget; default `50_000`. Per-anchor expansion,
    /// replay-stack and total-replay limits stay at finite backend
    /// defaults.
    pub max_aliases: usize,
}

impl YamlReadOptions {
    /// Options with [`InputLimits::default`] and the backend's default
    /// depth/alias budgets.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            limits: InputLimits::new(crate::DEFAULT_MAX_INPUT_BYTES),
            max_depth: 64,
            max_aliases: 50_000,
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

    /// Overrides `max_depth`.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn with_max_depth(mut self, max_depth: usize) -> Self {
        self.max_depth = max_depth;
        self
    }

    /// Overrides `max_aliases`.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn with_max_aliases(mut self, max_aliases: usize) -> Self {
        self.max_aliases = max_aliases;
        self
    }
}

impl Default for YamlReadOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// Write options for [`to_writer`].
///
/// Currently carries no knobs — the backend's default emitter layout
/// (block style, 1.1-compatible tags where required) is used. The
/// struct exists so the API shape matches the other formats; it is
/// `#[non_exhaustive]` and may grow options later.
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, Default)]
pub struct YamlWriteOptions {}

impl YamlWriteOptions {
    /// Default write options.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

/// Deserializes a single YAML document of type `T` from `reader`.
///
/// A second YAML document in the stream is
/// [`DocumentErrorKind::TrailingData`]; a prematurely ended document is
/// [`DocumentErrorKind::Syntax`]; a valid document that does not fit
/// `T` is [`DocumentErrorKind::DataModel`]. All enforced limits report
/// [`DocumentErrorKind::LimitExceeded`].
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
/// use ruviform_io_structured::yaml;
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct Cfg { name: String }
///
/// let mut input = b"name: demo\n".as_slice();
/// let cfg: Cfg = yaml::from_reader(&mut input, &yaml::YamlReadOptions::default())?;
/// assert_eq!(cfg.name, "demo");
/// # Ok::<(), ruviform_io_structured::DocumentError>(())
/// ```
pub fn from_reader<T: DeserializeOwned>(
    reader: &mut impl Read,
    options: &YamlReadOptions,
) -> Result<T, DocumentError> {
    let mut budget = serde_saphyr::Budget::default();
    budget.max_reader_input_bytes = usize::try_from(options.limits.max_bytes)
        .ok()
        .map(|b| b.saturating_add(1)); // adapter limit is authoritative; backend is belt-and-suspenders
    budget.max_depth = options.max_depth;
    budget.max_aliases = options.max_aliases;
    let caps = budget.clone();

    let mut backend = serde_saphyr::Options::default();
    backend.budget = Some(budget);
    // Keep input snippets out of backend-rendered diagnostics.
    backend.with_snippet = false;

    let mut bounded = BoundedRead::new(reader, options.limits.max_bytes);
    serde_saphyr::from_reader_with_options(&mut bounded, backend).map_err(|e| {
        map_error(
            e,
            bounded.was_limit_exceeded(),
            options.limits.max_bytes,
            &caps,
        )
    })
}

/// Serializes `value` as a single YAML document into `writer`.
///
/// Success means the serializer's output reached `writer` — not that it
/// was flushed, synced or published. Failure may leave a partial
/// document behind.
///
/// **RT contract:** non-RT only.
///
/// # Errors
///
/// `Io` for writer failures, `DataModel` for unserializable values.
///
/// # Examples
///
/// ```
/// use ruviform_io_structured::yaml;
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct Cfg { name: String }
///
/// let mut out = Vec::new();
/// yaml::to_writer(&mut out, &Cfg { name: "demo".into() }, &yaml::YamlWriteOptions::new())?;
/// assert_eq!(String::from_utf8(out).unwrap(), "name: demo\n");
/// # Ok::<(), ruviform_io_structured::DocumentError>(())
/// ```
pub fn to_writer<T: Serialize + ?Sized>(
    writer: &mut impl Write,
    value: &T,
    _options: &YamlWriteOptions,
) -> Result<(), DocumentError> {
    // `to_io_writer` binds `T: Serialize` (implicitly Sized); `&T`
    // re-borrows the unsized value into a Sized, still-`Serialize`
    // reference.
    serde_saphyr::to_io_writer(writer, &value).map_err(|e| {
        use serde_saphyr::SerializeError as S;
        let kind = match &e {
            S::IO { .. } | S::Format { .. } => DocumentErrorKind::Io,
            _ => DocumentErrorKind::DataModel,
        };
        DocumentError::with_source(kind, FORMAT, e)
    })
}

/// Maps a `serde-saphyr` failure to the normalized error model.
///
/// The byte-budget trip flag is authoritative over the backend's own
/// I/O and reader-budget errors. Wrapper variants (`WithSnippet`,
/// `AliasError`) are unwrapped for classification while the original
/// error stays the diagnostic `source`.
fn map_error(
    e: serde_saphyr::Error,
    limit_tripped: bool,
    max_bytes: u64,
    caps: &serde_saphyr::Budget,
) -> DocumentError {
    if limit_tripped {
        return DocumentError::from_kind(DocumentErrorKind::LimitExceeded, FORMAT).with_limit(
            DocumentLimit {
                name: "max_bytes",
                max: max_bytes,
            },
        );
    }
    let position = e.location().map(|loc| DocumentPosition {
        byte: loc.span().byte_offset(),
        line: Some(loc.line()),
        column: Some(loc.column()),
        record: None,
    });
    let (kind, limit) = classify(&e, caps);
    DocumentError::with_parts(kind, FORMAT, position, limit, Some(Box::new(e)))
}

/// The classification half of [`map_error`]; recursive only for the
/// wrapper variants.
fn classify(
    e: &serde_saphyr::Error,
    caps: &serde_saphyr::Budget,
) -> (DocumentErrorKind, Option<DocumentLimit>) {
    use serde_saphyr::Error as E;
    match e {
        // Wrappers — classify the underlying error.
        E::WithSnippet { error, .. } | E::AliasError { error, .. } => classify(error, caps),

        E::IOError { .. } => (DocumentErrorKind::Io, None),
        E::InvalidUtf8Input => (DocumentErrorKind::InvalidEncoding, None),
        E::MultipleDocuments { .. } => (DocumentErrorKind::TrailingData, None),

        E::Budget { breach, .. } => match breach {
            // A closing event with no matching opener is malformed
            // input, not a resource limit.
            BudgetBreach::SequenceUnbalanced => (DocumentErrorKind::Syntax, None),
            _ => (DocumentErrorKind::LimitExceeded, breach_limit(breach, caps)),
        },
        E::AliasReplayCounterOverflow { .. } => (DocumentErrorKind::LimitExceeded, None),
        E::AliasReplayLimitExceeded {
            max_total_replayed_events,
            ..
        } => (
            DocumentErrorKind::LimitExceeded,
            Some(DocumentLimit {
                name: "max_total_replayed_events",
                max: *max_total_replayed_events as u64,
            }),
        ),
        E::AliasExpansionLimitExceeded {
            max_expansions_per_anchor,
            ..
        } => (
            DocumentErrorKind::LimitExceeded,
            Some(DocumentLimit {
                name: "max_expansions_per_anchor",
                max: *max_expansions_per_anchor as u64,
            }),
        ),
        E::AliasReplayStackDepthExceeded { max_depth, .. } => (
            DocumentErrorKind::LimitExceeded,
            Some(DocumentLimit {
                name: "max_replay_stack_depth",
                max: *max_depth as u64,
            }),
        ),

        // Explicitly-tagged or policy-gated input the adapter does not
        // support — reported, never silently dropped.
        E::UnsupportedTag { .. }
        | E::MergeKeyNotAllowed { .. }
        | E::UnsupportedIncludeForm { .. } => (DocumentErrorKind::Unsupported, None),

        // Value/type mismatches against the target `T` or the YAML
        // data model.
        E::Message { .. }
        | E::InvalidOptions { .. }
        | E::Unexpected { .. }
        | E::MergeValueNotMapOrSeqOfMaps { .. }
        | E::InvalidBinaryBase64 { .. }
        | E::BinaryNotUtf8 { .. }
        | E::TaggedScalarCannotDeserializeIntoString { .. }
        | E::InvalidBooleanStrict { .. }
        | E::InvalidCharNull { .. }
        | E::InvalidCharNotSingleScalar { .. }
        | E::NullIntoString { .. }
        | E::BytesNotSupportedMissingBinaryTag { .. }
        | E::UnexpectedValueForUnit { .. }
        | E::ExpectedEmptyMappingForUnitStruct { .. }
        | E::UnexpectedValueForUnitEnumVariant { .. }
        | E::ExpectedStringKeyForExternallyTaggedEnum { .. }
        | E::ExternallyTaggedEnumExpectedScalarOrMapping { .. }
        | E::TaggedEnumMismatch { .. }
        | E::SerdeVariantId { .. }
        | E::ExpectedMappingEndAfterEnumVariantValue { .. }
        | E::RecursiveReferencesRequireWeakTypes { .. }
        | E::InvalidScalar { .. }
        | E::NonFiniteFloat { .. }
        | E::SerdeInvalidType { .. }
        | E::SerdeInvalidValue { .. }
        | E::SerdeInvalidLength { .. }
        | E::SerdeUnknownVariant { .. }
        | E::SerdeUnknownField { .. }
        | E::SerdeMissingField { .. }
        | E::QuotingRequired { .. }
        | E::CannotBorrowTransformedString { .. }
        | E::UnresolvedProperty { .. }
        | E::InvalidPropertyName { .. }
        | E::PropertyRequiredButUnset { .. }
        | E::PropertyRequiredButEmpty { .. } => (DocumentErrorKind::DataModel, None),

        // Grammar-level document violations and anything not handled
        // above — including variants added by future backend versions
        // (the enum is non_exhaustive) and feature-gated validation
        // errors we do not enable — classify conservatively as Syntax:
        // the document demonstrably failed to parse.
        //
        // Intentional Syntax variants: `Eof`, `ExternalMessage`,
        // `UnexpectedSequenceEnd`, `UnexpectedMappingEnd`,
        // `UnexpectedContainerEndWhileSkippingNode`,
        // `UnexpectedContainerEndWhileReadingKeyNode`,
        // `ContainerEndMismatch`, `FoldedBlockScalarMustIndentContent`,
        // `UnknownAnchor`, `IndentationError`, `DuplicateMappingKey`,
        // `CyclicInclude`, `ResolverError`, `HookError`, plus the
        // backend-internal structural faults
        // (`InternalSeedReusedForMapKey`, `ValueRequestedBeforeKey`,
        // `InternalDepthUnderflow`, `InternalRecursionStackEmpty`).
        _ => (DocumentErrorKind::Syntax, None),
    }
}

/// Names the exceeded budget for a [`BudgetBreach`], using the
/// effective caps so callers can distinguish "my option" from a
/// backend default.
fn breach_limit(breach: &BudgetBreach, caps: &serde_saphyr::Budget) -> Option<DocumentLimit> {
    let (name, max) = match breach {
        BudgetBreach::Events { .. } => ("max_events", Some(caps.max_events)),
        BudgetBreach::Aliases { .. } => ("max_aliases", Some(caps.max_aliases)),
        BudgetBreach::Anchors { .. } => ("max_anchors", Some(caps.max_anchors)),
        BudgetBreach::Depth { .. } => ("max_depth", Some(caps.max_depth)),
        BudgetBreach::InclusionDepth { .. } => (
            "max_inclusion_depth",
            Some(caps.max_inclusion_depth as usize),
        ),
        BudgetBreach::Documents { .. } => ("max_documents", Some(caps.max_documents)),
        BudgetBreach::Nodes { .. } => ("max_nodes", Some(caps.max_nodes)),
        BudgetBreach::ScalarBytes { .. } => {
            ("max_total_scalar_bytes", Some(caps.max_total_scalar_bytes))
        }
        BudgetBreach::MergeKeys { .. } => ("max_merge_keys", Some(caps.max_merge_keys)),
        BudgetBreach::AliasAnchorRatio { .. } => ("alias_anchor_ratio", None),
        BudgetBreach::InputBytes { .. } => ("max_reader_input_bytes", caps.max_reader_input_bytes),
        BudgetBreach::RecordedAnchorEvents { .. } => (
            "max_recorded_anchor_events",
            Some(caps.max_recorded_anchor_events),
        ),
        BudgetBreach::RecordedAnchorBytes { .. } => (
            "max_recorded_anchor_bytes",
            Some(caps.max_recorded_anchor_bytes),
        ),
        // SequenceUnbalanced is handled by the caller; cfg-gated and
        // future breach kinds cannot be attributed.
        _ => return None,
    };
    max.map(|m| DocumentLimit {
        name,
        max: m as u64,
    })
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
    options: &YamlReadOptions,
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
    options: &YamlWriteOptions,
    publish: ruviform_io_file::PublishOptions,
) -> Result<ruviform_io_file::PublishReceipt, crate::DocumentFileWriteError> {
    crate::file::write_document(path, publish, |writer| to_writer(writer, value, options))
}
