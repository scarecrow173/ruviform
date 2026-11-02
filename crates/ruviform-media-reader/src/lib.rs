//! `ruviform-media-reader` — the generic media read path: sequential
//! decode in source-frame order, a bounded non-RT prefetch, and an RT
//! playback consumer (Issue #197).
//!
//! The crate sits on top of the [`ruviform_media_io`] boundary — a
//! [`SourceReader`] pumps a [`Demuxer`]/[`Decoder`] pair for one stream
//! and delivers decoded samples addressed by *source frame* (sample
//! index at the stream's rate), while a [`Prefetch`] pipeline moves
//! those samples through a fixed [`PcmBlock`] pool across two SPSC
//! rings into a [`PrefetchConsumer`] an audio callback can fill from
//! without allocating, locking or touching the OS.
//!
//! ## Consumer boundary
//!
//! - **Playback (canonical)**: [`SourceReader::read_planar`] and the
//!   prefetch path carry planar `f32` in the stream's declared rate and
//!   layout. This is the bounded-cache representation.
//! - **Observation (native)**: [`DecodeMode::Native`] yields
//!   [`Frame::NativeAudio`] — native-scale `f64`, original
//!   PTS/timebase provenance and native format facts preserved. The
//!   reader never forces the `f32` playback representation onto that
//!   path.
//!
//! ## Frame vocabulary
//!
//! *Source frames* are sample indices of the decoded stream — what the
//! media contains. *Engine frames* are indices of the caller's
//! processing clock. The mapping between them (clip offset, project
//! placement, loop/rate semantics) is an input contract supplied by
//! the caller — see [`FrameMap`]. The reader and prefetch never
//! invent that mapping; they consume positions the caller supplies.
//!
//! ## Fault contract
//!
//! Every span the pipeline cannot serve as decoded content is
//! classified — never silently replaced with "verified" silence:
//! [`SourceFault`] markers carry corrupt/truncated/invalid-sample/
//! unsupported classes, [`FillReport`] distinguishes `written` /
//! `underrun` / `faulted` / `eof` / `mismatch`, and the declared
//! bounded fallback (zero-fill) is always reported through both the
//! report and [`PrefetchTelemetry`].
//!
//! ## RT contract
//!
//! This is a *mixed-RT* crate; every public API is labelled
//! per-method:
//!
//! - **non-RT only**: [`SourceReader`], [`PrefetchHandle`],
//!   [`PrefetchProducer`], `Prefetch::manual`/`spawn`, stream
//!   metadata — all may allocate, block on I/O and run unbounded
//!   codec work.
//! - **RT-suitable**: [`PrefetchConsumer::fill`],
//!   [`PrefetchConsumer::discard_pending`],
//!   [`PrefetchConsumer::position`], [`PrefetchConsumer::generation`]
//!   and their report types — no allocation, no locks, no blocking,
//!   no OS I/O, when used under the documented preconditions
//!   (pre-allocated pool, no concurrent access).
//!
//! [`Demuxer`]: ruviform_media_io::Demuxer
//! [`Decoder`]: ruviform_media_io::Decoder

mod mapping;
mod metadata;
mod prefetch;
mod reader;

pub use mapping::{EngineFrame, FrameMap, RequestedRange, SourceFrame};
pub use metadata::{SourceMetadata, stream_metadata};
pub use prefetch::{
    DiscardReport, FaultCounts, FillReport, PcmBlock, Prefetch, PrefetchAudit, PrefetchConfig,
    PrefetchConsumer, PrefetchHandle, PrefetchItem, PrefetchProducer, PrefetchTelemetry,
    PumpOutcome, SourceFault,
};
pub use reader::{ReadOutcome, SeekReceipt, SourceReader};
