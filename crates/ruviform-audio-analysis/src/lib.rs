//! `ruviform-audio-analysis` — audio analysis layer above
//! `ruviform-dsp` kernels.
//!
//! # Responsibility
//!
//! Analyzers that turn PCM blocks into measurements — level
//! metering, FFT spectrum analysis and UI waveform-peak data —
//! each following the `prepare` (allocating, non-RT) / `process`
//! (pre-allocated, RT-suitable) separation:
//!
//! - [`LevelMeter`] — per-channel peak/RMS snapshot over one block.
//! - [`SpectrumAnalyzer`] — windowed FFT magnitude spectrum
//!   (`n/2+1` bins, unnormalized).
//! - [`WaveformPeaks`] — min/max decimation producing drawing data.
//!
//! Conversion DSP stays in `ruviform-dsp`; this crate owns analyzer
//! state and snapshots only. No I/O, caching or RT→non-RT handoff
//! channels live here — consumers move snapshots over the existing
//! `ruviform-rt` conventions.
//!
//! # Permitted dependencies
//!
//! `ruviform-audio-core` (buffer views), `ruviform-dsp` (`Fft` /
//! `Window` kernels) and `ruviform-media-core` (`Error` / `Result`).
//! Same-tier foundations (`ruviform-media-io`,
//! `ruviform-video-proc`, …) must not be depended on.
//!
//! # RT contract
//!
//! `process`-family methods are RT-suitable: they write only into
//! buffers allocated at `prepare` time and never allocate, block,
//! or perform I/O. Constructors/`prepare` and
//! [`WaveformPeaks::peaks`] are non-RT only (allocation).

pub mod level;
pub mod peaks;
pub mod spectrum;

pub use level::{ChannelLevel, LevelMeter};
pub use peaks::{PeakPair, WaveformPeaks};
pub use spectrum::SpectrumAnalyzer;
