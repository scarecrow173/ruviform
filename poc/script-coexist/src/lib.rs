//! RF-SCRIPT-01 PoC — Luau+Rhai 単一バイナリ共存・Backend Registry/Typed Port。
//!
//! Issue #559。RF-SCRIPT-00 契約（Issue #558, `docs/specs/issue-558-script-contract.md`
//! 等）をミラーした**最小ローカル実装**であり、本番 crate/API ではない。
//! `ScriptBackendRegistry` は契約のみで本番実装が存在しないため、本 crate の
//! `contract` モジュールが契約型を PoC 用にミラーして実装する。
//!
//! 検証対象:
//! - 同一実行ファイルへの Luau(mlua)+Rhai 同時リンク・初期化
//! - 言語/backend requirement + version による実行時明示選択
//! - 同一意味 artifact の canonical digest 一致（混種 Workflow 参照）
//! - Fail-Closed（F-01..F-18 / M-01..M-13 の該当行）
//! - 公開型への vendor 型非漏出・値モデル（Absent/Null/i64/Time/ID）

#![forbid(unsafe_code)]

pub mod analyzer;
pub mod backends;
pub mod canonical;
pub mod codec;
pub mod contract;
pub mod plan;
pub mod value;

/// よく使う型の prelude。
pub mod prelude {
    pub use crate::analyzer::{
        AnalyzeResult, AnalyzerRegistry, AnalyzerRequirement, ScriptAnalyzer,
    };
    pub use crate::backends::luau::LuauBackend;
    pub use crate::backends::rhai::RhaiBackend;
    pub use crate::contract::{
        BackendCapabilities, BackendDescriptor, BudgetKind, CancelGranularity, CancelToken,
        CapabilityId, ExecutionBudget, HostApiSurface, HostBinding, HostEffect, HostHandler,
        HostSignature, POC_CONTRACT_VERSION, PlacementHint, RegistryError, ResolveError,
        ResolveErrorReason, ScriptBackend, ScriptBackendId, ScriptBackendRegistry,
        ScriptLanguageId, ScriptRequirement, ScriptSession, SessionConfig, SessionError, SessionId,
        ValueKind,
    };
    pub use crate::plan::{
        DefinitionRef, DefinitionStore, PlanError, ResolvedArtifact, WorkflowPlan,
    };
    pub use crate::value::{
        ArtifactKind, ArtifactSource, BudgetKind as ErrorBudgetKind, DefinitionDigest,
        DiagnosticSeverity, ProducedBy, Rational, ResourceDomainId, ScriptArtifact,
        ScriptDiagnostic, ScriptError, ScriptErrorKind, ScriptResult, ScriptSource, ScriptValue,
        SourceDigest, SourcePosition, SourceRange, SourceRevision, TimeDomainId,
    };
}
