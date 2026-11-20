---
id: "TSPEC-034XVt9RlIlSHn2EWXDisA"
type: "test-spec"
status: "approved"
title: "Issue #630 L1 runner — テスト仕様"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "docs/specs/issue-630-l1-runner.md"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-630-l1-runner.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/real-adapter-l1-e2e-tests.md"
    - "docs/test-specs/issue-630-test-plugin-suite.md"
    - "docs/test-specs/issue-630-clap-test-plugin.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-630-l1-runner.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-630-l1-runner.md"
    - "docs/designs/issue-630-l1-runner.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #630 L1 runner — テスト仕様

## Purpose

Pin the observable contract of the test-plugin L1 out-of-process path
so the production `PluginHost`/`ruviform-plugin-export` wire substrate
is verified against the *independent* test fixtures — the same
guarantee shape as the adapter e2e oracle
(`docs/test-specs/real-adapter-l1-e2e-tests.md`), now exercised on
fixture binaries whose plugin-side implementation shares no code with
the host bridge. Retirable when the suite's L1 coverage is superseded
by a broader fixture family or an external validator lane.

## Feature

Test-plugin fixtures (`test_gain` VST3 + CLAP) served out-of-process
through the production L1 runner path.

## Rules

- Rule: Feature gating — the runner bins exist only under
  `required-features = ["l1-runner"]`; the e2e file compiles to
  nothing without the feature (`#![cfg]`), so `env!("CARGO_BIN_EXE_*")`
  is never expanded and default `cargo test` runs see no tests.
- Rule: Fixture resolution — the runner resolves
  `{DLL_PREFIX}ruviform_plugin_test_{vst3,clap}{DLL_SUFFIX}` via
  `RUVIFORM_L1_REAL_PLUGIN_DIR` override → exe-dir default; a
  resolution/load failure exits 1 before `Hello` (defined rejection,
  parent converges to `Err(HostError::Ipc)`), never a hang.
- Rule: Descriptor handshake — the child's `Scanned` wire descriptor
  field-matches the parent pin (`id`/`name`/`vendor`/`version`/
  `features`, order-preserving): VST3 `test_gain`/`"Ruviform Test"`/
  `"0.0.0"`/`["Fx"]` with id = `tuid_hex` of the fixture CID;
  CLAP `test_gain`/`"Ruviform Test"`/`"0.0.0"`/
  `["audio-effect", "stereo"]` id `org.ruviform-test.clap-test-gain`.
- Rule: Lifecycle — `Instantiate` → `Initialize` → `Activate` →
  `StartProcessing` reaches `Processing`; `latency_frames` reports the
  defined `lookahead_blocks × max_frames` (2 × 512 = 1024); clean
  `StopProcessing`/`Deactivate`/`Destroy` exits the child 0 with no
  notification.
- Rule: Process law — 32 stereo shm blocks with a normalized
  `ParamValue(0.25)` event; delivered blocks equal `0.5 ×` an earlier
  input (non-identity `[0.0, 2.0]` gain law); first `lookahead_blocks`
  calls and underruns are defined silence; every call `Ok(CONTINUE)`;
  ≥8 matched calls of the 30 eligible calls must deliver. The
  delivered count keeps to *matched* calls only — per-call
  `audio_counters` deltas classify `passthrough_submits > 0`
  (substituted `last_output` replay, whose samples satisfy the
  membership check without fresh child work) out of delivery
  evidence (`ruviform#766`/`ruviform#836` pattern; `ruviform#768`
  oracle audit). The identical `ParamValue` rides every call, so a
  dropped submit re-lands it structurally — no bounded-resend
  window (the gain overwrite is idempotent).
- Rule: Defined rejections — `SaveState`/`RestoreState` return
  `Err(HostError::Unsupported)` wire replies (bridges keep the
  `Plugin` defaults); `OpenGui`/`CloseGui` short-circuit host-side as
  `Err(Unsupported)`; no fake round-trip is claimed.
- Rule: Crash semantics — `kill_child` converges to `Failed`, emits
  exactly one `InstanceFailed` notification carrying an exit status,
  keeps defined `Err(Failed)` rejections + silence afterwards, no
  automatic restart, `Destroy` frees the slot.
- Rule: Spawn failure — a nonexistent `spec.program` fails
  `Instantiate` with `SpawnFailed{cause: Spawn}` pre-`Hello`; no
  instance minted, no `Failed`, no notification, no hang.
- Rule: Artifact honesty — missing cdylib/runner is an `assert!`
  failure, never a silent skip; the lane task is the only intended
  entry point (`#[ignore]` + lane `--ignored` selection).
- Rule: Independence — the parent oracle pins literals/`vst3::uid`
  derivations computed independently of the child's real-binary scan;
  no fake plugin port appears anywhere in the path.

## Examples

- Example: full series — spawn runner → handshake descriptor
  field-match → 32 blocks at 48 kHz/512 frames stereo → gain `0.5`
  delivered-block membership on matched calls (substituted replays
  classified out by the `passthrough_submits` delta) →
  `Err(Unsupported)` state/GUI verbs → clean teardown exit 0.
- Example: kill — `kill_child` mid-`Processing` → `Failed` + one
  `InstanceFailed{exit.code|signal}` notification → `Initialize`/
  `SaveState` → `Err(Failed)` → silence `process` → `Destroy` ok.
- Example: missing runner — `OutOfProcessSpec` program
  `ruviform-plugin-test-{vst3,clap}-l1-runner-definitely-missing` →
  `Err(SpawnFailed{cause: Spawn})`, `Scan` still replays the
  registered descriptor, no notification.

## Guarantees

- A correct fixture + host pair passes all rules on Windows OS-L1;
  a descriptor, gain-law, lifecycle, or crash-semantics regression in
  either side fails a named test.
- The `#[ignore]`d tests run only through `mise run
  test-l1-real-plugin` (or an equivalent explicit `--ignored`
  selection), keeping default `cargo test` fast and feature-free.

## Non-goals

- State Save/Reload/Preset round-trip — `Err(Unsupported)` is the
  pinned expectation (adapter-side feature gap; separate Issue).
- Native editor / GUI-L2 surface — `OpenGui`/`CloseGui` `Unsupported`
  is the pinned expectation; real-window dock evidence lives in the
  Issue #631 lane.
- MIDI/Tempo/Loop over L1 — no instrument class exists yet; the wire
  codec's coverage stays at the adapter-fixture level.
- External SDK validator conformance — Deferred; no external authority
  is claimed.
- macOS/Linux lane execution — tasks are platform-neutral; per-OS
  evidence is follow-up.

## Risk

- If the descriptor pin were derived from the same scan the child
  performs, a systematic scan bug would self-verify — the literal
  `vst3::uid`/descriptor pins exist precisely to keep the two sources
  independent.
- If `EXPECTED_GAIN` matched the identity mapping (`1.0`), a
  normalized↔plain domain confusion would pass silently — the
  non-identity `[0.0, 2.0]` law (`0.25 → 0.5`) is the oracle.
