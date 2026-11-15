# ruviform-plugin-test-clap

**Test-only crate.** This package builds a real CLAP shared object
that exists solely as a deterministic test fixture for verifying the
Ruviform production plugin host — it is never a product distribution
artifact, no production crate depends on this crate, and nothing here
ships to consumers (Issue #630, CLAP slice).

## What it provides

A single CLAP plugin binary exporting `clap_entry` — a
`SinglePluginEntry` factory exposing exactly one plugin:

| Plugin | CLAP id | Surface |
| --- | --- | --- |
| `test_gain` | `org.ruviform-test.clap-test-gain` | `clap_plugin` + `clap.ext.audio-ports` + `clap.ext.params` |

Vendor is `"Ruviform Test"`, name `test_gain`, version `0.0.0`,
features `audio-effect` + `stereo` — distinct from the
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

This slice ships `test_gain` only. State (de)serialization is not
implemented — preset/state coverage is a later plugin class
(`test_stateful`, currently blocked on the production host bridges'
`save_state`/`restore_state` gap). `test_instrument` /
`test_fault` / `test_editor`, out-of-process runner paths, external
SDK validators, and the `ruviform-audio-test` runtime integration
are subsequent slices of the same suite.
