//! C5 analyzer port: static analysis contracts independent of any
//! runtime backend.
//!
//! An analyzer inspects a [`ScriptSource`] and returns diagnostics and
//! completions. It is **not** a runtime: analyzer success never grants
//! execution permission, and the analyzer registry resolves
//! independently of [`crate::ScriptBackendRegistry`].
//!
//! `ScriptAnalyzer::analyze` takes `&self` — implementations own their
//! interior mutability (the `&mut self` shape in `ffi-poc` was a `PoC`
//! artifact; the Synvelis consumer mirror already uses `&self`).
//!
//! Resolution is fail-closed with the same rules as the backend
//! registry: unknown language/analyzer, zero candidates, contract
//! version incompatibility, and missing capabilities are typed errors;
//! ambiguity is reported, never resolved arbitrarily.
//!
//! Stale-result detection: [`AnalyzeResult`] carries the
//! [`crate::SourceRevision`] it was computed against.
//! [`verify_result_revision`] rejects results whose revision does not
//! match the request source (F-14). Consumers remain responsible for
//! verifying revisions themselves — the helper is provided, not
//! enforced.

use std::collections::BTreeSet;
use std::fmt;

use crate::CONTRACT_VERSION;
use crate::id::{AnalyzerId, CapabilityId, ScriptBackendVersion, ScriptLanguageId};
use crate::value::{DiagnosticSeverity, ScriptSource, SourcePosition, SourceRange, SourceRevision};

/// Schema version for analyzer type-definition inputs.
pub const TYPE_DEFINITION_SCHEMA_VERSION: u32 = 1;

/// How much diagnostic capability an analyzer advertises or a request
/// demands. Ordered: `Unsupported < Syntax < SyntaxAndType`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum DiagnosticLevel {
    /// No diagnostics.
    #[default]
    Unsupported,
    /// Syntax-level diagnostics only.
    Syntax,
    /// Syntax and type-level diagnostics.
    SyntaxAndType,
}

/// A type-definition input (e.g. a `.d.luau`-style declaration file)
/// made available to the analyzer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeDefinition {
    /// Package/name the definitions belong to.
    pub package_name: String,
    /// Definition source text with its own provenance.
    pub source: ScriptSource,
    /// Schema version of this input envelope.
    pub schema_version: u32,
}

impl TypeDefinition {
    /// Creates a type-definition input stamped with
    /// [`TYPE_DEFINITION_SCHEMA_VERSION`].
    #[must_use]
    pub fn new(package_name: impl Into<String>, source: ScriptSource) -> Self {
        Self {
            package_name: package_name.into(),
            source,
            schema_version: TYPE_DEFINITION_SCHEMA_VERSION,
        }
    }
}

/// Capabilities an analyzer advertises in its descriptor.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AnalyzerCapabilities {
    /// Highest diagnostic level supported.
    pub diagnostics: DiagnosticLevel,
    /// Whether completion requests are supported.
    pub completion: bool,
    /// Maximum accepted source size in bytes, if limited.
    pub max_source_bytes: Option<u64>,
    /// Languages this analyzer supports (advertised; resolution keys
    /// on [`AnalyzerDescriptor::language`]).
    pub supported_languages: Vec<ScriptLanguageId>,
}

impl AnalyzerCapabilities {
    /// Derives the capability IDs this analyzer effectively offers:
    /// `"analyze.diagnostics"` when diagnostics are supported,
    /// `"analyze.types"` additionally at [`DiagnosticLevel::SyntaxAndType`],
    /// and `"analyze.completion"` when completion is supported.
    #[must_use]
    pub fn capability_ids(&self) -> BTreeSet<CapabilityId> {
        let mut ids = BTreeSet::new();
        if self.diagnostics != DiagnosticLevel::Unsupported {
            ids.insert(CapabilityId::new("analyze.diagnostics"));
        }
        if self.diagnostics == DiagnosticLevel::SyntaxAndType {
            ids.insert(CapabilityId::new("analyze.types"));
        }
        if self.completion {
            ids.insert(CapabilityId::new("analyze.completion"));
        }
        ids
    }
}

/// Static description of a registered [`ScriptAnalyzer`].
#[derive(Clone, Debug, PartialEq)]
pub struct AnalyzerDescriptor {
    /// Unique analyzer identifier.
    pub analyzer_id: AnalyzerId,
    /// Primary language this analyzer handles.
    pub language: ScriptLanguageId,
    /// Implementation/contract version pair.
    pub version: ScriptBackendVersion,
    /// Advertised capabilities.
    pub capabilities: AnalyzerCapabilities,
}

/// What a consumer needs from an analyzer.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AnalyzerRequirement {
    /// Required language.
    pub language: ScriptLanguageId,
    /// Optional specific analyzer hint. `None` = any analyzer for the
    /// language.
    pub analyzer: Option<AnalyzerId>,
    /// Capability IDs that must be advertised (matched against
    /// [`AnalyzerCapabilities::capability_ids`]).
    pub required_capabilities: BTreeSet<CapabilityId>,
    /// Minimum acceptable `contract_version`.
    pub min_contract_version: u32,
}

/// Errors from [`AnalyzerRegistry::register`].
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum AnalyzerRegistryError {
    /// An analyzer with this ID is already registered.
    DuplicateAnalyzer {
        /// The duplicated analyzer ID.
        analyzer_id: AnalyzerId,
    },
}

impl fmt::Display for AnalyzerRegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateAnalyzer { analyzer_id } => {
                write!(f, "duplicate analyzer id: {analyzer_id}")
            }
        }
    }
}

impl std::error::Error for AnalyzerRegistryError {}

/// Why an [`AnalyzerRequirement`] matched no analyzer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AnalyzerResolveReason {
    /// No registered analyzer implements the required language.
    UnknownLanguage,
    /// The hinted analyzer ID is not registered.
    UnknownAnalyzer,
    /// Language/analyzer filters matched nothing.
    NoCandidates,
    /// Candidates exist but none speaks a compatible contract version.
    UnsupportedContractVersion,
    /// Candidates exist but none advertises the required capabilities.
    MissingCapability,
}

/// Fail-closed analyzer resolution errors — same shape as
/// [`crate::ResolveError`] in analyzer vocabulary.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum AnalyzerResolveError {
    /// No analyzer satisfied the requirement.
    NoMatchingAnalyzer {
        /// Distinguishing reason.
        reason: AnalyzerResolveReason,
        /// Human-readable detail.
        detail: String,
    },
    /// More than one analyzer satisfied the requirement.
    AmbiguousAnalyzer {
        /// IDs of the matching candidates, sorted for determinism.
        candidates: Vec<AnalyzerId>,
    },
}

impl fmt::Display for AnalyzerResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoMatchingAnalyzer { reason, detail } => {
                write!(f, "no matching analyzer ({reason:?}): {detail}")
            }
            Self::AmbiguousAnalyzer { candidates } => {
                write!(f, "ambiguous analyzer candidates: {candidates:?}")
            }
        }
    }
}

impl std::error::Error for AnalyzerResolveError {}

/// What an analysis request asks for.
#[derive(Clone, Debug, PartialEq)]
pub struct WantedCapabilities {
    /// Minimum diagnostic level required. `Unsupported` means the
    /// consumer does not require diagnostics.
    pub diagnostics: DiagnosticLevel,
    /// Whether completion results are required.
    pub completion: bool,
}

impl WantedCapabilities {
    /// Derives the capability IDs this request requires, for
    /// subset-checking against [`AnalyzerCapabilities::capability_ids`].
    #[must_use]
    pub fn capability_ids(&self) -> BTreeSet<CapabilityId> {
        let mut ids = BTreeSet::new();
        if self.diagnostics != DiagnosticLevel::Unsupported {
            ids.insert(CapabilityId::new("analyze.diagnostics"));
        }
        if self.diagnostics == DiagnosticLevel::SyntaxAndType {
            ids.insert(CapabilityId::new("analyze.types"));
        }
        if self.completion {
            ids.insert(CapabilityId::new("analyze.completion"));
        }
        ids
    }
}

/// An analysis request.
#[derive(Clone, Debug, PartialEq)]
pub struct AnalyzeRequest {
    /// Source to analyze (its `language`/`revision`/`digest` are
    /// provenance carried into the result).
    pub source: ScriptSource,
    /// Language to analyze *as* — may differ from `source.language`
    /// only when the consumer deliberately reinterprets the text.
    pub language: ScriptLanguageId,
    /// Optional type-definition inputs. `None` = no snapshot provided;
    /// `Some(vec![])` = explicitly empty snapshot.
    pub type_definitions: Option<Vec<TypeDefinition>>,
    /// What the consumer wants back.
    pub wanted: WantedCapabilities,
    /// Optional cursor position for completion requests.
    pub cursor: Option<SourcePosition>,
}

/// Kind of completion item.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CompletionKind {
    /// Plain text insertion.
    Text,
    /// Method/member completion.
    Method,
    /// Field/property completion.
    Field,
    /// Keyword completion.
    Keyword,
    /// Other backend-specific kind.
    Other,
}

/// A completion candidate.
#[derive(Clone, Debug, PartialEq)]
pub struct Completion {
    /// Completion kind.
    pub kind: CompletionKind,
    /// Display label.
    pub label: String,
    /// Text to insert (defaults to `label` semantics when absent).
    pub insert_text: Option<String>,
    /// Source range to replace, when the backend can compute it.
    pub replace_range: Option<SourceRange>,
    /// Optional detail string (signature, doc snippet).
    pub detail: Option<String>,
}

/// A diagnostic produced by an analyzer — richer than
/// [`crate::ScriptDiagnostic`] with a stable optional `code`.
#[derive(Clone, Debug, PartialEq)]
pub struct Diagnostic {
    /// Severity.
    pub severity: DiagnosticSeverity,
    /// Human-readable message.
    pub message: String,
    /// Source span, when known.
    pub range: Option<SourceRange>,
    /// Optional stable diagnostic code (analyzer vocabulary).
    pub code: Option<String>,
}

/// Result of [`ScriptAnalyzer::analyze`].
///
/// `source_revision` echoes the analyzed source's revision so consumers
/// can detect stale results — see [`verify_result_revision`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AnalyzeResult {
    /// Revision of the source this result was computed against.
    pub source_revision: SourceRevision,
    /// Diagnostics produced.
    pub diagnostics: Vec<Diagnostic>,
    /// Completions produced (empty unless requested and supported).
    pub completions: Vec<Completion>,
}

/// Errors from [`ScriptAnalyzer::analyze`] and registry convenience
/// methods.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum AnalyzerError {
    /// No analyzer is registered/resolvable for the request.
    Unavailable {
        /// Human-readable reason.
        reason: String,
    },
    /// The analyzer does not support a requested capability.
    Unsupported {
        /// The missing capability ID.
        capability: CapabilityId,
    },
    /// The result's `source_revision` does not match the request —
    /// the result is stale and must be rejected (F-14).
    StaleResult,
    /// The request itself is invalid (e.g. source too large, bad
    /// cursor, unsupported language for this analyzer).
    InvalidRequest {
        /// Human-readable reason.
        reason: String,
    },
    /// A versioned input's schema version is not supported.
    VersionMismatch {
        /// Which field mismatched.
        field: String,
        /// Expected version.
        expected: String,
        /// Actual version found.
        actual: String,
    },
    /// Analyzer-internal failure.
    Internal {
        /// Human-readable detail.
        message: String,
    },
}

impl fmt::Display for AnalyzerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable { reason } => write!(f, "analyzer unavailable: {reason}"),
            Self::Unsupported { capability } => {
                write!(f, "analyzer does not support capability {capability}")
            }
            Self::StaleResult => f.write_str("analyzer result is stale (source revision mismatch)"),
            Self::InvalidRequest { reason } => write!(f, "invalid analyze request: {reason}"),
            Self::VersionMismatch {
                field,
                expected,
                actual,
            } => write!(
                f,
                "version mismatch on {field}: expected {expected}, got {actual}"
            ),
            Self::Internal { message } => write!(f, "analyzer internal error: {message}"),
        }
    }
}

impl std::error::Error for AnalyzerError {}

/// Verifies that `result` was computed against `request`'s source
/// revision. Returns [`AnalyzerError::StaleResult`] on mismatch (F-14).
///
/// Consumers should call this (or [`AnalyzerRegistry::analyze`], which
/// does it for them) before trusting analyzer output.
pub fn verify_result_revision(
    request: &AnalyzeRequest,
    result: &AnalyzeResult,
) -> Result<(), AnalyzerError> {
    if result.source_revision == request.source.revision {
        Ok(())
    } else {
        Err(AnalyzerError::StaleResult)
    }
}

/// A static analyzer: inspects sources without executing them.
///
/// `analyze` takes `&self`; implementations own interior mutability.
/// Implementations must be deterministic for identical requests and
/// must not perform I/O outside their declared capabilities.
pub trait ScriptAnalyzer {
    /// Returns the analyzer's descriptor.
    fn descriptor(&self) -> &AnalyzerDescriptor;

    /// Analyzes `request`.
    ///
    /// # Errors
    ///
    /// [`AnalyzerError::Unsupported`] when `request.wanted` exceeds the
    /// advertised capabilities, [`AnalyzerError::InvalidRequest`] for
    /// malformed requests, [`AnalyzerError::StaleResult`] is produced
    /// by consumers via [`verify_result_revision`], not by this call.
    fn analyze(&self, request: &AnalyzeRequest) -> Result<AnalyzeResult, AnalyzerError>;
}

/// Registry of [`ScriptAnalyzer`] implementations — independent of
/// [`crate::ScriptBackendRegistry`]. Ships empty; consumers register
/// real analyzers explicitly.
#[derive(Default)]
pub struct AnalyzerRegistry {
    analyzers: std::collections::BTreeMap<AnalyzerId, Box<dyn ScriptAnalyzer>>,
}

impl AnalyzerRegistry {
    /// Creates an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers `analyzer` under its descriptor's `analyzer_id`.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyzerRegistryError::DuplicateAnalyzer`] if the ID
    /// is taken.
    pub fn register(
        &mut self,
        analyzer: Box<dyn ScriptAnalyzer>,
    ) -> Result<(), AnalyzerRegistryError> {
        let id = analyzer.descriptor().analyzer_id.clone();
        if self.analyzers.contains_key(&id) {
            return Err(AnalyzerRegistryError::DuplicateAnalyzer { analyzer_id: id });
        }
        self.analyzers.insert(id, analyzer);
        Ok(())
    }

    /// Resolves `requirement` to exactly one analyzer, fail-closed —
    /// same rules as [`crate::ScriptBackendRegistry::resolve`].
    ///
    /// # Errors
    ///
    /// [`AnalyzerResolveError::NoMatchingAnalyzer`] with a
    /// distinguishing reason, or [`AnalyzerResolveError::AmbiguousAnalyzer`].
    pub fn resolve(
        &self,
        requirement: &AnalyzerRequirement,
    ) -> Result<&dyn ScriptAnalyzer, AnalyzerResolveError> {
        let language_known = self
            .analyzers
            .values()
            .any(|a| a.descriptor().language == requirement.language);
        if !language_known {
            return Err(AnalyzerResolveError::NoMatchingAnalyzer {
                reason: AnalyzerResolveReason::UnknownLanguage,
                detail: format!(
                    "no registered analyzer implements language {:?}",
                    requirement.language
                ),
            });
        }
        if let Some(analyzer_id) = &requirement.analyzer
            && !self.analyzers.contains_key(analyzer_id)
        {
            return Err(AnalyzerResolveError::NoMatchingAnalyzer {
                reason: AnalyzerResolveReason::UnknownAnalyzer,
                detail: format!("analyzer {analyzer_id} is not registered"),
            });
        }

        let mut candidates: Vec<&dyn ScriptAnalyzer> = self
            .analyzers
            .values()
            .map(Box::as_ref)
            .filter(|a| {
                let d = a.descriptor();
                d.language == requirement.language
                    && requirement
                        .analyzer
                        .as_ref()
                        .is_none_or(|id| &d.analyzer_id == id)
            })
            .collect();

        if candidates.is_empty() {
            return Err(AnalyzerResolveError::NoMatchingAnalyzer {
                reason: AnalyzerResolveReason::NoCandidates,
                detail: format!(
                    "language {:?} and analyzer hint are known but no analyzer satisfies both",
                    requirement.language
                ),
            });
        }

        candidates.retain(|a| {
            let v = &a.descriptor().version;
            v.contract_version == CONTRACT_VERSION
                && v.contract_version >= requirement.min_contract_version
        });
        if candidates.is_empty() {
            return Err(AnalyzerResolveError::NoMatchingAnalyzer {
                reason: AnalyzerResolveReason::UnsupportedContractVersion,
                detail: format!(
                    "no candidate speaks contract version {CONTRACT_VERSION} with min {}",
                    requirement.min_contract_version
                ),
            });
        }

        candidates.retain(|a| {
            let caps = a.descriptor().capabilities.capability_ids();
            requirement.required_capabilities.is_subset(&caps)
        });
        if candidates.is_empty() {
            return Err(AnalyzerResolveError::NoMatchingAnalyzer {
                reason: AnalyzerResolveReason::MissingCapability,
                detail: "no candidate advertises the required capabilities".to_string(),
            });
        }

        match candidates.as_slice() {
            [single] => Ok(*single),
            _ => Err(AnalyzerResolveError::AmbiguousAnalyzer {
                candidates: candidates
                    .iter()
                    .map(|a| a.descriptor().analyzer_id.clone())
                    .collect(),
            }),
        }
    }

    /// Resolves `requirement`, checks `request.wanted` against the
    /// resolved analyzer's capabilities, runs `analyze`, and verifies
    /// the result's `source_revision` — the consumer fail-closed path
    /// in one call.
    ///
    /// Resolution failures surface as [`AnalyzerError::Unavailable`];
    /// capability shortfalls as [`AnalyzerError::Unsupported`]; stale
    /// revisions as [`AnalyzerError::StaleResult`].
    ///
    /// # Errors
    ///
    /// See variant docs on [`AnalyzerError`].
    pub fn analyze(
        &self,
        requirement: &AnalyzerRequirement,
        request: &AnalyzeRequest,
    ) -> Result<AnalyzeResult, AnalyzerError> {
        let analyzer = self
            .resolve(requirement)
            .map_err(|e| AnalyzerError::Unavailable {
                reason: e.to_string(),
            })?;
        let supported = analyzer.descriptor().capabilities.capability_ids();
        for wanted in request.wanted.capability_ids() {
            if !supported.contains(&wanted) {
                return Err(AnalyzerError::Unsupported { capability: wanted });
            }
        }
        let result = analyzer.analyze(request)?;
        verify_result_revision(request, &result)?;
        Ok(result)
    }

    /// Lists all registered analyzers in analyzer-ID order.
    #[must_use]
    pub fn list(&self) -> Vec<&dyn ScriptAnalyzer> {
        self.analyzers.values().map(Box::as_ref).collect()
    }

    /// Number of registered analyzers.
    #[must_use]
    pub fn len(&self) -> usize {
        self.analyzers.len()
    }

    /// Returns `true` if no analyzers are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.analyzers.is_empty()
    }
}
