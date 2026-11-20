# ruviform-plugin-test-clap

**Test-only crate.** This package builds a real CLAP shared object
that exists solely as a deterministic test fixture for verifying the
Ruviform production plugin host — it is never a product distribution
artifact, no production crate depends on this crate, and nothing here
ships to consumers (Issue #630, CLAP slice).

## What it provides

A single CLAP plugin binary exporting `clap_entry` — a custom
`Entry` + `PluginFactoryImpl` exposing six plugin descriptors
through one factory (`plugin_count() == 6`), plus a second
entry-level factory (`clap.factory.preset-discovery`) that
advertises the `test_stateful` preset catalog:

| Plugin | CLAP id | Surface |
| --- | --- | --- |
| `test_gain` | `org.ruviform-test.clap-test-gain` | `clap_plugin` + `clap.ext.audio-ports` + `clap.ext.params` |
| `test_fault` | `org.ruviform-test.clap-test-fault` | `clap_plugin` + `clap.ext.audio-ports` + `clap.ext.params` |
| `test_stateful` | `org.ruviform-test.clap-test-stateful` | `clap_plugin` + `clap.ext.audio-ports` + `clap.ext.params` + `clap.ext.state` + `clap.ext.preset-load` |
| `test_instrument` | `org.ruviform-test.clap-test-instrument` | `clap_plugin` + `clap.ext.audio-ports` + `clap.ext.note-ports` + `clap.ext.params` |
| `test_editor` | `org.ruviform-test.clap-test-editor` | `clap_plugin` + `clap.ext.audio-ports` + `clap.ext.gui` |
| `test_spawn_fail` | `org.ruviform-test.clap-test-spawn-fail` | descriptor only — `create_plugin` always returns `None` (spawn-rejection oracle) |

Vendor is `"Ruviform Test"`, version `0.0.0`; features are
`audio-effect` + `stereo` for every class except
`test_instrument`, which reports `instrument` + `synthesizer` +
`stereo`. The ids are distinct from the `ruviform-plugin-clap`
adapter fixture's `org.ruviform.clap-gain-poc` / `"Ruviform"`
descriptor, so both modules coexist in one host scan.

## `test_gain` semantics

- One stereo main audio input port and one stereo main audio output
  port (`in_place_pair` declared — in-place blocks are supported).
  No note ports, no MIDI consumption, no transport use, no state
  extension.
- One automatable parameter, id `0`, name `Gain`: plain range
  `[0.0, 2.0]`, default `1.0`, dimensionless. The normalized↔plain
  mapping is deliberately **non-identity** (`plain = normalized * 2`),
  so a host that confuses the two domains produces measurably wrong
  output. On the CLAP wire the `params` extension speaks the plain
  domain; the neutral event path keeps normalized values — the same
  oracle shape as the VST3 `test_gain` slice.
- DSP law: `out[c][n] = in[c][n] * gain(n)` — minimal and
  deterministic, no signal generators. In-place blocks scale the
  shared region (`out[c] *= gain`) without ever constructing aliased
  `&`/`&mut` slices.
- Parameter changes apply **sample-accurately** at the event's frame
  offset; same-offset events are last-wins; zero-frame blocks flush
  offset-0 events into the persisted gain for the next block.
- Activation resets gain to the parameter default — deterministic
  per-activation state, the same law the VST3 slice declares.

## `test_fault` semantics

- Same port shape as `test_gain`; one stepped `Fault` parameter
  (plain `[0.0, 8.0]`, default `0.0` = `none`).
- The parameter decodes `floor(plain)` clamped to `[0, 8]` — eight
  fault bands dispatched inside the arming block's `process()`:

  | plain band | mode | behavior |
  | --- | --- | --- |
  | `[0.0, 1.0)` | `none` | pass-through (default, fully inert) |
  | `[1.0, 2.0)` | `abort` | `std::process::abort()` |
  | `[2.0, 3.0)` | `exit` | `std::process::exit(round((plain - 2) * 100))` |
  | `[3.0, 4.0)` | `hang` | unresponsive wait (never returns) |
  | `[4.0, 5.0)` | `error` | `CLAP_PROCESS_ERROR` — a defined rejection, not a crash |
  | `[5.0, 6.0)` | `rt_alloc` | bounded heap round-trip; `200.0` marker in the first output sample |
  | `[6.0, 7.0)` | `bad_time` | claims an infinite tail via `CLAP_PROCESS_TAIL` |
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
- CLAP-only leg: `bad_time` also pushes a `ParamValue` event
  stamped beyond the block boundary and `invalid` a `ParamValue`
  with a `NaN` value into `events.output`. The Ruviform v1 host
  bridge discards plugin output events (`OutputEvents::void`), so
  only the fixture's ABI emission face is observable — the push is
  best-effort and its failure is ignored (an honest record, not a
  hidden oracle).

## `test_stateful` semantics

- Same port shape, gain parameter and DSP law as `test_gain` — the
  class exists to carry real state, not new DSP.
- `clap.ext.state` `save` writes one plugin-defined 20-byte v2
  stream `[u32 LE "RFST"][u32 LE version=2][f64 LE plain gain]
  [u32 LE FNV-1a-32 checksum of the first 16 bytes]`; `load`
  drains the whole input stream and accepts canonical v2, 16-byte
  v1 blobs (`version=1`, migrated — the next save emits v2), and
  v2 blobs carrying a valid checksum plus an unknown extension
  tail (degraded/forward-compatible — the tail is dropped on the
  next save). Wrong magic, length, version, checksum or a
  non-finite value is a defined rejection that leaves the stored
  gain unchanged; finite out-of-range values clamp to the
  parameter range.
- Unlike `test_gain`, activation does **not** reset the gain — a
  restored state survives `activate`, which is what makes the
  save → mutate → restore → save round-trip meaningful.
- Preset surface: `clap.ext.preset-load` restores a catalog preset
  by `load_key` through the same `clap.ext.state` `load` tail, and
  the entry's `clap.factory.preset-discovery` factory advertises
  the built-in catalog — provider `org.ruviform-test.clap-presets`
  ("Ruviform Test Presets") with four presets (`Default` `1.0`,
  `Half` `0.5`, `Boost` `1.5`, `Mute` `0.0`) tagged to the
  `clap-test-stateful` plugin id, the same names and gains the
  VST3 `"Factory"` program list carries.
- The stateless classes ship no `clap.ext.state` extension, so the
  host bridge reports `HostError::Unsupported` honestly;
  `test_stateful` is the suite's real-ABI state oracle.

## `test_editor` semantics

- Parameterless pass-through DSP (the `test_fault` unarmed law, no
  parameters declared at all) plus the plugin side of
  `clap.ext.gui` — the CLAP counterpart of the VST3 `test_editor`
  fixture.
- `is_api_supported`/`get_preferred_api` answer `win32` only, and
  only on Windows builds — embedded *and* floating; other API
  types and non-Windows builds are honest `false`/`None`.
- `create` → `set_parent` → `show`/`hide` → `destroy` follow the
  production host bridge's `open_gui`/`close_gui` contract. On
  Windows `set_parent` validates the supplied `Window` is a Win32
  window whose handle passes `IsWindow`, then creates a real
  `WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS` child HWND under the
  host parent; `destroy` frees it. An opaque `ParentLease` payload
  (e.g. `ParentLease::new(0, 0)`) is *not* a window and is rejected
  — never treated as one.
- Size verbs are honest: `get_size` reports the current size
  (640x480 initially), `can_resize` accepts and `set_size` records
  the requested size and resizes the attached child HWND —
  `adjust_size`/`get_resize_hints` decline (no constraints declared),
  `set_size` before `create` rejects, `set_scale` accepts only
  finite positive factors.
- Floating mode is the fixture's popup surface: `create` with
  `is_floating` records the mode, `set_transient` accepts only a
  live Win32 host window (embedded mode is the honest refusal),
  `suggest_title` names the popup, and `show` creates a real
  `WS_POPUP` HWND owned by the transient parent (titled
  `rvf-test-editor-popup` or the suggested title) on first call —
  `hide`/`destroy` converge it. `set_parent` on a floating GUI is
  the defined rejection in the opposite direction.
- Win32 calls are confined to `src/editor.rs`'s
  `#[cfg(target_os = "windows")]` `win32` module behind
  `#[expect(unsafe_code)]` + SAFETY comments — the crate's only
  `unsafe`. The window class registers once per module with
  `lpfnWndProc = DefWindowProcW` (a cdylib-registered class outlives
  the module — a module-local `wnd_proc` would dangle after
  `FreeLibrary`), class name `"RuviformClapTestEditor"`, distinct
  from the VST3 fixture's class. There is no message pump or thread
  of our own — the host calls the gui verbs on the parent HWND's
  thread.
- A real parent-HWND `open_gui` **success** path is the
  `scarecrow173/ruviform#631` real-attach lane's evidence, not
  claimed by this fixture's default tests; the `#[ignore]`d
  `set_parent_creates_real_child_hwnd` unit test is the manual
  opt-in check.

## `test_instrument` semantics

- Same stereo main audio port pair and `Gain` parameter as
  `test_gain` (`id` `0`, plain `[0.0, 2.0]`, default `1.0`), plus
  one input note port (`"Notes"`, id `0`, supported dialects
  `CLAP` | `MIDI`, preferred `MIDI`) — the suite's MIDI +
  transport oracle. Structured note events and raw MIDI 1.0 events
  translate into the same neutral kinds the VST3 fixture drains
  from `IEventList`, and the `Process::transport` snapshot maps to
  `TransportInfo` (CLAP carries no ppqn — the fixture fixes
  `960`).
- Voice and mix law are the VST3 fixture's: up to 8 polyphonic
  triangle voices at `440 * 2^((key - 69) / 12)` with a `u64`
  Q32.32 phase accumulator; `NoteOn` retriggers/first-inactive/
  steal-oldest, `velocity == 0` behaves as note-off, `NoteOff`
  starts the fixed 2400-frame linear release; CC7 master volume
  and CC1 tremolo depth are the only live controllers. Mix is
  `out[c][n] = voice_sum * gain * volume * tremolo + click[n]`
  with the 4 Hz triangle LFO, written identically to both output
  channels.
- Transport click oracle: while the transport snapshot reports
  `Playing` with full musical timing, a single-frame impulse of
  amplitude `0.5 * volume` fires at every ppqn-tick boundary
  strictly inside the block (exact `i128` ceiling division);
  `Stopped`/`Paused`, missing musical info or a zero tempo produce
  silence. No click state crosses block boundaries, so loop/seek
  is exercised purely through the per-block snapshot.

## `test_spawn_fail` semantics

- The listed-but-uncreatable plugin: `plugin_descriptor` enumerates
  it (`audio-effect` + `stereo`, vendor `"Ruviform Test"`) but
  `create_plugin` deliberately returns `None` — the suite's
  spawn-phase create-rejection oracle. Scan-level enumeration
  still sees the descriptor, so a host's create-failure path is
  exercised without breaking discovery (through the Ruviform host
  bridge the deferred ABI create surfaces as `Initialize` →
  `PluginFailed`).

## Independence boundary

The plugin-side implementation (DSP law, lifecycle mapping, port and
parameter contracts) is written directly on the `clack-*` `=0.2.0`
binding crates plus the neutral `ruviform-plugin-core` /
`ruviform-audio-core` contracts. It deliberately does not reuse
`ruviform-plugin-clap` internals, so host↔plugin round trips are not
a self-verification loop of a single adapter implementation. The only
shared layer is the `clack` binding crates themselves — the
SDK-equivalent binding layer recorded in
`docs/designs/issue-630-test-plugin-suite.md`.

`ruviform-plugin-clap` appears only as a **dev-dependency**
(`features = ["host"]`) so `tests/host_path.rs` can drive the real
built binary through the production `ClapLibrary` host bridge; the
adapter itself stays pristine and this crate never enters its
dependency tree. `clack-host` is likewise a dev-dependency — its
safe host-side fixtures drive the plugin-side ABI in-process for
unit/lifecycle/RT tests.

## Realtime contract

`process` performs no allocation, blocking, waiting or I/O. Channel
and event scratch is per-instance state sized at `activate`; the
audio path uses zero-copy `PlanarBufferView` views over the host's
channel buffers, with raw-pointer null/alignment/range-overlap
validation before any channel slice is built. Hostile inputs
(oversized frame counts, asymmetric or mixed in-place classification,
f64-only pairs, lying event counts) are validated or bounded at the
boundary. Verified by the `rt_` `assert_no_alloc` tests
(`mise run test-rt`).

## Scope

The module ships `test_gain`, `test_fault`, `test_stateful`,
`test_instrument`, `test_editor` and `test_spawn_fail` through one
plugin factory, plus the `test_stateful` preset catalog through
`clap.factory.preset-discovery`. Out-of-process editor attach
(L1 `OpenGui`/`CloseGui` is `HostError::Unsupported` by design), the
CLAP real-attach lane leg, external SDK validators, and consumer
oracle checks are subsequent slices of the same suite.
