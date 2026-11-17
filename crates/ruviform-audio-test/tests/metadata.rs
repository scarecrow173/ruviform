//! Run metadata / Case Schema v1 vocabulary tests (Issue #628).

use ruviform_audio_test::clock::{JitterSpec, VirtualClockConfig};
use ruviform_audio_test::metadata::{
    CaseRequirement, CaseResult, EnvironmentKind, RunMetadata, RunParameters, VerificationLevel,
};

fn params() -> RunParameters {
    RunParameters {
        seed: 0x00C0_FFEE,
        fixture_version: 1,
        clock_config: VirtualClockConfig {
            sample_rate: 48_000,
            epoch: 1,
            offset_ticks: 0,
            drift_ppm: -250,
            jitter: JitterSpec {
                seed: 9,
                period_frames: 128,
                amplitude_ticks: 2,
            },
        },
    }
}

#[test]
fn deterministic_constructor_uses_case_schema_v1_defaults() {
    let m = RunMetadata::deterministic(
        "rf628.pcm.sine.48k",
        "oracle::pcm_reference_block",
        params(),
        "scarecrow173/ruviform",
        "deadbeef",
    );
    assert_eq!(m.case_id, "rf628.pcm.sine.48k");
    assert_eq!(m.requirement, CaseRequirement::Required);
    assert_eq!(m.result, CaseResult::NotRun); // honest default
    assert_eq!(m.level, VerificationLevel::OsL1);
    assert_eq!(m.environment_kind, EnvironmentKind::DeterministicBackend);
    assert_eq!(m.reproducibility_key(), (0x00C0_FFEE, 1));
}

#[test]
fn builder_records_result_and_evidence() {
    let m = RunMetadata::deterministic("c", "o", params(), "r", "s")
        .with_result(CaseResult::Pass, "max_abs_diff=0")
        .with_expected("sha256:abc")
        .with_platform("windows", "x86_64")
        .with_evidence("evidence/rf628/pcm-sine.log");
    assert_eq!(m.result, CaseResult::Pass);
    assert_eq!(m.observed, "max_abs_diff=0");
    assert_eq!(m.expected, "sha256:abc");
    assert_eq!(m.os, "windows");
    assert_eq!(m.arch, "x86_64");
    assert_eq!(m.evidence, ["evidence/rf628/pcm-sine.log"]);
}

#[test]
fn identical_parameters_reproduce_identical_metadata() {
    let a = RunMetadata::deterministic("c", "o", params(), "r", "s");
    let b = RunMetadata::deterministic("c", "o", params(), "r", "s");
    assert_eq!(a, b);
    // clock_config participates in reproducibility
    let mut p2 = params();
    p2.clock_config.drift_ppm = 0;
    let c = RunMetadata::deterministic("c", "o", p2, "r", "s");
    assert_ne!(a.parameters.clock_config, c.parameters.clock_config);
}

// --- JSON serialization (Issue #628 close-out) ---

fn full_metadata() -> RunMetadata {
    RunMetadata::deterministic(
        "rf628.pcm.sine.48k",
        "oracle::pcm_reference_block",
        params(),
        "scarecrow173/ruviform",
        "deadbeef",
    )
    .with_result(CaseResult::Pass, "max_abs_diff=0")
    .with_expected("sha256:abc")
    .with_platform("windows", "x86_64")
    .with_evidence("evidence/rf628/ac-inventory.md")
}

#[test]
fn json_string_round_trip_preserves_all_fields() {
    let m = full_metadata();
    let json = m.to_json().unwrap();
    let back = RunMetadata::from_json(&json).unwrap();
    assert_eq!(m, back);
}

#[test]
fn json_file_save_load_round_trip() {
    let m = full_metadata();
    let path = std::env::temp_dir().join(format!(
        "rf628-metadata-{}-save-load.json",
        std::process::id()
    ));
    m.save_json(&path).unwrap();
    let back = RunMetadata::load_json(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    assert_eq!(m, back);
}

#[test]
fn json_wire_uses_case_schema_v1_kebab_case() {
    let m = full_metadata();
    let v: serde_json::Value = serde_json::from_str(&m.to_json().unwrap()).unwrap();
    // Struct fields stay snake_case; enum values use the kebab-case
    // Case Schema v1 vocabulary.
    assert_eq!(v["case_id"], "rf628.pcm.sine.48k");
    assert_eq!(v["requirement"], "required");
    assert_eq!(v["result"], "pass");
    assert_eq!(v["level"], "os-l1");
    assert_eq!(v["environment_kind"], "deterministic-backend");
    assert_eq!(v["parameters"]["seed"], 0x00C0_FFEEu64);
    assert_eq!(v["parameters"]["fixture_version"], 1);
    assert_eq!(v["parameters"]["clock_config"]["sample_rate"], 48_000);
    assert_eq!(
        v["parameters"]["clock_config"]["jitter"]["period_frames"],
        128
    );
}

#[test]
fn enum_wire_spellings_match_case_schema_v1() {
    use serde_json::Value;

    let cases: &[(Value, &str)] = &[
        (
            serde_json::to_value(CaseRequirement::Required).unwrap(),
            "required",
        ),
        (
            serde_json::to_value(CaseRequirement::Deferred).unwrap(),
            "deferred",
        ),
        (
            serde_json::to_value(CaseRequirement::NotApplicable).unwrap(),
            "not-applicable",
        ),
        (serde_json::to_value(CaseResult::Pass).unwrap(), "pass"),
        (serde_json::to_value(CaseResult::Fail).unwrap(), "fail"),
        (serde_json::to_value(CaseResult::NotRun).unwrap(), "not-run"),
        (
            serde_json::to_value(CaseResult::Unsupported).unwrap(),
            "unsupported",
        ),
        (
            serde_json::to_value(VerificationLevel::OsL1).unwrap(),
            "os-l1",
        ),
        (
            serde_json::to_value(VerificationLevel::GuiL2).unwrap(),
            "gui-l2",
        ),
        (
            serde_json::to_value(VerificationLevel::ExtL3).unwrap(),
            "ext-l3",
        ),
    ];
    for (value, expected) in cases {
        assert_eq!(*value, Value::String((*expected).to_owned()));
    }
}

#[test]
fn environment_kind_serde_matches_as_str() {
    for kind in [
        EnvironmentKind::DeterministicBackend,
        EnvironmentKind::RealAbiTestPlugin,
        EnvironmentKind::RealWindow,
        EnvironmentKind::VirtualOsEndpoint,
        EnvironmentKind::PhysicalDevice,
    ] {
        let json = serde_json::to_string(&kind).unwrap();
        assert_eq!(json, format!("\"{}\"", kind.as_str()));
        let back: EnvironmentKind = serde_json::from_str(&json).unwrap();
        assert_eq!(back, kind);
    }
}

#[test]
fn environment_kind_wire_spellings_match_case_schema_v1() {
    // Case Schema v1 closed-set spellings (issue-627 contract). All
    // variants' `as_str` spellings are part of that closed set and
    // are pinned literally — `RealAbiTestPlugin` emits the schema
    // `test-plugin` spelling since the Issue #725 Phase B emit flip
    // (the historical `real-abi-test-plugin` spelling remains
    // accepted on deserialize via the alias; see the test below).
    assert_eq!(
        EnvironmentKind::DeterministicBackend.as_str(),
        "deterministic-backend"
    );
    assert_eq!(EnvironmentKind::RealAbiTestPlugin.as_str(), "test-plugin");
    assert_eq!(
        EnvironmentKind::VirtualOsEndpoint.as_str(),
        "virtual-os-endpoint"
    );
    assert_eq!(EnvironmentKind::RealWindow.as_str(), "real-window");
    assert_eq!(EnvironmentKind::PhysicalDevice.as_str(), "physical-device");
}

#[test]
fn environment_kind_deserialize_accepts_both_vocab_spellings() {
    // Issue #725 Phase B: deserialize accepts the canonical Case
    // Schema v1 `test-plugin` spelling and the historical
    // `real-abi-test-plugin` spelling (alias — documents and
    // evidence emitted before the flip still read back); emit is
    // unified on `test-plugin`.
    for spelling in ["test-plugin", "real-abi-test-plugin"] {
        let kind: EnvironmentKind = serde_json::from_str(&format!("\"{spelling}\"")).unwrap();
        assert_eq!(kind, EnvironmentKind::RealAbiTestPlugin);
    }
    // Emit is the schema spelling: serialize and `as_str` produce
    // `test-plugin` (the alias is deserialize-side only).
    assert_eq!(
        serde_json::to_string(&EnvironmentKind::RealAbiTestPlugin).unwrap(),
        "\"test-plugin\""
    );
    assert_eq!(EnvironmentKind::RealAbiTestPlugin.as_str(), "test-plugin");
    // Spellings outside the accepted pair stay rejected.
    for bad in ["test_plugin", "TestPlugin", "plugin-test", "testplugin"] {
        assert!(
            serde_json::from_str::<EnvironmentKind>(&format!("\"{bad}\"")).is_err(),
            "unexpected acceptance of {bad}"
        );
    }
}

#[test]
fn run_metadata_from_json_accepts_test_plugin_spelling() {
    // Issue #725 Phase B: a RunMetadata document carrying the schema
    // vocabulary `test-plugin` deserializes to RealAbiTestPlugin,
    // and re-emitting produces the canonical `test-plugin` spelling.
    let m = full_metadata();
    let json = m.to_json().unwrap();
    let schema_vocab = json.replace("\"deterministic-backend\"", "\"test-plugin\"");
    let parsed: RunMetadata = RunMetadata::from_json(&schema_vocab).unwrap();
    assert_eq!(parsed.environment_kind, EnvironmentKind::RealAbiTestPlugin);
    // Re-emit produces the canonical schema spelling.
    let reemitted: serde_json::Value = serde_json::from_str(&parsed.to_json().unwrap()).unwrap();
    assert_eq!(reemitted["environment_kind"], "test-plugin");
    // The historical spelling also deserializes (documents emitted
    // before the Phase B flip still read back) and re-emits
    // normalized to the canonical spelling.
    let legacy_doc = parsed
        .to_json()
        .unwrap()
        .replace("\"test-plugin\"", "\"real-abi-test-plugin\"");
    let back = RunMetadata::from_json(&legacy_doc).unwrap();
    assert_eq!(back.environment_kind, EnvironmentKind::RealAbiTestPlugin);
    let renormalized: serde_json::Value = serde_json::from_str(&back.to_json().unwrap()).unwrap();
    assert_eq!(renormalized["environment_kind"], "test-plugin");
    assert_eq!(back, parsed);
}

#[test]
fn json_rejects_unknown_vocabulary_and_malformed_input() {
    // camelCase / unknown spellings must not deserialize.
    let bad = full_metadata()
        .to_json()
        .unwrap()
        .replace("\"deterministic-backend\"", "\"DeterministicBackend\"");
    assert!(RunMetadata::from_json(&bad).is_err());
    assert!(RunMetadata::from_json("not json").is_err());
    // Missing required field.
    assert!(RunMetadata::from_json("{}").is_err());
}

#[test]
fn load_json_reports_io_error_for_missing_file() {
    let path = std::env::temp_dir().join(format!(
        "rf628-metadata-{}-missing.json",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    assert!(RunMetadata::load_json(&path).is_err());
}
