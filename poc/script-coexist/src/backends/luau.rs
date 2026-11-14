//! Luau backend（mlua `luau` feature、vendored Luau VM）。
//!
//! - `Lua::new()` の safe state（std lib 非搭載）で session を構築。
//! - instruction budget は `set_interrupt` の safepoint tick 数で近似
//!   （Luau は正確な命令カウント API を公開しない — 粒度差は evidence に記録）。
//! - memory budget は `set_memory_limit`、wall-time/cancel は interrupt
//!   callback 内で `check_limits`、host-call budget は host wrapper で強制。
//! - i64 は f64 精度を超える値を codec の `{"$rv":"int"}` tag で厳密に運ぶ
//!   （Luau number は binary64 — `native_integer: false` を正直に広告）。

use std::collections::BTreeSet;
use std::sync::Arc;

use mlua::{MultiValue, Value as LuaValue};

use crate::backends::{SessionCtl, call_host_handler, check_limits, finish_result, open_checked};
use crate::codec::{dematerialize, materialize};
use crate::contract::{
    BackendCapabilities, BackendDescriptor, BudgetKind, CancelGranularity, CancelToken,
    ScriptBackend, ScriptBackendId, ScriptLanguageId, ScriptSession, SessionConfig, SessionError,
    SessionId, ValueKind,
};
use crate::value::{ArtifactKind, ScriptError, ScriptErrorKind, ScriptResult, ScriptValue};

const BACKEND_ID: &str = "luau-mlua";
const LANGUAGE: &str = "luau";

/// Luau backend。session 毎に独立した `mlua::Lua` を持つ。
pub struct LuauBackend {
    descriptor: BackendDescriptor,
}

impl LuauBackend {
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
        let enforced_budgets: BTreeSet<BudgetKind> = [
            BudgetKind::MaxInstructions,
            BudgetKind::MaxWallTime,
            BudgetKind::MaxMemoryBytes,
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
                    // Luau number は binary64。i64 厳密性は codec tag 経由。
                    native_integer: false,
                },
            },
        }
    }
}

impl Default for LuauBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptBackend for LuauBackend {
    fn descriptor(&self) -> &BackendDescriptor {
        &self.descriptor
    }

    fn open_session(&self, config: &SessionConfig) -> Result<Box<dyn ScriptSession>, SessionError> {
        let (ctl, session_id) = open_checked(config, &self.descriptor)?;
        let session = LuauSession::open(config, ctl, session_id, &self.descriptor)?;
        Ok(Box::new(session))
    }
}

/// `mlua::Value` → `ScriptValue`（tag dematerialize 込み）。
fn from_lua(v: LuaValue) -> Result<ScriptValue, String> {
    let sv = match v {
        LuaValue::Nil => ScriptValue::Null,
        LuaValue::Boolean(b) => ScriptValue::Bool(b),
        LuaValue::Integer(i) => ScriptValue::Integer(i),
        LuaValue::Number(f) => ScriptValue::Number(f),
        LuaValue::String(s) => {
            ScriptValue::String(s.to_str().map_err(|e| e.to_string())?.to_owned())
        }
        LuaValue::Table(t) => table_to_script(&t)?,
        other => return Err(format!("unsupported lua value type: {}", other.type_name())),
    };
    dematerialize(sv)
}

/// Lua table → ScriptValue。整数 key 1..=n のみなら Array、それ以外は
/// String key のみ許容する Record。混在・非文字列 key は fail-closed。
fn table_to_script(t: &mlua::Table) -> Result<ScriptValue, String> {
    let mut pairs = Vec::new();
    for pair in t.clone().pairs::<LuaValue, LuaValue>() {
        let (k, v) = pair.map_err(|e| e.to_string())?;
        pairs.push((k, v));
    }
    // array 判定: key が 1..=n の整数のみ。
    let mut indices: Vec<i64> = Vec::with_capacity(pairs.len());
    let mut all_pos_int = !pairs.is_empty();
    for (k, _) in &pairs {
        match k {
            LuaValue::Integer(i) if *i >= 1 => indices.push(*i),
            _ => all_pos_int = false,
        }
    }
    indices.sort_unstable();
    indices.dedup();
    let is_array = all_pos_int
        && !indices.is_empty()
        && indices == (1..=indices.len() as i64).collect::<Vec<_>>();
    if is_array {
        let mut items: Vec<(i64, LuaValue)> = Vec::with_capacity(pairs.len());
        for (k, v) in pairs {
            match k {
                LuaValue::Integer(i) => items.push((i, v)),
                _ => return Err("non-integer array key".to_owned()),
            }
        }
        items.sort_by_key(|(i, _)| *i);
        let mut out = Vec::with_capacity(items.len());
        for (_, v) in items {
            out.push(from_lua(v)?);
        }
        return Ok(ScriptValue::Array(out));
    }
    // Record: 全 key が String（数値 key は 10 進文字列として許容）。
    let mut rec = std::collections::BTreeMap::new();
    for (k, v) in pairs {
        let key = match k {
            LuaValue::String(s) => s.to_str().map_err(|e| e.to_string())?.to_owned(),
            LuaValue::Integer(i) => i.to_string(),
            other => return Err(format!("unsupported table key type: {}", other.type_name())),
        };
        rec.insert(key, from_lua(v)?);
    }
    Ok(ScriptValue::Record(rec))
}

/// `ScriptValue` → `mlua::Value`（materialize + Table 構築）。
fn to_lua_state(lua: &mlua::Lua, v: &ScriptValue) -> mlua::Result<LuaValue> {
    let v = materialize(v);
    match v {
        ScriptValue::Null | ScriptValue::Absent => Ok(LuaValue::Nil),
        ScriptValue::Bool(b) => Ok(LuaValue::Boolean(b)),
        ScriptValue::Integer(i) => Ok(LuaValue::Integer(i)),
        ScriptValue::Number(f) => Ok(LuaValue::Number(f)),
        ScriptValue::String(s) => Ok(LuaValue::String(lua.create_string(&s)?)),
        ScriptValue::Array(items) => {
            let t = lua.create_table()?;
            for (i, item) in items.iter().enumerate() {
                t.set(i + 1, to_lua_state(lua, item)?)?;
            }
            Ok(LuaValue::Table(t))
        }
        ScriptValue::Record(rec) => {
            let t = lua.create_table()?;
            for (k, val) in rec {
                // Absent field は canonical 同様 key ごと省略。
                if matches!(val, ScriptValue::Absent) {
                    continue;
                }
                t.set(k.as_str(), to_lua_state(lua, &val)?)?;
            }
            Ok(LuaValue::Table(t))
        }
        // materialize 済みのため到達しないはずだが、万一に備え error。
        other => Err(mlua::Error::runtime(format!(
            "unreachable materialized value: {other:?}"
        ))),
    }
}

/// mlua::Error → ScriptErrorKind の分類（vendor 型は公開面に出さない）。
fn classify(err: &mlua::Error) -> ScriptErrorKind {
    use mlua::Error as E;
    match err {
        E::SyntaxError { .. } => ScriptErrorKind::Parse,
        E::MemoryError(_) => ScriptErrorKind::BudgetExceeded(BudgetKind::MaxMemoryBytes),
        E::CallbackError { cause, .. } | E::WithContext { cause, .. } => classify(cause),
        E::RuntimeError(_) => ScriptErrorKind::Runtime,
        _ => ScriptErrorKind::Backend,
    }
}

/// 1 session 分の Luau VM + 制御状態。
pub struct LuauSession {
    lua: mlua::Lua,
    ctl: Arc<SessionCtl>,
    session_id: SessionId,
    backend_id: ScriptBackendId,
    config: SessionConfig,
    descriptor: BackendDescriptor,
    closed: bool,
}

impl LuauSession {
    fn open(
        config: &SessionConfig,
        ctl: Arc<SessionCtl>,
        session_id: SessionId,
        descriptor: &BackendDescriptor,
    ) -> Result<Self, SessionError> {
        // safe state（std lib 非搭載）。`Lua::new` は infallible。
        let lua = mlua::Lua::new();

        // memory budget: set_memory_limit（lua_Alloc 経由の強制）。
        if let Some(max) = config.budget.max_memory_bytes {
            lua.set_memory_limit(max as usize)
                .map_err(|e| SessionError::Backend {
                    detail: e.to_string(),
                })?;
        }

        // instruction/wall-time/cancel: interrupt callback（safepoint）。
        // set_interrupt は infallible で callback を登録する。
        {
            let ctl = Arc::clone(&ctl);
            let budget = config.budget.clone();
            lua.set_interrupt(move |_lua| {
                let ops = ctl
                    .instruction_ops
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                    + 1;
                if check_limits(&ctl, &budget, ops) {
                    // pending_error に正確な ScriptError を記録済み。
                    Err(mlua::Error::runtime("poc: limit exceeded"))
                } else {
                    Ok(mlua::VmState::Continue)
                }
            });
        }

        // INPUT 注入（値モデル往復 Fixture の入力）。
        if !config.inputs.is_empty() {
            let input_rec = ScriptValue::Record(config.inputs.clone());
            let lv = to_lua_state(&lua, &input_rec).map_err(|e| SessionError::Backend {
                detail: e.to_string(),
            })?;
            lua.globals()
                .set("INPUT", lv)
                .map_err(|e| SessionError::Backend {
                    detail: e.to_string(),
                })?;
        }

        // host bindings を Lua global function として登録。
        let bindings = Arc::new(config.host_bindings.clone());
        let handlers = Arc::new(config.host_handlers.clone());
        for binding in &config.host_bindings.bindings {
            let name = binding.name.clone();
            let ctl = Arc::clone(&ctl);
            let bindings = Arc::clone(&bindings);
            let handlers = Arc::clone(&handlers);
            let max_host_calls = config.budget.max_host_calls;
            let f = lua
                .create_function(move |lua, args: MultiValue| {
                    // vendor args → ScriptValue（tag dematerialize 込み）。
                    let mut sv_args = Vec::with_capacity(args.len());
                    for v in args.iter() {
                        match from_lua(v.clone()) {
                            Ok(sv) => sv_args.push(sv),
                            Err(m) => {
                                ctl.fail(ScriptError::new(
                                    ScriptErrorKind::Host(m.clone()),
                                    m.clone(),
                                ));
                                return Err(mlua::Error::runtime(m));
                            }
                        }
                    }
                    match call_host_handler(
                        &ctl,
                        &bindings,
                        &handlers,
                        max_host_calls,
                        &name,
                        sv_args,
                    ) {
                        Ok(sv) => to_lua_state(lua, &sv),
                        Err(m) => {
                            ctl.fail(ScriptError::new(
                                ScriptErrorKind::Host(m.clone()),
                                m.clone(),
                            ));
                            Err(mlua::Error::runtime(m))
                        }
                    }
                })
                .map_err(|e| SessionError::Backend {
                    detail: e.to_string(),
                })?;
            lua.globals()
                .set(binding.name.as_str(), f)
                .map_err(|e| SessionError::Backend {
                    detail: e.to_string(),
                })?;
        }

        Ok(Self {
            lua,
            ctl,
            session_id,
            backend_id: descriptor.id.clone(),
            config: config.clone(),
            descriptor: descriptor.clone(),
            closed: false,
        })
    }

    /// vendor error → ScriptError（制御エラー優先、なければ分類）。
    fn map_error(&self, e: mlua::Error) -> ScriptError {
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

impl ScriptSession for LuauSession {
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
        let outcome: mlua::Result<LuaValue> = (|| {
            match entry {
                None => {
                    // chunk 評価: 戻り値（最初の値）を使う。
                    let chunk = self
                        .lua
                        .load(self.config.source.text.as_str())
                        .set_name("<poc-session>");
                    let mv = chunk.eval::<MultiValue>()?;
                    Ok(mv.into_iter().next().unwrap_or(LuaValue::Nil))
                }
                Some(name) => {
                    // chunk を実行して global を定義し、entry 関数を呼ぶ。
                    let chunk = self
                        .lua
                        .load(self.config.source.text.as_str())
                        .set_name("<poc-session>");
                    chunk.exec()?;
                    let f = self.lua.globals().get::<mlua::Function>(name)?;
                    let mv = f.call::<MultiValue>(())?;
                    Ok(mv.into_iter().next().unwrap_or(LuaValue::Nil))
                }
            }
        })();
        match outcome {
            Ok(v) => {
                let sv = from_lua(v).map_err(|m| {
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
