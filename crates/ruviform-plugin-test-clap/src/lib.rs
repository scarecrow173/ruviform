//! `ruviform-plugin-test-clap` — the Ruviform **test-only** CLAP test
//! plugin suite (Issue #630, CLAP slice: `test_gain`).
//!
//! ## Test-only declaration
//!
//! This crate builds a real CLAP shared object (`.clap` cdylib) that
//! exists **only as a test fixture** for verifying the Ruviform
//! production plugin host — it is never a product distribution
//! artifact, no production crate depends on this crate, and nothing
//! here ships to consumers. The binary is a deterministic,
//! license-free stand-in for commercial plugins so host discovery /
//! instantiation / processing ABI paths can be exercised without
//! third-party plugins.
//!
//! ## Independence boundary
//!
//! The plugin-side implementation (DSP law, lifecycle mapping, port
//! and parameter contracts) is written **directly** on the `clack`
//! binding crates plus the neutral `ruviform-plugin-core` /
//! `ruviform-audio-core` contracts — it deliberately does not reuse
//! `ruviform-plugin-clap` internals (which are `pub(crate)` anyway),
//! so host↔plugin round trips through this module are not a
//! self-verification loop of a single adapter implementation. The
//! only shared layer is the `clack-*` `=0.2.0` binding crates — the
//! SDK-equivalent binding layer recorded in
//! `docs/designs/issue-630-test-plugin-suite.md`.
//!
//! ## Module contents (this slice)
//!
//! - `test_gain` — one stereo main audio port pair, one automatable
//!   gain parameter; `out[c][n] = in[c][n] * gain(n)` with
//!   sample-accurate parameter application at event frame offsets.
//!
//! All `clack`/`clap-sys` binding types stay private — the crate
//! exports no public Rust API; hosts reach the plugin purely through
//! the ABI entry point below.
//!
//! ## Entry point
//!
//! | Symbol | Status |
//! | --- | --- |
//! | `clap_entry` | required by the CLAP module spec — a
//!   `SinglePluginEntry` factory exposing exactly `test_gain` |
//!
//! ## Process path (RT)
//!
//! `process` validates the `clap_audio_buffer` descriptors at the
//! boundary (frame bounds, channel counts, pointer null/alignment,
//! per-channel range-overlap alias detection) and drives the gain
//! processor through zero-copy
//! `ruviform_audio_core::PlanarBufferView` views over the host's
//! channel buffers — `ChannelPair::InPlace` blocks scale the shared
//! region in place with no aliased `&`/`&mut` pair ever constructed.
//! Channel-pointer, event and sort scratch is per-instance state
//! allocated at `activate`; the process path performs only bounded
//! work and never allocates, blocks or does I/O — verified by the
//! `rt_` `assert_no_alloc` tests.

mod block;
mod gain;
mod plugin;

use clack_plugin::entry::SinglePluginEntry;
use clack_plugin::prelude::clack_export_entry;

pub(crate) use plugin::TestGainClapPlugin;

// The CLAP standard entry point: `#[no_mangle] pub static clap_entry`.
// The macro emits its own `#[allow(unsafe_code)]` for the generated
// static; no unsafe code is written in this crate.
clack_export_entry!(SinglePluginEntry::<TestGainClapPlugin>);
