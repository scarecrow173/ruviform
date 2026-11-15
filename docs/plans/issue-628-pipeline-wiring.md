---
id: "PLAN-RF628pwpipeline0000001"
type: "plan"
status: "approved"
title: "Issue #628 pipeline wiring — implementation plan"
created: "2026-11-15"
updated: "2026-11-15"
owners:
  - "scarecrow173"
relations:
  source:
    - "docs/specs/issue-628-pipeline-wiring.md"
    - "docs/designs/issue-628-pipeline-wiring.md"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-628-pipeline-wiring.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-628-pipeline-wiring.md"
    - "docs/designs/issue-628-pipeline-wiring.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-628-pipeline-wiring.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-628-pipeline-wiring.md"
    - "docs/test-specs/issue-628-pipeline-wiring.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-628-pipeline-wiring.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-628-pipeline-wiring.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #628 pipeline wiring — implementation plan

## Scope split

Single plan — 3 legs share the `pipeline` module but are independently
testable. Order: shared plumbing → output leg → input leg → midi leg →
RT contract tests → docs/evidence.

## File map

| File | Change |
| --- | --- |
| `crates/ruviform-audio-test/Cargo.toml` | add path deps: `ruviform-audio`, `ruviform-audio-plugin-bridge`, `ruviform-graph`, `ruviform-plugin-core` (reason comments) |
| `crates/ruviform-audio-test/src/lib.rs` | `pub mod pipeline;` + doc mention |
| `crates/ruviform-audio-test/src/pipeline.rs` | module doc, `PipelineError`, re-exports |
| `crates/ruviform-audio-test/src/pipeline/common.rs` | expected PCM block assembly, PcmBlock compare, report collection |
| `crates/ruviform-audio-test/src/pipeline/output.rs` | `PlaybackPipeline` |
| `crates/ruviform-audio-test/src/pipeline/input.rs` | `InputCapture`, `InputPipeline`, `GainNode` |
| `crates/ruviform-audio-test/src/pipeline/midi.rs` | `MidiLeg`, pipeline-local `InputPump` |
| `crates/ruviform-audio-test/tests/pipeline_output.rs` | output leg tests |
| `crates/ruviform-audio-test/tests/pipeline_input.rs` | input leg tests |
| `crates/ruviform-audio-test/tests/pipeline_midi.rs` | midi leg tests |
| `crates/ruviform-audio-test/tests/pipeline_faults.rs` | fault propagation tests |
| `crates/ruviform-audio-test/tests/rt_contract.rs` | add `rt_pipeline_*` tests |
| `crates/ruviform-audio-test/README.md` | pipeline API boundary + honest limits |
| `docs/{specs,designs,plans,tasks,test-specs}/issue-628-pipeline-wiring.md` | this chain |
| `docs/impl/ir|exp/issue-628-pipeline-wiring.*` | impl records |
| `evidence/rf628/` | verification records |

## Dependency graph

```
common.rs (oracle helpers)
  ├── output.rs (PlaybackPipeline)
  ├── input.rs  (InputCapture + InputPipeline + GainNode)
  └── midi.rs   (MidiLeg + InputPump)
pipeline.rs re-exports all
```

## Risk register

| Risk | Mitigation |
| --- | --- |
| `Session::open` callback wiring differs from assumed `plan.drive` | read `session.rs` first; adapt graph to whatever `GraphPlan` expects |
| typed runtime `input_writer` frames vs variable callback frames | process per captured block; frames <= max_frames enforced by `ProcessError` surface |
| `InputTx` pub(crate) blocks direct reuse | pipeline-local `InputPump` mirrors contract; document as mirror not fork |
| `InputStage` anchor/stamp base mismatch → all events rejected | anchor `stamp_base` must equal cue `position_base`; test asserts zero rejected in happy path |
| capture timestamp not flowing into typed runtime | record as observed metadata + NotRun note (honest gap) |
| `FramePosition` fault on output leg doc/impl mismatch in backend | avoid on output tests; note as finding in evidence |

## Verification matrix

| Check | Command |
| --- | --- |
| fmt | `mise run fmt-check` |
| check | `mise run check ruviform-audio-test` |
| lint | `mise run lint ruviform-audio-test` |
| test | `mise run test ruviform-audio-test` |
| rt | `mise run test-rt` (verify `rt_pipeline_*` collected) |
| prek | `mise exec -- prek run --all-files` |
| downstream | `mise run check ruviform-audio` `mise run check ruviform-audio-plugin-bridge` (non-regression; product code untouched so check suffices) |
