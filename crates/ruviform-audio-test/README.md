# ruviform-audio-test

Test-only deterministic Audio/MIDI test runtime (Issue #628 —
first slice: deterministic core; second slice: scripted port-trait
backend).

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

`publish = false` is inherited from the workspace; `assert_no_alloc`
is a dev-dependency only.

## Responsibilities (this slice)

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
  Pass/Fail/NotRun/Unsupported).
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
  `MidiScenario` → `TimedMidiInput` feed for `MockPort`.

## Non-responsibilities (later slices)

Production pipeline wiring (Input→Recording, Output→Playback,
MIDI→Timestamp/Host adapter), consumer oracles (AudioLens/Synvelis),
CI lane additions, Windows WASAPI/WinMM virtual endpoints and real
VST3/CLAP ABI plugins (EXT-L3), JSON metadata serialization.

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
  `inject_dropped`/`inject_unsupported` (MIDI).

## Verification levels

Success here is OS-L1 deterministic evidence only — never a claim of
real OS API / GUI / plugin ABI success (POLICIES.md 2026-11-15
virtual-first terms).

RT contract: clock reads, PCM `fill`, and the `TraceSink` path are
allocation-free/non-blocking/I/O-free — proven by `rt_`-prefixed
`assert_no_alloc` tests collected by `mise run test-rt`.

See `docs/specs/issue-628-scripted-backend.md`,
`docs/designs/issue-628-scripted-backend.md`,
`docs/plans/issue-628-scripted-backend.md`, and
`docs/test-specs/issue-628-scripted-backend.md` (second slice);
`docs/{specs,designs,plans,test-specs}/issue-628-deterministic-test-runtime.md`
(first slice).
