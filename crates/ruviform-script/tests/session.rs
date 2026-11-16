//! Session contract tests: `open_session` fail-closed paths, evaluate,
//! cooperative cancel, idempotent close (test-spec R7/E7).

mod common;

use std::time::Duration;

use common::{FakeBackend, expect_open_err, session_config};
use ruviform_script::prelude::*;

fn backend() -> FakeBackend {
    FakeBackend::new("luau", "fake-luau")
}

#[test]
fn open_session_happy_path_and_evaluate() {
    let backend = backend();
    let mut session = backend
        .open_session(session_config("luau", "return 1 + 1"))
        .expect("open");
    assert_eq!(session.backend_id().as_str(), "fake-luau");
    let result = session.evaluate(None).expect("evaluate");
    assert_eq!(result, ScriptResult::default());
    session.close();
    session.close(); // idempotent
}

#[test]
fn open_session_language_mismatch_fails() {
    let backend = backend();
    let err = expect_open_err(backend.open_session(session_config("rhai", "x")));
    assert!(matches!(err, SessionError::LanguageMismatch { .. }));
}

#[test]
fn open_session_unsupported_budget_fails_closed() {
    let backend = backend(); // supports no budget dimensions
    let mut config = session_config("luau", "x");
    config.budget = ExecutionBudget {
        max_instructions: Some(1000),
        ..ExecutionBudget::unlimited()
    };
    let err = expect_open_err(backend.open_session(config));
    assert!(matches!(
        err,
        SessionError::UnsupportedBudget {
            kind: BudgetKind::Instructions
        }
    ));
}

#[test]
fn open_session_supported_budget_passes() {
    let backend =
        FakeBackend::new("luau", "fake-luau").with_supported_budgets(&[BudgetKind::Instructions]);
    let mut config = session_config("luau", "x");
    config.budget = ExecutionBudget {
        max_instructions: Some(1000),
        ..ExecutionBudget::unlimited()
    };
    assert!(backend.open_session(config).is_ok());
}

#[test]
fn open_session_source_too_large_fails() {
    let mut fake = backend();
    fake.max_source_bytes = Some(4);
    let err = expect_open_err(fake.open_session(session_config("luau", "way too long")));
    assert!(matches!(err, SessionError::SourceTooLarge { .. }));
}

#[test]
fn open_session_invalid_host_surface_fails() {
    let backend = backend();
    let mut config = session_config("luau", "x");
    config.host_bindings = HostApiSurface {
        bindings: vec![HostBinding {
            name: "os.exec".to_string(),
            signature: HostSignature {
                args: vec![],
                returns: ValueKind::Any,
            },
            effect: HostEffect::Effect,
            schema_version: HOST_SURFACE_SCHEMA_VERSION,
            timeout_hint: Some(Duration::from_secs(1)),
        }],
        schema_version: HOST_SURFACE_SCHEMA_VERSION,
    };
    let err = expect_open_err(backend.open_session(config));
    assert!(matches!(err, SessionError::InvalidHostSurface { .. }));
}

#[test]
fn open_session_resume_digest_mismatch_fails() {
    let backend = backend();
    let mut config = session_config("luau", "x");
    config.resume = Some(source_digest("other text"));
    let err = expect_open_err(backend.open_session(config));
    assert!(matches!(err, SessionError::SourceDigestMismatch { .. }));
}

#[test]
fn open_session_resume_digest_match_passes() {
    let backend = backend();
    let mut config = session_config("luau", "x");
    config.resume = Some(config.source.digest.clone());
    assert!(backend.open_session(config).is_ok());
}

#[test]
fn cancel_is_cooperative_and_idempotent() {
    let backend = backend();
    let mut session = backend
        .open_session(session_config("luau", "x"))
        .expect("open");
    let token = session.cancel_token();
    assert!(!token.is_cancelled());
    session.cancel();
    session.cancel(); // idempotent
    assert!(token.is_cancelled());
    token.cancel(); // consumer-side cancel via clone also works
    let err = session.evaluate(None).expect_err("cancelled");
    assert_eq!(err.kind, ScriptErrorKind::Cancelled);
}

#[test]
fn evaluate_returns_configured_result() {
    let mut backend = backend();
    backend.eval_result = ScriptResult {
        return_value: Some(ScriptValue::Integer(42)),
        ..ScriptResult::default()
    };
    let mut session = backend
        .open_session(session_config("luau", "x"))
        .expect("open");
    let result = session.evaluate(Some("main")).expect("evaluate");
    assert_eq!(result.return_value, Some(ScriptValue::Integer(42)));
}

#[test]
fn evaluate_returns_configured_error() {
    let mut backend = backend();
    backend.eval_error = Some(ScriptError::new(ScriptErrorKind::Runtime, "boom"));
    let mut session = backend
        .open_session(session_config("luau", "x"))
        .expect("open");
    let err = session.evaluate(None).expect_err("eval error");
    assert_eq!(err.kind, ScriptErrorKind::Runtime);
}

#[test]
fn session_ids_are_unique() {
    let backend = backend();
    let s1 = backend
        .open_session(session_config("luau", "a"))
        .expect("open");
    let s2 = backend
        .open_session(session_config("luau", "b"))
        .expect("open");
    assert_ne!(s1.session_id(), s2.session_id());
}

#[test]
fn session_source_language_is_carried() {
    // The configured source language flows into open_session; a
    // mismatch is rejected (covered above) and a match proceeds.
    let backend = backend();
    let session = backend
        .open_session(session_config("luau", "x"))
        .expect("open");
    assert_eq!(session.backend_id().as_str(), "fake-luau");
}
