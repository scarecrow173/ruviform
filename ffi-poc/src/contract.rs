//! RF-SCRIPT-00 契約（docs/designs/issue-558-script-contract-design.md C5）の
//! `PoC` ミラー型。契約の公開型を FFI/vendor 型なしで再現するが、
//! **本番契約型ではない**。配置・命名・細部は #558/#559 の統合 Gate で
//! 再判断される `PoC` 専用実装である。
//!
//! ミラー対象: `ScriptAnalyzer`/`AnalyzerRegistry`/`AnalyzerCapabilities`/
//! `AnalyzeRequest`/`AnalyzeResult`/`Diagnostic`/`Completion`/
//! `ScriptBackendVersion`（`impl_version`/`contract_version` 分離）。
//! 契約語彙: diagnostics `Unsupported|Syntax|SyntaxAndType`、`source_revision`
//! 保持、Analyzer 未登録 `Unavailable`（F-18/FX-N-24）、未実装能力
//! `Unsupported`（F-13/FX-N-13）、stale revision は Consumer 拒否（F-14/FX-N-14）。

use std::fmt;

/// Consumer が管理するソース版識別子。編集ごとに単調増加する不透明値。
/// Analyzer 結果はこの値を保持し、Consumer は stale 結果を拒否する。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceRevision(pub u64);

/// 言語識別子（契約 `ScriptLanguageId` のミラー）。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScriptLanguageId(pub String);

impl ScriptLanguageId {
    /// Luau 言語 ID。
    #[must_use]
    pub fn luau() -> Self {
        Self("luau".to_string())
    }
}

impl fmt::Display for ScriptLanguageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// 解析対象ソース（契約 `ScriptSource` の Analyzer 関連部分のミラー）。
/// 契約の `backend_hint`/`schema_version`/`digest` は Runtime/Artifact 経路の
/// 語彙であり、本 `PoC` の Analyzer Port では revision と本文のみを扱う。
#[derive(Debug, Clone)]
pub struct ScriptSource {
    /// UTF-8 ソース本文。
    pub text: String,
    /// 言語 ID。
    pub language: ScriptLanguageId,
    /// Consumer 管理の版識別子。
    pub revision: SourceRevision,
}

/// Diagnostics 深度の能力段階（契約 `AnalyzerCapabilities.diagnostics`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DiagnosticLevel {
    /// 解析能力なし。
    Unsupported,
    /// 構文 Diagnostics のみ。
    Syntax,
    /// 構文 + 型 Diagnostics。
    SyntaxAndType,
}

/// Analyzer が広告する能力集合（契約 `AnalyzerCapabilities` のミラー）。
#[derive(Debug, Clone)]
pub struct AnalyzerCapabilities {
    /// Diagnostics 深度。
    pub diagnostics: DiagnosticLevel,
    /// Completion を提供するか。
    pub completion: bool,
    /// 受理する最大ソースバイト数。0 = 無制限。
    pub max_source_bytes: u64,
    /// 対応言語。
    pub supported_languages: Vec<ScriptLanguageId>,
}

/// Backend 実装バージョン + 契約互換版（契約 `ScriptBackendVersion` のミラー）。
/// `impl_version`（実装 semver/opaque）と `contract_version`（契約版）を分離し、
/// `contract_version` 不一致は Fail Closed。
// フィールド名は RF-SCRIPT-00 契約 `ScriptBackendVersion` の語彙をそのまま
// ミラーするため、postfix 共通は契約準拠であり変更しない。
#[allow(clippy::struct_field_names)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptBackendVersion {
    /// 実装バージョン（semver 文字列）。
    pub impl_version: String,
    /// 契約互換バージョン。
    pub contract_version: u32,
}

/// Analyzer の静的記述（契約 `BackendDescriptor` 相当の Analyzer 版）。
#[derive(Debug, Clone)]
pub struct AnalyzerDescriptor {
    /// Analyzer 実装識別子。
    pub analyzer_id: String,
    /// 対象言語。
    pub language: ScriptLanguageId,
    /// バージョン（impl/contract 分離）。
    pub version: ScriptBackendVersion,
    /// 広告能力。
    pub capabilities: AnalyzerCapabilities,
}

/// 解析要求時に要求する能力（契約 `wanted_capabilities` のミラー）。
#[derive(Debug, Clone, Default)]
pub struct WantedCapabilities {
    /// 要求する Diagnostics 深度。`None` = diagnostics 不要。
    pub diagnostics: Option<DiagnosticLevel>,
    /// Completion を要求するか。
    pub completion: bool,
}

/// Consumer 提供の型定義スナップショット 1 件（契約
/// `type_definitions: Option<DefinitionSnapshotRef>` の `PoC` ミラー）。
/// `.d.luau` 相当の型定義ソースを名前付きで保持する。
#[derive(Debug, Clone)]
pub struct TypeDefinition {
    /// 型定義のパッケージ名（Luau の packageName 相当）。
    pub package_name: String,
    /// 型定義ソース（UTF-8）。
    pub source: String,
}

/// カーソル位置。UTF-8 バイトオフセット（`source.text` の byte index）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cursor {
    /// バイトオフセット。
    pub byte_offset: usize,
}

/// 解析要求（契約 `AnalyzeRequest` のミラー）。
#[derive(Debug, Clone)]
pub struct AnalyzeRequest {
    /// 解析対象ソース。
    pub source: ScriptSource,
    /// 対象言語（`source.language` と一致することを Registry が検査）。
    pub language: ScriptLanguageId,
    /// Consumer 提供の型定義。
    pub type_definitions: Vec<TypeDefinition>,
    /// 要求能力。
    pub wanted: WantedCapabilities,
    /// Completion カーソル。`None` = Completion を要求しない。
    pub cursor: Option<Cursor>,
}

/// ソース内位置（0 始まり line / 行内バイト column — Luau `Position` と同じ
/// 意味論。UTF-8 の column はバイト単位）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourcePosition {
    /// 0 始まり行番号。
    pub line: u32,
    /// 行内バイトオフセット。
    pub column: u32,
}

/// ソース範囲。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceRange {
    /// 開始位置。
    pub start: SourcePosition,
    /// 終了位置。
    pub end: SourcePosition,
}

/// Diagnostic 深刻度（契約 `Diagnostic.severity` のミラー）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DiagnosticSeverity {
    /// 情報。
    Information,
    /// 警告。
    Warning,
    /// エラー。
    Error,
}

/// Diagnostic 種別（構文 / 型 / その他を区別する `PoC` 語彙）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DiagnosticKind {
    /// 構文（parse）エラー。
    Syntax,
    /// 型エラー。
    Type,
    /// その他（lint 等、将来拡張用）。
    Other,
}

/// 構造化 Diagnostic（契約 `Diagnostic` のミラー）。
/// `source_revision` を保持し、stale 判定は Consumer が行う。
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// 深刻度。
    pub severity: DiagnosticSeverity,
    /// 種別。
    pub kind: DiagnosticKind,
    /// 機械可読コード（Luau の error code または "parse"）。
    pub code: String,
    /// 人間可読メッセージ。
    pub message: String,
    /// ソース範囲。
    pub range: SourceRange,
    /// 解析対象の `source_revision`。
    pub source_revision: SourceRevision,
}

/// Completion 種別（契約 `Completion.kind` のミラー）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CompletionKind {
    /// プロパティ。
    Property,
    /// 束縛（変数/関数）。
    Binding,
    /// キーワード。
    Keyword,
    /// 文字列。
    String,
    /// 型。
    Type,
    /// モジュール。
    Module,
    /// 生成関数。
    GeneratedFunction,
    /// require パス。
    RequirePath,
    /// ホットコメント。
    HotComment,
    /// その他。
    Other,
}

/// 補完候補（契約 `Completion` のミラー）。
#[derive(Debug, Clone)]
pub struct Completion {
    /// 候補ラベル。
    pub label: String,
    /// 種別。
    pub kind: CompletionKind,
    /// 詳細（型文字列等、任意）。
    pub detail: Option<String>,
    /// 挿入テキスト（任意）。
    pub insert_text: Option<String>,
    /// 置換範囲。
    pub replace_range: SourceRange,
    /// 解析対象の `source_revision`。
    pub source_revision: SourceRevision,
}

/// 解析結果（契約 `AnalyzeResult` のミラー）。
/// `source_revision` は要求時の `ScriptSource.revision` をそのまま保持する。
/// `completions` は Completion を要求しなかった場合 `None`。
#[derive(Debug, Clone)]
pub struct AnalyzeResult {
    /// 解析対象の版識別子。
    pub source_revision: SourceRevision,
    /// 構造化 Diagnostics。
    pub diagnostics: Vec<Diagnostic>,
    /// Completion 候補（要求時のみ `Some`）。
    pub completions: Option<Vec<Completion>>,
}

/// Analyzer エラー（契約 F-13/F-14/F-18/FX-N-13/FX-N-14/FX-N-24 の語彙ミラー）。
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum AnalyzerError {
    /// Analyzer 未登録/利用不可（F-18/FX-N-24）。Runtime Backend の解決とは独立。
    Unavailable,
    /// 未実装能力の要求（F-13/FX-N-13）。`capability` は "completion" 等の語彙。
    Unsupported {
        /// 未対応と判明した能力名。
        capability: String,
    },
    /// `source_revision` 不一致の stale 結果（F-14/FX-N-14 — Consumer 側拒否）。
    StaleResult,
    /// 要求自体が不正（UTF-8/カーソル範囲/サイズ上限等）。
    InvalidRequest {
        /// 不正の理由。
        reason: String,
    },
    /// ABI/契約 version 不一致（fail-closed）。
    VersionMismatch {
        /// 不一致のあった項目。
        field: String,
        /// 期待値。
        expected: String,
        /// 実測値。
        actual: String,
    },
    /// FFI/ネイティブ層の障害（status code + message）。
    Bridge {
        /// shim のステータスコード。
        status: i32,
        /// shim のエラーメッセージ。
        message: String,
    },
    /// その他の内部エラー。
    Internal {
        /// 理由。
        message: String,
    },
}

impl fmt::Display for AnalyzerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable => write!(f, "analyzer unavailable"),
            Self::Unsupported { capability } => write!(f, "unsupported capability: {capability}"),
            Self::StaleResult => write!(f, "stale analyzer result"),
            Self::InvalidRequest { reason } => write!(f, "invalid request: {reason}"),
            Self::VersionMismatch {
                field,
                expected,
                actual,
            } => write!(
                f,
                "version mismatch on {field}: expected {expected}, got {actual}"
            ),
            Self::Bridge { status, message } => {
                write!(f, "bridge error (status {status}): {message}")
            }
            Self::Internal { message } => write!(f, "internal error: {message}"),
        }
    }
}

impl std::error::Error for AnalyzerError {}

/// `AnalyzeResult` が要求時の `source_revision` と一致するかを検査する
/// Consumer 側 stale 拒否（契約 F-14/FX-N-14 の責務は Consumer）。
/// 不一致なら `AnalyzerError::StaleResult` を返す。
pub fn verify_result_revision(
    result: &AnalyzeResult,
    expected: SourceRevision,
) -> Result<(), AnalyzerError> {
    if result.source_revision == expected {
        Ok(())
    } else {
        Err(AnalyzerError::StaleResult)
    }
}
