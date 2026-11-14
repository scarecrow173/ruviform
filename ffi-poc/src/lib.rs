//! ruviform-ffi-poc — RF-SCRIPT-02 (Issue #560) Luau.Analysis Native Bridge `PoC`。
//!
//! RF-SCRIPT-00 契約（ScriptAnalyzer/AnalyzerRegistry）の最小ローカルミラーと、
//! vendored Luau.Analysis (Luau 0.740) を呼ぶ狭小 C ABI FFI shim を提供する。
//! **本番公開 crate ではない** — 採否は #558/#559 の統合 Gate で別判断。
//!
//! 境界規約:
//! - C++/Luau/vendor 型・ポインタは公開 API に出さない（JSON + status code のみ）。
//! - `unsafe` は `ffi` モジュールに局所許可で隔離する。
//! - Analyzer の成否は実行 Grant を意味しない（契約 C5 — Runtime とは独立 Port）。

pub mod analyzer;
pub mod contract;
mod ffi;
#[cfg(test)]
mod ffi_boundary_tests;

pub use analyzer::{
    AnalyzerRegistry, LuauAnalyzer, LuauAnalyzerOptions, ScriptAnalyzer, default_wanted,
    revision_of,
};
pub use contract::{
    AnalyzeRequest, AnalyzeResult, AnalyzerCapabilities, AnalyzerDescriptor, AnalyzerError,
    Completion, CompletionKind, Cursor, Diagnostic, DiagnosticKind, DiagnosticLevel,
    DiagnosticSeverity, ScriptBackendVersion, ScriptLanguageId, ScriptSource, SourcePosition,
    SourceRange, SourceRevision, TypeDefinition, WantedCapabilities, verify_result_revision,
};
