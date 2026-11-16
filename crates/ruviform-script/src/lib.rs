//! `ruviform-script` — language-neutral Script/Analyzer contract port.
//!
//! This crate is the product home of the RF-SCRIPT-00 neutral contract
//! (`docs/designs/issue-558-script-contract-design.md`, approved):
//! the typed vocabulary that script backends and analyzers implement
//! and that consumers (e.g. Synvelis `synvelis-script`) program against.
//! It is a **headless contract crate** — it contains no VM, no FFI,
//! no vendor types (mlua/rhai/etc.), and no `unsafe`.
//!
//! # Contract sections
//!
//! - **C1 identifiers/registry** — [`ScriptLanguageId`],
//!   [`ScriptBackendId`], [`ScriptBackendVersion`],
//!   [`BackendCapabilities`], [`BackendDescriptor`],
//!   [`ScriptRequirement`], [`ScriptBackendRegistry`] with fail-closed
//!   [`ResolveError`].
//! - **C2 runtime port** — [`ScriptBackend`], [`ScriptSession`],
//!   [`SessionConfig`], [`SessionError`], [`ScriptResult`]. Session
//!   traits do not unconditionally require `Send`/`Sync`; placement is
//!   advertised via [`BackendCapabilities`] and requested via
//!   [`PlacementHint`].
//! - **C3 value model** — [`ScriptValue`] (closed variant set,
//!   `Absent` ≠ `Null`, strict `Integer(i64)`), [`ScriptError`],
//!   [`ScriptSource`]/[`SourceDigest`]/[`SourceRevision`],
//!   [`ScriptArtifact`]/[`ArtifactKind`].
//! - **C4 budget/cancel** — [`ExecutionBudget`], [`BudgetKind`],
//!   [`CancelGranularity`], [`CancelToken`]. All APIs are non-RT;
//!   RT-context detection is the caller's responsibility.
//! - **C5 analyzer port** — [`ScriptAnalyzer`],
//!   [`AnalyzerRegistry`], [`AnalyzeRequest`]/[`AnalyzeResult`],
//!   [`Diagnostic`]/[`Completion`], [`AnalyzerError`],
//!   [`verify_result_revision`]. Independent of the runtime registry.
//! - **C6 host API descriptors** — [`HostApiSurface`],
//!   [`HostBinding`], [`HostSignature`], [`HostEffect`]. The neutral
//!   contract carries descriptors only; [`HostApiSurface::validate_neutral`]
//!   rejects OS/FS/Network/Shell-style binding names.
//!
//! # Canonical form and codec
//!
//! [`canonical_value`] is the single authoritative canonical
//! serialization of [`ScriptValue`]; [`definition_digest`] and
//! [`source_digest`] produce `"sha256:v1:<hex>"` digests over it.
//! [`materialize`]/[`dematerialize`] map typed values to the vendor-
//! neutral `{"$rv": …}` tag-record codec and back, failing closed on
//! unknown tags.
//!
//! # Non-goals (this slice)
//!
//! No real Luau/Rhai backend or analyzer, no FFI, no external VM
//! dependency, no synvelis adapter wiring. Dependency selection
//! (mlua vendored vs native bridge) is deferred to a later slice per
//! `docs/designs/issue-692-script-port.md` (AC4 record).

#![forbid(unsafe_code)]

pub mod analyzer;
pub mod budget;
pub mod canonical;
pub mod capability;
pub mod codec;
pub mod host;
pub mod id;
pub mod registry;
pub mod session;
pub mod value;

// Root re-exports: the contract vocabulary is also reachable as
// `ruviform_script::ScriptValue` etc. The module paths remain the
// canonical namespaces; [`prelude`] is the glob-import convenience.
pub use analyzer::{
    AnalyzeRequest, AnalyzeResult, AnalyzerCapabilities, AnalyzerDescriptor, AnalyzerError,
    AnalyzerRegistry, AnalyzerRegistryError, AnalyzerRequirement, AnalyzerResolveError,
    AnalyzerResolveReason, Completion, CompletionKind, Diagnostic, DiagnosticLevel, ScriptAnalyzer,
    TYPE_DEFINITION_SCHEMA_VERSION, TypeDefinition, WantedCapabilities, verify_result_revision,
};
pub use budget::{BudgetKind, CancelGranularity, CancelToken, ExecutionBudget, PlacementHint};
pub use canonical::{
    CanonicalError, TAG_KEY, canonical_number, canonical_value, definition_digest, hex_decode,
    hex_encode, sha256_hex, source_digest,
};
pub use capability::{BackendCapabilities, ValueKind};
pub use codec::{CodecError, F64_SAFE_I64, dematerialize, is_tagged, materialize};
pub use host::{
    HOST_SURFACE_SCHEMA_VERSION, HostApiSurface, HostBinding, HostEffect, HostSignature,
    HostSurfaceError,
};
pub use id::{
    AnalyzerId, CapabilityId, ResourceDomainId, ScriptBackendId, ScriptBackendVersion,
    ScriptLanguageId, SessionId, TimeDomainId,
};
pub use registry::{
    BackendDescriptor, RegistryError, ResolveError, ResolveErrorReason, ScriptBackendRegistry,
    ScriptRequirement,
};
pub use session::{ScriptBackend, ScriptSession, SessionConfig, SessionError};
pub use value::{
    ARTIFACT_SCHEMA_VERSION, ArtifactKind, ArtifactSource, DefinitionDigest, DiagnosticSeverity,
    ProducedBy, Rational, SOURCE_SCHEMA_VERSION, ScriptArtifact, ScriptDiagnostic, ScriptError,
    ScriptErrorKind, ScriptResult, ScriptSource, ScriptValue, SourceDigest, SourcePosition,
    SourceRange, SourceRevision,
};

/// The RF-SCRIPT-00 contract version implemented by this crate.
///
/// [`BackendDescriptor::version`](crate::BackendDescriptor) and
/// [`AnalyzerDescriptor::version`](crate::AnalyzerDescriptor) advertise
/// `contract_version`; registry resolution fails closed unless it equals
/// this constant and satisfies `min_contract_version`.
pub const CONTRACT_VERSION: u32 = 1;

/// Commonly used contract types.
pub mod prelude {
    pub use crate::CONTRACT_VERSION;
    pub use crate::analyzer::{
        AnalyzeRequest, AnalyzeResult, AnalyzerCapabilities, AnalyzerDescriptor, AnalyzerError,
        AnalyzerRegistry, AnalyzerRegistryError, AnalyzerRequirement, AnalyzerResolveError,
        AnalyzerResolveReason, Completion, CompletionKind, Diagnostic, DiagnosticLevel,
        ScriptAnalyzer, TYPE_DEFINITION_SCHEMA_VERSION, TypeDefinition, WantedCapabilities,
        verify_result_revision,
    };
    pub use crate::budget::{
        BudgetKind, CancelGranularity, CancelToken, ExecutionBudget, PlacementHint,
    };
    pub use crate::canonical::{
        CanonicalError, TAG_KEY, canonical_number, canonical_value, definition_digest, hex_decode,
        hex_encode, sha256_hex, source_digest,
    };
    pub use crate::capability::{BackendCapabilities, ValueKind};
    pub use crate::codec::{CodecError, F64_SAFE_I64, dematerialize, is_tagged, materialize};
    pub use crate::host::{
        HOST_SURFACE_SCHEMA_VERSION, HostApiSurface, HostBinding, HostEffect, HostSignature,
        HostSurfaceError,
    };
    pub use crate::id::{
        AnalyzerId, CapabilityId, ResourceDomainId, ScriptBackendId, ScriptBackendVersion,
        ScriptLanguageId, SessionId, TimeDomainId,
    };
    pub use crate::registry::{
        BackendDescriptor, RegistryError, ResolveError, ResolveErrorReason, ScriptBackendRegistry,
        ScriptRequirement,
    };
    pub use crate::session::{ScriptBackend, ScriptSession, SessionConfig, SessionError};
    pub use crate::value::{
        ARTIFACT_SCHEMA_VERSION, ArtifactKind, ArtifactSource, DefinitionDigest,
        DiagnosticSeverity, ProducedBy, Rational, SOURCE_SCHEMA_VERSION, ScriptArtifact,
        ScriptDiagnostic, ScriptError, ScriptErrorKind, ScriptResult, ScriptSource, ScriptValue,
        SourceDigest, SourcePosition, SourceRange, SourceRevision,
    };
}
