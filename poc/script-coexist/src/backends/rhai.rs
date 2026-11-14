//! Rhai backend（pure-Rust `rhai` crate、`sync` feature で Engine を Send 化）。
//!
//! - `Engine::new_raw()`（std package 非搭載の最小面）で session を構築。
//! - instruction/wall-time/cancel は `on_progress` の ops 計数 +
//!   `check_limits` で強制。`set_max_operations` のハード上限は
//!   `max_instructions` を `on_progress` 側で行うため未使用。
//! - memory budget は Rhai が公開 API を持たないため `enforced_budgets`
//!   から外し、要求時は `SessionError::UnsupportedBudget` で fail-closed。
//! - i64 は Rhai INT (i64) でネイティブに厳密往復（`native_integer: true`）。
//!   f64 精度を超える値も INT のまま運べるが、codec 統一のため
//!   materialize/dematerialize の tag 経路は両 backend で共通。

use std::collections::BTreeSet;
use std::sync::Arc;

use rhai::{Dynamic, Engine, EvalAltResult, Position, Scope};

use crate::backends::{SessionCtl, call_host_handler, check_limits, finish_result, open_checked};
use crate::codec::{dematerialize, materialize};
use crate::contract::{
    BackendCapabilities, BackendDescriptor, BudgetKind, CancelGranularity, CancelToken,
    ScriptBackend, ScriptBackendId, ScriptLanguageId, ScriptSession, SessionConfig, SessionError,
    SessionId, ValueKind,
};
use crate::value::{ArtifactKind, ScriptError, ScriptErrorKind, ScriptResult, ScriptValue};

const BACKEND_ID: &str = "rhai-embedded";
const LANGUAGE: &str = "rhai";

/// Rhai backend。session 毎に独立した `Engine` を持つ。
pub struct RhaiBackend {
    descriptor: BackendDescriptor,
}

impl RhaiBackend {
    pub fn new() -> Self {
        let value_kinds: BTreeSet<ValueKind> = [
            ValueKind::Absent,
            ValueKind::Null,
            ValueKind::Bool,
            ValueKind::Integer,
            ValueKind::Number,
            ValueKind::String,
            ValueKind::Bytes,
            ValueKind::Array,
            ValueKind::Record,
            ValueKind::Time,
            ValueKind::ResourceId,
        ]
        .into_iter()
        .collect();
        // MaxMemoryBytes は Rhai 公開 API に存在しない → 広告しない。
        let enforced_budgets: BTreeSet<BudgetKind> = [
            BudgetKind::MaxInstructions,
            BudgetKind::MaxWallTime,
            BudgetKind::MaxOutputBytes,
            BudgetKind::MaxHostCalls,
        ]
        .into_iter()
        .collect();
        let artifact_kinds: BTreeSet<ArtifactKind> =
            [ArtifactKind::ActionRecipe, ArtifactKind::WorkflowDefinition]
                .into_iter()
                .collect();
        Self {
            descriptor: BackendDescriptor {
                id: ScriptBackendId::new(BACKEND_ID),
                language: ScriptLanguageId::new(LANGUAGE),
                impl_version: env!("CARGO_PKG_VERSION").to_owned(),
                contract_version: crate::contract::POC_CONTRACT_VERSION,
                capabilities: BackendCapabilities {
                    value_kinds,
                    enforced_budgets,
                    owner_thread_only: false,
                    send: true,
                    sync: false,
                    max_source_bytes: Some(1 << 20),
                    artifact_kinds,
                    cancel_granularity: CancelGranularity::Safepoint,
                    shared_vm_state: false,
                    // Rhai INT は i64。
                    native_integer: true,
                },
            },
        }
    }
}

impl Default for RhaiBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptBackend for RhaiBackend {
    fn descriptor(&self) -> &BackendDescriptor {
        &self.descriptor
    }

    fn open_session(&self, config: &SessionConfig) -> Result<Box<dyn ScriptSession>, SessionError> {
        let (ctl, session_id) = open_checked(config, &self.descriptor)?;
        let session = RhaiSession::open(config, ctl, session_id, &self.descriptor)?;
        Ok(Box::new(session))
    }
}

/// `Dynamic` → `ScriptValue`（unit→Absent、tag dematerialize 込み）。
fn from_dynamic(v: Dynamic) -> Result<ScriptValue, String> {
    let sv = if v.is_unit() {
        ScriptValue::Absent
    } else if v.is_bool() {
        ScriptValue::Bool(v.as_bool().map_err(|e| e.to_string())?)
    } else if v.is_int() {
        ScriptValue::Integer(v.as_int().map_err(|e| e.to_string())?)
    } else if v.is_float() {
        ScriptValue::Number(v.as_float().map_err(|e| e.to_string())?)
    } else if v.is_string() {
        ScriptValue::String(v.to_string())
    } else if v.is_array() {
        let arr = v.cast::<rhai::Array>();
        let mut items = Vec::with_capacity(arr.len());
        for item in arr {
            items.push(from_dynamic(item)?);
        }
        ScriptValue::Array(items)
    } else if v.is_map() {
        let map = v.cast::<rhai::Map>();
        let mut rec = std::collections::BTreeMap::new();
        for (k, val) in map {
            rec.insert(k.to_string(), from_dynamic(val)?);
        }
        ScriptValue::Record(rec)
    } else {
        return Err(format!("unsupported rhai dynamic type: {}", v.type_name()));
    };
    dematerialize(sv)
}

/// `ScriptValue` → `Dynamic`（materialize + Map/Array 構築）。
fn to_dynamic(v: &ScriptValue) -> Result<Dynamic, String> {
    let v = materialize(v);
    let d = match v {
        ScriptValue::Absent => Dynamic::UNIT,
        ScriptValue::Null => {
            // materialize で tag record 化済みのはず。万一残っていれば
            // UNIT ではなく tag を作る（念のため materialize 済み前提）。
            return Err("unreachable: Null should be materialized".to_owned());
        }
        ScriptValue::Bool(b) => Dynamic::from(b),
        ScriptValue::Integer(i) => Dynamic::from(i),
        ScriptValue::Number(f) => Dynamic::from(f),
        ScriptValue::String(s) => Dynamic::from(s),
        ScriptValue::Array(items) => {
            let mut arr = rhai::Array::with_capacity(items.len());
            for item in items {
                arr.push(to_dynamic(&item)?);
            }
            Dynamic::from_array(arr)
        }
        ScriptValue::Record(rec) => {
            let mut map = rhai::Map::new();
            for (k, val) in rec {
                if matches!(val, ScriptValue::Absent) {
                    continue;
                }
                map.insert(k.as_str().into(), to_dynamic(&val)?);
            }
            Dynamic::from_map(map)
        }
        // materialize 済みのため到達しないはず。
        other => return Err(format!("unreachable materialized value: {other:?}")),
    };
    Ok(d)
}

/// EvalAltResult → ScriptErrorKind の分類。
fn classify(e: &EvalAltResult) -> ScriptErrorKind {
    use EvalAltResult as E;
    match e {
        E::ErrorSystem(..) => ScriptErrorKind::Backend,
        E::ErrorTooManyOperations(..) => {
            ScriptErrorKind::BudgetExceeded(BudgetKind::MaxInstructions)
        }
        E::ErrorParsing(..) => ScriptErrorKind::Parse,
        _ => ScriptErrorKind::Runtime,
    }
}

fn runtime_err(msg: impl Into<String>) -> Box<EvalAltResult> {
    EvalAltResult::ErrorRuntime(Dynamic::from(msg.into()), Position::NONE).into()
}

/// host fn 登録用の共有コンテキスト。
struct HostCtx {
    ctl: Arc<SessionCtl>,
    bindings: crate::contract::HostApiSurface,
    handlers: std::collections::BTreeMap<String, crate::contract::HostHandler>,
    max_host_calls: Option<u64>,
    name: String,
}

impl HostCtx {
    /// Dynamic args → ScriptValue → call_host_handler → Dynamic。
    fn invoke(&self, args: Vec<Dynamic>) -> Result<Dynamic, Box<EvalAltResult>> {
        let mut sv_args = Vec::with_capacity(args.len());
        for v in args {
            match from_dynamic(v) {
                Ok(sv) => sv_args.push(sv),
                Err(m) => {
                    self.ctl.fail(ScriptError::new(
                        ScriptErrorKind::Host(m.clone()),
                        m.clone(),
                    ));
                    return Err(runtime_err(m));
                }
            }
        }
        match call_host_handler(
            &self.ctl,
            &self.bindings,
            &self.handlers,
            self.max_host_calls,
            &self.name,
            sv_args,
        ) {
            Ok(sv) => to_dynamic(&sv).map_err(runtime_err),
            Err(m) => {
                self.ctl.fail(ScriptError::new(
                    ScriptErrorKind::Host(m.clone()),
                    m.clone(),
                ));
                Err(runtime_err(m))
            }
        }
    }
}

/// arity 別に `register_fn` するマクロ（0..=4 引数）。
macro_rules! register_host_fn {
    ($engine:expr, $ctx:expr, $name:expr $(, $p:ident)*) => {{
        let ctx = $ctx.clone();
        let name = $name.to_owned();
        $engine.register_fn(name.clone(), move |$($p: Dynamic),*| {
            ctx.invoke(vec![$($p),*])
        })
    }};
}

/// 1 session 分の Rhai Engine + 制御状態。
pub struct RhaiSession {
    engine: Engine,
    ctl: Arc<SessionCtl>,
    session_id: SessionId,
    backend_id: ScriptBackendId,
    config: SessionConfig,
    descriptor: BackendDescriptor,
    closed: bool,
}

impl RhaiSession {
    fn open(
        config: &SessionConfig,
        ctl: Arc<SessionCtl>,
        session_id: SessionId,
        descriptor: &BackendDescriptor,
    ) -> Result<Self, SessionError> {
        // std package 非搭載の最小 Engine。
        let mut engine = Engine::new_raw();

        // instruction/wall-time/cancel: on_progress（ops 計数 + check_limits）。
        {
            let ctl = Arc::clone(&ctl);
            let budget = config.budget.clone();
            engine.on_progress(move |ops: u64| {
                if check_limits(&ctl, &budget, ops) {
                    Some(Dynamic::from("poc: limit exceeded"))
                } else {
                    None
                }
            });
        }

        // INPUT を Scope 経由で注入するための値を用意。
        // （eval_with_scope で push する）

        // host bindings を register_fn で登録（arity 0..=4）。
        let bindings = config.host_bindings.clone();
        let handlers = config.host_handlers.clone();
        for binding in &config.host_bindings.bindings {
            let ctx = Arc::new(HostCtx {
                ctl: Arc::clone(&ctl),
                bindings: bindings.clone(),
                handlers: handlers.clone(),
                max_host_calls: config.budget.max_host_calls,
                name: binding.name.clone(),
            });
            let arity = binding.signature.args.len();
            // register_fn は &mut Engine を返す（登録トークン）。副作用のみ必要。
            match arity {
                0 => register_host_fn!(engine, ctx, binding.name),
                1 => register_host_fn!(engine, ctx, binding.name, a0),
                2 => register_host_fn!(engine, ctx, binding.name, a0, a1),
                3 => register_host_fn!(engine, ctx, binding.name, a0, a1, a2),
                4 => register_host_fn!(engine, ctx, binding.name, a0, a1, a2, a3),
                _ => {
                    return Err(SessionError::InvalidHostSurface {
                        reason: format!(
                            "host binding {:?} arity {arity} unsupported (max 4)",
                            binding.name
                        ),
                    });
                }
            };
        }

        Ok(Self {
            engine,
            ctl,
            session_id,
            backend_id: descriptor.id.clone(),
            config: config.clone(),
            descriptor: descriptor.clone(),
            closed: false,
        })
    }

    /// vendor error → ScriptError（制御エラー優先、なければ分類）。
    fn map_error(&self, e: Box<EvalAltResult>) -> ScriptError {
        if let Some(controlled) = self.ctl.take_error() {
            return controlled
                .with_backend(&self.backend_id)
                .with_session(self.session_id);
        }
        let kind = classify(&e);
        ScriptError::new(kind, e.to_string())
            .with_backend(&self.backend_id)
            .with_session(self.session_id)
    }
}

impl ScriptSession for RhaiSession {
    fn session_id(&self) -> SessionId {
        self.session_id
    }

    fn backend_id(&self) -> &ScriptBackendId {
        &self.backend_id
    }

    fn evaluate(&mut self, entry: Option<&str>) -> Result<ScriptResult, ScriptError> {
        crate::contract::ensure_non_rt()?;
        if self.closed {
            return Err(ScriptError::new(
                ScriptErrorKind::Runtime,
                "session is closed",
            ));
        }
        // INPUT を Scope に積む。
        let mut scope = Scope::new();
        if !self.config.inputs.is_empty() {
            let input_rec = ScriptValue::Record(self.config.inputs.clone());
            match to_dynamic(&input_rec) {
                Ok(d) => {
                    scope.push_dynamic("INPUT", d);
                }
                Err(m) => {
                    return Err(ScriptError::new(ScriptErrorKind::Internal, m)
                        .with_backend(&self.backend_id)
                        .with_session(self.session_id));
                }
            }
        }

        let outcome: Result<Dynamic, Box<EvalAltResult>> = match entry {
            None => self
                .engine
                .eval_with_scope::<Dynamic>(&mut scope, &self.config.source.text),
            Some(name) => {
                // 関数定義は AST 単位。compile → call_fn で entry を呼ぶ。
                // （この経路では Scope/INPUT は関数本体から見えない —
                // PoC の制限として記録。Fixture は entry=None を使う。）
                match self.engine.compile(&self.config.source.text) {
                    Ok(ast) => self.engine.call_fn::<Dynamic>(&mut scope, &ast, name, ()),
                    Err(e) => {
                        return Err(ScriptError::new(
                            ScriptErrorKind::Parse,
                            format!("{e} at {:?}", e.position()),
                        )
                        .with_backend(&self.backend_id)
                        .with_session(self.session_id));
                    }
                }
            }
        };

        match outcome {
            Ok(d) => {
                let sv = from_dynamic(d).map_err(|m| {
                    // i64 範囲外の tag int は ValueOutOfRange（F-12）。
                    let kind = if m.contains("out of i64 range") {
                        ScriptErrorKind::ValueOutOfRange
                    } else {
                        ScriptErrorKind::Type
                    };
                    ScriptError::new(kind, format!("value decode: {m}"))
                        .with_backend(&self.backend_id)
                        .with_session(self.session_id)
                })?;
                finish_result(sv, &self.config, &self.descriptor, &self.config.budget).map_err(
                    |e| {
                        e.with_backend(&self.backend_id)
                            .with_session(self.session_id)
                    },
                )
            }
            Err(e) => Err(self.map_error(e)),
        }
    }

    fn cancel(&mut self) {
        self.ctl
            .cancel
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }

    fn cancel_token(&self) -> CancelToken {
        self.ctl.cancel_token()
    }

    fn close(&mut self) {
        self.closed = true;
    }
}
