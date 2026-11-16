//! Analyzer registry and analyze-contract tests (test-spec R2/R5/E5).

mod common;

use common::{FakeAnalyzer, analyze_request};
use ruviform_script::prelude::*;

fn luau_requirement() -> AnalyzerRequirement {
    AnalyzerRequirement {
        language: ScriptLanguageId::new("luau"),
        ..AnalyzerRequirement::default()
    }
}

#[test]
fn analyzer_register_rejects_duplicate_id() {
    let mut registry = AnalyzerRegistry::new();
    registry
        .register(Box::new(FakeAnalyzer::new("luau", "fake-analyzer")))
        .expect("first");
    let err = registry
        .register(Box::new(FakeAnalyzer::new("rhai", "fake-analyzer")))
        .expect_err("duplicate");
    assert!(matches!(
        err,
        AnalyzerRegistryError::DuplicateAnalyzer { .. }
    ));
}

#[test]
fn analyzer_resolve_returns_unique_match() {
    let mut registry = AnalyzerRegistry::new();
    registry
        .register(Box::new(FakeAnalyzer::new("luau", "fake-analyzer")))
        .expect("register");
    registry
        .register(Box::new(FakeAnalyzer::new("rhai", "fake-rhai-analyzer")))
        .expect("register");
    let analyzer = registry.resolve(&luau_requirement()).expect("resolve");
    assert_eq!(analyzer.descriptor().analyzer_id.as_str(), "fake-analyzer");
}

#[test]
fn analyzer_resolve_fail_closed_matrix() {
    let mut registry = AnalyzerRegistry::new();
    registry
        .register(Box::new(FakeAnalyzer::new("luau", "fake-analyzer")))
        .expect("register");

    // Unknown language.
    let req = AnalyzerRequirement {
        language: ScriptLanguageId::new("cobol"),
        ..AnalyzerRequirement::default()
    };
    assert!(matches!(
        registry.resolve(&req),
        Err(AnalyzerResolveError::NoMatchingAnalyzer {
            reason: AnalyzerResolveReason::UnknownLanguage,
            ..
        })
    ));

    // Unknown analyzer hint.
    let req = AnalyzerRequirement {
        analyzer: Some(AnalyzerId::new("nope")),
        ..luau_requirement()
    };
    assert!(matches!(
        registry.resolve(&req),
        Err(AnalyzerResolveError::NoMatchingAnalyzer {
            reason: AnalyzerResolveReason::UnknownAnalyzer,
            ..
        })
    ));

    // Incompatible contract version.
    let mut registry2 = AnalyzerRegistry::new();
    registry2
        .register(Box::new(
            FakeAnalyzer::new("luau", "fake-analyzer").with_contract_version(42),
        ))
        .expect("register");
    assert!(matches!(
        registry2.resolve(&luau_requirement()),
        Err(AnalyzerResolveError::NoMatchingAnalyzer {
            reason: AnalyzerResolveReason::UnsupportedContractVersion,
            ..
        })
    ));

    // Missing capability (fake analyzer advertises diagnostics+types+
    // completion; an unknown capability ID is not covered).
    let req_missing = AnalyzerRequirement {
        required_capabilities: [CapabilityId::new("analyze.future_feature")]
            .into_iter()
            .collect(),
        ..luau_requirement()
    };
    assert!(matches!(
        registry.resolve(&req_missing),
        Err(AnalyzerResolveError::NoMatchingAnalyzer {
            reason: AnalyzerResolveReason::MissingCapability,
            ..
        })
    ));

    // Ambiguity.
    let mut registry3 = AnalyzerRegistry::new();
    registry3
        .register(Box::new(FakeAnalyzer::new("luau", "fake-a")))
        .expect("register");
    registry3
        .register(Box::new(FakeAnalyzer::new("luau", "fake-b")))
        .expect("register");
    assert!(matches!(
        registry3.resolve(&luau_requirement()),
        Err(AnalyzerResolveError::AmbiguousAnalyzer { .. })
    ));
}

#[test]
fn analyzer_registry_analyze_happy_path() {
    let mut registry = AnalyzerRegistry::new();
    registry
        .register(Box::new(FakeAnalyzer::new("luau", "fake-analyzer")))
        .expect("register");
    let request = analyze_request("luau", "local x = 1");
    let result = registry
        .analyze(&luau_requirement(), &request)
        .expect("analyze");
    assert_eq!(result.source_revision, request.source.revision);
}

#[test]
fn analyzer_registry_analyze_rejects_stale_result() {
    let mut registry = AnalyzerRegistry::new();
    let mut stale_analyzer = FakeAnalyzer::new("luau", "fake-analyzer");
    stale_analyzer.echo_revision = false;
    registry
        .register(Box::new(stale_analyzer))
        .expect("register");
    let request = analyze_request("luau", "local x = 1");
    let err = registry
        .analyze(&luau_requirement(), &request)
        .expect_err("stale must fail");
    assert!(matches!(err, AnalyzerError::StaleResult));
}

#[test]
fn analyzer_registry_analyze_unavailable_when_empty() {
    let registry = AnalyzerRegistry::new();
    let request = analyze_request("luau", "x");
    let err = registry
        .analyze(&luau_requirement(), &request)
        .expect_err("empty registry");
    assert!(matches!(err, AnalyzerError::Unavailable { .. }));
}

#[test]
fn analyzer_registry_analyze_rejects_unwanted_capability() {
    let mut registry = AnalyzerRegistry::new();
    // Analyzer without completion support.
    registry
        .register(Box::new(
            FakeAnalyzer::new("luau", "fake-analyzer").with_capabilities(AnalyzerCapabilities {
                diagnostics: DiagnosticLevel::Syntax,
                completion: false,
                max_source_bytes: None,
                supported_languages: vec![ScriptLanguageId::new("luau")],
            }),
        ))
        .expect("register");
    let request = AnalyzeRequest {
        wanted: WantedCapabilities {
            diagnostics: DiagnosticLevel::Syntax,
            completion: true,
        },
        ..analyze_request("luau", "x")
    };
    let err = registry
        .analyze(&luau_requirement(), &request)
        .expect_err("completion unsupported");
    assert!(matches!(err, AnalyzerError::Unsupported { .. }));
}

#[test]
fn verify_result_revision_detects_stale() {
    let request = analyze_request("luau", "x");
    let fresh = AnalyzeResult {
        source_revision: request.source.revision,
        ..Default::default()
    };
    assert!(verify_result_revision(&request, &fresh).is_ok());

    let stale = AnalyzeResult {
        source_revision: SourceRevision::new(999),
        ..Default::default()
    };
    assert!(matches!(
        verify_result_revision(&request, &stale),
        Err(AnalyzerError::StaleResult)
    ));
}

#[test]
fn fake_analyzer_analyze_is_immutable_borrow() {
    // `analyze(&self)` — callable through a shared reference, proving
    // the trait does not require `&mut self`.
    let analyzer = FakeAnalyzer::new("luau", "fake-analyzer");
    let request = analyze_request("luau", "x");
    let result = ScriptAnalyzer::analyze(&analyzer, &request).expect("analyze");
    assert_eq!(result.source_revision, request.source.revision);
}
