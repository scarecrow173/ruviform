//! RF-SCRIPT-01 PoC — `ScriptValue`/`ScriptError`/`ScriptSource`/
//! `ScriptArtifact` の契約ミラー（契約設計 C3）。
//!
//! 閉じた variant 集合・Absent≠Null・i64 厳密・Time/ResourceId 保持・
//! VM 固有型/Grant/Secret/製品型の非露出を型レベルで体現する。

use std::collections::BTreeMap;
use std::fmt;

use crate::canonical;
use crate::contract::{ScriptBackendId, ScriptLanguageId, SessionId};

// ---------------------------------------------------------------------------
// 識別子 newtype
// ---------------------------------------------------------------------------

/// Time のドメイン識別子（ClockDomainId 相当の不透明文字列）。
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TimeDomainId(pub String);

/// ResourceId のドメイン識別子（リソース種別の不透明文字列）。
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResourceDomainId(pub String);

/// Time の timebase（有理数 num/den。暗黙の単位丸め禁止 — C3）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rational {
    pub num: i64,
    pub den: i64,
}

/// Source の内容ハッシュ（`"sha256:v1:<hex>"` 形式）。
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceDigest(pub String);

impl fmt::Display for SourceDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Artifact payload の正規化ハッシュ（`"sha256:v1:<hex>"` 形式）。
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DefinitionDigest(pub String);

impl fmt::Display for DefinitionDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Consumer が管理する source 編集版（単調増加）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct SourceRevision(pub u64);

/// ソース位置（offset/line/col）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourcePosition {
    pub offset: u64,
    pub line: u32,
    pub col: u32,
}

/// 診断の範囲（byte offset）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceRange {
    pub start: u64,
    pub end: u64,
}

// ---------------------------------------------------------------------------
// ScriptValue（C3 閉じた variant 集合）
// ---------------------------------------------------------------------------

/// 言語中立の値モデル。VM テーブル/関数/userdata/coroutine/thread・
/// ネイティブポインタ・Grant/Secret/製品型・Rust Any を含まない。
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum ScriptValue {
    /// フィールド未提供/optional 未設定。`Null` と区別する。
    Absent,
    /// 明示的 null。
    Null,
    Bool(bool),
    /// 厳密な i64。表現不能な整数は `ValueOutOfRange` で拒否。
    Integer(i64),
    /// IEEE-754 binary64。
    Number(f64),
    String(String),
    Bytes(Vec<u8>),
    Array(Vec<ScriptValue>),
    Record(BTreeMap<String, ScriptValue>),
    /// domain + timebase + count を保持。暗黙のドメイン変換・単位丸め禁止。
    Time {
        domain: TimeDomainId,
        timebase: Rational,
        count: i64,
    },
    /// domain + kind + value（opaque string）。VM/契約は解釈しない。
    ResourceId {
        domain: ResourceDomainId,
        kind: String,
        value: String,
    },
}

impl ScriptValue {
    pub fn record(entries: impl IntoIterator<Item = (String, ScriptValue)>) -> Self {
        Self::Record(entries.into_iter().collect())
    }

    pub fn as_record(&self) -> Option<&BTreeMap<String, ScriptValue>> {
        match self {
            Self::Record(r) => Some(r),
            _ => None,
        }
    }

    pub fn get(&self, key: &str) -> Option<&ScriptValue> {
        self.as_record().and_then(|r| r.get(key))
    }

    pub fn is_absent(&self) -> bool {
        matches!(self, Self::Absent)
    }
}

// ---------------------------------------------------------------------------
// ScriptError（C3 言語中立エラー）
// ---------------------------------------------------------------------------

/// 予算種別（`ExecutionBudget` の項目に対応）。
/// contract.rs は本型を re-export して単一の正本とする。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum BudgetKind {
    MaxInstructions,
    MaxWallTime,
    MaxMemoryBytes,
    MaxOutputBytes,
    MaxHostCalls,
}

/// 言語中立エラー分類（契約設計 C3 `ScriptErrorKind` + PoC 追加分）。
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum ScriptErrorKind {
    Parse,
    Compile,
    Type,
    Runtime,
    BudgetExceeded(BudgetKind),
    Cancelled,
    /// 未対応の能力・型（`Unsupported{what}`）。
    Unsupported(String),
    /// Host binding 由来のエラー（`Host{detail}`）。
    Host(String),
    Backend,
    Internal,
    /// i64 で厳密に表現できない整数の越境（F-12/FX-N-12）。
    ValueOutOfRange,
    /// non-RT 保証など契約違反の検出（F-15/FX-N-15）。
    ContractViolation,
}

/// `ScriptError`（契約設計 C3）。vendor エラー型は公開面に出さず、
/// Backend が本型へ写像する責務を負う。
#[derive(Clone, Debug)]
pub struct ScriptError {
    pub kind: ScriptErrorKind,
    // Box<str>: Err variant を 128B 未満に抑え clippy::result_large_err を
    // 通すための意図的な選択（エラーは頻繁に生成されない）。
    pub message: Box<str>,
    pub position: Option<SourcePosition>,
    pub backend_id: Option<ScriptBackendId>,
    pub session_id: Option<SessionId>,
    pub cause: Option<Box<ScriptError>>,
}

impl ScriptError {
    pub fn new(kind: ScriptErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into().into_boxed_str(),
            position: None,
            backend_id: None,
            session_id: None,
            cause: None,
        }
    }

    pub fn with_backend(mut self, id: &ScriptBackendId) -> Self {
        self.backend_id = Some(id.clone());
        self
    }

    pub fn with_session(mut self, id: SessionId) -> Self {
        self.session_id = Some(id);
        self
    }

    pub fn contract_violation(message: impl Into<String>) -> Self {
        Self::new(ScriptErrorKind::ContractViolation, message)
    }

    pub fn value_out_of_range(message: impl Into<String>) -> Self {
        Self::new(ScriptErrorKind::ValueOutOfRange, message)
    }
}

impl fmt::Display for ScriptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)
    }
}

impl std::error::Error for ScriptError {}

// ---------------------------------------------------------------------------
// ScriptSource / SourceDigest / SourceRevision（C3）
// ---------------------------------------------------------------------------

/// `ScriptSource`（契約設計 C3）。`digest` は内容同一性、
/// `revision` は consumer 側の編集版。
#[derive(Clone, Debug)]
pub struct ScriptSource {
    pub text: String,
    pub language: ScriptLanguageId,
    pub backend_hint: Option<ScriptBackendId>,
    pub schema_version: u32,
    pub digest: SourceDigest,
    pub revision: SourceRevision,
}

impl ScriptSource {
    /// UTF-8 ソースから digest を計算して構築する。
    pub fn new(
        text: impl Into<String>,
        language: ScriptLanguageId,
        revision: SourceRevision,
    ) -> Self {
        let text = text.into();
        Self {
            digest: canonical::source_digest(&text),
            text,
            language,
            backend_hint: None,
            schema_version: 1,
            revision,
        }
    }
}

// ---------------------------------------------------------------------------
// ScriptArtifact（C3 型付き定義文書）
// ---------------------------------------------------------------------------

/// Artifact 種別（契約設計 C3 `ArtifactKind`）。
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum ArtifactKind {
    ActionRecipe,
    WorkflowDefinition,
    ConsumerExtension { domain: String },
}

impl ArtifactKind {
    /// script が返す `"kind"` 文字列との対応（PoC 規約）。
    pub fn from_script_str(s: &str) -> Option<Self> {
        match s {
            "action_recipe" => Some(Self::ActionRecipe),
            "workflow_definition" => Some(Self::WorkflowDefinition),
            _ => s
                .strip_prefix("consumer_extension:")
                .map(|d| Self::ConsumerExtension {
                    domain: d.to_owned(),
                }),
        }
    }

    pub fn as_script_str(&self) -> String {
        match self {
            Self::ActionRecipe => "action_recipe".to_owned(),
            Self::WorkflowDefinition => "workflow_definition".to_owned(),
            Self::ConsumerExtension { domain } => format!("consumer_extension:{domain}"),
        }
    }
}

/// Artifact の由来情報。
#[derive(Clone, Debug)]
pub struct ArtifactSource {
    pub language: ScriptLanguageId,
    pub backend_id: ScriptBackendId,
    pub source_digest: SourceDigest,
    pub source_revision: SourceRevision,
}

/// Artifact の生成者情報。
#[derive(Clone, Debug)]
pub struct ProducedBy {
    pub backend_id: ScriptBackendId,
    pub impl_version: String,
}

/// `ScriptArtifact`（契約設計 C3）。`payload` は canonical form で
/// シリアライズ可能な型付き定義本体。VM 状態・Grant/Secret を含めない。
#[derive(Clone, Debug)]
pub struct ScriptArtifact {
    pub schema_version: u32,
    pub kind: ArtifactKind,
    pub digest: DefinitionDigest,
    pub payload: ScriptValue,
    pub source: ArtifactSource,
    pub produced_by: ProducedBy,
}

// ---------------------------------------------------------------------------
// ScriptResult / ScriptDiagnostic（C2）
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

/// 実行時/解析時の構造化診断（`source_revision` は consumer 側で
/// `AnalyzeResult` が保持 — C5）。
#[derive(Clone, Debug)]
pub struct ScriptDiagnostic {
    pub severity: DiagnosticSeverity,
    pub code: String,
    pub message: String,
    pub range: Option<SourceRange>,
}

/// `ScriptResult`（契約設計 C2）。
/// `artifact` は戻り値が artifact 形状（`kind` を持つ Record）の場合のみ
/// `Some`。値モデル往復など artifact を生じない評価では `None`。
#[derive(Clone, Debug)]
pub struct ScriptResult {
    pub artifact: Option<ScriptArtifact>,
    pub return_value: Option<ScriptValue>,
    pub diagnostics: Vec<ScriptDiagnostic>,
}
