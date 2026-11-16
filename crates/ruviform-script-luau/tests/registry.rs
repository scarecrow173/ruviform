//! `AnalyzerRegistry` end-to-end — 実 `LuauAnalyzer` と stub
//! `ScriptAnalyzer` を使い、中立契約の fail-closed 解決行列を検証する。
//! （registry 自体の網羅行列は `crates/ruviform-script` の責務 —
//! ここでは実 backend 統合でのみ観測できる経路を検証する。）

use std::collections::BTreeSet;

use ruviform_script::{
    AnalyzeRequest, AnalyzeResult, AnalyzerCapabilities, AnalyzerDescriptor, AnalyzerError,
    AnalyzerId, AnalyzerRegistry, AnalyzerRegistryError, AnalyzerRequirement, AnalyzerResolveError,
    AnalyzerResolveReason, CONTRACT_VERSION, DiagnosticLevel, ScriptAnalyzer, ScriptBackendVersion,
    ScriptLanguageId, ScriptSource, SourcePosition, SourceRevision, WantedCapabilities,
};
use ruviform_script_luau::{LuauAnalyzer, LuauAnalyzerOptions, luau_language};

fn luau() -> ScriptLanguageId {
    luau_language()
}

fn request(text: &str, rev: u64) -> AnalyzeRequest {
    AnalyzeRequest {
        source: ScriptSource::new(text, luau(), None, SourceRevision::new(rev)),
        language: luau(),
        type_definitions: None,
        wanted: WantedCapabilities {
            diagnostics: DiagnosticLevel::SyntaxAndType,
            completion: false,
        },
        cursor: None,
    }
}

fn requirement() -> AnalyzerRequirement {
    AnalyzerRequirement {
        language: luau(),
        analyzer: None,
        required_capabilities: BTreeSet::new(),
        min_contract_version: CONTRACT_VERSION,
    }
}

fn connect(opts: &LuauAnalyzerOptions) -> LuauAnalyzer {
    LuauAnalyzer::connect(opts).expect("analyzer connect")
}

/// fail-closed 行列用の最小 stub。descriptor を差し替えて
/// `contract_version` / revision / 言語の各ゲートを検証する。
struct StubAnalyzer {
    descriptor: AnalyzerDescriptor,
    result_revision: SourceRevision,
}

impl StubAnalyzer {
    fn new(analyzer_id: &str, language: ScriptLanguageId, contract_version: u32) -> Self {
        Self {
            descriptor: AnalyzerDescriptor {
                analyzer_id: AnalyzerId::new(analyzer_id),
                language,
                version: ScriptBackendVersion::new("stub/0.0.0", contract_version),
                capabilities: AnalyzerCapabilities {
                    diagnostics: DiagnosticLevel::SyntaxAndType,
                    completion: true,
                    max_source_bytes: None,
                    supported_languages: vec![luau()],
                },
            },
            result_revision: SourceRevision::new(0),
        }
    }

    fn with_revision(mut self, revision: SourceRevision) -> Self {
        self.result_revision = revision;
        self
    }
}

impl ScriptAnalyzer for StubAnalyzer {
    fn descriptor(&self) -> &AnalyzerDescriptor {
        &self.descriptor
    }

    fn analyze(&self, _request: &AnalyzeRequest) -> Result<AnalyzeResult, AnalyzerError> {
        Ok(AnalyzeResult {
            source_revision: self.result_revision,
            diagnostics: vec![],
            completions: vec![],
        })
    }
}

#[test]
fn empty_registry_fails_closed() {
    let registry = AnalyzerRegistry::new();
    assert!(registry.is_empty());

    // analyze convenience: resolve 失敗は Unavailable{reason} に写像。
    let err = registry
        .analyze(&requirement(), &request("return 1", 1))
        .expect_err("empty registry must fail closed");
    assert!(
        matches!(err, AnalyzerError::Unavailable { .. }),
        "got {err:?}"
    );

    // resolve は型付き reason を保持する。
    let err = registry
        .resolve(&requirement())
        .map(|_| ())
        .expect_err("resolve");
    assert!(
        matches!(
            err,
            AnalyzerResolveError::NoMatchingAnalyzer {
                reason: AnalyzerResolveReason::UnknownLanguage,
                ..
            }
        ),
        "got {err:?}"
    );
}

#[test]
fn register_and_analyze_end_to_end() {
    let mut registry = AnalyzerRegistry::new();
    let analyzer = connect(&LuauAnalyzerOptions::default());
    registry.register(Box::new(analyzer)).expect("register");
    assert_eq!(registry.len(), 1);
    assert!(!registry.is_empty());

    let resolved = registry.resolve(&requirement()).expect("resolve");
    assert_eq!(resolved.descriptor().analyzer_id.as_str(), "luau-analysis");

    let result = registry
        .analyze(
            &requirement(),
            &request("local x: number = 1\nreturn x\n", 7),
        )
        .expect("registry analyze");
    assert_eq!(result.source_revision, SourceRevision::new(7));
}

#[test]
fn duplicate_analyzer_id_rejected() {
    let mut registry = AnalyzerRegistry::new();
    registry
        .register(Box::new(connect(&LuauAnalyzerOptions::default())))
        .expect("register first");
    let err = registry
        .register(Box::new(connect(&LuauAnalyzerOptions::default())))
        .expect_err("duplicate analyzer id must fail");
    assert!(
        matches!(err, AnalyzerRegistryError::DuplicateAnalyzer { .. }),
        "got {err:?}"
    );
}

#[test]
fn multiple_instances_require_analyzer_hint() {
    let mut registry = AnalyzerRegistry::new();
    registry
        .register(Box::new(
            LuauAnalyzer::connect_with_analyzer_id(
                &LuauAnalyzerOptions::default(),
                AnalyzerId::new("luau-instance-a"),
            )
            .expect("connect a"),
        ))
        .expect("register a");
    registry
        .register(Box::new(
            LuauAnalyzer::connect_with_analyzer_id(
                &LuauAnalyzerOptions::default(),
                AnalyzerId::new("luau-instance-b"),
            )
            .expect("connect b"),
        ))
        .expect("register b");

    // hint なし → 同一言語の複数候補は AmbiguousAnalyzer（fail-closed）。
    let err = registry
        .resolve(&requirement())
        .map(|_| ())
        .expect_err("ambiguous");
    assert!(
        matches!(err, AnalyzerResolveError::AmbiguousAnalyzer { .. }),
        "got {err:?}"
    );

    // registry.analyze 経由では Unavailable{reason} に写像される。
    let err = registry
        .analyze(&requirement(), &request("local x = 1", 3))
        .expect_err("ambiguous must fail closed");
    assert!(
        matches!(&err, AnalyzerError::Unavailable { reason } if reason.contains("ambiguous")),
        "got {err:?}"
    );

    // hint 指定 → 一意に解決して解析できる。
    let mut hinted = requirement();
    hinted.analyzer = Some(AnalyzerId::new("luau-instance-a"));
    let resolved = registry.resolve(&hinted).expect("resolve with hint");
    assert_eq!(
        resolved.descriptor().analyzer_id.as_str(),
        "luau-instance-a"
    );
    let result = registry
        .analyze(&hinted, &request("local x = 1", 3))
        .expect("analyze with hint");
    assert_eq!(result.source_revision, SourceRevision::new(3));
}

#[test]
fn unknown_language_and_analyzer_hint_fail_closed() {
    let mut registry = AnalyzerRegistry::new();
    registry
        .register(Box::new(connect(&LuauAnalyzerOptions::default())))
        .expect("register");

    // 未登録言語
    let mut req = requirement();
    req.language = ScriptLanguageId::new("rhai");
    let err = registry
        .resolve(&req)
        .map(|_| ())
        .expect_err("unknown language");
    assert!(
        matches!(
            err,
            AnalyzerResolveError::NoMatchingAnalyzer {
                reason: AnalyzerResolveReason::UnknownLanguage,
                ..
            }
        ),
        "got {err:?}"
    );

    // 未登録 analyzer hint
    let mut req = requirement();
    req.analyzer = Some(AnalyzerId::new("nonexistent"));
    let err = registry
        .resolve(&req)
        .map(|_| ())
        .expect_err("unknown analyzer");
    assert!(
        matches!(
            err,
            AnalyzerResolveError::NoMatchingAnalyzer {
                reason: AnalyzerResolveReason::UnknownAnalyzer,
                ..
            }
        ),
        "got {err:?}"
    );
}

#[test]
fn contract_version_gate_fails_closed() {
    // stub が contract_version=0 を広告 → UnsupportedContractVersion。
    let mut registry = AnalyzerRegistry::new();
    registry
        .register(Box::new(StubAnalyzer::new("stub-old-contract", luau(), 0)))
        .expect("register stub");
    let err = registry
        .resolve(&requirement())
        .map(|_| ())
        .expect_err("old contract");
    assert!(
        matches!(
            err,
            AnalyzerResolveError::NoMatchingAnalyzer {
                reason: AnalyzerResolveReason::UnsupportedContractVersion,
                ..
            }
        ),
        "got {err:?}"
    );

    // 実 analyzer (contract_version=1) に min_contract_version=2 を要求
    // → UnsupportedContractVersion。
    let mut registry = AnalyzerRegistry::new();
    registry
        .register(Box::new(connect(&LuauAnalyzerOptions::default())))
        .expect("register");
    let mut req = requirement();
    req.min_contract_version = CONTRACT_VERSION + 1;
    let err = registry
        .resolve(&req)
        .map(|_| ())
        .expect_err("min contract too high");
    assert!(
        matches!(
            err,
            AnalyzerResolveError::NoMatchingAnalyzer {
                reason: AnalyzerResolveReason::UnsupportedContractVersion,
                ..
            }
        ),
        "got {err:?}"
    );
}

#[test]
fn capability_gate_fails_closed() {
    // enable_completion=false の実 analyzer に analyze.completion を要求
    // → MissingCapability。
    let mut registry = AnalyzerRegistry::new();
    registry
        .register(Box::new(connect(&LuauAnalyzerOptions::default())))
        .expect("register");
    let mut req = requirement();
    req.required_capabilities
        .insert(ruviform_script::CapabilityId::new("analyze.completion"));
    let err = registry
        .resolve(&req)
        .map(|_| ())
        .expect_err("missing capability");
    assert!(
        matches!(
            err,
            AnalyzerResolveError::NoMatchingAnalyzer {
                reason: AnalyzerResolveReason::MissingCapability,
                ..
            }
        ),
        "got {err:?}"
    );

    // enable_completion=true の analyzer なら解決できる。
    let mut registry = AnalyzerRegistry::new();
    registry
        .register(Box::new(connect(&LuauAnalyzerOptions {
            enable_completion: true,
            ..LuauAnalyzerOptions::default()
        })))
        .expect("register");
    registry.resolve(&req).expect("resolve with capability");
}

#[test]
fn wanted_capability_beyond_descriptor_is_unsupported() {
    // registry.analyze は wanted ⊆ caps を analyzer 呼出し前に検査する。
    let mut registry = AnalyzerRegistry::new();
    registry
        .register(Box::new(connect(&LuauAnalyzerOptions::default())))
        .expect("register");
    let mut req = request("local x = 1", 5);
    req.wanted.completion = true;
    req.cursor = Some(SourcePosition {
        offset: 5,
        line: 0,
        col: 5,
    });
    let err = registry
        .analyze(&requirement(), &req)
        .expect_err("completion beyond descriptor");
    assert!(
        matches!(&err, AnalyzerError::Unsupported { capability } if capability.as_str() == "analyze.completion"),
        "got {err:?}"
    );
}

#[test]
fn stale_result_rejected_by_registry() {
    // stub が request と異なる source_revision を返す → StaleResult
    // （verify_result_revision 経路 — F-14）。
    let mut registry = AnalyzerRegistry::new();
    registry
        .register(Box::new(
            StubAnalyzer::new("stub-stale", luau(), CONTRACT_VERSION)
                .with_revision(SourceRevision::new(999)),
        ))
        .expect("register stub");
    let err = registry
        .analyze(&requirement(), &request("return 1", 1))
        .expect_err("stale result must fail");
    assert_eq!(err, AnalyzerError::StaleResult);
}
