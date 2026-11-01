---
id: "IMPL-034QiCAnjapH74RQucPXy1"
type: "impl"
status: "completed"
title: "Issue #185 Generic Probe PoC implementation record"
created: "2026-11-01"
updated: "2026-11-01"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/185"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-audio/src/typed/probe.rs"
      - type: "file"
        path: "crates/ruviform-audio/tests/typed_probe.rs"
      - type: "file"
        path: "docs/discovery/issue-185-generic-probe-observation-inventory.md"
      - type: "file"
        path: "docs/specs/issue-185-generic-probe-rt-capture.md"
      - type: "file"
        path: "docs/adr/issue-185-generic-probe-contract-placement-and-rt-handoff-model.md"
      - type: "file"
        path: "docs/designs/issue-185-generic-probe-typed-runtime-observation-rt-capture.md"
      - type: "file"
        path: "docs/plans/implement-issue-185-generic-probe-poc.md"
      - type: "file"
        path: "docs/test-specs/issue-185-generic-probe-tests.md"
      - type: "file"
        path: "docs/tasks/p185-t1-level-extract-and-binding-centralization.md"
      - type: "file"
        path: "docs/tasks/p185-t2-runtime-probe-plumbing-taps.md"
      - type: "file"
        path: "docs/tasks/p185-t3-probe-contract-broker-collector.md"
      - type: "file"
        path: "docs/tasks/p185-t4-probe-poc-tests-verification.md"
    modified:
      - type: "file"
        path: "crates/ruviform-audio-analysis/src/level.rs"
      - type: "file"
        path: "crates/ruviform-audio/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-audio/src/typed/compile.rs"
      - type: "file"
        path: "crates/ruviform-audio/src/typed/io.rs"
      - type: "file"
        path: "crates/ruviform-audio/src/typed/mod.rs"
      - type: "file"
        path: "crates/ruviform-audio/src/typed/runtime.rs"
      - type: "file"
        path: "crates/ruviform-audio/tests/common/mod.rs"
      - type: "file"
        path: "Cargo.lock"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../../plans/implement-issue-185-generic-probe-poc.md"
    - "../../tasks/p185-t1-level-extract-and-binding-centralization.md"
    - "../../tasks/p185-t2-runtime-probe-plumbing-taps.md"
    - "../../tasks/p185-t3-probe-contract-broker-collector.md"
    - "../../tasks/p185-t4-probe-poc-tests-verification.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../discovery/issue-185-generic-probe-observation-inventory.md"
    - "../../specs/issue-185-generic-probe-rt-capture.md"
    - "../../adr/issue-185-generic-probe-contract-placement-and-rt-handoff-model.md"
    - "../../designs/issue-185-generic-probe-typed-runtime-observation-rt-capture.md"
    - "../../test-specs/issue-185-generic-probe-tests.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "../../test-specs/issue-185-generic-probe-tests.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #185 Generic Probe PoC implementation record

## Summary

Implemented the Issue #185 minimum Generic Probe/Observation PoC on the
typed multi-bus runtime (#184/#195). `ruviform_audio::typed::probe` provides
a capability-declarative, selectively executable probe contract
(`Off`/`Monitor`/`Capture`) over logical Node/Port observation points, with
preallocated bounded PCM capture handed off RT→non-RT through a pooled
chunk channel, POD telemetry on a separate drop-newest channel, and a
non-RT `ProbeBroker`/`ProbeCollector` control+delivery surface. Probe
observation is non-destructive: ON/OFF output PCM is bit-identical and the
RT path performs zero allocation in every mode.

## Context

Follows the approved document chain: discovery
`docs/discovery/issue-185-generic-probe-observation-inventory.md`, spec
`docs/specs/issue-185-generic-probe-rt-capture.md`, ADR
`docs/adr/issue-185-generic-probe-contract-placement-and-rt-handoff-model.md`,
design `docs/designs/issue-185-generic-probe-typed-runtime-observation-rt-capture.md`,
plan `docs/plans/implement-issue-185-generic-probe-poc.md`, test-spec
`docs/test-specs/issue-185-generic-probe-tests.md`, tasks T-1…T-4.
Depends on the merged typed IR/runtime contracts (#184 PR #225, #195 PR #227).
Review-protocol substitute approval was applied pre-implementation: two
independent reviewers + validator pass against existing code; all P0/P1
findings were fixed in the spec/ADR/design before coding.

## Implementation

### Changes

- `crates/ruviform-audio-analysis/src/level.rs` — extracted
  `ChannelLevel::measure(&[f32])` from `LevelMeter::process` so probes share
  the exact Peak/RMS contract without owning a meter (T-1).
- `crates/ruviform-audio/Cargo.toml` — added `ruviform-audio-analysis` and
  `ruviform-observability` dependencies.
- `crates/ruviform-audio/src/typed/compile.rs` — `ScheduledNode.outputs`
  is now `Vec<(PortId, usize)>` so logical port→slot mapping is
  authoritative; `InputBinding::effective_slot()` centralizes the
  scratch-vs-direct observation rule; `own_latency`/`latency_accum` retained
  per node for tap metadata (T-1).
- `crates/ruviform-audio/src/typed/io.rs` — `NodeIo` uses the authoritative
  effective-slot rule and port-aware outputs.
- `crates/ruviform-audio/src/typed/probe.rs` — new contract layer:
  `ProbeId`/`ProbeMode`/`CaptureConfig`, capability resolution
  (`Configured`/`Inferred`/`Unavailable`/`Unsupported`/`Unknown`),
  `RecordMeta` envelope (probe/node/port, plan generation, absolute
  `sample_position` from `frame_cursor`, frames/channels/sample-rate,
  per-tap `effective_latency`, loss/continuity), `ProbeBroker` (non-RT
  commands incl. generation-scoped `Rebind`), `ProbeDeck`/`RtProbe` (RT tap
  execution with stale-binding guard → `Unbound`, unobserved-stale →
  `Unobserved`, free-pool exhaustion → `lost_blocks`, telemetry overflow →
  `lost_records`), chunk pool + `chunks_out`/`telemetry` channels, and
  `ProbeCollector` returning chunks to the pool (T-3).
- `crates/ruviform-audio/src/typed/runtime.rs` — `frame_cursor`,
  `published_plan`, `set_probe_deck`, generation-scoped probe command
  drain inside plan activation, and four non-destructive tap sites
  (graph-input, node-input, node-output, graph-output) via `TapCtx` (T-2).
- `crates/ruviform-audio/tests/typed_probe.rs` + `tests/common/mod.rs` —
  12 acceptance tests per the test-spec incl. `rt_probe_*` alloc tests.

### Decisions During Implementation

- Capture budget is *not* capped by pool size: the pool bounds outstanding
  in-flight chunks (collector returns them), while `max_total_frames`
  bounds total delivered capture. Free-pool exhaustion records
  `lost_blocks` without blocking RT.
- `chunks_out` capacity equals pool size so pushes are structurally
  non-blocking while the recycle invariant holds (debug-asserted, per
  test-spec non-goal of a formal proof); telemetry floods cannot starve
  capture because the transports are separate.
- Rebind commands carry the plan generation they were issued against; the
  deck drains them during activation and marks stale/mismatched bindings
  `Unbound`, so a republished graph can never be observed through a
  wrong-slot binding.
- OFF mode is a cheap active-tap scan: tap sites return before touching
  arena slots, borrow nothing, and emit no records.
- `BudgetExceeded` (explicit `0` budget) exercises the same
  `BudgetExhausted` → `Monitor` path as natural exhaustion.

## Related Experiments

### Adopted

None — the PoC reuses `ChannelLevel::measure`, `ruviform_rt::channel` SPSC,
`try_borrow`/stamp freshness, and `ruviform-observability` counters rather
than prototyping alternatives.

### Rejected

None recorded.

## Validation

- `mise run fmt-check` — pass.
- `mise run check` (`cargo check --workspace --all-targets`) — pass.
- `mise run lint` (`cargo clippy --workspace --all-targets -- -D warnings`) — pass.
- `mise run test` (`cargo test --workspace`) — pass; `typed_probe` suite 12/12 green.
- `mise run test-rt` (`cargo test --workspace rt_`) — pass, incl.
  `rt_probe_off`/`rt_probe_monitor`/`rt_probe_capture` zero-allocation
  windows via `assert_no_alloc`.
- `mise run verify-ci` — see Validation caveats below.
- Behavioral coverage: capability classification + deterministic
  enumeration, per-probe mode transitions, ON/OFF output bit-identity,
  same-span pre/post Peak/RMS, bounded capture metadata, budget
  exhaustion → `BudgetExhausted` + Monitor return, explicit stop →
  `CaptureStopped{Requested}`, republish rebind + stale-binding guard,
  collector stall/unsubscribe never blocking RT, `Unobserved`/`Unbound`
  statuses, defined `ProbeError` surface, per-tap effective latency
  (incl. delayed-merge/PDC fixture), continuous `frame_cursor` positions.

## Risks

- `chunks_out` structural non-fullness is argued by the pool invariant and
  debug-asserted, not formally proven (declared non-goal in test-spec).
- ON/OFF load delta is covered by zero-alloc proofs and the cheap-OFF gate
  (no slot borrow/record emission); a dedicated timing benchmark is not
  part of the PoC scope.
- TimedEvent/Control payload observation, plugin-internal stages,
  persistent storage, and cross-session ID stability remain deferred per
  the spec's non-goals.
- The loss-tolerant Probe capture contract is explicitly separate from the
  lossless Recording Writer contract (#187); nothing here may be reused as
  a recording guarantee.

## Follow-ups

- Consumer impact classification: `ruviform-audio` gains a new public
  `typed::probe` module plus typed-runtime accessors — additive only;
  synvelis/audio-lens: no impact (no existing consumer APIs changed).
- `#193` (M7 boundary / G7 RT Safety Gate) may cite `rt_probe_*` tests as
  additional evidence.
- Future domain probes (MIDI/Event/Control) extend `ProbePayload` per the
  spec's extension point.
