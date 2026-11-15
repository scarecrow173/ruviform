# ruviform-plugin-test-vst3

**Test-only crate.** This package builds real VST3 module binaries that
exist solely as deterministic test fixtures for verifying the Ruviform
production plugin host — they are never product distribution artifacts,
no production crate depends on this crate, and nothing here ships to
consumers (Issue #630).

## What it provides

A single VST3 module binary exporting `GetPluginFactory` plus the
platform module hooks (`InitDll`/`ExitDll` on Windows,
`ModuleEntry`/`ModuleExit` on Linux, `bundleEntry`/`bundleExit` on
macOS) that hosts four plugins (eight classes):

| Class | CID role | Surface |
| --- | --- | --- |
| `test_gain` ("Audio Module Class") | component | `IComponent` + `IAudioProcessor` + `IProcessContextRequirements` |
| `test_gain Controller` ("Component Controller Class") | controller | `IEditController` |
| `test_fault` ("Audio Module Class") | component | `IComponent` + `IAudioProcessor` + `IProcessContextRequirements` |
| `test_fault Controller` ("Component Controller Class") | controller | `IEditController` |
| `test_editor` ("Audio Module Class") | component | `IComponent` + `IAudioProcessor` + `IProcessContextRequirements` |
| `test_editor Controller` ("Component Controller Class") | controller | `IEditController` + `createView("editor")` → `IPlugView` |
| `test_stateful` ("Audio Module Class") | component | `IComponent` + `IAudioProcessor` + `IProcessContextRequirements` + `IComponent::setState`/`getState` |
| `test_stateful Controller` ("Component Controller Class") | controller | `IEditController` + `setComponentState`/`setState`/`getState` |

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

## `test_fault` semantics

- Same bus shape as `test_gain`; one stepped `Fault` parameter
  (plain `[0.0, 4.0]`, default `0.0` = `none`).
- Unarmed (`none`) it is pure pass-through; an in-band `ParamValue`
  event arms `abort`/`exit`/`hang`/`error` dispatch inside the
  arming block's `process()` — the L1 out-of-process
  crash/hang/timeout/isolation oracle. `error` returns `kResultFalse`
  (a defined rejection, not a crash); `abort`/`exit`/`hang` are
  L1-only faults.

## `test_editor` semantics

- Parameterless pass-through processor — the class exists to carry
  the suite's `IPlugView` editor fixture, not to exercise DSP.
- `createView("editor")` returns an owned `IPlugView`; every other
  view name returns null. The view reports a fixed 640x480 size,
  refuses resize (`canResize` → `kResultFalse`, symmetric with the
  host bridge's `resizeView` refusal), and acknowledges the
  notification verbs.
- On Windows, `attached(parent, "HWND")` creates a real
  `WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS` HWND under the
  host-provided parent — the AC-4 plugin-fixture half. Null,
  non-window, wrong-platform, repeated-attach, and creation-failure
  inputs are honest `kResultFalse` rejections; off Windows every
  attach is a defined rejection. `removed()` destroys the child and
  always reports `kResultOk`.

## `test_stateful` semantics

- Same bus shape, gain parameter (plain `[0.0, 2.0]`, default
  `1.0`) and DSP law as `test_gain` — the class exists to carry real
  state, not new DSP.
- Component and controller each persist a 16-byte payload
  `[u32 LE "RFST"][u32 LE version=1][f64 LE]` through `IBStream`:
  the component stores the **plain** gain, the controller stores the
  **normalized** gain, and `setComponentState` converts between the
  two domains. Wrong length, magic, version or a non-finite value is
  a defined rejection; finite out-of-range values clamp to the
  parameter range.
- Unlike `test_gain`, activation does **not** reset the gain — a
  restored state survives `setActive`, which is what makes the
  save → mutate → restore → save round-trip meaningful.
- The stateless classes keep `kNotImplemented` on the state verbs;
  `test_stateful` is the suite's real-ABI state oracle.

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

This slice ships `test_gain`, `test_fault`, `test_editor`, and
`test_stateful`. The stateless classes report `kNotImplemented` on
the state verbs honestly. CLAP `test_editor`, the `test_instrument`
class, out-of-process editor attach, and the `ruviform-audio-test`
runtime integration are subsequent slices of the same suite. A real-HWND
`open_gui` attach's Pass evidence belongs to the Issue `scarecrow173/ruviform#631`
real-dock lane; this crate's default test suite pins only the
defined-rejection paths headlessly.
