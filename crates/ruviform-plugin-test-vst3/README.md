# ruviform-plugin-test-vst3

**Test-only crate.** This package builds real VST3 module binaries that
exist solely as deterministic test fixtures for verifying the Ruviform
production plugin host — they are never product distribution artifacts,
no production crate depends on this crate, and nothing here ships to
consumers (Issue #630, first slice).

## What it provides

A single VST3 module binary exporting `GetPluginFactory` plus the
platform module hooks (`InitDll`/`ExitDll` on Windows,
`ModuleEntry`/`ModuleExit` on Linux, `bundleEntry`/`bundleExit` on
macOS) that hosts two classes:

| Class | CID role | Surface |
| --- | --- | --- |
| `test_gain` ("Audio Module Class") | component | `IComponent` + `IAudioProcessor` + `IProcessContextRequirements` |
| `test_gain Controller` ("Component Controller Class") | controller | `IEditController` |

Vendor is `"Ruviform Test"` and the class IDs are fresh `uid`s,
distinct from the `ruviform-plugin-vst3` adapter fixture's `"Gain"` /
`"Ruviform"` classes — both modules coexist in one host scan.

## `test_gain` semantics

- One stereo `kMain` audio input bus and one stereo `kMain` audio
  output bus (mono arrangements also negotiate; the law is
  per-channel). No event buses, no MIDI, no transport.
- One automatable parameter, id `0`, name `Gain`: plain range
  `[0.0, 2.0]`, default `1.0`, dimensionless. The normalized↔plain
  mapping is deliberately **non-identity** (`plain = normalized * 2`),
  so a host that confuses the two domains produces measurably wrong
  output.
- DSP law: `out[c][n] = in[c][n] * gain(n)` — minimal and
  deterministic, no signal generators.
- Parameter changes apply **sample-accurately** at the event's frame
  offset; same-offset events are last-wins; zero-frame blocks flush
  offset-0 events into the persisted gain for the next block.

## Independence boundary

The plugin-side implementation (DSP law, lifecycle mapping, bus and
parameter contracts) is written directly on the `vst3` binding crate
plus the neutral `ruviform-plugin-core` / `ruviform-audio-core`
contracts. It deliberately does not reuse `ruviform-plugin-vst3`
internals, so host↔plugin round trips are not a self-verification loop
of a single adapter implementation. The only shared layer is the
`vst3` crate itself — the SDK-equivalent binding layer recorded in
`docs/designs/issue-630-test-plugin-suite.md`.

`ruviform-plugin-vst3` appears only as a **dev-dependency**
(`features = ["host"]`) so `tests/host_path.rs` can drive the real
built binary through the production `Vst3Library` host bridge; the
adapter itself stays pristine and this crate never enters its
dependency tree.

## Realtime contract

`process` performs no allocation, blocking, waiting or I/O. Channel
and event scratch is per-instance state sized at `setupProcessing`;
the audio path uses zero-copy `PlanarBufferView` views over the host's
`channelBuffers32`, with bounded per-channel bounce buffers for
in-place aliasing. Hostile inputs (null/unaligned/overlapping
pointers, oversized frame counts, lying queue/point counts) are
validated or bounded at the boundary. Verified by the `rt_`
`assert_no_alloc` tests (`mise run test-rt`).

## Scope

This slice ships `test_gain` only. State (de)serialization reports
`kNotImplemented` honestly — preset/state coverage is a later plugin
class (`test_stateful`). CLAP, the `test_instrument` / `test_fault` /
`test_editor` classes, out-of-process runner paths, and the
`ruviform-audio-test` runtime integration are subsequent slices of the
same suite.
