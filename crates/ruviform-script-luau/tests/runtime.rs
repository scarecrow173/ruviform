//! `LuauBackend`/`LuauSession` end-to-end — 実 vendored Luau VM
//! （mlua Rust API）に対する open/evaluate/cancel/budget/artifact/
//! fail-closed 行列と `ScriptBackendRegistry` resolve→eval 疎通。
//! deterministic OS-L1（Windows `x86_64`）。consumer gate 委譲事項は
//! `docs/test-specs/issue-716-runtime-backend.md` を参照。

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use ruviform_script::{
    ArtifactKind, BudgetKind, CONTRACT_VERSION, CancelGranularity, CapabilityId, ExecutionBudget,
    HOST_SURFACE_SCHEMA_VERSION, HostApiSurface, HostBinding, HostEffect, HostSignature,
    PlacementHint, ResolveError, ResolveErrorReason, ScriptBackend, ScriptBackendRegistry,
    ScriptErrorKind, ScriptRequirement, ScriptResult, ScriptSession, ScriptSource, ScriptValue,
    SessionConfig, SessionError, SourceRevision, ValueKind, canonical_value, definition_digest,
    source_digest,
};
use ruviform_script_luau::{LuauBackend, luau_language};

fn backend() -> LuauBackend {
    LuauBackend::new()
}

fn config(text: &str) -> SessionConfig {
    SessionConfig {
        budget: ExecutionBudget::unlimited(),
        host_bindings: HostApiSurface::empty(),
        artifact_kinds: BTreeSet::new(),
        source: ScriptSource::new(text, luau_language(), None, SourceRevision::new(1)),
        placement_hint: Some(PlacementHint::Any),
        resume: None,
    }
}

fn open(text: &str) -> Box<dyn ScriptSession> {
    backend().open_session(config(text)).expect("open_session")
}

fn eval(text: &str) -> ScriptResult {
    open(text).evaluate(None).expect("evaluate")
}

fn open_err(cfg: SessionConfig) -> SessionError {
    match backend().open_session(cfg) {
        Err(e) => e,
        Ok(_) => panic!("expected open_session failure"),
    }
}

/// `&dyn ScriptBackend` は Debug 非実装のため `expect_err` が使えない
/// — `tests/common/mod.rs` の `expect_resolve_err` と同じ回避。
fn resolve_err(result: Result<&dyn ScriptBackend, ResolveError>) -> ResolveError {
    match result {
        Err(e) => e,
        Ok(_) => panic!("expected resolve failure"),
    }
}

// ---------- descriptor / registry ----------

#[test]
fn descriptor_advertises_contract() {
    let d = backend().descriptor().clone();
    assert_eq!(d.backend_id.as_str(), "luau-mlua");
    assert_eq!(d.language, luau_language());
    assert_eq!(d.version.contract_version, CONTRACT_VERSION);
    assert!(d.version.impl_version.contains("luau"));
    let caps = &d.capabilities;
    assert_eq!(
        caps.artifact_kinds,
        BTreeSet::from([ArtifactKind::ActionRecipe, ArtifactKind::WorkflowDefinition])
    );
    assert_eq!(
        caps.capability_ids,
        BTreeSet::from([CapabilityId::new("session.evaluate")])
    );
    assert!(caps.owner_thread_only);
    assert!(!caps.send);
    assert!(!caps.sync);
    assert!(!caps.shared_vm_state);
    assert_eq!(caps.cancel_granularity, CancelGranularity::Cooperative);
}

#[test]
fn registry_resolve_open_evaluate() {
    let mut registry = ScriptBackendRegistry::new();
    registry.register(Box::new(backend())).expect("register");
    let requirement = ScriptRequirement {
        language: luau_language(),
        backend: None,
        required_capabilities: BTreeSet::from([CapabilityId::new("session.evaluate")]),
        min_contract_version: CONTRACT_VERSION,
        artifact_kinds: BTreeSet::new(),
    };
    let resolved = registry.resolve(&requirement).expect("resolve");
    assert_eq!(resolved.descriptor().backend_id.as_str(), "luau-mlua");
    let mut session = resolved
        .open_session(config("return 40 + 2"))
        .expect("open via resolved backend");
    let result = session.evaluate(None).expect("evaluate");
    assert_eq!(result.return_value, Some(ScriptValue::Integer(42)));
}

#[test]
fn registry_fail_closed() {
    // 空 registry → UnknownLanguage。
    let registry = ScriptBackendRegistry::new();
    let req = ScriptRequirement {
        language: luau_language(),
        ..ScriptRequirement::default()
    };
    let err = resolve_err(registry.resolve(&req));
    assert!(matches!(
        err,
        ResolveError::NoMatchingBackend {
            reason: ResolveErrorReason::UnknownLanguage,
            ..
        }
    ));

    let mut registry = ScriptBackendRegistry::new();
    registry.register(Box::new(backend())).expect("register");

    // 未広告 capability（host.call）→ MissingCapability。
    let req = ScriptRequirement {
        required_capabilities: BTreeSet::from([CapabilityId::new("host.call")]),
        ..req
    };
    let err = resolve_err(registry.resolve(&req));
    assert!(matches!(
        err,
        ResolveError::NoMatchingBackend {
            reason: ResolveErrorReason::MissingCapability,
            ..
        }
    ));

    // ConsumerExtension artifact kind 要求 → MissingCapability。
    let req = ScriptRequirement {
        required_capabilities: BTreeSet::new(),
        artifact_kinds: BTreeSet::from([ArtifactKind::ConsumerExtension {
            domain: "x".to_string(),
        }]),
        ..req
    };
    let err = resolve_err(registry.resolve(&req));
    assert!(matches!(
        err,
        ResolveError::NoMatchingBackend {
            reason: ResolveErrorReason::MissingCapability,
            ..
        }
    ));
}

// ---------- open_session fail-closed ----------

#[test]
fn open_language_mismatch() {
    let mut cfg = config("return 1");
    cfg.source = ScriptSource::new(
        "return 1",
        ruviform_script::ScriptLanguageId::new("rhai"),
        None,
        SourceRevision::new(1),
    );
    let err = open_err(cfg);
    assert!(matches!(err, SessionError::LanguageMismatch { .. }));
}

#[test]
fn open_source_digest_mismatch() {
    // 宣言 digest が text の再計算値と乖離 → provenance 偽装を拒否。
    let mut cfg = config("return 1");
    cfg.source.digest = source_digest("different source");
    let err = open_err(cfg);
    assert!(matches!(err, SessionError::SourceDigestMismatch { .. }));
}

#[test]
fn open_resume_digest() {
    let mut cfg = config("return 1");
    cfg.resume = Some(source_digest("other"));
    let err = open_err(cfg);
    assert!(matches!(err, SessionError::SourceDigestMismatch { .. }));

    let mut cfg = config("return 1");
    cfg.resume = Some(cfg.source.digest.clone());
    backend().open_session(cfg).expect("resume match opens");
}

#[test]
fn open_source_too_large() {
    let mut cfg = config("");
    cfg.source = ScriptSource::new(
        "x".repeat((1 << 20) + 1),
        luau_language(),
        None,
        SourceRevision::new(1),
    );
    let err = open_err(cfg);
    assert!(matches!(err, SessionError::SourceTooLarge { .. }));
}

#[test]
fn open_unsupported_budget_dimensions() {
    // max_instructions — mlua に命令数 budget API がない。
    let mut cfg = config("return 1");
    cfg.budget = ExecutionBudget {
        max_instructions: Some(1000),
        ..ExecutionBudget::unlimited()
    };
    let err = open_err(cfg);
    assert!(matches!(
        err,
        SessionError::UnsupportedBudget {
            kind: BudgetKind::Instructions
        }
    ));

    // max_memory_bytes = 0 — mlua では unlimited 意味のため拒否。
    let mut cfg = config("return 1");
    cfg.budget = ExecutionBudget {
        max_memory_bytes: Some(0),
        ..ExecutionBudget::unlimited()
    };
    let err = open_err(cfg);
    assert!(matches!(
        err,
        SessionError::UnsupportedBudget {
            kind: BudgetKind::MemoryBytes
        }
    ));
}

#[test]
fn open_supported_budgets_accepted() {
    let mut cfg = config("return 1");
    cfg.budget = ExecutionBudget {
        max_wall_time: Some(Duration::from_secs(60)),
        max_memory_bytes: Some(64 << 20),
        max_output_bytes: Some(1 << 20),
        max_host_calls: Some(0),
        ..ExecutionBudget::unlimited()
    };
    backend().open_session(cfg).expect("supported budgets open");
}

#[test]
fn open_artifact_kinds_fail_closed() {
    // 広告外 kind（ConsumerExtension）→ Backend。
    let mut cfg = config("return {}");
    cfg.artifact_kinds = BTreeSet::from([ArtifactKind::ConsumerExtension {
        domain: "x".to_string(),
    }]);
    let err = open_err(cfg);
    assert!(matches!(err, SessionError::Backend { .. }));

    // 2 件指定 → Backend（1 session 1 artifact）。
    let mut cfg = config("return {}");
    cfg.artifact_kinds =
        BTreeSet::from([ArtifactKind::ActionRecipe, ArtifactKind::WorkflowDefinition]);
    let err = open_err(cfg);
    assert!(matches!(err, SessionError::Backend { .. }));
}

#[test]
fn open_invalid_host_surface() {
    // 予約 prefix → validate_neutral 失敗。
    let mut cfg = config("return 1");
    cfg.host_bindings = HostApiSurface {
        bindings: vec![HostBinding {
            name: "os.exec".to_string(),
            signature: HostSignature {
                args: vec![],
                returns: ValueKind::Any,
            },
            effect: HostEffect::Effect,
            schema_version: HOST_SURFACE_SCHEMA_VERSION,
            timeout_hint: None,
        }],
        schema_version: HOST_SURFACE_SCHEMA_VERSION,
    };
    let err = open_err(cfg);
    assert!(matches!(err, SessionError::InvalidHostSurface { .. }));

    // schema_version 不一致 → InvalidHostSurface。
    let mut cfg = config("return 1");
    cfg.host_bindings.schema_version = 999;
    let err = open_err(cfg);
    assert!(matches!(err, SessionError::InvalidHostSurface { .. }));
}

#[test]
fn open_valid_host_surface_accepted_but_not_executable() {
    // 妥当な surface は受理されるが、実行 seam がないため script から
    // binding 名は到達不能（nil index → Runtime error で fail-closed）。
    let mut cfg = config("return action.invoke");
    cfg.host_bindings = HostApiSurface {
        bindings: vec![HostBinding {
            name: "action.invoke".to_string(),
            signature: HostSignature {
                args: vec![],
                returns: ValueKind::Any,
            },
            effect: HostEffect::Effect,
            schema_version: HOST_SURFACE_SCHEMA_VERSION,
            timeout_hint: None,
        }],
        schema_version: HOST_SURFACE_SCHEMA_VERSION,
    };
    let mut session = backend().open_session(cfg).expect("valid surface opens");
    // `action` global は存在しない → nil index で Runtime error。
    let err = session.evaluate(None).expect_err("binding unreachable");
    assert_eq!(err.kind, ScriptErrorKind::Runtime);
}

#[test]
fn open_source_schema_version() {
    let mut cfg = config("return 1");
    cfg.source.schema_version = 999;
    let err = open_err(cfg);
    assert!(matches!(err, SessionError::Backend { .. }));
}

// ---------- evaluate / value mapping ----------

#[test]
fn evaluate_scalar_values() {
    assert_eq!(
        eval("return 40 + 2").return_value,
        Some(ScriptValue::Integer(42))
    );
    assert_eq!(
        eval("return 4.5").return_value,
        Some(ScriptValue::Number(4.5))
    );
    assert_eq!(
        eval("return \"hi\"").return_value,
        Some(ScriptValue::String("hi".to_string()))
    );
    assert_eq!(
        eval("return true").return_value,
        Some(ScriptValue::Bool(true))
    );
    // `return nil` と return なしはともに default。
    assert_eq!(eval("return nil"), ScriptResult::default());
    assert_eq!(eval("local x = 1"), ScriptResult::default());
}

#[test]
fn evaluate_integral_number_lowers_to_integer() {
    // Luau number (f64) の整数値は i64 範囲内なら Integer へ降格。
    assert_eq!(
        eval("return 42").return_value,
        Some(ScriptValue::Integer(42))
    );
    // i64 範囲外の整数値は Number のまま。
    assert_eq!(
        eval("return 2^63").return_value,
        Some(ScriptValue::Number(9.223_372_036_854_776e18))
    );
}

#[test]
fn evaluate_non_finite_number() {
    let result = eval("return 0/0");
    assert!(
        matches!(result.return_value, Some(ScriptValue::Number(f)) if f.is_nan()),
        "got {:?}",
        result.return_value
    );
}

#[test]
fn evaluate_collections() {
    assert_eq!(
        eval("return {1, 2, 3}").return_value,
        Some(ScriptValue::Array(vec![
            ScriptValue::Integer(1),
            ScriptValue::Integer(2),
            ScriptValue::Integer(3),
        ]))
    );
    let record = eval("return {a = 1, b = \"x\"}").return_value;
    assert!(matches!(record, Some(ScriptValue::Record(_))));
    // nested table。
    let nested = eval("return {outer = {inner = {v = 7}}}").return_value;
    assert!(matches!(nested, Some(ScriptValue::Record(_))));
    // 空 table → 空 Record。
    assert_eq!(
        eval("return {}").return_value,
        Some(ScriptValue::Record(BTreeMap::default()))
    );
}

#[test]
fn evaluate_multi_return_rejected() {
    let mut session = open("return 1, 2");
    let err = session.evaluate(None).expect_err("multi-return");
    assert_eq!(err.kind, ScriptErrorKind::Internal);
}

#[test]
fn evaluate_unsupported_value_types() {
    for src in [
        "return function() end",
        "return coroutine.create(function() end)",
        "return tostring",
    ] {
        let mut session = open(src);
        let err = session.evaluate(None).expect_err(src);
        assert_eq!(err.kind, ScriptErrorKind::Unsupported, "src: {src}");
    }
}

#[test]
fn evaluate_table_shape_violations() {
    // mixed string/integer keys → Internal。
    let mut s = open("return {[1] = \"a\", x = 1}");
    assert_eq!(
        s.evaluate(None).expect_err("mixed").kind,
        ScriptErrorKind::Internal
    );
    // sparse integer keys → Internal。
    let mut s = open("return {[2] = \"x\"}");
    assert_eq!(
        s.evaluate(None).expect_err("sparse").kind,
        ScriptErrorKind::Internal
    );
    // 深さ > 64 の nested table → Internal（括弧ではなく table を nest）。
    let deep = format!("return {}1{}", "{a=".repeat(70), "}".repeat(70));
    let mut s = open(&deep);
    assert_eq!(
        s.evaluate(None).expect_err("deep").kind,
        ScriptErrorKind::Internal
    );
    // 非 UTF-8 string → Internal。
    let mut s = open("return string.char(255)");
    assert_eq!(
        s.evaluate(None).expect_err("utf8").kind,
        ScriptErrorKind::Internal
    );
}

#[test]
fn evaluate_parse_and_runtime_errors() {
    let mut s = open("local x = ");
    assert_eq!(
        s.evaluate(None).expect_err("parse").kind,
        ScriptErrorKind::Parse
    );
    let mut s = open("error(\"boom\")");
    assert_eq!(
        s.evaluate(None).expect_err("runtime").kind,
        ScriptErrorKind::Runtime
    );
    let mut s = open("local x = nil return x.y");
    assert_eq!(
        s.evaluate(None).expect_err("nil index").kind,
        ScriptErrorKind::Runtime
    );
}

#[test]
fn evaluate_bytecode_rejected() {
    // Luau bytecode magic — Text mode が拒否する（kind は VM 依存、
    // 失敗自体が契約）。
    let mut cfg = config("");
    cfg.source = ScriptSource::new("\u{1b}Luau", luau_language(), None, SourceRevision::new(1));
    let mut session = backend().open_session(cfg).expect("open");
    assert!(session.evaluate(None).is_err(), "bytecode must be rejected");
}

#[test]
fn evaluate_entry_unsupported() {
    let mut session = open("return 1");
    let err = session.evaluate(Some("main")).expect_err("entry");
    assert_eq!(err.kind, ScriptErrorKind::Unsupported);
}

#[test]
fn sandbox_strips_escape_globals() {
    // require/loadstring/print/debug は除去済み — digest pin 外の
    // code 実行・stdout 汚染・debug 公開を遮断。
    let result = eval(
        "return type(require) .. \",\" .. type(loadstring) .. \",\" .. type(print) .. \",\" .. type(debug)",
    );
    assert_eq!(
        result.return_value,
        Some(ScriptValue::String("nil,nil,nil,nil".to_string()))
    );
}

#[test]
fn evaluate_is_deterministic() {
    let src = "local t = {b = 2, a = 1} return {sum = t.a + t.b, name = \"x\"}";
    let r1 = eval(src);
    let r2 = eval(src);
    assert_eq!(r1, r2);
}

// ---------- budget ----------

#[test]
fn budget_wall_time_exceeded() {
    let mut cfg = config("while true do end");
    cfg.budget = ExecutionBudget {
        max_wall_time: Some(Duration::from_millis(1)),
        ..ExecutionBudget::unlimited()
    };
    let mut session = backend().open_session(cfg).expect("open");
    let err = session.evaluate(None).expect_err("infinite loop");
    assert_eq!(err.kind, ScriptErrorKind::BudgetExceeded);
}

#[test]
fn budget_wall_time_within_limit_passes() {
    let mut cfg = config("return 1");
    cfg.budget = ExecutionBudget {
        max_wall_time: Some(Duration::from_secs(60)),
        ..ExecutionBudget::unlimited()
    };
    let mut session = backend().open_session(cfg).expect("open");
    assert!(session.evaluate(None).is_ok());
}

#[test]
fn budget_memory_exceeded() {
    // 100 MiB 文字列確保 → 1 MiB limit で MemoryError → BudgetExceeded。
    let mut cfg = config("return string.rep(\"x\", 100 * 1024 * 1024)");
    cfg.budget = ExecutionBudget {
        max_memory_bytes: Some(1 << 20),
        ..ExecutionBudget::unlimited()
    };
    let mut session = backend().open_session(cfg).expect("open");
    let err = session.evaluate(None).expect_err("memory");
    assert_eq!(err.kind, ScriptErrorKind::BudgetExceeded);
}

#[test]
fn budget_output_exceeded() {
    // canonical "\"...100 chars...\"" ≈ 102 bytes > 64 limit。
    let mut cfg = config("return string.rep(\"x\", 100)");
    cfg.budget = ExecutionBudget {
        max_output_bytes: Some(64),
        ..ExecutionBudget::unlimited()
    };
    let mut session = backend().open_session(cfg).expect("open");
    let err = session.evaluate(None).expect_err("output");
    assert_eq!(err.kind, ScriptErrorKind::BudgetExceeded);

    // 小さい出力は通る。
    let mut cfg = config("return \"ok\"");
    cfg.budget = ExecutionBudget {
        max_output_bytes: Some(64),
        ..ExecutionBudget::unlimited()
    };
    let mut session = backend().open_session(cfg).expect("open");
    assert!(session.evaluate(None).is_ok());
}

#[test]
fn budget_host_calls_vacuous() {
    // host call 実行経路が存在しないため max_host_calls=0 でも受理
    // （違反し得る経路がない — capability 広告もしない）。
    let mut cfg = config("return 1");
    cfg.budget = ExecutionBudget {
        max_host_calls: Some(0),
        ..ExecutionBudget::unlimited()
    };
    let mut session = backend().open_session(cfg).expect("open");
    assert!(session.evaluate(None).is_ok());
}

// ---------- cancel ----------

#[test]
fn cancel_before_evaluate() {
    let mut session = open("return 1");
    let token = session.cancel_token();
    token.cancel(); // consumer 側 clone 経由の cancel
    let err = session.evaluate(None).expect_err("cancelled");
    assert_eq!(err.kind, ScriptErrorKind::Cancelled);
}

#[test]
fn cancel_via_session() {
    let mut session = open("return 1");
    session.cancel();
    session.cancel(); // idempotent
    let err = session.evaluate(None).expect_err("cancelled");
    assert_eq!(err.kind, ScriptErrorKind::Cancelled);
}

#[test]
fn cancel_mid_evaluation_from_another_thread() {
    // cooperative cancel: safepoint で観測され Cancelled を返す。
    // `while true do end` は必ず safepoint を踏むため結果は決定的
    // （cancel が eval 完了に間に合わないことはあり得ない — 無限 loop）。
    let mut session = open("while true do end");
    let token = session.cancel_token();
    let handle = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(50));
        token.cancel();
    });
    let err = session.evaluate(None).expect_err("mid-eval cancel");
    handle.join().expect("join");
    assert_eq!(err.kind, ScriptErrorKind::Cancelled);
}

// ---------- session lifecycle ----------

#[test]
fn evaluate_is_single_shot() {
    let mut session = open("return 1");
    assert!(session.evaluate(None).is_ok());
    let err = session.evaluate(None).expect_err("second evaluate");
    assert_eq!(err.kind, ScriptErrorKind::Internal);
}

#[test]
fn close_is_idempotent_and_blocks_evaluate() {
    let mut session = open("return 1");
    session.close();
    session.close(); // idempotent
    let err = session.evaluate(None).expect_err("evaluate after close");
    assert_eq!(err.kind, ScriptErrorKind::Internal);
}

#[test]
fn session_identity() {
    let backend = backend();
    let s1 = backend.open_session(config("return 1")).expect("open 1");
    let s2 = backend.open_session(config("return 2")).expect("open 2");
    assert_ne!(s1.session_id(), s2.session_id());
    assert_eq!(s1.backend_id().as_str(), "luau-mlua");
}

#[test]
fn placement_hint_dedicated_thread_accepted() {
    // placement hint は advisory — backend は owner-thread-only を
    // 広告しつつ hint を無視して caller thread で評価する。
    let mut cfg = config("return 7");
    cfg.placement_hint = Some(PlacementHint::DedicatedThread);
    let mut session = backend().open_session(cfg).expect("open");
    let result = session.evaluate(None).expect("evaluate");
    assert_eq!(result.return_value, Some(ScriptValue::Integer(7)));
}

// ---------- artifact ----------

#[test]
fn artifact_produced_with_provenance() {
    let mut cfg = config("return {name = \"gate\", version = 1}");
    cfg.artifact_kinds = BTreeSet::from([ArtifactKind::ActionRecipe]);
    cfg.source = ScriptSource::new(
        "return {name = \"gate\", version = 1}",
        luau_language(),
        None,
        SourceRevision::new(9),
    );
    let expected_digest = cfg.source.digest.clone();
    let mut session = backend().open_session(cfg).expect("open");
    let result = session.evaluate(None).expect("evaluate");

    let artifact = result.artifact.expect("artifact");
    assert_eq!(artifact.kind, ArtifactKind::ActionRecipe);
    assert_eq!(
        artifact.schema_version,
        ruviform_script::ARTIFACT_SCHEMA_VERSION
    );
    assert_eq!(
        artifact.payload.get("name"),
        Some(&ScriptValue::String("gate".to_string()))
    );
    assert_eq!(
        artifact.payload.get("version"),
        Some(&ScriptValue::Integer(1))
    );
    // provenance: source digest/revision + producer identity。
    assert_eq!(artifact.source.source_digest, expected_digest);
    assert_eq!(artifact.source.source_revision, SourceRevision::new(9));
    assert_eq!(artifact.produced_by.backend.as_str(), "luau-mlua");
    assert_eq!(
        artifact.produced_by.version.contract_version,
        CONTRACT_VERSION
    );
    // digest は payload の canonical definition_digest と一致
    // （consumer 側検証と同じ計算 — backend は値を改竄しない）。
    let payload_value = ScriptValue::Record(artifact.payload.clone());
    assert_eq!(
        artifact.digest,
        definition_digest(&payload_value).expect("definition_digest")
    );
    // return_value も lift 値をそのまま返す。
    assert_eq!(result.return_value, Some(payload_value));
}

#[test]
fn artifact_empty_record() {
    let mut cfg = config("return {}");
    cfg.artifact_kinds = BTreeSet::from([ArtifactKind::WorkflowDefinition]);
    let mut session = backend().open_session(cfg).expect("open");
    let result = session.evaluate(None).expect("evaluate");
    let artifact = result.artifact.expect("artifact");
    assert_eq!(artifact.kind, ArtifactKind::WorkflowDefinition);
    assert!(artifact.payload.is_empty());
}

#[test]
fn artifact_non_record_return_rejected() {
    let mut cfg = config("return 1");
    cfg.artifact_kinds = BTreeSet::from([ArtifactKind::ActionRecipe]);
    let mut session = backend().open_session(cfg).expect("open");
    let err = session.evaluate(None).expect_err("non-record artifact");
    assert_eq!(err.kind, ScriptErrorKind::Internal);
}

#[test]
fn artifact_nil_return_produces_default() {
    let mut cfg = config("return nil");
    cfg.artifact_kinds = BTreeSet::from([ArtifactKind::ActionRecipe]);
    let mut session = backend().open_session(cfg).expect("open");
    assert_eq!(
        session.evaluate(None).expect("eval"),
        ScriptResult::default()
    );
}

#[test]
fn no_artifact_kind_means_no_artifact() {
    // artifact_kinds 空 → record return でも artifact=None。
    let result = eval("return {a = 1}");
    assert!(result.artifact.is_none());
    assert!(matches!(result.return_value, Some(ScriptValue::Record(_))));
}

#[test]
fn canonical_size_matches_output_budget_semantics() {
    // output budget の計測は canonical 直列化 — consumer が見る
    // canonical 形と一致することを確認（record key sort 等）。
    let result = eval("return {b = 2, a = 1}");
    let canonical = canonical_value(result.return_value.as_ref().unwrap()).unwrap();
    assert_eq!(canonical, "{\"a\":1,\"b\":2}");
}
