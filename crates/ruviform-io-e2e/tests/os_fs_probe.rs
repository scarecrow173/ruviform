//! OS/FS capability probe: the publish durability ladder executed on
//! the real filesystem, reporting *measured* `DurabilityEvidence` —
//! never the requested level.
//!
//! Slice-2 rule F11
//! (`docs/test-specs/issue-306-io-e2e-fault-fuzz.md`). This fixture
//! records what this host actually achieved; the evidence report
//! (`evidence/rf306/fault-fuzz-report.md`) scopes the claim to the
//! measured environment. macOS/Linux real-OS runs, Network/Cloud
//! filesystems and performance characteristics are `NotRun`/
//! `Unsupported` — this test asserts only that every outcome is an
//! honest one: either the requested durability was achieved with
//! matching evidence, or the publish refused with `Unsupported`
//! (never a silent downgrade).

mod common;

use std::io::Write;

use common::{TestDir, exists, read};
use ruviform_io_file::{
    Durability, DurabilityEvidence, PublishEffect, PublishFailureKind, PublishMode, PublishOptions,
    StagedFile,
};

const PAYLOAD: &[u8] = b"osfs-probe-payload\x00\x01\x02";

#[test]
fn publish_durability_ladder_reports_measured_evidence() {
    let dir = TestDir::new("osfs");
    let mut results: Vec<String> = Vec::new();
    let mut successes = 0_usize;
    let mut unsupported = 0_usize;

    for (mode_index, mode) in [PublishMode::CreateNew, PublishMode::Replace]
        .into_iter()
        .enumerate()
    {
        for (durability_index, durability) in [
            Durability::BestEffort,
            Durability::FileSync,
            Durability::FileAndDirectorySync,
        ]
        .into_iter()
        .enumerate()
        {
            let dest = dir.dest(&format!("probe-{mode_index}-{durability_index}.bin"));
            let mut staged = StagedFile::create(&dest).expect("stage");
            staged.write_all(PAYLOAD).expect("payload");
            let sealed = staged.seal();

            match sealed.publish(PublishOptions::new(mode, durability)) {
                Ok(mut receipt) => {
                    // Evidence is what the platform *observed*, and
                    // must correspond to the requested level — a
                    // silent downgrade would show up here.
                    let expected = match durability {
                        Durability::BestEffort => DurabilityEvidence::None,
                        Durability::FileSync => DurabilityEvidence::FileSynced,
                        Durability::FileAndDirectorySync => {
                            DurabilityEvidence::FileAndDirectorySynced
                        }
                        _ => unreachable!("probe iterates the three known variants"),
                    };
                    assert_eq!(
                        receipt.durability(),
                        expected,
                        "{mode:?} + {durability:?}: evidence must match achieved work"
                    );
                    assert!(exists(&dest));
                    assert_eq!(read(&dest).as_deref(), Some(PAYLOAD));
                    // A retained temp after success would carry a
                    // recovery handle — consume it honestly if so.
                    if let Some(handle) = receipt.take_recovery_handle() {
                        let _ = handle.cleanup();
                    }
                    successes += 1;
                    results.push(format!(
                        "mode={mode:?} durability={durability:?} -> OK evidence={}",
                        receipt.durability()
                    ));
                }
                Err(failure) => {
                    // The only acceptable refusal is an honest
                    // capability report — the destination stays
                    // untouched and the outcome is certain.
                    assert_eq!(
                        failure.kind(),
                        PublishFailureKind::Unsupported,
                        "{mode:?} + {durability:?}: unexpected failure kind"
                    );
                    assert_eq!(failure.effect(), PublishEffect::NotPublished);
                    assert!(!exists(&dest), "refused publish must not create dest");
                    if let Some(handle) = failure.recovery_handle() {
                        let _ = handle.inspect();
                    }
                    unsupported += 1;
                    results.push(format!(
                        "mode={mode:?} durability={durability:?} -> Unsupported"
                    ));
                }
            }
        }
    }

    eprintln!(
        "OS/FS durability probe ({} success, {} unsupported):\n{}",
        successes,
        unsupported,
        results.join("\n")
    );
    assert!(
        successes > 0,
        "the probe must observe at least one real publish to mean anything"
    );
}
