# ruviform-audio-test

Test-only deterministic Audio/MIDI test runtime core (Issue #628
first slice).

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

## Non-responsibilities (later slices)

Scripted port-trait backends (`DeviceEnumerator`/`Device`/streams),
fault injection (loss/xrun/disconnect/permission/queue-full),
production pipeline wiring (Input→Recording, Output→Playback,
MIDI→Timestamp/Host adapter), CI lane integration, consumer oracles
(AudioLens/Synvelis), Windows WASAPI/WinMM virtual endpoints and real
VST3/CLAP ABI plugins (EXT-L3), JSON metadata serialization.

## Verification levels

Success here is OS-L1 deterministic evidence only — never a claim of
real OS API / GUI / plugin ABI success (POLICIES.md 2026-11-15
virtual-first terms).

RT contract: clock reads, PCM `fill`, and the `TraceSink` path are
allocation-free/non-blocking/I/O-free — proven by `rt_`-prefixed
`assert_no_alloc` tests collected by `mise run test-rt`.

See `docs/specs/issue-628-deterministic-test-runtime.md`,
`docs/designs/issue-628-deterministic-test-runtime-design.md`,
`docs/plans/issue-628-deterministic-test-runtime-plan.md`, and
`docs/test-specs/issue-628-deterministic-test-runtime.md`.
