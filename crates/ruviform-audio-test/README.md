# ruviform-audio-test

Test-only deterministic Audio/MIDI test runtime (Issue #628 —
first slice: deterministic core; second slice: scripted port-trait
backend; third slice: pipeline wiring; close-out slice: JSON run
metadata + `Cancellation` fault class).

## Status: test-only

**This crate must never enter product distribution.** It is a
workspace member for local/CI verification and downstream consumer
test harnesses (AudioLens/Synvelis). No production crate may depend on
it — the dependency direction is consumer-of-production-contracts
only:

- `ruviform-audio-core` — `AudioBufferViewMut`, `SampleRate` buffer/time
  contracts
- `ruviform-media-core` — `TransportScheduler`/`TransportPlanBuilder`/
  `SpanSink`, `MidiEvent`, `TimeBase`/`MediaTime`
- `ruviform-rt` — `MasterClock` atomic tick model
- `ruviform-audio` — `Session::open`/`Graph`/`GraphPlan`, typed
  `Compiler`/`GraphRuntime`/`RtRuntime` (pipeline legs only)
- `ruviform-audio-plugin-bridge` — `MidiInputEnumerator`/
  `MidiInputPort`/`InputStage` (pipeline MIDI leg only)
- `ruviform-graph` / `ruviform-plugin-core` — graph build/validate and
  `Event`/`EventKind` contracts for the pipeline legs
- `serde` / `serde_json` — `RunMetadata` JSON serialization only
  (non-RT test-driver API; never on RT paths)

`publish = false` is inherited from the workspace; `assert_no_alloc`
is a dev-dependency only.

## Responsibilities

- `clock` — [`VirtualClock`] / [`VirtualClockDriver`] /
  [`VirtualClockConfig`] / [`JitterSpec`]: deterministic sample-frame
  tick + monotonic time with offset, ppm drift, per-period jitter,
  pause/resume and seek. Atomic wait-free reads.
- `pcm` — [`PcmPattern`] / [`PcmFixture`] / [`PcmSegment`] /
  [`PcmScenario`] / [`PcmBlock`]: bit-deterministic PCM generation
  (silence, constant, impulse, ramp, square, sine, sweep, seeded
  noise), position-independent per absolute frame, allocation-free
  `fill`/`fill_view`.
- `midi` — [`MidiScenario`] / [`MidiCue`] / [`MidiTraceConfig`] /
  [`run_midi_trace`] / [`TraceSink`] / [`MidiTrace`]: compile cues into
  a production `TransportPlan`, drive a production
  `TransportScheduler`, capture delivered events at the `SpanSink`
  boundary. No production MIDI output port exists yet — the sink is
  the outgoing-trace record point.
- `oracle` — independent expected-value generators
  (`pcm_reference_block`, `midi_expected_playthrough`) and comparison
  helpers; never calls the generators/scheduler under test.
- `metadata` — [`RunMetadata`] / [`RunParameters`] with Case Schema v1
  vocabulary (`environment_kind = deterministic-backend`,
  `level = OS-L1`, Required/Deferred/NotApplicable ×
  Pass/Fail/NotRun/Unsupported) and JSON persistence
  (`to_json`/`from_json`/`save_json`/`load_json` — non-RT
  test-driver API; wire format is the kebab-case Case Schema v1
  vocabulary).
- `backend` — [`ScriptedBackend`] / [`ScriptedDeviceHandle`] /
  [`DeviceSpec`] / [`StreamScript`] / [`StreamFault`] /
  [`PumpReport`] / [`MidiFeed`]: a deterministic implementation of the
  production `DeviceEnumerator`/`Device`/`InputStream`/`OutputStream`
  port traits. Callbacks fire only through manual
  `pump_*_next` calls on the test driver thread — no OS API, no
  threads, no real hardware. Scripted open/start/stop/callback faults,
  frame-position jumps, capture suppression, hot-unplug/reappear,
  PCM scenario replay (input) and post-callback capture (output),
  virtual-clock timestamps with optional latency offsets, and a thin
  `MidiScenario` → `TimedMidiInput` feed for `MockPort`. Fault classes
  include `Cancellation` — the Issue fault-injector "cancellation"
  mapped onto existing observables only (aborted delivery +
  `Backend("cancelled")` + stream termination; no production contract
  changes).
- `pipeline` — three harness legs that drive the scripted backend
  through production consumer contracts (third slice):
  - `output` — [`PlaybackPipeline`]: `Session::open` +
    `GraphPlan` drive the production `OutputDataCallback`; captured
    PCM and per-pump `PumpReport`s are exposed for
    Expected/Observed comparison.
  - `input` — [`InputPipeline`]: `Device::open_input_stream` feeds a
    bounded `InputCapture` pool; each captured block is processed
    exactly once through the typed runtime boundary
    (`input_writer` → `RtRuntime::process` → `output_reader`).
    `InputCallbackInfo` (timestamp/clock domain/frame position/
    optional capture timestamp) is preserved verbatim.
  - `midi` — [`MidiLeg`]: `MockBackend`/`MockPort` + `MidiFeed`
    through production `MidiInputEnumerator` → `MidiInputPort::open` →
    `InputStage`. `InputTx::pump` is `pub(crate)`, so a test-owned
    `InputPump` mirrors its algorithm while driving the public
    `InputStage::collect_block` contract.
  - `common` — independent Expected/Observed helpers
    (`expected_block`, `expected_fixture_block`, `scale_expected`,
    `compare_planar`, `compare_pcm_block`). Expected values come from
    `pcm_reference_block`, never from the consumer under test.

## Non-responsibilities (out of scope)

MIDI `attach()`/real endpoint wiring (the `InputStage`-direct leg
stops at the public collect contract — owned by the Issue #630
runtime-integration slice), Windows WASAPI/WinMM virtual endpoints
and real VST3/CLAP ABI plugins (EXT-L3, Issue #629 area), real
OS/GUI/plugin-ABI evidence (GUI-L2/EXT-L3), and MIDI-side
cancellation injection (deferred — audio streams only). Consumer
oracles live in the consumer repos (AudioLens/Synvelis). No new CI
lane is required: the crate's tests are collected by the existing
`mise run test`/`mise run test-rt` tasks.

## Honest limits of `pipeline`

- **`Session::open` installs a no-op error callback** — `DeviceError`
  values fired inside the production output callback are not
  observable through the session surface; only `PumpReport` counts
  (`errors_fired`, `invalidated`) survive. Recorded as a finding;
  production code is unchanged.
- **`InputPump` mirrors production `InputTx::pump`** — the algorithm
  is duplicated because the production type is `pub(crate)`. Drift
  risk is accepted for this slice and noted in
  `docs/impl/ir/issue-628-pipeline-wiring.md`.
- **Deterministic-backend evidence is OS-L1 only** — scripted callback
  firing is manual (`pump_*_next` on the test driver thread); real
  RT-thread scheduling, real xruns, duplex, and latency are out of
  scope.

## Honest limits of `backend`

- **OS-L1 evidence only.** Passing tests prove the port *contract* is
  implementable deterministically; they are not evidence about any
  real OS audio path.
- **Callback firing is manual.** `start()` only marks the stream
  started — delivery happens on `pump_*_next`, on the caller's
  thread. The `rt_` alloc-free proof covers the callback-firing
  section, not RT-thread scheduling.
- **No mid-stream format change.** The contract fixes `format()` at
  open; rate/channel changes are `ClockDiscontinuity` + re-open.
- **Honest fault mapping.** Permission denial maps to
  `Backend("permission_denied")`/`DeviceBusy`; queue-full to
  `Backend("queue_full")`/`Xrun` (audio) or `MockPort`
  `inject_dropped`/`inject_unsupported` (MIDI). Cancellation maps to
  an aborted delivery + `Backend("cancelled")` + stream termination —
  an `Error` fault carrying `Backend("cancelled")` alone does *not*
  terminate the stream.

## Verification levels

Success here is OS-L1 deterministic evidence only — never a claim of
real OS API / GUI / plugin ABI success (POLICIES.md 2026-11-15
virtual-first terms).

RT contract: clock reads, PCM `fill`, the `TraceSink` path, and the
three `pipeline` callback/processing legs are allocation-free/
non-blocking/I/O-free — proven by `rt_`-prefixed `assert_no_alloc`
tests collected by `mise run test-rt`.

See `docs/{specs,designs,plans,tasks,test-specs}/issue-628-close-out.md`
(close-out slice); `docs/{specs,designs,plans,test-specs}/issue-628-pipeline-wiring.md`
(third slice); `docs/{specs,designs,plans,test-specs}/issue-628-scripted-backend.md`
(second slice);
`docs/{specs,designs,plans,test-specs}/issue-628-deterministic-test-runtime.md`
(first slice).
