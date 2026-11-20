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
macOS) that hosts seven plugins (twelve classes):

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
| `test_instrument` ("Audio Module Class") | component | `IComponent` + `IAudioProcessor` + `IProcessContextRequirements` (musical-time fields) |
| `test_instrument Controller` ("Component Controller Class") | controller | `IEditController` |
| `test_spawn_fail` ("Audio Module Class") | component | enumerable; `createInstance` always declines (spawn-rejection oracle) |
| `test_stateful_single` ("Audio Module Class") | component **and** controller | one COM object: `IComponent` + `IAudioProcessor` + `IProcessContextRequirements` + `IEditController`; `getControllerClassId` reports its own CID |

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
  (plain `[0.0, 8.0]`, default `0.0` = `none`).
- The parameter decodes `floor(plain)` clamped to `[0, 8]` — eight
  fault bands dispatched inside the arming block's `process()`:

  | plain band | mode | behavior |
  | --- | --- | --- |
  | `[0.0, 1.0)` | `none` | pass-through (default, fully inert) |
  | `[1.0, 2.0)` | `abort` | `std::process::abort()` |
  | `[2.0, 3.0)` | `exit` | `std::process::exit(round((plain - 2) * 100))` |
  | `[3.0, 4.0)` | `hang` | unresponsive wait (never returns) |
  | `[4.0, 5.0)` | `error` | `kResultFalse` — a defined rejection, not a crash |
  | `[5.0, 6.0)` | `rt_alloc` | bounded heap round-trip; `200.0` marker in the first output sample |
  | `[6.0, 7.0)` | `bad_time` | claims an infinite tail via `getTailSamples()` |
  | `[7.0, 8.0]` | `invalid` | fills every output channel/sample with `NaN` |

  `plain = normalized * 8.0`, so the normalized arm values are
  `abort` `0.125`, `exit(0)` `0.25`, `exit(42)` `0.3025`, `hang`
  `0.375`, `error` `0.5`, `rt_alloc` `0.625`, `bad_time` `0.75`,
  `invalid` `0.875`.

- Unarmed (`none`) it is pure pass-through; an in-band `ParamValue`
  event arms the dispatch — the L1 out-of-process
  crash/hang/timeout/isolation oracle. `abort`/`exit`/`hang` are
  L1-only faults; `error`/`rt_alloc`/`bad_time`/`invalid` are
  nonfatal and disarm back to pass-through.

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
- Popup surface: `onKeyDown` with `key_code == VK_P` (`0x50`)
  toggles a plugin-owned `WS_POPUP | WS_VISIBLE | WS_CLIPSIBLINGS`
  HWND titled `rvf-test-editor-popup-{seq}`, owned by the editor
  child's root ancestor so it dies with the editor. Open/close
  record `po:{seq}`/`pc:{seq}` tokens in the same event log the
  `kd:` tokens write to the window title; `removed()` folds any
  open popup before destroying the child. Off Windows no HWND is
  created but the logical `po:`/`pc:` tokens are still recorded.
- Event log: the fixture's only observable channel is the child
  HWND's window text — `rvf-test-editor-attach-{seq}` followed by
  `|` and the comma-joined event tokens, so a host observer reads
  the whole record through `GetWindowTextW`:

  | token | verb |
  | --- | --- |
  | `f:{state}` | `onFocus` (`1` focused, `0` unfocused) |
  | `kd:{key:04x}:{key_code:04x}:{modifiers:04x}` | `onKeyDown` |
  | `ku:{key:04x}:{key_code:04x}:{modifiers:04x}` | `onKeyUp` |
  | `po:{seq}` / `pc:{seq}` | popup open / close (see above) |

## `test_stateful` semantics

- Same bus shape, gain parameter (plain `[0.0, 2.0]`, default
  `1.0`) and DSP law as `test_gain` — the class exists to carry real
  state, not new DSP.
- Component and controller each persist a 20-byte v2 payload
  `[u32 LE "RFST"][u32 LE version=2][f64 LE][u32 LE FNV-1a-32
  checksum of the first 16 bytes]` through `IBStream`: the component
  stores the **plain** gain, the controller stores the
  **normalized** gain, and `setComponentState` converts between the
  two domains. Restore accepts three provenances — canonical v2,
  16-byte v1 blobs (`version=1`, migrated: the next save emits
  canonical v2), and v2 blobs with a valid checksum plus an unknown
  extension tail (degraded/forward-compatible: the tail is dropped
  on the next save). Wrong magic, length, version, checksum or a
  non-finite value is a defined rejection that leaves the stored
  gain unchanged; finite out-of-range values clamp to the
  parameter range.
- Unlike `test_gain`, activation does **not** reset the gain — a
  restored state survives `setActive`, which is what makes the
  save → mutate → restore → save round-trip meaningful.
- Preset surface: the controller carries `IUnitInfo` +
  `IUnitHandler`/`IUnitHandler2` — one root unit (`kRootUnitId`)
  and one `"Factory"` program list (id `0`) holding the named
  programs `Default` `1.0`, `Half` `0.5`, `Boost` `1.5`, `Mute`
  `0.0` — and the component carries `IProgramListData`/`IUnitData`.
  Program and unit data payloads reuse the same `RFST` codec as
  the state verbs — a preset is a serialized gain — and
  `IUnitData` on the root unit carries the live gain.
  `IUnitInfo::setUnitProgramData` is honestly `kNotImplemented`;
  the supported write path is the component's data verbs.
- The stateless classes keep `kNotImplemented` on the state verbs;
  `test_stateful` is the suite's real-ABI state oracle.

## `test_instrument` semantics

- Same stereo `kMain` bus pair and the same `Gain` parameter as
  `test_gain` (`id` `0`, plain `[0.0, 2.0]`, default `1.0`,
  `plain = normalized * 2`). Unlike every other class it also
  declares `IProcessContextRequirements` musical-time fields and
  drains `IEventList` MIDI from `ProcessData.inputEvents` into
  neutral note/CC events — the suite's MIDI + transport oracle.
  MIDI arrives through the process call's event list; the class
  declares no event bus. Reported subCategories are
  `"Instrument|Fx"` (every other audio class reports `"Fx"`).
- Voice law: up to 8 polyphonic triangle voices at
  `440 * 2^((key - 69) / 12)` (equal temperament, `f64`
  coefficient table — every platform computes the identical
  IEEE-754 result), a `u64` Q32.32 phase accumulator and no
  transcendental calls on the audio path. `NoteOn` retriggers the
  voice holding the same key, else takes the first inactive
  voice, else steals the oldest; `velocity == 0` follows the
  running-status convention and behaves as note-off. `NoteOff`
  starts a fixed 2400-frame linear release. `ControlChange` 7
  sets master volume (default `u32::MAX` = unity) and CC 1 the
  tremolo depth (default `0` = off); every other controller,
  pitch-bend, pressure and program event is a defined no-op.
- Mix law: `out[c][n] = voice_sum * gain * volume * tremolo +
  click[n]` where `tremolo = 1 - depth * (0.5 + 0.5 * lfo)` with a
  4 Hz triangle LFO (`lfo ∈ [-1, 1]`), so depth `0` is
  transparent and depth `1` pulses between `0` and `1`; the mono
  voice sum is written identically to every output channel.
- Transport click oracle: while transport is `Playing` with full
  musical timing (position ticks, tempo and ppqn all valid), a
  single-frame impulse of amplitude `0.5 * volume` fires at every
  ppqn-tick boundary strictly inside the block — `Stopped`/
  `Paused`, missing musical info or a zero tempo produce silence.
  Tick→frame conversion is exact `i128` ceiling division and no
  click state crosses block boundaries, so loop/seek is exercised
  purely through the per-block transport snapshot.

## `test_spawn_fail` semantics

- The listed-but-uncreatable class: `countClasses`/`getClassInfo*`
  enumerate it (`"Audio Module Class"`, subCategories `"Fx"`,
  `kManyInstances`) but `createInstance` deliberately declines
  with `kInvalidArgument` — the suite's spawn-phase
  `CreateInstance`-rejection oracle. Scan-level enumeration still
  sees the class, so a host's create-failure path is exercised
  without breaking discovery (through the Ruviform host bridge the
  deferred ABI create surfaces as `Initialize` → `PluginFailed`).

## `test_stateful_single` semantics

- The suite's **single-component** class: one COM object implements
  `IComponent`, `IAudioProcessor` and `IEditController`, and
  `getControllerClassId` reports the class's *own* CID — the VST3
  discriminator that drives a conforming host's `single_component`
  arm (controller = a cast on the component object, one
  `initialize`/`terminate`, no `IConnectionPoint` pairing, an empty
  controller payload in framed state). Every other class in the
  suite is a distributed pair and can never reach that arm.
- Same gain parameter, DSP law, 20-byte RFST v2 state blob (with
  the same v1-migration and degraded-tail acceptance) and
  activation-does-not-reset law as `test_stateful` — the class
  exists to vary the *topology*, not the DSP or the codec. The gain
  lives in one shared cell bound to both the component state verbs
  and the controller value/state verbs: a single component has one
  source of truth, not a mirror that can drift.
- Deliberate strictness for verification: `terminate` is legal only
  while initialized (a host double-`terminate` on the shared object
  is a defined rejection, not a silent accept), and
  `setComponentHandler` enforces the VST3 once-contract (a second
  call is a defined rejection). The controller state verbs stay
  honest — a conforming host never calls them on a single
  component, but they are real surface with specified semantics.

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

The module ships `test_gain`, `test_fault`, `test_editor`,
`test_stateful`, `test_instrument`, `test_spawn_fail`, and
`test_stateful_single`. The stateless classes report
`kNotImplemented` on the state verbs honestly. Out-of-process
editor attach and a real-HWND `open_gui` attach's Pass evidence
belong to the Issue `scarecrow173/ruviform#631`
real-dock lane; this crate's default test suite pins only the
defined-rejection paths headlessly.
