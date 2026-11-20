# ruviform-plugin-test-clap

**Test-only crate.** This package builds a real CLAP shared object
that exists solely as a deterministic test fixture for verifying the
Ruviform production plugin host — it is never a product distribution
artifact, no production crate depends on this crate, and nothing here
ships to consumers (Issue #630, CLAP slice).

## What it provides

A single CLAP plugin binary exporting `clap_entry` — a custom
`Entry` + `PluginFactoryImpl` exposing five plugins through one
factory (`plugin_count() == 5`):

| Plugin | CLAP id | Surface |
| --- | --- | --- |
| `test_gain` | `org.ruviform-test.clap-test-gain` | `clap_plugin` + `clap.ext.audio-ports` + `clap.ext.params` |
| `test_fault` | `org.ruviform-test.clap-test-fault` | `clap_plugin` + `clap.ext.audio-ports` + `clap.ext.params` |
| `test_stateful` | `org.ruviform-test.clap-test-stateful` | `clap_plugin` + `clap.ext.audio-ports` + `clap.ext.params` + `clap.ext.state` |
| `test_instrument` | `org.ruviform-test.clap-test-instrument` | `clap_plugin` + `clap.ext.audio-ports` + `clap.ext.note-ports` + `clap.ext.params` |
| `test_editor` | `org.ruviform-test.clap-test-editor` | `clap_plugin` + `clap.ext.audio-ports` + `clap.ext.gui` |

Vendor is `"Ruviform Test"`, version `0.0.0`, features
`audio-effect` + `stereo` — distinct from the
`ruviform-plugin-clap` adapter fixture's `org.ruviform.clap-gain-poc`
/ `"Ruviform"` descriptor, so both modules coexist in one host scan.

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
- Unarmed (`none`) it is pure pass-through; an in-band `ParamValue`
  event arms `abort`/`exit`/`hang`/`error`/`rt_alloc`/`bad_time`/
  `invalid` dispatch inside the arming block's `process()` — the L1
  out-of-process crash/hang/timeout/isolation oracle. `error` returns
  `CLAP_PROCESS_ERROR` (a defined rejection, not a crash);
  `rt_alloc` performs a bounded heap round-trip in `process()` and
  reports it through a `200.0` marker in the first output sample;
  `bad_time` claims an infinite tail via `CLAP_PROCESS_TAIL`; `invalid`
  fills every output sample with `NaN`. `abort`/`exit`/`hang` are
  L1-only faults; `error`/`rt_alloc`/`bad_time`/`invalid` are
  nonfatal and disarm back to pass-through.

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

This slice ships `test_gain`, `test_fault`, `test_stateful`,
`test_instrument` and `test_editor`. Out-of-process editor attach
(L1 `OpenGui`/`CloseGui` is `HostError::Unsupported` by design), the
CLAP real-attach lane leg, external SDK validators, and consumer
oracle checks are subsequent slices of the same suite.
