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
