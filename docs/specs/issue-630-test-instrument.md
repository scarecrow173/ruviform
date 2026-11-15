---
id: "SPEC-034Z9kQm4xVtW2sLpNbRcA"
type: "spec"
status: "approved"
title: "Issue #630 test_instrument slice — deterministic MIDI/transport synth fixture + L1 real-ABI oracle"
created: "2026-11-16"
updated: "2026-11-16"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "https://github.com/scarecrow173/ruviform/issues/628"
    - "docs/designs/issue-630-test-plugin-suite.md"
    - "docs/specs/issue-630-test-plugin-suite.md"
    - "docs/specs/issue-630-l1-runner.md"
    - "docs/specs/issue-630-adapter-state-bridge.md"
    - "docs/specs/issue-630-test-stateful.md"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/src/instrument.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/src/events.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/src/instrument.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/src/instrument_core.rs"
      - type: "file"
        path: "docs/specs/issue-630-test-instrument.md"
      - type: "file"
        path: "docs/designs/issue-630-test-instrument.md"
      - type: "file"
        path: "docs/plans/issue-630-test-instrument.md"
      - type: "file"
        path: "docs/tasks/issue-630-test-instrument.md"
      - type: "file"
        path: "docs/test-specs/issue-630-test-instrument.md"
      - type: "file"
        path: "docs/impl/ir/issue-630-test-instrument.md"
      - type: "file"
        path: "docs/impl/exp/issue-630-test-instrument.jsonl"
      - type: "file"
        path: "evidence/rf630/test-instrument-verification.md"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/src/component.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/src/controller.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/src/factory.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/tests/host_path.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/tests/l1_real_test_plugin_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/src/block.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/src/plugin.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/tests/host_path.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/tests/l1_real_test_plugin_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-clap/src/host.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/wire.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/data.rs"
      - type: "file"
        path: "crates/ruviform-plugin-export/src/lib.rs"
      - type: "file"
        path: "Cargo.lock"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
      - type: "file"
        path: "docs/impl/exp/README.md"
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-630-test-plugin-suite.md"
    - "docs/designs/issue-630-test-plugin-suite.md"
    - "docs/specs/issue-630-l1-runner.md"
    - "docs/specs/issue-630-test-stateful.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-630-test-plugin-suite.md"
    - "docs/designs/issue-630-test-plugin-suite.md"
    - "docs/specs/issue-630-l1-runner.md"
    - "docs/test-specs/issue-630-test-plugin-suite.md"
    - "POLICIES.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-630-test-plugin-suite.md"
    - "docs/specs/issue-630-l1-runner.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #630 test_instrument slice — deterministic MIDI/transport synth fixture + L1 real-ABI oracle

## What Should Be Built

A `test_instrument` plugin class inside each existing Issue #630
test fixture cdylib (`ruviform-plugin-test-vst3`,
`ruviform-plugin-test-clap`) that is a **genuine instrument** —
NoteOn/NoteOff voices, velocity, MIDI CC, and transport clock —
rendering **deterministic audio** through the real plugin ABIs, plus
the L1 out-of-process E2E oracle that drives it through the
production host's discovery → instantiation → processing →
uninstantiation path:

- **`test_instrument` class per format** — a mono-summed
  polyphonic triangle-wave voice (up to `MAX_VOICES`, steal-oldest
  policy) with a fixed 2400-frame linear release, CC7 master volume,
  CC1 tremolo depth, and a transport-clock click oracle. The DSP law
  is the fixture's own: it must **not** reuse the production
  adapter's translation or DSP code — only the SDK/binding layer
  (`vst3`/`clack-*`) and the neutral `ruviform-plugin-core` /
  `ruviform-media-core` contracts are shared.
- **Event input surfaces** — VST3: the component drains
  `IEventList` MIDI (`NoteOn`/`NoteOff`/CC/…) into neutral
  `EventKind::Midi` via a fixture-owned `events.rs` translator and
  reads `ProcessContext` musical/transport fields into neutral
  `TransportInfo` (PPQ → PPQN-960 ticks, validated tempo/timing
  flags — no fabricated timing). CLAP: `block.rs` translates
  structured note events + raw MIDI 1.0 events into the same
  neutral kinds and maps the CLAP `Process::transport` snapshot
  into `TransportInfo` (CLAP carries no PPQN → fixture fixes
  `CLAP_PPQN = 960`).
- **Deterministic oracles** — note-on at full velocity/gain/volume
  puts the triangle's `-1.0` extreme at frame 0 of every retriggered
  block; one note-off then empty blocks decays to exact silence;
  CC7 = 0 silences (`-0.0` is numerically silent), CC7 = max
  restores; a note-on at frame offset 100 gates the voice start;
  playing transport at 120 BPM / PPQN 960 clicks a `0.5` impulse at
  the next PPQN boundary inside the block (position 956 → frame
  100, position 940 → frame 500); stopped transport is silent; an
  active loop range does not disturb per-block click derivation.
- **L1 E2E inside the existing `l1_real_test_plugin_e2e` target**
  (both crates, `#![cfg(feature = "l1-runner")]` + `#[ignore]`):
  four instrument cases per format — note lifecycle, single
  note-off release-to-silence, CC7 + frame-offset gating, and
  transport click/seek/loop — all gated on the L1 lookahead
  pipeline's delivery windows and asserting fixture presence
  (missing cdylib/runner is an assert-fail, never a silent skip).
- **Host-side MIDI/transport forwarding** — the production CLAP
  host bridge (`ruviform-plugin-clap/src/host.rs`) learns to encode
  neutral `EventKind::Midi` into CLAP note events / raw MIDI 1.0
  events and to project `TransportInfo` into the CLAP transport
  snapshot; the VST3 adapter's existing `IEventList`/`ProcessContext`
  path is exercised unchanged.
- **L1 wire fix (shared, format-neutral)** — the fixed 33-byte
  event slot in the L1 shared-memory protocol gets a real codec:
  `wire::encode_event_record`/`decode_event_record` zero-pad each
  event to `EVENT_WIRE_SIZE`; host staging and export decoding move
  to the record codec. MIDI events (28-byte variable encoding) were
  previously unrepresentable in a slot — this is the wire-contract
  owner's fix, not a fixture workaround.
- **Doc/evidence chain** for this slice.

## Why Now

Issue #630's instrument slice is the event/transport counterpart of
the gain/fault/editor/stateful slices: every prior fixture was an
`Fx` audio processor whose only inputs were parameter events and
PCM. The production host's MIDI (`IEventList` / CLAP note+raw-MIDI)
and transport (`ProcessContext` / CLAP transport snapshot) forwarding
paths therefore had **no independent oracle** — a host/fixture pair
reusing one adapter implementation would verify itself. This slice
adds a real instrument to both fixture cdylibs so NoteOn/NoteOff,
velocity, CC, and transport reach deterministic audio through the
real ABIs, closing the self-verification loop for the event surface.

## Acceptance Criteria (this slice)

- [ ] `test_instrument` discoverable + instantiable through the real
  VST3 class scan (factory 10 classes, 5 audio descriptors;
  `"Instrument|Fx"` subCategories → `["Instrument", "Fx"]`) and the
  CLAP factory scan (`plugin_count() == 4`, id
  `org.ruviform-test.clap-test-instrument`, features
  `instrument`/`synthesizer`/`stereo`), driven to `Processing`.
- [ ] In-process host-path tests (real cdylib through the
  production `Vst3Library`/`ClapLibrary` bridges): retriggered
  note-on → `-1.0` at frame 0; single note-off → 2400-frame release
  → exact silence tail.
- [ ] L1 E2E (ignored, `l1-runner` feature, real cdylib + runner):
  note lifecycle, release-to-silence, CC7/offset gating, and
  transport click/seek/loop cases pass per format on Windows OS-L1
  via `mise run test-l1-real-plugin` (existing target — no new
  lane line needed).
- [ ] MIDI `EventKind::Midi` survives the L1 shared-memory wire:
  fixed 33-byte records encode/decode symmetrically, nonzero
  padding and malformed records are defined rejections.
- [ ] `rt_`-safe: the `process`/`clap_process` path stays
  alloc/lock/wait-free — event/transport translation happens on the
  audio path with bounded scratch only.
- [ ] Honest boundaries recorded: fixtures never import production
  adapter internals; the CLAP `attach()` runtime-integration leg
  (issue #628 scope) stays deferred; external SDK validators,
  non-Windows OS runs, and `*-msrv`/`*-full` lanes are out of scope.

## Non-Goals

- No `#628` MIDI `attach()` runtime-integration work — deferred
  unless the coordinator explicitly approves it.
- No production **adapter** DSP/translation reuse inside fixtures —
  the fixture owns its event→neutral and transport→neutral
  translators independently; the only production-side changes are
  the host bridge's MIDI/transport **forwarding** (CLAP `host.rs`)
  and the format-neutral L1 wire record codec.
- `test_gain`/`test_fault`/`test_editor`/`test_stateful` class,
  descriptor, CID and DSP contracts unchanged; their behavior is
  preserved (host_path counts updated for the new class only).
- No preset/patch management, no GUI, no real-world MIDI file
  playback — events arrive only through the host `process` ABI.
- PR uses `Refs #630` — the Issue stays open for later slices.
