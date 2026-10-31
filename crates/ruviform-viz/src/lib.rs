//! `ruviform-viz` — generic interactive visualization primitives for
//! Ruviform (Issue #157).
//!
//! Reusable, application-agnostic primitives that turn chunked
//! random-access data into display-shaped series:
//!
//! - [`source`]: the input ports — [`WaveformSource`] (bounded PCM
//!   range reads) and [`SpectrumSource`] (bounded time×frequency tile
//!   reads, complex or magnitude). Applications implement these over
//!   whatever storage they have; `viz` pulls only the ranges a view
//!   needs and never requires whole-dataset loads.
//! - [`waveform`]: min/max/RMS LOD pyramids ([`WaveformLod`]) built via
//!   chunked reads, plus [`WaveformSeries::view`] — a viewport-driven
//!   decimation that serves extreme zoom-out from the pyramid and
//!   extreme zoom-in straight from the source.
//! - [`spectrogram`]: generic STFT analysis ([`Stft`], powered by
//!   `ruviform-numeric`'s f64 real FFT) and [`Spectrogram`] — a tile
//!   provider that works identically over analyzed data and over an
//!   app-provided [`SpectrumSource`] (no re-analysis of stored
//!   intermediate spectra).
//! - [`timeline`]: coordinate conversion and layout —
//!   [`SampleTimebase`] (sample ↔ seconds, exact via `media-core`
//!   rational time), [`FrameGrid`] (window/hop ↔ sample),
//!   [`Viewport`] (axis ↔ pixel), lane assignment, selection and
//!   hit-testing primitives that carry no diagnostic semantics.
//! - [`display`]: display-only mappings kept separate from numeric
//!   data — [`DbRange`], [`FreqAxis`], [`ColorMap`],
//!   [`Quantization`]. Stored values stay `f64`; rounding happens
//!   only at this boundary.
//! - [`cache`]: [`BoundedCache`], a byte-budgeted LRU for view data.
//! - [`encode`]: binary transfer encoding (`f64`/`f32`/`u8`) and the
//!   [`PagedBytes`] port — intentionally the same shape as
//!   `ruviform_ui::DataSource` so an app wires display payloads to
//!   `rvf://data` with a trivial adapter, while `viz` itself never
//!   depends on `ruviform-ui` or any GUI backend.
//!
//! # Precision contract
//!
//! All internal values are `f64` end to end. Quantization (`f32`,
//! `u8`, dB-range mapping, colormap indexing) happens only at the
//! `display`/`encode` boundary and is always caller-selected — the
//! stored analysis data is never altered. `NaN` is the uniform
//! "missing/invalid" marker: sources report gaps as `NaN`, and it
//! propagates through buckets, spectra and tiles.
//!
//! # Load contract
//!
//! - Queries are bounded by the requested resolution (`bins`,
//!   `cols`/`rows`) — response size scales with the viewport, not the
//!   dataset.
//! - Heavy builds ([`WaveformLod::build`], [`Stft::analyze`]) are
//!   chunked, report progress, and observe a
//!   [`ruviform_exec::CancellationToken`] at chunk boundaries.
//! - `viz` owns no threads, event loops or UI types: the GUI event-loop
//!   non-blocking requirement is satisfied by running these APIs on
//!   job workers (`ruviform-ui`'s Service/Job model) or CLI code.
//!
//! # RT contract
//!
//! **Non-RT only.** Every type in this crate is UI-thread or
//! job-worker-thread only.
//!
//! Contract source: `docs/designs/issue-157-viz-primitives.md`.

// Public names deliberately echo their module (source::WaveformSource,
// waveform::WaveformLod) — the `ruviform-exec` precedent.
#![allow(clippy::module_name_repetitions)]

pub mod cache;
pub mod demo;
pub mod display;
pub mod encode;
pub mod error;
pub mod source;
pub mod spectrogram;
pub mod timeline;
pub mod waveform;

pub use cache::BoundedCache;
pub use demo::DemoWaveform;
pub use display::{ColorMap, DbRange, FreqAxis, LinearRange, Quantization};
pub use encode::{Bytes, LodBytes, PagedBytes, encode_buckets, encode_tile};
pub use error::{Result, VizError};
pub use source::{
    BinAxis, PcmBlock, SliceWaveform, SpectrumData, SpectrumInfo, SpectrumSource, SpectrumTile,
    VecSpectrum, WaveformInfo, WaveformSource,
};
pub use spectrogram::{Aggregate, RowSpec, Spectrogram, Stft, StftConfig, Tile, TileRequest};
pub use timeline::{
    FrameGrid, LaneAssignment, LaneItem, SampleTimebase, Selection, Viewport, assign_lanes,
    hit_test, items_in,
};
pub use waveform::{LodBucket, LodConfig, LodLevel, ViewBin, WaveformLod, WaveformSeries};

/// Returns the crate version string.
///
/// **RT contract:** non-RT only.
#[must_use]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
