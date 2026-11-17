//! Runtime `ScriptBackend`/`ScriptSession` — vendored Luau VM への
//! eval/budget/artifact 実行経路（Issue #716 / RF-SCRIPT-03 follow-up）。
//!
//! `mlua` 0.12.2（`luau` feature → vendored Luau 0.740）の safe Rust
//! API のみで構成する。依存選定の記録は
//! `docs/adr/issue-716-runtime-backend.md`（#692 AC4 再評価 —
//! analyzer = native bridge / runtime = mlua Rust API の hybrid 確定）。
//!
//! 契約メモ（中立契約 `crates/ruviform-script` が正本）:
//!
//! - session 毎に fresh `Lua` — VM 可変状態を session 間で共有しない
//!   （`BackendCapabilities::shared_vm_state = false`）。
//! - `evaluate` は session あたり 1 回（single-shot）— 2 回目は
//!   `ScriptErrorKind::Internal` で fail-closed。`entry` override は
//!   非対応（`Unsupported`）。
//! - host binding は記述子の検証・受理のみ — 中立契約に handler
//!   実行 seam が存在しないため VM へは注入しない。`max_host_calls`
//!   は host call 経路が存在しないため vacuous に充足される。
//! - budget: `max_wall_time` = interrupt callback の deadline 検査、
//!   `max_memory_bytes` = `Lua::set_memory_limit`、`max_output_bytes`
//!   = eval 後の canonical 直列化サイズ検査。`max_instructions` は
//!   mlua に命令数 budget API がなく `UnsupportedBudget` で拒否。
//! - cancel は契約 `CancelToken` を interrupt closure へ clone 注入
//!   — consumer 側の `cancel_token().cancel()` も safepoint で観測
//!   される。`CancelGranularity::Cooperative` を正直に広告する。
//! - `unsafe` は本 module に存在しない（mlua safe API のみ）。
//!   Luau/vendor 型・mlua 型は公開面に出さない。

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};
use std::time::Instant;

use mlua::{Lua, LuaOptions, MultiValue, StdLib, Value as LuaValue, VmState};
use ruviform_script::CONTRACT_VERSION;
use ruviform_script::budget::{BudgetKind, CancelGranularity, CancelToken, ExecutionBudget};
use ruviform_script::canonical::{canonical_value, source_digest};
use ruviform_script::capability::BackendCapabilities;
use ruviform_script::host::HOST_SURFACE_SCHEMA_VERSION;
use ruviform_script::id::{CapabilityId, ScriptBackendId, ScriptBackendVersion, SessionId};
use ruviform_script::registry::BackendDescriptor;
use ruviform_script::session::{ScriptBackend, ScriptSession, SessionConfig, SessionError};
use ruviform_script::value::{
    ArtifactKind, ArtifactSource, ProducedBy, SOURCE_SCHEMA_VERSION, ScriptArtifact, ScriptError,
    ScriptErrorKind, ScriptResult, ScriptValue,
};

use crate::analyzer::luau_language;
use crate::ffi::bridge_versions;

/// Backend ID this implementation advertises.
const BACKEND_ID: &str = "luau-mlua";

/// Chunk name reported to Luau for diagnostics/traces.
const CHUNK_NAME: &str = "session.luau";

/// Maximum source size accepted at `open_session` — the RF-SCRIPT-00
/// `PoC` contract ceiling of `1 MiB` (analyzer 側と同一 ceiling)。
const MAX_SOURCE_BYTES: u64 = 1 << 20;

/// Maximum nesting depth accepted when lifting a Lua table into the
/// neutral [`ScriptValue`] model — deeper structures are a typed
/// contract violation, never a stack overflow.
const MAX_VALUE_DEPTH: usize = 64;

/// Maximum `ScriptValue` nodes produced by one `lift` — bounds the
/// work a shared-subtable DAG can force inside Rust code where no VM
/// safepoint or memory limit can fire.
const MAX_LIFT_NODES: usize = 1_000_000;

/// Globals mlua's `configure_luau` installs that must not reach
/// script code: `require`/`loadstring` execute source outside the
/// digest pin, `print` corrupts the host's stdout contract, and
/// `debug` exposes internals no declarative spec needs.
const STRIPPED_GLOBALS: [&str; 4] = ["require", "loadstring", "print", "debug"];

/// Interrupt hit-flag values shared between the session and the Luau
/// interrupt callback.
const HIT_CANCELLED: u8 = 1;
const HIT_DEADLINE: u8 = 2;

/// Process-wide session id counter — session IDs are unique across
/// backend instances.
static NEXT_SESSION: AtomicU64 = AtomicU64::new(1);

/// The `luau-mlua` runtime backend — stateless; every session owns a
/// fresh Luau VM.
#[derive(Debug)]
pub struct LuauBackend {
    descriptor: BackendDescriptor,
}

impl LuauBackend {
    /// Creates the backend with the contract-v1 `luau-mlua` descriptor.
    #[must_use]
    pub fn new() -> Self {
        let versions = bridge_versions();
        // impl_version は crate version + shim 報告の Luau revision
        // を合成する（analyzer descriptor と同一 provenance 語彙）。
        // luau_version が空なら crate version のみを載せる。
        let impl_version = if versions.luau_version.is_empty() {
            env!("CARGO_PKG_VERSION").to_owned()
        } else {
            format!(
                "{}+luau{}",
                env!("CARGO_PKG_VERSION"),
                versions.luau_version
            )
        };
        Self {
            descriptor: BackendDescriptor {
                backend_id: ScriptBackendId::new(BACKEND_ID),
                language: luau_language(),
                version: ScriptBackendVersion::new(impl_version, CONTRACT_VERSION),
                capabilities: BackendCapabilities {
                    artifact_kinds: BTreeSet::from([
                        ArtifactKind::ActionRecipe,
                        ArtifactKind::WorkflowDefinition,
                    ]),
                    capability_ids: BTreeSet::from([CapabilityId::new("session.evaluate")]),
                    owner_thread_only: true,
                    send: false,
                    sync: false,
                    shared_vm_state: false,
                    cancel_granularity: CancelGranularity::Cooperative,
                },
            },
        }
    }

    /// `open_session` の budget 次元検証 + wall-clock deadline の
    /// 事前計算。`Instant + Duration` の overflow は panic ではなく
    /// `UnsupportedBudget` で fail-closed する。
    fn validate_budget(budget: &ExecutionBudget) -> Result<Option<Instant>, SessionError> {
        if budget.max_instructions.is_some() {
            return Err(SessionError::UnsupportedBudget {
                kind: BudgetKind::Instructions,
            });
        }
        if let Some(limit) = budget.max_memory_bytes {
            // mlua maps 0 to "unlimited" — a zero-byte budget would
            // silently fail open, so it is rejected instead.
            if limit == 0 || usize::try_from(limit).is_err() {
                return Err(SessionError::UnsupportedBudget {
                    kind: BudgetKind::MemoryBytes,
                });
            }
        }
        budget
            .max_wall_time
            .map(|d| {
                Instant::now()
                    .checked_add(d)
                    .ok_or(SessionError::UnsupportedBudget {
                        kind: BudgetKind::WallTime,
                    })
            })
            .transpose()
    }

    /// `artifact_kinds` 要求の検証 — 広告集合外・複数指定は
    /// `SessionError::Backend` で fail-closed（1 session 1 artifact）。
    fn validate_artifact_kinds(
        requested: &BTreeSet<ArtifactKind>,
        advertised: &BTreeSet<ArtifactKind>,
    ) -> Result<Option<ArtifactKind>, SessionError> {
        if requested.len() > 1 {
            return Err(SessionError::Backend {
                detail: "at most one artifact kind may be requested per session".to_owned(),
            });
        }
        let kind = requested.iter().next().cloned();
        if let Some(kind) = &kind
            && !advertised.contains(kind)
        {
            return Err(SessionError::Backend {
                detail: format!("unsupported artifact kind: {kind:?}"),
            });
        }
        Ok(kind)
    }

    /// Creates a fresh sandboxed Luau VM for one session.
    ///
    /// `Lua::new_with` (not `Lua::new`) so state-creation failure is a
    /// typed [`SessionError`] instead of mlua's internal `expect`
    /// panic; `catch_unwind` covers the remaining `mlua_expect` paths
    /// inside state setup. [`STRIPPED_GLOBALS`] are then removed —
    /// mlua's `configure_luau` installs them unconditionally and they
    /// would let script source escape the source-digest pin
    /// (`require`/`loadstring`) or corrupt the host stdout contract
    /// (`print`).
    fn new_vm() -> Result<Lua, SessionError> {
        let lua = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            Lua::new_with(StdLib::ALL_SAFE, LuaOptions::default())
        }))
        .map_err(|_| SessionError::ResourceExhausted {
            detail: "failed to create Luau state".to_owned(),
        })?
        .map_err(|e| SessionError::ResourceExhausted {
            detail: format!("failed to create Luau state: {e}"),
        })?;
        let globals = lua.globals();
        for name in STRIPPED_GLOBALS {
            globals
                .raw_set(name, LuaValue::Nil)
                .map_err(|e| SessionError::Backend {
                    detail: format!("failed to strip global `{name}`: {e}"),
                })?;
        }
        Ok(lua)
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

    fn open_session(&self, config: SessionConfig) -> Result<Box<dyn ScriptSession>, SessionError> {
        let descriptor = self.descriptor();
        // fail-closed 検証は全て VM 生成より前 — 失敗経路で VM コストを
        // 払わない（順序は FakeBackend の語彙に整合 + backend 固有分）。
        if config.source.language != descriptor.language {
            return Err(SessionError::LanguageMismatch {
                expected: descriptor.language.clone(),
                actual: config.source.language.clone(),
            });
        }
        if let Err(e) = config.host_bindings.validate_neutral() {
            return Err(SessionError::InvalidHostSurface {
                reason: e.to_string(),
            });
        }
        if config.host_bindings.schema_version != HOST_SURFACE_SCHEMA_VERSION {
            return Err(SessionError::InvalidHostSurface {
                reason: format!(
                    "unsupported host surface schema_version {}",
                    config.host_bindings.schema_version
                ),
            });
        }
        let bytes = u64::try_from(config.source.text.len()).unwrap_or(u64::MAX);
        if bytes > MAX_SOURCE_BYTES {
            return Err(SessionError::SourceTooLarge {
                bytes,
                max: MAX_SOURCE_BYTES,
            });
        }
        // provenance 偽装の fail-closed: 宣言 digest が text の
        // 再計算値と一致することを境界で検証する（artifact の
        // ArtifactSource.source_digest が text と乖離しない保証）。
        let actual_digest = source_digest(&config.source.text);
        if config.source.digest != actual_digest {
            return Err(SessionError::SourceDigestMismatch {
                expected: config.source.digest.clone(),
                actual: actual_digest,
            });
        }
        if let Some(expected) = &config.resume
            && *expected != config.source.digest
        {
            return Err(SessionError::SourceDigestMismatch {
                expected: expected.clone(),
                actual: config.source.digest.clone(),
            });
        }
        if config.source.schema_version != SOURCE_SCHEMA_VERSION {
            return Err(SessionError::Backend {
                detail: format!(
                    "unsupported source schema_version {}",
                    config.source.schema_version
                ),
            });
        }
        let deadline = Self::validate_budget(&config.budget)?;
        let artifact_kind = Self::validate_artifact_kinds(
            &config.artifact_kinds,
            &descriptor.capabilities.artifact_kinds,
        )?;

        let lua = Self::new_vm()?;
        if let Some(limit) = config.budget.max_memory_bytes {
            let limit = usize::try_from(limit).map_err(|_| SessionError::UnsupportedBudget {
                kind: BudgetKind::MemoryBytes,
            })?;
            lua.set_memory_limit(limit)
                .map_err(|e| SessionError::Backend {
                    detail: format!("failed to set memory limit: {e}"),
                })?;
        }

        // Contract `CancelToken` を interrupt closure へ clone 注入 —
        // session.cancel_token() が返す clone を consumer が cancel
        // しても safepoint で観測される。deadline/cancel の区別は
        // 共有 hit flag で post-eval に写像する（pcall 飲み込み対策）。
        let token = CancelToken::new();
        let hit = Arc::new(AtomicU8::new(0));
        lua.set_interrupt({
            let token = token.clone();
            let hit = Arc::clone(&hit);
            move |_lua| {
                if token.is_cancelled() {
                    hit.store(HIT_CANCELLED, Ordering::Relaxed);
                    return Err(mlua::Error::external(
                        "ruviform-script-luau: evaluation cancelled",
                    ));
                }
                if deadline.is_some_and(|dl| Instant::now() >= dl) {
                    hit.store(HIT_DEADLINE, Ordering::Relaxed);
                    return Err(mlua::Error::external(
                        "ruviform-script-luau: wall-clock budget exceeded",
                    ));
                }
                Ok(VmState::Continue)
            }
        });

        Ok(Box::new(LuauSession {
            session_id: SessionId(NEXT_SESSION.fetch_add(1, Ordering::Relaxed)),
            backend_id: descriptor.backend_id.clone(),
            version: descriptor.version.clone(),
            token,
            hit,
            lua: Some(lua),
            config,
            artifact_kind,
            evaluated: false,
            closed: false,
        }))
    }
}

/// A live Luau evaluation session — owns its `Lua` state until
/// [`ScriptSession::close`].
struct LuauSession {
    session_id: SessionId,
    backend_id: ScriptBackendId,
    version: ScriptBackendVersion,
    token: CancelToken,
    hit: Arc<AtomicU8>,
    lua: Option<Lua>,
    config: SessionConfig,
    artifact_kind: Option<ArtifactKind>,
    evaluated: bool,
    closed: bool,
}

impl LuauSession {
    /// Lifts an `mlua::Value` into the closed [`ScriptValue`] model.
    /// `nodes` is the remaining [`MAX_LIFT_NODES`] budget — every
    /// table element visit consumes one, which bounds the total work
    /// a shared-subtable DAG can force inside Rust code.
    // `f as i64` below is guarded by the fract/range check.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "f as i64 is guarded by the fract/range check"
    )]
    fn lift(value: LuaValue, depth: usize, nodes: &mut usize) -> Result<ScriptValue, ScriptError> {
        if depth > MAX_VALUE_DEPTH {
            return Err(ScriptError::new(
                ScriptErrorKind::Internal,
                format!("value nesting exceeds {MAX_VALUE_DEPTH}"),
            ));
        }
        Ok(match value {
            LuaValue::Nil => ScriptValue::Null,
            LuaValue::Boolean(b) => ScriptValue::Bool(b),
            LuaValue::Integer(i) => ScriptValue::Integer(i),
            LuaValue::Number(f) => {
                // Luau has a single f64 number type; integral values in
                // i64 range lower to the exact-integer variant so spec
                // fields like `version: 1` decode identically. Non-
                // finite values (NaN/±inf) have no integral form and
                // stay `Number` — the canonical form renders them as
                // deterministic tag records.
                if f.fract() == 0.0
                    && (-9.223_372_036_854_776e18..9.223_372_036_854_776e18).contains(&f)
                {
                    ScriptValue::Integer(f as i64)
                } else {
                    ScriptValue::Number(f)
                }
            }
            LuaValue::String(s) => ScriptValue::String(
                s.to_str()
                    .map_err(|_| {
                        ScriptError::new(ScriptErrorKind::Internal, "non-UTF-8 Luau string")
                    })?
                    .to_owned(),
            ),
            LuaValue::Table(t) => Self::lift_table(&t, depth, nodes)?,
            other => {
                return Err(ScriptError::new(
                    ScriptErrorKind::Unsupported,
                    format!("unsupported Luau value type: {other:?}"),
                ));
            }
        })
    }

    /// Lifts a Lua table: all-string keys → record, consecutive
    /// integer keys `1..=n` → array, empty → record. Mixed or sparse
    /// shapes have no honest neutral form and are rejected.
    fn lift_table(
        table: &mlua::Table,
        depth: usize,
        nodes: &mut usize,
    ) -> Result<ScriptValue, ScriptError> {
        let mut strings: BTreeMap<String, ScriptValue> = BTreeMap::new();
        let mut ints: Vec<(i64, ScriptValue)> = Vec::new();
        for pair in table.clone().pairs::<LuaValue, LuaValue>() {
            *nodes = nodes.checked_sub(1).ok_or_else(|| {
                ScriptError::new(
                    ScriptErrorKind::Internal,
                    format!("lifted value exceeds {MAX_LIFT_NODES} nodes"),
                )
            })?;
            let (key, val) = pair.map_err(|e| {
                ScriptError::new(ScriptErrorKind::Runtime, format!("table iteration: {e}"))
            })?;
            let lifted = Self::lift(val, depth + 1, nodes)?;
            match key {
                LuaValue::String(s) => {
                    let name = s.to_str().map_err(|_| {
                        ScriptError::new(ScriptErrorKind::Internal, "non-UTF-8 table key")
                    })?;
                    strings.insert(name.to_owned(), lifted);
                }
                LuaValue::Integer(i) => ints.push((i, lifted)),
                other => {
                    return Err(ScriptError::new(
                        ScriptErrorKind::Internal,
                        format!("unsupported table key type: {other:?}"),
                    ));
                }
            }
        }
        if strings.is_empty() && !ints.is_empty() {
            ints.sort_by_key(|(k, _)| *k);
            let is_sequence = ints.first().is_some_and(|(k, _)| *k == 1)
                && ints.windows(2).all(|w| w[1].0 == w[0].0 + 1);
            if is_sequence {
                return Ok(ScriptValue::Array(
                    ints.into_iter().map(|(_, v)| v).collect(),
                ));
            }
            return Err(ScriptError::new(
                ScriptErrorKind::Internal,
                "sparse integer-keyed table has no array form",
            ));
        }
        if !strings.is_empty() && !ints.is_empty() {
            return Err(ScriptError::new(
                ScriptErrorKind::Internal,
                "mixed string/integer keys have no record form",
            ));
        }
        Ok(ScriptValue::Record(strings))
    }

    /// Maps an interrupt flag to the typed error it represents, if any.
    fn interrupt_error(&self) -> Option<ScriptError> {
        match self.hit.load(Ordering::Relaxed) {
            HIT_CANCELLED => Some(ScriptError::new(
                ScriptErrorKind::Cancelled,
                "evaluation cancelled",
            )),
            HIT_DEADLINE => Some(ScriptError::new(
                ScriptErrorKind::BudgetExceeded,
                "wall-clock budget exceeded",
            )),
            _ => None,
        }
    }

    /// Maps an `mlua::Error` to the neutral kind, honoring interrupt
    /// flags first so budget/cancel wins over the wrapper error.
    fn map_eval_error(&self, err: &mlua::Error) -> ScriptError {
        if let Some(error) = self.interrupt_error() {
            return error;
        }
        let kind = match err {
            mlua::Error::SyntaxError { .. } => ScriptErrorKind::Parse,
            mlua::Error::MemoryError(_) => ScriptErrorKind::BudgetExceeded,
            mlua::Error::RuntimeError(_) => ScriptErrorKind::Runtime,
            _ => ScriptErrorKind::Backend,
        };
        ScriptError::new(kind, err.to_string())
    }

    /// Lowers the evaluated `MultiValue` into a [`ScriptResult`] —
    /// output budget, artifact payload, and digest provenance.
    fn lower_result(&self, values: MultiValue) -> Result<ScriptResult, ScriptError> {
        if values.is_empty() {
            return Ok(ScriptResult::default());
        }
        if values.len() > 1 {
            return Err(ScriptError::new(
                ScriptErrorKind::Internal,
                format!(
                    "script returned {} values; exactly one is allowed",
                    values.len()
                ),
            ));
        }
        let mut nodes = MAX_LIFT_NODES;
        let value = Self::lift(
            values.into_iter().next().unwrap_or(LuaValue::Nil),
            0,
            &mut nodes,
        )?;
        if matches!(value, ScriptValue::Null) {
            // `return nil` declares no artifact and no return value.
            return Ok(ScriptResult::default());
        }

        // Serialized-output budget — the lifted value is the largest
        // thing that crosses the boundary. canonical_value は lift
        // 済み値に対して常に成功するが、失敗時は fail-closed に
        // Internal へ写像する。
        let canonical = canonical_value(&value).map_err(|e| {
            ScriptError::new(
                ScriptErrorKind::Internal,
                format!("value is not canonicalizable: {e}"),
            )
        })?;
        if let Some(limit) = self.config.budget.max_output_bytes {
            let size = u64::try_from(canonical.len()).unwrap_or(u64::MAX);
            if size > limit {
                return Err(ScriptError::new(
                    ScriptErrorKind::BudgetExceeded,
                    format!("output size {size} exceeds budget {limit}"),
                ));
            }
        }

        let artifact = match &self.artifact_kind {
            Some(kind) => {
                // Artifact payload は record 契約 — 非 Record の
                // return は artifact を構成できない（silent truncate
                // せず fail-closed）。
                let ScriptValue::Record(map) = &value else {
                    return Err(ScriptError::new(
                        ScriptErrorKind::Internal,
                        "artifact payload must be a record",
                    ));
                };
                Some(
                    ScriptArtifact::new(
                        kind.clone(),
                        map.clone(),
                        ArtifactSource {
                            source_digest: self.config.source.digest.clone(),
                            source_revision: self.config.source.revision,
                        },
                        ProducedBy {
                            backend: self.backend_id.clone(),
                            version: self.version.clone(),
                        },
                    )
                    .map_err(|e| {
                        ScriptError::new(
                            ScriptErrorKind::Internal,
                            format!("artifact digest failed: {e}"),
                        )
                    })?,
                )
            }
            None => None,
        };
        Ok(ScriptResult {
            artifact,
            return_value: Some(value),
            diagnostics: Vec::new(),
        })
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
        if self.closed {
            return Err(ScriptError::new(
                ScriptErrorKind::Internal,
                "evaluate after close",
            ));
        }
        if self.token.is_cancelled() {
            return Err(ScriptError::new(
                ScriptErrorKind::Cancelled,
                "evaluation cancelled",
            ));
        }
        // The session contract is evaluate-once: a second run would
        // see mutated globals and a possibly stale interrupt flag, so
        // the session is spent after the first attempt — success or
        // not.
        if self.evaluated {
            return Err(ScriptError::new(
                ScriptErrorKind::Internal,
                "session already evaluated",
            ));
        }
        self.evaluated = true;
        if entry.is_some() {
            return Err(ScriptError::new(
                ScriptErrorKind::Unsupported,
                "named entry points are not supported by this backend",
            ));
        }
        let Some(lua) = self.lua.as_ref() else {
            return Err(ScriptError::new(
                ScriptErrorKind::Internal,
                "session holds no Lua state",
            ));
        };
        let values = lua
            .load(self.config.source.text.as_str())
            .set_name(CHUNK_NAME)
            // Text-only: a bytecode-magic source must not reach the VM.
            .set_mode(mlua::chunk::ChunkMode::Text)
            .eval::<MultiValue>()
            .map_err(|e| self.map_eval_error(&e))?;
        // Defense in depth: an interrupt that fired but was swallowed
        // by script `pcall` must never surface as success.
        if let Some(error) = self.interrupt_error() {
            return Err(error);
        }
        self.lower_result(values)
    }

    fn cancel_token(&self) -> CancelToken {
        self.token.clone()
    }

    fn close(&mut self) {
        self.closed = true;
        self.lua = None;
    }
}
