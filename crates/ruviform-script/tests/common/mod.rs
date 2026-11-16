//! Shared fake fixtures for `ruviform-script` contract tests.
//!
//! These are deterministic test doubles implementing the public
//! [`ScriptBackend`]/[`ScriptSession`]/[`ScriptAnalyzer`] traits. They
//! exist only under `tests/` — the production surface ships no fake
//! backend and registers nothing by default.

#![allow(dead_code)]

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicU64, Ordering};

use ruviform_script::analyzer::{
    AnalyzeRequest, AnalyzeResult, AnalyzerCapabilities, AnalyzerDescriptor, AnalyzerError,
    DiagnosticLevel, ScriptAnalyzer, WantedCapabilities,
};
use ruviform_script::budget::{BudgetKind, CancelToken, ExecutionBudget, PlacementHint};
use ruviform_script::canonical::source_digest;
use ruviform_script::capability::BackendCapabilities;
use ruviform_script::host::HostApiSurface;
use ruviform_script::id::{
    AnalyzerId, CapabilityId, ScriptBackendId, ScriptBackendVersion, ScriptLanguageId, SessionId,
};
use ruviform_script::registry::{BackendDescriptor, ResolveError};
use ruviform_script::session::{ScriptBackend, ScriptSession, SessionConfig, SessionError};
use ruviform_script::value::{
    ArtifactKind, ScriptError, ScriptErrorKind, ScriptResult, ScriptSource, ScriptValue,
    SourceDigest, SourceRevision,
};

/// A configurable fake backend for contract tests.
pub struct FakeBackend {
    descriptor: BackendDescriptor,
    /// Budget kinds this fake claims to support; any requested
    /// dimension outside this set fails `open_session` with
    /// `UnsupportedBudget`.
    pub supported_budget_kinds: BTreeSet<BudgetKind>,
    /// Source size limit enforced at `open_session`.
    pub max_source_bytes: Option<u64>,
    /// Result `evaluate` returns on the happy path.
    pub eval_result: ScriptResult,
    /// Error `evaluate` returns instead, when set.
    pub eval_error: Option<ScriptError>,
    /// Session IDs are handed out from this counter.
    next_session: AtomicU64,
}

impl FakeBackend {
    /// Creates a fake backend for `language`/`backend_id` with default
    /// capabilities (evaluate + cancel, no artifact kinds).
    #[must_use]
    pub fn new(language: &str, backend_id: &str) -> Self {
        Self {
            descriptor: BackendDescriptor {
                backend_id: ScriptBackendId::new(backend_id),
                language: ScriptLanguageId::new(language),
                version: ScriptBackendVersion::new("fake-0.1.0", ruviform_script::CONTRACT_VERSION),
                capabilities: BackendCapabilities {
                    capability_ids: [CapabilityId::new("session.evaluate")]
                        .into_iter()
                        .collect(),
                    ..BackendCapabilities::default()
                },
            },
            supported_budget_kinds: BTreeSet::new(),
            max_source_bytes: None,
            eval_result: ScriptResult::default(),
            eval_error: None,
            next_session: AtomicU64::new(1),
        }
    }

    /// Overrides the advertised contract version (for version-mismatch
    /// tests).
    pub fn with_contract_version(mut self, contract_version: u32) -> Self {
        self.descriptor.version.contract_version = contract_version;
        self
    }

    /// Adds advertised capability IDs.
    pub fn with_capabilities(mut self, ids: &[&str]) -> Self {
        self.descriptor
            .capabilities
            .capability_ids
            .extend(ids.iter().map(|s| CapabilityId::new(*s)));
        self
    }

    /// Adds advertised artifact kinds.
    pub fn with_artifact_kinds(mut self, kinds: &[ArtifactKind]) -> Self {
        self.descriptor
            .capabilities
            .artifact_kinds
            .extend(kinds.iter().cloned());
        self
    }

    /// Sets which budget dimensions `open_session` accepts.
    pub fn with_supported_budgets(mut self, kinds: &[BudgetKind]) -> Self {
        self.supported_budget_kinds = kinds.iter().copied().collect();
        self
    }
}

impl ScriptBackend for FakeBackend {
    fn descriptor(&self) -> &BackendDescriptor {
        &self.descriptor
    }

    fn open_session(&self, config: SessionConfig) -> Result<Box<dyn ScriptSession>, SessionError> {
        let d = self.descriptor();
        if config.source.language != d.language {
            return Err(SessionError::LanguageMismatch {
                expected: d.language.clone(),
                actual: config.source.language.clone(),
            });
        }
        if let Err(e) = config.host_bindings.validate_neutral() {
            return Err(SessionError::InvalidHostSurface {
                reason: e.to_string(),
            });
        }
        if let Some(max) = self.max_source_bytes {
            let bytes = u64::try_from(config.source.text.len()).unwrap_or(u64::MAX);
            if bytes > max {
                return Err(SessionError::SourceTooLarge { bytes, max });
            }
        }
        let budget = &config.budget;
        let requested = [
            (BudgetKind::Instructions, budget.max_instructions.is_some()),
            (BudgetKind::WallTime, budget.max_wall_time.is_some()),
            (BudgetKind::MemoryBytes, budget.max_memory_bytes.is_some()),
            (BudgetKind::OutputBytes, budget.max_output_bytes.is_some()),
            (BudgetKind::HostCalls, budget.max_host_calls.is_some()),
        ];
        for (kind, requested_set) in requested {
            if requested_set && !self.supported_budget_kinds.contains(&kind) {
                return Err(SessionError::UnsupportedBudget { kind });
            }
        }
        if let Some(expected) = &config.resume
            && *expected != config.source.digest
        {
            return Err(SessionError::SourceDigestMismatch {
                expected: expected.clone(),
                actual: config.source.digest.clone(),
            });
        }
        let id = SessionId(self.next_session.fetch_add(1, Ordering::SeqCst));
        Ok(Box::new(FakeSession {
            session_id: id,
            backend_id: d.backend_id.clone(),
            token: CancelToken::new(),
            eval_result: self.eval_result.clone(),
            eval_error: self.eval_error.clone(),
            closed: false,
        }))
    }
}

/// The session half of [`FakeBackend`].
pub struct FakeSession {
    session_id: SessionId,
    backend_id: ScriptBackendId,
    token: CancelToken,
    eval_result: ScriptResult,
    eval_error: Option<ScriptError>,
    closed: bool,
}

impl ScriptSession for FakeSession {
    fn session_id(&self) -> SessionId {
        self.session_id
    }

    fn backend_id(&self) -> &ScriptBackendId {
        &self.backend_id
    }

    fn evaluate(&mut self, _entry: Option<&str>) -> Result<ScriptResult, ScriptError> {
        if self.token.is_cancelled() {
            return Err(ScriptError::new(ScriptErrorKind::Cancelled, "cancelled"));
        }
        if let Some(err) = &self.eval_error {
            return Err(err.clone());
        }
        Ok(self.eval_result.clone())
    }

    fn cancel_token(&self) -> CancelToken {
        self.token.clone()
    }

    fn close(&mut self) {
        self.closed = true;
    }
}

impl FakeSession {
    /// Whether `close` has been called (test observation only).
    #[must_use]
    pub fn is_closed(&self) -> bool {
        self.closed
    }
}

/// A configurable fake analyzer for contract tests.
pub struct FakeAnalyzer {
    descriptor: AnalyzerDescriptor,
    /// When true, `analyze` echoes `request.source.revision`; when
    /// false it returns revision 0 (stale-result tests).
    pub echo_revision: bool,
    /// Error `analyze` returns instead of a result, when set.
    pub analyze_error: Option<AnalyzerError>,
}

impl FakeAnalyzer {
    /// Creates a fake analyzer for `language`/`analyzer_id`.
    #[must_use]
    pub fn new(language: &str, analyzer_id: &str) -> Self {
        Self {
            descriptor: AnalyzerDescriptor {
                analyzer_id: AnalyzerId::new(analyzer_id),
                language: ScriptLanguageId::new(language),
                version: ScriptBackendVersion::new(
                    "fake-analyzer-0.1.0",
                    ruviform_script::CONTRACT_VERSION,
                ),
                capabilities: AnalyzerCapabilities {
                    diagnostics: DiagnosticLevel::SyntaxAndType,
                    completion: true,
                    max_source_bytes: None,
                    supported_languages: vec![ScriptLanguageId::new(language)],
                },
            },
            echo_revision: true,
            analyze_error: None,
        }
    }

    /// Overrides the advertised contract version.
    pub fn with_contract_version(mut self, contract_version: u32) -> Self {
        self.descriptor.version.contract_version = contract_version;
        self
    }

    /// Overrides advertised capabilities.
    pub fn with_capabilities(mut self, caps: AnalyzerCapabilities) -> Self {
        self.descriptor.capabilities = caps;
        self
    }
}

impl ScriptAnalyzer for FakeAnalyzer {
    fn descriptor(&self) -> &AnalyzerDescriptor {
        &self.descriptor
    }

    fn analyze(&self, request: &AnalyzeRequest) -> Result<AnalyzeResult, AnalyzerError> {
        if let Some(err) = &self.analyze_error {
            return Err(err.clone());
        }
        Ok(AnalyzeResult {
            source_revision: if self.echo_revision {
                request.source.revision
            } else {
                SourceRevision::new(0)
            },
            diagnostics: Vec::new(),
            completions: Vec::new(),
        })
    }
}

/// Builds a minimal valid [`SessionConfig`] for `language`.
#[must_use]
pub fn session_config(language: &str, text: &str) -> SessionConfig {
    SessionConfig {
        budget: ExecutionBudget::unlimited(),
        host_bindings: HostApiSurface::empty(),
        artifact_kinds: BTreeSet::new(),
        source: ScriptSource::new(
            text,
            ScriptLanguageId::new(language),
            None,
            SourceRevision::new(1),
        ),
        placement_hint: Some(PlacementHint::Any),
        resume: None,
    }
}

/// Builds a minimal valid [`AnalyzeRequest`] for `language`.
#[must_use]
pub fn analyze_request(language: &str, text: &str) -> AnalyzeRequest {
    AnalyzeRequest {
        source: ScriptSource::new(
            text,
            ScriptLanguageId::new(language),
            None,
            SourceRevision::new(7),
        ),
        language: ScriptLanguageId::new(language),
        type_definitions: None,
        wanted: WantedCapabilities {
            diagnostics: DiagnosticLevel::Syntax,
            completion: false,
        },
        cursor: None,
    }
}

/// A sample typed value exercising exotic variants for codec tests.
///
/// Canonical-safe: `Absent` appears only as a record field (omitted by
/// the canonical form), never as a reachable array element or top-level
/// value — reachable `Absent` has no canonical form by contract. The
/// `>2^53` `Integer` members exercise the `int` codec tag; note their
/// typed canonical form (exact decimal) differs from the materialized
/// tag record — the documented codec/canonical exception.
#[must_use]
pub fn exotic_value() -> ScriptValue {
    use ruviform_script::id::{ResourceDomainId, TimeDomainId};
    use ruviform_script::value::Rational;
    let mut record = std::collections::BTreeMap::new();
    record.insert("nested".to_string(), ScriptValue::Bool(true));
    record.insert("absent_field".to_string(), ScriptValue::Absent);
    ScriptValue::Array(vec![
        ScriptValue::Null,
        ScriptValue::Bool(false),
        ScriptValue::Integer(i64::MAX),
        ScriptValue::Integer(-9_007_199_254_740_993), // < -2^53 → int tag
        ScriptValue::Number(1.5),
        ScriptValue::Number(-0.0),
        ScriptValue::Number(f64::NAN),
        ScriptValue::Number(f64::INFINITY),
        ScriptValue::String("he\"llo\n".to_string()),
        ScriptValue::Bytes(vec![0xde, 0xad, 0xbe, 0xef]),
        ScriptValue::Time {
            domain: TimeDomainId::new("audio.transport"),
            timebase: Rational::new(1, 48000),
            count: 1024,
        },
        ScriptValue::ResourceId {
            domain: ResourceDomainId::new("track"),
            kind: "audio".to_string(),
            value: "trk-42".to_string(),
        },
        ScriptValue::Record(record),
    ])
}

/// The digest a `resume` mismatch test expects.
#[must_use]
pub fn other_digest() -> SourceDigest {
    source_digest("different source")
}

/// Unwraps the `Err` side of [`ruviform_script::ScriptBackendRegistry::resolve`]
/// without requiring `Debug` on the `&dyn ScriptBackend` success type.
///
/// # Panics
/// Panics when `result` is `Ok`.
pub fn expect_resolve_err(result: Result<&dyn ScriptBackend, ResolveError>) -> ResolveError {
    match result {
        Err(err) => err,
        Ok(_) => panic!("expected resolve failure"),
    }
}

/// Unwraps the `Err` side of [`ScriptBackend::open_session`] without
/// requiring `Debug` on the `Box<dyn ScriptSession>` success type.
///
/// # Panics
/// Panics when `result` is `Ok`.
pub fn expect_open_err(result: Result<Box<dyn ScriptSession>, SessionError>) -> SessionError {
    match result {
        Err(err) => err,
        Ok(_) => panic!("expected open_session failure"),
    }
}
