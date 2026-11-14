//! RF-SCRIPT-01 PoC — RF-SCRIPT-00 契約（Issue #558）のローカルミラー。
//!
//! `docs/designs/issue-558-script-contract-design.md` の C1/C2/C4/C6 の
//! 型・意味論を PoC 用に最小実装したもの。**本番 API ではない。**
//! `ScriptBackendRegistry` は契約のみで本番実装が存在しないため、
//! 本モジュールが契約型をミラーした PoC registry を提供する。

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use crate::value::{
    ArtifactKind, ScriptError, ScriptResult, ScriptSource, ScriptValue, SourceDigest,
};

// BudgetKind の正本は value.rs（ScriptErrorKind::BudgetExceeded と共用）。
// re-export して contract 側の語彙としても使えるようにする。
pub use crate::value::BudgetKind;

/// PoC がミラーする契約バージョン。契約設計 C1 `contract_version`。
pub const POC_CONTRACT_VERSION: u32 = 1;

// ---------------------------------------------------------------------------
// 識別子（C1）
// ---------------------------------------------------------------------------

macro_rules! string_id {
    ($(#[$m:meta])* $name:ident) => {
        $(#[$m])*
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            /// 不透明文字列から識別子を構築する。検証は行わない
            /// （契約上は小文字・一意であることが慣習。resolve 側で失敗する）。
            pub fn new(s: impl Into<String>) -> Self {
                Self(s.into())
            }
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl From<String> for $name {
            fn from(s: String) -> Self {
                Self(s)
            }
        }

        impl From<&str> for $name {
            fn from(s: &str) -> Self {
                Self(s.to_owned())
            }
        }
    };
}

string_id!(
    /// 言語識別子（例: `"luau"`, `"rhai"`）。`"lua"`/`"lua54"` は別 ID。
    ScriptLanguageId
);
string_id!(
    /// Backend 実装識別子（例: `"luau-mlua"`, `"rhai-embedded"`）。
    ScriptBackendId
);
string_id!(
    /// Analyzer 実装識別子。
    AnalyzerId
);
string_id!(
    /// Capability 識別子（`"budget.max_memory_bytes"` 等の不透明名）。
    CapabilityId
);

/// Session 識別子。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SessionId(pub u64);

/// 協調的 cancel のハンドル。`ScriptSession::cancel` と同じ内部フラグを
/// 共有するため、評価を別スレッドで走らせている間に cancel を発火できる
/// （契約の cooperative cancel を PoC で観測可能にするための最小手段。
/// 言語中立で vendor 型を含まない）。
#[derive(Clone)]
pub struct CancelToken {
    flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl CancelToken {
    pub(crate) fn from_flag(flag: std::sync::Arc<std::sync::atomic::AtomicBool>) -> Self {
        Self { flag }
    }

    /// cancel を発火する（冪等）。
    pub fn cancel(&self) {
        self.flag.store(true, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.flag.load(std::sync::atomic::Ordering::Relaxed)
    }
}

impl std::fmt::Debug for CancelToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CancelToken")
            .field("cancelled", &self.is_cancelled())
            .finish()
    }
}

/// 配置特性の要求（任意・情報提供用）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PlacementHint {
    /// `"owner_thread"` / `"worker"` 等の不透明ヒント。
    pub kind: String,
}

// ---------------------------------------------------------------------------
// Capabilities（C1 `BackendCapabilities`）
// ---------------------------------------------------------------------------

/// `ScriptValue` の variant に対応する値種別タグ。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum ValueKind {
    Absent,
    Null,
    Bool,
    Integer,
    Number,
    String,
    Bytes,
    Array,
    Record,
    Time,
    ResourceId,
}

/// Cancel が観測される粒度（能力広告。M-02 参照）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CancelGranularity {
    /// VM safepoint で観測（協調的 cancel）。
    Safepoint,
    /// Host 呼出し境界でのみ観測。
    HostBoundaryOnly,
    /// Cancel 非対応。
    Unsupported,
}

/// Backend が広告する能力集合（契約設計 C1）。
///
/// `capability_ids()` が resolve の `required_capabilities` 検査に使う
/// 平坦な ID 集合を導出する。
#[derive(Clone, Debug)]
pub struct BackendCapabilities {
    /// 対応する `ScriptValue` 種別。
    pub value_kinds: BTreeSet<ValueKind>,
    /// 強制可能な Budget 種別。
    pub enforced_budgets: BTreeSet<BudgetKind>,
    /// owner-thread 配置のみをサポートするか。
    pub owner_thread_only: bool,
    /// Session を Send 可能スレッドへ移動できるか（広告のみ）。
    pub send: bool,
    /// Session を共有参照できるか（広告のみ）。
    pub sync: bool,
    /// 受理する最大 source byte 数。
    pub max_source_bytes: Option<u64>,
    /// 生成可能な Artifact 種別。
    pub artifact_kinds: BTreeSet<ArtifactKind>,
    /// Cancel 粒度。
    pub cancel_granularity: CancelGranularity,
    /// Session 間で VM 可変状態を共有するか（契約上は明示時のみ例外）。
    pub shared_vm_state: bool,
    /// VM が i64 をネイティブに保持できるか（codec の情報提供用。
    /// `false` でも codec 経由で i64 を保存できる場合がある）。
    pub native_integer: bool,
}

impl BackendCapabilities {
    /// resolve 検査用の平坦な Capability ID 集合。
    pub fn capability_ids(&self) -> BTreeSet<CapabilityId> {
        let mut ids = BTreeSet::new();
        for kind in &self.value_kinds {
            let id = match kind {
                ValueKind::Absent => "value.absent",
                ValueKind::Null => "value.null",
                ValueKind::Bool => "value.bool",
                ValueKind::Integer => "value.integer",
                ValueKind::Number => "value.number",
                ValueKind::String => "value.string",
                ValueKind::Bytes => "value.bytes",
                ValueKind::Array => "value.array",
                ValueKind::Record => "value.record",
                ValueKind::Time => "value.time",
                ValueKind::ResourceId => "value.resource_id",
            };
            ids.insert(CapabilityId::new(id));
        }
        for kind in &self.enforced_budgets {
            let id = match kind {
                BudgetKind::MaxInstructions => "budget.max_instructions",
                BudgetKind::MaxWallTime => "budget.max_wall_time",
                BudgetKind::MaxMemoryBytes => "budget.max_memory_bytes",
                BudgetKind::MaxOutputBytes => "budget.max_output_bytes",
                BudgetKind::MaxHostCalls => "budget.max_host_calls",
            };
            ids.insert(CapabilityId::new(id));
        }
        if self.owner_thread_only {
            ids.insert(CapabilityId::new("placement.owner_thread"));
        }
        if self.send {
            ids.insert(CapabilityId::new("placement.send"));
        }
        if self.sync {
            ids.insert(CapabilityId::new("placement.sync"));
        }
        if self.cancel_granularity != CancelGranularity::Unsupported {
            ids.insert(CapabilityId::new("cancel.supported"));
        }
        if self.cancel_granularity == CancelGranularity::Safepoint {
            ids.insert(CapabilityId::new("cancel.safepoint"));
        }
        if self.shared_vm_state {
            ids.insert(CapabilityId::new("session.shared_vm_state"));
        }
        if self.native_integer {
            ids.insert(CapabilityId::new("value.native_integer"));
        }
        for kind in &self.artifact_kinds {
            let id = match kind {
                ArtifactKind::ActionRecipe => "artifact.action_recipe",
                ArtifactKind::WorkflowDefinition => "artifact.workflow_definition",
                ArtifactKind::ConsumerExtension { .. } => "artifact.consumer_extension",
            };
            ids.insert(CapabilityId::new(id));
        }
        ids
    }
}

/// Backend の静的記述（契約設計 C2 `BackendDescriptor`）。
#[derive(Clone, Debug)]
pub struct BackendDescriptor {
    pub id: ScriptBackendId,
    pub language: ScriptLanguageId,
    pub impl_version: String,
    pub contract_version: u32,
    pub capabilities: BackendCapabilities,
}

// ---------------------------------------------------------------------------
// Runtime Port（C2）
// ---------------------------------------------------------------------------

/// `ExecutionBudget`（契約設計 C4）。各項目 `None` = 無制限。
#[derive(Clone, Debug, Default)]
pub struct ExecutionBudget {
    pub max_instructions: Option<u64>,
    pub max_wall_time: Option<Duration>,
    pub max_memory_bytes: Option<u64>,
    pub max_output_bytes: Option<u64>,
    pub max_host_calls: Option<u64>,
}

impl ExecutionBudget {
    /// 全項目無制限。
    pub fn unlimited() -> Self {
        Self::default()
    }

    /// 要求されている Budget 種別の集合。
    pub fn requested_kinds(&self) -> BTreeSet<BudgetKind> {
        let mut kinds = BTreeSet::new();
        if self.max_instructions.is_some() {
            kinds.insert(BudgetKind::MaxInstructions);
        }
        if self.max_wall_time.is_some() {
            kinds.insert(BudgetKind::MaxWallTime);
        }
        if self.max_memory_bytes.is_some() {
            kinds.insert(BudgetKind::MaxMemoryBytes);
        }
        if self.max_output_bytes.is_some() {
            kinds.insert(BudgetKind::MaxOutputBytes);
        }
        if self.max_host_calls.is_some() {
            kinds.insert(BudgetKind::MaxHostCalls);
        }
        kinds
    }
}

/// Host binding の副作用分類（助言情報。権限根拠にしない — C6）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum HostEffect {
    Pure,
    Query,
    Effect,
}

/// `ScriptValue` 入出力の型付きシグネチャ。
#[derive(Clone, Debug)]
pub struct HostSignature {
    pub args: Vec<ValueKind>,
    pub returns: ValueKind,
}

/// Script が呼び出せる Host 関数の記述子（C6）。実装・権限は consumer 責務。
#[derive(Clone, Debug)]
pub struct HostBinding {
    pub name: String,
    pub signature: HostSignature,
    pub effect: HostEffect,
    pub schema_version: u32,
    pub timeout_hint: Option<Duration>,
}

/// Host API 面の記述子集合（C6）。中立契約に OS/FS/Net/Shell binding を
/// 含めない — 必要なら consumer が注入する。
#[derive(Clone, Debug, Default)]
pub struct HostApiSurface {
    pub bindings: Vec<HostBinding>,
    pub schema_version: u32,
}

/// `validate_neutral` が拒否する binding 名 prefix（PoC の consumer
/// ポリシー模擬。契約は中立面のみ規定し、拒否自体は consumer 責務 — F-01
/// 範囲の M-01/FX-N-18）。
const UNSAFE_BINDING_PREFIXES: &[&str] = &[
    "os.", "fs.", "net.", "shell.", "process.", "env.", "sys.", "danger.",
];

impl HostApiSurface {
    pub fn binding(&self, name: &str) -> Option<&HostBinding> {
        self.bindings.iter().find(|b| b.name == name)
    }

    /// 中立契約面として安全か検査する。OS/FS/Network/Shell 系の binding
    /// 名を拒否する（FX-N-18）。
    pub fn validate_neutral(&self) -> Result<(), String> {
        for binding in &self.bindings {
            for prefix in UNSAFE_BINDING_PREFIXES {
                if binding.name.starts_with(prefix) {
                    return Err(format!(
                        "unsafe host binding name {:?} (prefix {prefix:?})",
                        binding.name
                    ));
                }
            }
        }
        Ok(())
    }
}

/// PoC の mock Host handler。引数は codec 済み `ScriptValue`、
/// 戻り値も `ScriptValue`。実装は consumer 責務の模擬。
pub type HostHandler = Arc<dyn Fn(&[ScriptValue]) -> Result<ScriptValue, String> + Send + Sync>;

/// `open_session` 時の設定（契約設計 C2 `SessionConfig`）。
#[derive(Clone)]
pub struct SessionConfig {
    pub budget: ExecutionBudget,
    pub host_bindings: HostApiSurface,
    /// PoC 拡張: binding 名 → mock handler。`host_bindings` で宣言された
    /// 面に対応する実装を consumer が注入する想定。
    pub host_handlers: BTreeMap<String, HostHandler>,
    /// この Session が生成してよい Artifact 種別。
    pub artifact_kinds: BTreeSet<ArtifactKind>,
    pub source: ScriptSource,
    /// Resume/再読込時に期待する source digest（F-08 検出用）。
    pub resume: Option<SourceDigest>,
    pub placement_hint: Option<PlacementHint>,
    /// Session 開始時に VM へ `INPUT` として push される入力値
    /// （PoC 拡張。値モデル往復 Fixture の入力注入に使う）。
    pub inputs: BTreeMap<String, ScriptValue>,
}

/// `open_session` の失敗（契約設計 C2 `SessionError`）。
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum SessionError {
    /// 要求された Budget 種別が Backend 未対応（F-06）。
    UnsupportedBudget { kind: BudgetKind },
    /// Host binding 面が不正（未宣言 handler・unsafe 名等）。
    InvalidHostSurface { reason: String },
    /// Source digest 不一致（F-08/M-12）。
    SourceDigestMismatch {
        expected: SourceDigest,
        actual: SourceDigest,
    },
    /// `max_source_bytes` 超過（M-10/FX-N-22）。
    SourceTooLarge { bytes: u64, max: u64 },
    /// source.language が Backend の language と不一致。
    LanguageMismatch {
        expected: ScriptLanguageId,
        actual: ScriptLanguageId,
    },
    /// Session 間状態共有の契約違反（F-16/M-05）。
    IsolationViolation { detail: String },
    /// リソース不足。
    ResourceExhausted { detail: String },
    /// Backend 内部エラー。
    Backend { detail: String },
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedBudget { kind } => write!(f, "unsupported budget: {kind:?}"),
            Self::InvalidHostSurface { reason } => write!(f, "invalid host surface: {reason}"),
            Self::SourceDigestMismatch { expected, actual } => {
                write!(
                    f,
                    "source digest mismatch: expected {expected}, got {actual}"
                )
            }
            Self::SourceTooLarge { bytes, max } => {
                write!(f, "source too large: {bytes} > {max}")
            }
            Self::LanguageMismatch { expected, actual } => {
                write!(f, "language mismatch: expected {expected}, got {actual}")
            }
            Self::IsolationViolation { detail } => write!(f, "isolation violation: {detail}"),
            Self::ResourceExhausted { detail } => write!(f, "resource exhausted: {detail}"),
            Self::Backend { detail } => write!(f, "backend error: {detail}"),
        }
    }
}

impl std::error::Error for SessionError {}

/// `resolve` の失敗理由（Fail-Closed 判定表 F-01〜F-06）。
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ResolveErrorReason {
    UnknownLanguage,
    UnknownBackend,
    UnsupportedContractVersion,
    MissingCapability,
    /// language/backend は既知だが契約検査を通る候補がない。
    NoCandidates,
}

/// `ScriptBackendRegistry::resolve` / `AnalyzerRegistry::resolve` の失敗。
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum ResolveError {
    NoMatchingBackend {
        reason: ResolveErrorReason,
        detail: String,
    },
    AmbiguousBackend {
        candidates: Vec<ScriptBackendId>,
    },
}

impl fmt::Display for ResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoMatchingBackend { reason, detail } => {
                write!(f, "no matching backend ({reason:?}): {detail}")
            }
            Self::AmbiguousBackend { candidates } => {
                write!(f, "ambiguous backend candidates: {candidates:?}")
            }
        }
    }
}

impl std::error::Error for ResolveError {}

/// `register` の失敗。
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum RegistryError {
    DuplicateBackend(ScriptBackendId),
}

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateBackend(id) => write!(f, "duplicate backend id: {id}"),
        }
    }
}

impl std::error::Error for RegistryError {}

/// `resolve` の入力要件（契約設計 C1 `ScriptRequirement`）。
#[derive(Clone, Debug, Default)]
pub struct ScriptRequirement {
    pub language: Option<ScriptLanguageId>,
    pub backend: Option<ScriptBackendId>,
    /// 必須 Capability ID 集合（`BackendCapabilities::capability_ids()`
    /// の部分集合であること）。
    pub required_capabilities: BTreeSet<CapabilityId>,
    /// 許容する最小 contract_version。
    pub min_contract_version: Option<u32>,
    /// 必要な Artifact 種別。
    pub artifact_kinds: BTreeSet<ArtifactKind>,
}

/// Runtime Backend 実装の trait（契約設計 C2）。vendor 型を出さない。
pub trait ScriptBackend {
    fn descriptor(&self) -> &BackendDescriptor;
    fn open_session(&self, config: &SessionConfig) -> Result<Box<dyn ScriptSession>, SessionError>;
}

/// 1 回の評価コンテキスト（契約設計 C2 `ScriptSession`）。
/// Send は要求する（別スレッドでの evaluate + cancel token による
/// 協調的 cancel 観測のため）。Sync は要求しない — 共有は consumer が
/// Mutex 等で行う。vendor 型を出さない。
pub trait ScriptSession: Send {
    fn session_id(&self) -> SessionId;
    fn backend_id(&self) -> &ScriptBackendId;
    /// スクリプトを評価し `ScriptResult` を返す。`entry` は任意の
    /// エントリ関数名（`None` = chunk の戻り値を使う）。
    fn evaluate(&mut self, entry: Option<&str>) -> Result<ScriptResult, ScriptError>;
    /// 冪等・協調的 cancel。blocking host call 中の即時中断は保証しない。
    fn cancel(&mut self);
    /// `cancel` と同じフラグを共有するハンドルを返す。
    /// 別スレッドで evaluate を走らせている間の cancel 発火用。
    fn cancel_token(&self) -> CancelToken;
    /// Session を閉じる（冪等）。
    fn close(&mut self);
}

// ---------------------------------------------------------------------------
// ScriptBackendRegistry（C1）
// ---------------------------------------------------------------------------

/// プロセス内 Backend 登録・実行時解決（契約設計 C1）。
/// 同一プロセス内で複数 Backend を同時登録可能。Feature 非排他。
pub struct ScriptBackendRegistry {
    backends: Vec<Box<dyn ScriptBackend>>,
}

impl Default for ScriptBackendRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptBackendRegistry {
    pub fn new() -> Self {
        Self {
            backends: Vec::new(),
        }
    }

    /// Backend を登録する。同一 `ScriptBackendId` の重複は拒否（F: register）。
    pub fn register(&mut self, backend: Box<dyn ScriptBackend>) -> Result<(), RegistryError> {
        let id = backend.descriptor().id.clone();
        if self.backends.iter().any(|b| b.descriptor().id == id) {
            return Err(RegistryError::DuplicateBackend(id));
        }
        self.backends.push(backend);
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.backends.len()
    }

    pub fn is_empty(&self) -> bool {
        self.backends.is_empty()
    }

    pub fn list(&self) -> impl Iterator<Item = &dyn ScriptBackend> {
        self.backends.iter().map(|b| b.as_ref())
    }

    /// 契約設計 C1 の Dispatch 規則どおりに Fail-Closed で解決する。
    pub fn resolve(&self, req: &ScriptRequirement) -> Result<&dyn ScriptBackend, ResolveError> {
        // Step 1: language/backend 指定でフィルタ。
        let candidates: Vec<&dyn ScriptBackend> = self
            .backends
            .iter()
            .map(|b| b.as_ref())
            .filter(|b| {
                req.language
                    .as_ref()
                    .is_none_or(|l| b.descriptor().language == *l)
                    && req
                        .backend
                        .as_ref()
                        .is_none_or(|id| b.descriptor().id == *id)
            })
            .collect();

        // language/backend が既知かどうかを理由区別のために確認する。
        if let Some(lang) = &req.language
            && !self
                .backends
                .iter()
                .any(|b| b.descriptor().language == *lang)
        {
            return Err(ResolveError::NoMatchingBackend {
                reason: ResolveErrorReason::UnknownLanguage,
                detail: format!("unknown language: {lang}"),
            });
        }
        if let Some(backend_id) = &req.backend
            && !self
                .backends
                .iter()
                .any(|b| b.descriptor().id == *backend_id)
        {
            return Err(ResolveError::NoMatchingBackend {
                reason: ResolveErrorReason::UnknownBackend,
                detail: format!("unknown backend: {backend_id}"),
            });
        }

        // Step 2: contract_version / capabilities / artifact_kinds を検査。
        let mut version_ok = Vec::new();
        let mut cap_ok = Vec::new();
        for b in candidates {
            let d = b.descriptor();
            let version_compatible = d.contract_version == POC_CONTRACT_VERSION
                && req
                    .min_contract_version
                    .is_none_or(|m| d.contract_version >= m);
            if !version_compatible {
                continue;
            }
            version_ok.push(b);
        }
        if version_ok.is_empty() {
            return Err(ResolveError::NoMatchingBackend {
                reason: ResolveErrorReason::UnsupportedContractVersion,
                detail: "no backend with compatible contract_version".to_owned(),
            });
        }

        for b in version_ok {
            let caps = &b.descriptor().capabilities;
            let advertised = caps.capability_ids();
            let missing_caps: Vec<_> = req
                .required_capabilities
                .difference(&advertised)
                .cloned()
                .collect();
            let missing_artifacts: Vec<_> = req
                .artifact_kinds
                .iter()
                .filter(|k| !caps.artifact_kinds.contains(k))
                .cloned()
                .collect();
            // artifact_kinds も capability ID として検査する。
            let mut all_missing = missing_caps;
            for k in missing_artifacts {
                let id = match &k {
                    ArtifactKind::ActionRecipe => CapabilityId::new("artifact.action_recipe"),
                    ArtifactKind::WorkflowDefinition => {
                        CapabilityId::new("artifact.workflow_definition")
                    }
                    ArtifactKind::ConsumerExtension { .. } => {
                        CapabilityId::new("artifact.consumer_extension")
                    }
                };
                all_missing.push(id);
            }
            if all_missing.is_empty() {
                cap_ok.push(b);
            }
        }

        // Step 3-5。
        match cap_ok.len() {
            0 => Err(ResolveError::NoMatchingBackend {
                reason: ResolveErrorReason::MissingCapability,
                detail: "no backend satisfies required capabilities/artifact kinds".to_owned(),
            }),
            1 => Ok(cap_ok[0]),
            _ => Err(ResolveError::AmbiguousBackend {
                candidates: cap_ok.iter().map(|b| b.descriptor().id.clone()).collect(),
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// non-RT ガード（F-15/M-06/FX-N-15）
// ---------------------------------------------------------------------------

thread_local! {
    /// PoC 模擬用: RT コンテキストを TLS で立てる。実 RT 検出ではなく、
    /// 「RT パスからの呼出しは契約違反」を API 入口で検査する仕組みの実演。
    static IN_RT_CONTEXT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// PoC テスト用: 現在の thread を RT コンテキストとして模擬する。
/// `false` で解除。公開面の `evaluate`/`open_session`/`analyze` 等の
/// 入口で `ensure_non_rt` が検査する。
#[doc(hidden)]
pub fn poc_simulate_rt_context(on: bool) {
    IN_RT_CONTEXT.with(|c| c.set(on));
}

/// 全 Script API は non-RT（契約設計 Non-Functional Constraints）。
/// RT パスからの呼出しは `ScriptError::ContractViolation` で拒否する。
pub fn ensure_non_rt() -> Result<(), ScriptError> {
    if IN_RT_CONTEXT.with(std::cell::Cell::get) {
        return Err(ScriptError::contract_violation(
            "script API called from RT context",
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 参照用 re-export（利用側が value.rs を直接触らなくてよいように）
// ---------------------------------------------------------------------------

pub use crate::value::{DefinitionDigest, DiagnosticSeverity, SourceRange, SourceRevision};
#[allow(unused_imports)]
pub use crate::value::{ScriptArtifact as Artifact, ScriptSource as Source};
