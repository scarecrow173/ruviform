//! Backend adapter。`ScriptBackend`/`ScriptSession` trait（contract.rs）を
//! mlua(Luau) と rhai で実装する。vendor 型はこのモジュール配下に閉じ、
//! 公開 trait の境界を越えない。

pub mod luau;
pub mod rhai;

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::canonical;
use crate::contract::{
    BackendDescriptor, BudgetKind, CancelToken, ExecutionBudget, HostApiSurface, HostHandler,
    SessionConfig, SessionError, SessionId, ValueKind,
};
use crate::value::{
    ArtifactKind, ArtifactSource, ProducedBy, ScriptArtifact, ScriptError, ScriptErrorKind,
    ScriptResult, ScriptValue,
};

/// Backend 横断で共有する session 制御状態（cancel/budget/host 計数）。
/// safepoint callback と host fn wrapper の双方から参照する。
pub(crate) struct SessionCtl {
    pub cancel: Arc<AtomicBool>,
    pub start: Instant,
    pub instruction_ops: AtomicU64,
    pub host_calls: AtomicU64,
    /// 制御された失敗（cancel/budget/host error）の最初の1件。
    /// vendor error に写像する前に正確な `ScriptError` を保持する。
    pub pending_error: Mutex<Option<ScriptError>>,
}

impl SessionCtl {
    pub fn new() -> Self {
        Self {
            cancel: Arc::new(AtomicBool::new(false)),
            start: Instant::now(),
            instruction_ops: AtomicU64::new(0),
            host_calls: AtomicU64::new(0),
            pending_error: Mutex::new(None),
        }
    }

    /// `ScriptSession::cancel` と cancel token が共有するフラグ。
    pub fn cancel_token(&self) -> CancelToken {
        CancelToken::from_flag(self.cancel.clone())
    }

    /// 制御された失敗を記録する（最初の1件のみ保持）。
    pub fn fail(&self, err: ScriptError) {
        let mut g = self.pending_error.lock().expect("pending_error poisoned");
        if g.is_none() {
            *g = Some(err);
        }
    }

    /// 記録済みの制御エラーを取り出す。
    pub fn take_error(&self) -> Option<ScriptError> {
        self.pending_error
            .lock()
            .expect("pending_error poisoned")
            .take()
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }
}

/// `open_session` 共通の fail-closed 検査（契約設計 C2 `SessionError`）。
pub(crate) fn validate_session_config(
    config: &SessionConfig,
    descriptor: &BackendDescriptor,
) -> Result<Arc<SessionCtl>, SessionError> {
    // source.language が backend language と一致するか。
    if config.source.language != descriptor.language {
        return Err(SessionError::LanguageMismatch {
            expected: descriptor.language.clone(),
            actual: config.source.language.clone(),
        });
    }
    // 要求 Budget が capability で強制可能か（F-06: 未対応は明示拒否、
    // silent ignore しない）。
    let enforced = &descriptor.capabilities.enforced_budgets;
    for kind in config.budget.requested_kinds() {
        if !enforced.contains(&kind) {
            return Err(SessionError::UnsupportedBudget { kind });
        }
    }
    // Host 面の中立性と handler 対応。
    config
        .host_bindings
        .validate_neutral()
        .map_err(|reason| SessionError::InvalidHostSurface { reason })?;
    for binding in &config.host_bindings.bindings {
        if !config.host_handlers.contains_key(&binding.name) {
            return Err(SessionError::InvalidHostSurface {
                reason: format!("declared binding {:?} has no handler", binding.name),
            });
        }
    }
    for name in config.host_handlers.keys() {
        if config.host_bindings.binding(name).is_none() {
            return Err(SessionError::InvalidHostSurface {
                reason: format!("handler {name:?} is not declared in host_bindings"),
            });
        }
    }
    // source サイズ上限（M-10）。
    if let Some(max) = descriptor.capabilities.max_source_bytes {
        let bytes = config.source.text.len() as u64;
        if bytes > max {
            return Err(SessionError::SourceTooLarge { bytes, max });
        }
    }
    // resume 時の digest 一致（F-08/M-12 stale source）。
    if let Some(expected) = &config.resume
        && *expected != config.source.digest
    {
        return Err(SessionError::SourceDigestMismatch {
            expected: expected.clone(),
            actual: config.source.digest.clone(),
        });
    }
    // session が生成してよい artifact 種別が capability 内か。
    for kind in &config.artifact_kinds {
        if !descriptor.capabilities.artifact_kinds.contains(kind) {
            return Err(SessionError::Backend {
                detail: format!("artifact kind {kind:?} not supported"),
            });
        }
    }
    Ok(Arc::new(SessionCtl::new()))
}

/// host binding の引数/戻り値を `HostSignature` で検査し、handler を呼ぶ。
/// `max_host_calls` 超過はここで `BudgetExceeded` として記録する。
pub(crate) fn call_host_handler(
    ctl: &Arc<SessionCtl>,
    bindings: &HostApiSurface,
    handlers: &BTreeMap<String, HostHandler>,
    max_host_calls: Option<u64>,
    name: &str,
    args: Vec<ScriptValue>,
) -> Result<ScriptValue, String> {
    let binding = bindings
        .binding(name)
        .ok_or_else(|| format!("undeclared host binding {name:?}"))?;
    // host call 計数 + 上限強制（max_host_calls の決定的な強制点）。
    let n = ctl.host_calls.fetch_add(1, Ordering::Relaxed) + 1;
    if let Some(max) = max_host_calls
        && n > max
    {
        ctl.fail(ScriptError::new(
            ScriptErrorKind::BudgetExceeded(BudgetKind::MaxHostCalls),
            format!("host-call budget exceeded: {n} > {max}"),
        ));
        return Err("host-call budget exceeded".to_owned());
    }
    // 型検査: arity と ValueKind。
    if args.len() != binding.signature.args.len() {
        return Err(format!(
            "host {name:?} arity mismatch: got {}, want {}",
            args.len(),
            binding.signature.args.len()
        ));
    }
    for (i, (arg, want)) in args.iter().zip(binding.signature.args.iter()).enumerate() {
        if value_kind(arg) != *want {
            return Err(format!(
                "host {name:?} arg {i} kind mismatch: got {:?}, want {want:?}",
                value_kind(arg)
            ));
        }
    }
    let handler = handlers
        .get(name)
        .ok_or_else(|| format!("no handler for {name:?}"))?;
    let result = handler(&args)?;
    if value_kind(&result) != binding.signature.returns {
        return Err(format!(
            "host {name:?} return kind mismatch: got {:?}, want {:?}",
            value_kind(&result),
            binding.signature.returns
        ));
    }
    Ok(result)
}

/// `ScriptValue` の `ValueKind`（HostSignature 検査用）。
pub(crate) fn value_kind(v: &ScriptValue) -> ValueKind {
    match v {
        ScriptValue::Absent => ValueKind::Absent,
        ScriptValue::Null => ValueKind::Null,
        ScriptValue::Bool(_) => ValueKind::Bool,
        ScriptValue::Integer(_) => ValueKind::Integer,
        ScriptValue::Number(_) => ValueKind::Number,
        ScriptValue::String(_) => ValueKind::String,
        ScriptValue::Bytes(_) => ValueKind::Bytes,
        ScriptValue::Array(_) => ValueKind::Array,
        ScriptValue::Record(_) => ValueKind::Record,
        ScriptValue::Time { .. } => ValueKind::Time,
        ScriptValue::ResourceId { .. } => ValueKind::ResourceId,
    }
}

/// safepoint/callback で共通の budget+cancel 検査。
/// 超過時は `ctl.fail` に正確な ScriptError を記録して `true` を返す。
/// 戻り値 true = 呼出側は vendor エラーを返して eval を中断せよ。
pub(crate) fn check_limits(ctl: &Arc<SessionCtl>, budget: &ExecutionBudget, op_count: u64) -> bool {
    if ctl.is_cancelled() {
        ctl.fail(ScriptError::new(
            ScriptErrorKind::Cancelled,
            "script cancelled",
        ));
        return true;
    }
    if let Some(max) = budget.max_instructions
        && op_count > max
    {
        ctl.fail(ScriptError::new(
            ScriptErrorKind::BudgetExceeded(BudgetKind::MaxInstructions),
            format!("instruction budget exceeded: {op_count} > {max}"),
        ));
        return true;
    }
    if let Some(max) = budget.max_wall_time
        && ctl.start.elapsed() > max
    {
        ctl.fail(ScriptError::new(
            ScriptErrorKind::BudgetExceeded(BudgetKind::MaxWallTime),
            "wall-time budget exceeded",
        ));
        return true;
    }
    false
}

/// Script 戻り値を `ScriptResult` に組み立てる。
/// 戻り値が `kind` を持つ Record の場合は `artifact` を構築し、
/// それ以外の値（値モデル往復 Fixture 用）は `return_value` のみを返す。
/// `max_output_bytes` は canonical byte 数で判定する。
pub(crate) fn finish_result(
    ret: ScriptValue,
    config: &SessionConfig,
    descriptor: &BackendDescriptor,
    budget: &ExecutionBudget,
) -> Result<ScriptResult, ScriptError> {
    // max_output_bytes: canonical byte 数で判定（payload/return 共通）。
    if let Some(max) = budget.max_output_bytes {
        let size = canonical::canonical_value(&ret)
            .map(|s| s.len() as u64)
            .unwrap_or(u64::MAX);
        if size > max {
            return Err(ScriptError::new(
                ScriptErrorKind::BudgetExceeded(BudgetKind::MaxOutputBytes),
                format!("output budget exceeded: {size} > {max}"),
            ));
        }
    }
    let artifact = build_artifact(&ret, config, descriptor)?;
    Ok(ScriptResult {
        artifact,
        return_value: Some(ret),
        diagnostics: Vec::new(),
    })
}

/// 戻り値 Record が artifact 形状（`kind` 文字列を持つ）なら
/// `ScriptArtifact` を構築する。それ以外は `None`。
fn build_artifact(
    ret: &ScriptValue,
    config: &SessionConfig,
    descriptor: &BackendDescriptor,
) -> Result<Option<ScriptArtifact>, ScriptError> {
    let rec = match ret.as_record() {
        Some(r) => r,
        None => return Ok(None),
    };
    let kind_str = match rec.get("kind") {
        Some(ScriptValue::String(s)) => s.as_str(),
        _ => return Ok(None),
    };
    let kind = ArtifactKind::from_script_str(kind_str).ok_or_else(|| {
        ScriptError::new(
            ScriptErrorKind::Unsupported(format!("artifact kind {kind_str:?}")),
            "unknown artifact kind",
        )
    })?;
    if !config.artifact_kinds.is_empty() && !config.artifact_kinds.contains(&kind) {
        return Err(ScriptError::new(
            ScriptErrorKind::Unsupported(format!("artifact kind {kind:?}")),
            "artifact kind not permitted by session config",
        ));
    }
    let digest = canonical::definition_digest(ret)
        .map_err(|e| ScriptError::new(ScriptErrorKind::Internal, e))?;
    Ok(Some(ScriptArtifact {
        schema_version: 1,
        kind,
        digest,
        payload: ret.clone(),
        source: ArtifactSource {
            language: config.source.language.clone(),
            backend_id: descriptor.id.clone(),
            source_digest: config.source.digest.clone(),
            source_revision: config.source.revision,
        },
        produced_by: ProducedBy {
            backend_id: descriptor.id.clone(),
            impl_version: descriptor.impl_version.clone(),
        },
    }))
}

/// session id 発番。
pub(crate) fn next_session_id() -> SessionId {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    SessionId(NEXT.fetch_add(1, Ordering::Relaxed))
}

/// `open_session` 共通の前段検証 + 制御状態の構築。
pub(crate) fn open_checked(
    config: &SessionConfig,
    descriptor: &BackendDescriptor,
) -> Result<(Arc<SessionCtl>, SessionId), SessionError> {
    validate_session_config(config, descriptor)?;
    Ok((Arc::new(SessionCtl::new()), next_session_id()))
}
