---
id: "TESTSPEC-RF628pwpipeline00001"
type: "test-spec"
status: "approved"
title: "Issue #628 pipeline wiring — test specification"
created: "2026-11-15"
updated: "2026-11-15"
owners:
  - "scarecrow173"
relations:
  source:
    - "docs/specs/issue-628-pipeline-wiring.md"
    - "docs/designs/issue-628-pipeline-wiring.md"
    - "docs/knowledge/docs-operations-and-agent-workflow.md"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-628-pipeline-wiring.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-628-pipeline-wiring.md"
    - "docs/designs/issue-628-pipeline-wiring.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-628-pipeline-wiring.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-628-pipeline-wiring.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-628-pipeline-wiring.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #628 pipeline wiring — test specification

## Feature: Output leg — `Session::open` + scripted output device

### Rule OUT-1: captured PCM equals oracle reference
Given a `PcmFixture`-backed graph node and `record_output` enabled,
when N blocks are pumped through `Session`'s production output callback,
the captured `PcmBlock`s must equal `oracle::pcm_reference_block` per
channel at each block's `start_frame`/`frames`.

### Rule OUT-2: fault surfaces honestly
Given a `FaultScript` injecting `DeviceError::Xrun` /
`QueueFull`-style / `Invalidate`, `pump_output_next` returns `Err` or
fires the error callback; the pipeline records the fault and does not
report PCM equality for the faulted block.

### Rule OUT-3: timestamps are deterministic
`PumpReport.callback_nanos`/`playback_nanos` derive from `VirtualClock`
+ scripted latency; advancing the `VirtualClockDriver` between pumps
produces the expected monotonic values.

## Feature: Input leg — typed runtime boundary

### Rule IN-1: captured input equals scenario oracle
Given `PcmScenario` covering all pumped frames, each `CaptureBlock`'s
PCM equals `oracle::pcm_reference_block` of the covering segment at the
block's `frame_position`.

### Rule IN-2: processed output equals expected transform
Given `GainNode{gain}`, observed output PCM equals
`expected_input * gain` computed independently (oracle input, not
captured input, as the reference).

### Rule IN-3: gap/Xrun propagates
Given a scenario gap, the backend fires `DeviceError::Xrun` via error
callback AND delivers silence for gap frames; observed error list
contains `Xrun` and gap-region expected PCM is 0.0.

### Rule IN-4: capture timestamp metadata observed
`CaptureBlock` records `callback_nanos`, optional `capture_nanos`,
`clock_domain`, `frame_position`; `capture_reporting=false` yields
`capture=None` honestly (no substitution with callback time).

### Rule IN-5: capture pool exhaustion is counted, not silent
Given the bounded `InputCapture` pool is full (callbacks fired without
drain), further callbacks increment `CaptureCounters::starved`; the
driver still drains and processes captured blocks on the next pump.

## Feature: MIDI leg — `InputStage` pump/collect

### Rule MID-1: in-block events land at expected offsets
Given `MidiScenario` cues at frame positions within pumped blocks,
`collect_block` returns `Event`s whose `frame_offset` equals
`cue_frame - block_start`, with `EventKind::Midi` carrying the cue's
`MidiEvent`.

### Rule MID-2: late events clamp to offset 0
Given a cue whose frame precedes `block_start`, it is delivered at
`frame_offset=0` and `counters.late` increments.

### Rule MID-3: counters are honest
`InputStage::counters()` cumulative `dropped`/`rejected`/`late` reflect
actual staging/collect outcomes; no silent loss.

### Rule MID-4: anchor mismatch rejects honestly
Given a cue whose stamp domain/base/epoch mismatches the published
`ClockAnchor`, the event is rejected (counted) rather than mis-mapped.

## Feature: RT contract

### Rule RT-1: callback paths allocate-free
`rt_pipeline_output_session_pump_does_not_alloc`,
`rt_pipeline_input_capture_and_process_does_not_alloc`, and
`rt_pipeline_midi_collect_does_not_alloc` are `rt_`-prefixed,
collected by `mise run test-rt`, and pass under `assert_no_alloc`.

## Feature: Honesty / boundaries

### Rule HON-1: no product-code change
`git diff` touches only `crates/ruviform-audio-test/**`,
`docs/**`, `evidence/rf628/**`, generated `Cargo.lock`/indexes.
Product crates' `src` unchanged.

### Rule HON-2: deterministic ≠ real-OS proof
README/docs state deterministic-backend Pass is OS-L1 evidence only;
real OS/device/threading/ABI paths are NotRun/Deferred.
