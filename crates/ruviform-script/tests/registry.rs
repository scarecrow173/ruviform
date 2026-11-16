//! Backend registry fail-closed resolution matrix (test-spec R1/E1/E2).

mod common;

use common::{FakeBackend, expect_resolve_err};
use ruviform_script::prelude::*;

fn luau_requirement() -> ScriptRequirement {
    ScriptRequirement {
        language: ScriptLanguageId::new("luau"),
        ..ScriptRequirement::default()
    }
}

#[test]
fn register_rejects_duplicate_backend_id() {
    let mut registry = ScriptBackendRegistry::new();
    registry
        .register(Box::new(FakeBackend::new("luau", "fake-luau")))
        .expect("first register");
    let err = registry
        .register(Box::new(FakeBackend::new("rhai", "fake-luau")))
        .expect_err("duplicate id must fail");
    assert!(matches!(err, RegistryError::DuplicateBackend { .. }));
}

#[test]
fn resolve_returns_unique_match() {
    let mut registry = ScriptBackendRegistry::new();
    registry
        .register(Box::new(FakeBackend::new("luau", "fake-luau")))
        .expect("register");
    registry
        .register(Box::new(FakeBackend::new("rhai", "fake-rhai")))
        .expect("register");

    let backend = registry
        .resolve(&luau_requirement())
        .expect("unique luau backend");
    assert_eq!(backend.descriptor().backend_id.as_str(), "fake-luau");

    // Backend hint narrows within the language.
    let req = ScriptRequirement {
        backend: Some(ScriptBackendId::new("fake-luau")),
        ..luau_requirement()
    };
    assert!(registry.resolve(&req).is_ok());
}

#[test]
fn resolve_unknown_language_fails_closed() {
    let mut registry = ScriptBackendRegistry::new();
    registry
        .register(Box::new(FakeBackend::new("luau", "fake-luau")))
        .expect("register");
    let req = ScriptRequirement {
        language: ScriptLanguageId::new("cobol"),
        ..ScriptRequirement::default()
    };
    let err = expect_resolve_err(registry.resolve(&req));
    assert!(matches!(
        err,
        ResolveError::NoMatchingBackend {
            reason: ResolveErrorReason::UnknownLanguage,
            ..
        }
    ));
}

#[test]
fn resolve_unknown_backend_hint_fails_closed() {
    let mut registry = ScriptBackendRegistry::new();
    registry
        .register(Box::new(FakeBackend::new("luau", "fake-luau")))
        .expect("register");
    let req = ScriptRequirement {
        backend: Some(ScriptBackendId::new("nope")),
        ..luau_requirement()
    };
    let err = expect_resolve_err(registry.resolve(&req));
    assert!(matches!(
        err,
        ResolveError::NoMatchingBackend {
            reason: ResolveErrorReason::UnknownBackend,
            ..
        }
    ));
}

#[test]
fn resolve_empty_registry_is_unknown_language() {
    let registry = ScriptBackendRegistry::new();
    let err = expect_resolve_err(registry.resolve(&luau_requirement()));
    assert!(matches!(
        err,
        ResolveError::NoMatchingBackend {
            reason: ResolveErrorReason::UnknownLanguage,
            ..
        }
    ));
}

#[test]
fn resolve_contradictory_hint_is_no_candidates() {
    let mut registry = ScriptBackendRegistry::new();
    // Known backend exists but implements a different language.
    registry
        .register(Box::new(FakeBackend::new("rhai", "fake-rhai")))
        .expect("register");
    let req = ScriptRequirement {
        language: ScriptLanguageId::new("luau"),
        backend: Some(ScriptBackendId::new("fake-rhai")),
        ..ScriptRequirement::default()
    };
    // language "luau" is unknown here → UnknownLanguage first.
    let err = expect_resolve_err(registry.resolve(&req));
    assert!(matches!(
        err,
        ResolveError::NoMatchingBackend {
            reason: ResolveErrorReason::UnknownLanguage,
            ..
        }
    ));

    // Now register a luau backend too: hint fake-rhai + language luau
    // → both known, but no backend satisfies both → NoCandidates.
    registry
        .register(Box::new(FakeBackend::new("luau", "fake-luau")))
        .expect("register");
    let err = expect_resolve_err(registry.resolve(&req));
    assert!(matches!(
        err,
        ResolveError::NoMatchingBackend {
            reason: ResolveErrorReason::NoCandidates,
            ..
        }
    ));
}

#[test]
fn resolve_incompatible_contract_version_fails_closed() {
    let mut registry = ScriptBackendRegistry::new();
    registry
        .register(Box::new(
            FakeBackend::new("luau", "fake-luau").with_contract_version(99),
        ))
        .expect("register");
    let err = expect_resolve_err(registry.resolve(&luau_requirement()));
    assert!(matches!(
        err,
        ResolveError::NoMatchingBackend {
            reason: ResolveErrorReason::UnsupportedContractVersion,
            ..
        }
    ));
}

#[test]
fn resolve_min_contract_version_filters() {
    let mut registry = ScriptBackendRegistry::new();
    registry
        .register(Box::new(FakeBackend::new("luau", "fake-luau")))
        .expect("register");
    let req = ScriptRequirement {
        min_contract_version: u32::MAX,
        ..luau_requirement()
    };
    let err = expect_resolve_err(registry.resolve(&req));
    assert!(matches!(
        err,
        ResolveError::NoMatchingBackend {
            reason: ResolveErrorReason::UnsupportedContractVersion,
            ..
        }
    ));
}

#[test]
fn resolve_missing_capability_fails_closed() {
    let mut registry = ScriptBackendRegistry::new();
    registry
        .register(Box::new(FakeBackend::new("luau", "fake-luau")))
        .expect("register");
    let req = ScriptRequirement {
        required_capabilities: [CapabilityId::new("artifact.action_recipe")]
            .into_iter()
            .collect(),
        ..luau_requirement()
    };
    let err = expect_resolve_err(registry.resolve(&req));
    assert!(matches!(
        err,
        ResolveError::NoMatchingBackend {
            reason: ResolveErrorReason::MissingCapability,
            ..
        }
    ));
}

#[test]
fn resolve_missing_artifact_kind_fails_closed() {
    let mut registry = ScriptBackendRegistry::new();
    registry
        .register(Box::new(FakeBackend::new("luau", "fake-luau")))
        .expect("register");
    let req = ScriptRequirement {
        artifact_kinds: [ArtifactKind::ActionRecipe].into_iter().collect(),
        ..luau_requirement()
    };
    let err = expect_resolve_err(registry.resolve(&req));
    assert!(matches!(
        err,
        ResolveError::NoMatchingBackend {
            reason: ResolveErrorReason::MissingCapability,
            ..
        }
    ));
}

#[test]
fn resolve_ambiguous_candidates_fail_closed() {
    let mut registry = ScriptBackendRegistry::new();
    registry
        .register(Box::new(FakeBackend::new("luau", "fake-luau-a")))
        .expect("register");
    registry
        .register(Box::new(FakeBackend::new("luau", "fake-luau-b")))
        .expect("register");
    let err = expect_resolve_err(registry.resolve(&luau_requirement()));
    match err {
        ResolveError::AmbiguousBackend { candidates } => {
            assert_eq!(candidates.len(), 2);
            assert!(candidates.contains(&ScriptBackendId::new("fake-luau-a")));
            assert!(candidates.contains(&ScriptBackendId::new("fake-luau-b")));
        }
        other => panic!("expected AmbiguousBackend, got {other:?}"),
    }
}

#[test]
fn resolve_capability_and_artifact_coverage_selects_match() {
    let mut registry = ScriptBackendRegistry::new();
    // Backend A: lacks the artifact kind. Backend B: has it.
    registry
        .register(Box::new(FakeBackend::new("luau", "fake-luau-a")))
        .expect("register");
    registry
        .register(Box::new(
            FakeBackend::new("luau", "fake-luau-b")
                .with_artifact_kinds(&[ArtifactKind::ActionRecipe])
                .with_capabilities(&["session.evaluate", "artifact.action_recipe"]),
        ))
        .expect("register");
    let req = ScriptRequirement {
        required_capabilities: [CapabilityId::new("artifact.action_recipe")]
            .into_iter()
            .collect(),
        artifact_kinds: [ArtifactKind::ActionRecipe].into_iter().collect(),
        ..luau_requirement()
    };
    let backend = registry.resolve(&req).expect("unique capable backend");
    assert_eq!(backend.descriptor().backend_id.as_str(), "fake-luau-b");
}

#[test]
fn registry_starts_empty_and_lists_registered() {
    let mut registry = ScriptBackendRegistry::new();
    assert!(registry.is_empty());
    registry
        .register(Box::new(FakeBackend::new("luau", "fake-luau")))
        .expect("register");
    assert_eq!(registry.len(), 1);
    assert_eq!(registry.list().len(), 1);
}
