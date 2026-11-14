//! RF-SCRIPT-01 PoC デモ実行ファイル。
//!
//! 単一バイナリで Luau+Rhai backend を同時登録し、次を実演する。
//! 1. language/backend requirement による実行時選択
//! 2. 同一意味 artifact の canonical digest 一致
//! 3. 交互評価（同一プロセス内 session 交互実行）
//!
//! 出力は evidence/rf559 の Run 記録として使う。

use std::collections::{BTreeMap, BTreeSet};

use script_coexist_poc::prelude::*;

fn luau_action_recipe() -> &'static str {
    r#"
local t = { kind = "action_recipe", name = "normalize", version = 1,
            inputs = { "audio_in" }, outputs = { "audio_out" },
            params = { gain = 0.5 } }
return t
"#
}

fn rhai_action_recipe() -> &'static str {
    r#"
#{
    kind: "action_recipe",
    name: "normalize",
    version: 1,
    inputs: ["audio_in"],
    outputs: ["audio_out"],
    params: #{ gain: 0.5 }
}
"#
}

fn luau_workflow() -> &'static str {
    // `references` は省略する — Luau の空 table `{}` は Record、
    // Rhai の `[]` は Array と decode され canonical が一致しないため。
    // references 省略は inspect_artifact で「参照なし」として受理される。
    r#"
return {
    kind = "workflow_definition",
    name = "mix_pipeline",
    version = 1,
    steps = { "normalize", "limit" }
}
"#
}

fn rhai_workflow() -> &'static str {
    r#"
#{
    kind: "workflow_definition",
    name: "mix_pipeline",
    version: 1,
    steps: ["normalize", "limit"]
}
"#
}

fn source(text: &str, lang: &str) -> ScriptSource {
    ScriptSource::new(text, ScriptLanguageId::new(lang), SourceRevision(1))
}

fn session_config(src: ScriptSource) -> SessionConfig {
    SessionConfig {
        budget: ExecutionBudget::unlimited(),
        host_bindings: HostApiSurface::default(),
        host_handlers: BTreeMap::new(),
        artifact_kinds: [ArtifactKind::ActionRecipe, ArtifactKind::WorkflowDefinition]
            .into_iter()
            .collect(),
        source: src,
        resume: None,
        placement_hint: None,
        inputs: BTreeMap::new(),
    }
}

fn main() {
    println!("== RF-SCRIPT-01 coexist PoC demo ==");
    println!("poc_contract_version = {POC_CONTRACT_VERSION}");

    // 1. 単一 registry に Luau+Rhai を同時登録（排他 Feature ではない）。
    let mut registry = ScriptBackendRegistry::new();
    registry
        .register(Box::new(LuauBackend::new()))
        .expect("register luau");
    registry
        .register(Box::new(RhaiBackend::new()))
        .expect("register rhai");
    println!("registered backends: {}", registry.len());
    for b in registry.list() {
        let d = b.descriptor();
        println!(
            "  backend={} language={} contract_v{} caps={}",
            d.id,
            d.language,
            d.contract_version,
            d.capabilities.capability_ids().len()
        );
    }

    // 2. language requirement による実行時明示選択 + 交互評価。
    for round in 0..2 {
        for lang in ["luau", "rhai"] {
            let req = ScriptRequirement {
                language: Some(ScriptLanguageId::new(lang)),
                ..Default::default()
            };
            let backend = registry.resolve(&req).expect("resolve");
            let src = if lang == "luau" {
                source(luau_action_recipe(), "luau")
            } else {
                source(rhai_action_recipe(), "rhai")
            };
            let mut session = backend
                .open_session(&session_config(src))
                .expect("open_session");
            let result = session.evaluate(None).expect("evaluate");
            let artifact = result.artifact.expect("artifact");
            println!(
                "round={} lang={} backend={} artifact_kind={:?} digest={}",
                round, lang, artifact.produced_by.backend_id, artifact.kind, artifact.digest
            );
            session.close();
        }
    }

    // 3. 同一意味 artifact の digest 一致を確認。
    let digests: BTreeSet<String> = ["luau", "rhai"]
        .iter()
        .map(|lang| {
            let req = ScriptRequirement {
                language: Some(ScriptLanguageId::new(*lang)),
                ..Default::default()
            };
            let backend = registry.resolve(&req).expect("resolve");
            let src = if *lang == "luau" {
                source(luau_action_recipe(), "luau")
            } else {
                source(rhai_action_recipe(), "rhai")
            };
            let mut session = backend.open_session(&session_config(src)).expect("open");
            let r = session.evaluate(None).expect("eval");
            r.artifact.expect("artifact").digest.0
        })
        .collect();
    println!("distinct digests across backends: {}", digests.len());
    assert_eq!(digests.len(), 1, "same-meaning artifacts must share digest");
    println!("SAME_DIGEST_OK");

    // 4. workflow_definition artifact も両言語から同一 digest で出力できる。
    let wf_digests: BTreeSet<String> = ["luau", "rhai"]
        .iter()
        .map(|lang| {
            let req = ScriptRequirement {
                language: Some(ScriptLanguageId::new(*lang)),
                ..Default::default()
            };
            let backend = registry.resolve(&req).expect("resolve");
            let src = if *lang == "luau" {
                source(luau_workflow(), "luau")
            } else {
                source(rhai_workflow(), "rhai")
            };
            let mut session = backend.open_session(&session_config(src)).expect("open");
            let r = session.evaluate(None).expect("eval");
            let a = r.artifact.expect("artifact");
            assert_eq!(a.kind, ArtifactKind::WorkflowDefinition);
            a.digest.0
        })
        .collect();
    println!(
        "distinct workflow digests across backends: {}",
        wf_digests.len()
    );
    assert_eq!(wf_digests.len(), 1);
    println!("WORKFLOW_SAME_DIGEST_OK");

    // 5. Fail-Closed 実演: 未登録 language は registry resolve で拒否。
    let err = registry
        .resolve(&ScriptRequirement {
            language: Some(ScriptLanguageId::new("python")),
            ..Default::default()
        })
        .map(|_| ())
        .unwrap_err();
    println!("fail-closed unknown language -> {err}");

    println!("demo complete");
}
