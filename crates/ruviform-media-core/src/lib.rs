//! `ruviform-media-core` — shared media value types for the Ruviform media
//! foundation: rational time ([`TimeBase`], [`MediaTime`],
//! [`MediaDuration`], [`MediaRange`], [`FrameRate`], [`Rounding`]),
//! container-level stream
//! descriptors ([`StreamId`], [`StreamKind`], [`StreamDescriptor`]), the
//! media-wide [`Error`]/[`Result`] types, display-side [`Timecode`]
//! conversion (including SMPTE drop-frame labeling), transport state
//! ([`TransportStatus`], [`TransportSnapshot`]) and musical time
//! ([`Tick`], [`TempoMap`], [`MusicalTimeInfo`] — the [`PPQN`]-tick
//! coordinate system convertible to [`MediaTime`] via a compiled tempo
//! map).
//!
//! Observation vocabulary for media-I/O consumers also lives here:
//! [`Fact`] (a value or the classified reason it is unavailable),
//! [`TimestampProvenance`]/[`TickTimestamp`]/[`MediaTimestamp`]
//! (carried-vs-derived-vs-estimated timestamp annotations) and
//! [`observe_timing`] (frame-rate-agnostic interval/drift/discontinuity
//! observation over raw tick sequences). These types report facts only;
//! interpreting them as validity, confidence or anomaly judgments is the
//! consumer's responsibility.
//!
//! Holds no processing or I/O.
//!
//! ## Allowed dependency layers
//!
//! Primitives (`ruviform-math`, `ruviform-rt`). Neither is used today: the
//! value types need only `core`/`std`. `ruviform-media-core` and
//! `ruviform-exec` must not depend on each other (same-tier dependency is
//! prohibited).
//!
//! ## RT contract
//!
//! All types are `Copy` value types: allocation-free, non-blocking, and
//! every conversion is a pure integer function (`i128`/`u128`
//! intermediates, exact rational math — `f64` is never a stored or core
//! representation). Per-API guarantees are documented as
//! `**RT contract:**` lines on each item, mirroring the RT ADR rule that
//! the guaranteed scope is defined per API. The [`observe_timing`]
//! analysis path is explicitly non-RT (it allocates).
//!
//! Contract source: `docs/designs/ruviform-architecture-design-baseline.md`
//! (crate table A.2, "時間モデル") and
//! `docs/adr/realtime-contract-and-verification-policy.md`.

// The public names and shapes below are fixed API decisions of the data
// model: they deliberately echo their module (`stream::StreamId`,
// `time::MediaTime`, …) while being re-exported at the crate root, and as
// a `publish = false` workspace-internal value-type crate this API relies
// on literal construction for structs. Every public enum carries
// enum-level `#[non_exhaustive]`: exhaustive matching is reserved for
// crate-internal code while external users (sibling crates included) get
// forward compatibility — see
// `docs/adr/public-enum-non-exhaustive-policy.md`.
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::exhaustive_structs)]

mod error;
mod fact;
mod midi;
mod music;
mod observe;
mod stream;
mod time;
mod timecode;
mod timed_input;
mod transport;
mod transport_schedule;

pub use error::{Error, Result};
pub use fact::Fact;
pub use midi::{
    LedgerKey, MidiChannel, MidiController, MidiEvent, MidiGroup, MidiKind, MidiProgram,
    MidiRangeError, MidiValue, NOTE_LEDGER_CAPACITY, NoteKey, NoteLedger,
};
pub use music::{
    BeatPosition, MusicalPosition, MusicalTimeInfo, PPQN, Ppqn, TempoEvent, TempoMap,
    TempoMapBuilder, Tick, TickDuration, TickPosition, TimeSignatureEvent,
};
pub use observe::{
    Discontinuity, IntervalStats, MediaTimestamp, TickTimestamp, TimestampProvenance,
    TimingObservation, observe_timing,
};
pub use stream::{StreamDescriptor, StreamId, StreamKind};
pub use time::{FrameRate, MediaDuration, MediaRange, MediaTime, Rounding, TimeBase};
pub use timecode::Timecode;
pub use timed_input::{
    ClockAnchor, ClockDomainId, InputStamp, MapRejection, MappedStamp, TimedInputEvent,
    map_stamp_to_frame, weaker_provenance,
};
pub use transport::{TransportSnapshot, TransportStatus};
pub use transport_schedule::{
    AppliedReceipt, ApplyDegradation, ApplyStatus, BlockContract, CallPartition, ClockEpochId,
    CommandId, DeliveryPolicy, EngineFrame, FaultKind, InBlockTransportSupport, LoopOccurrence,
    PlanGeneration, RejectReason, ScheduleBudget, ScheduleFault, ScheduledEvent, SpanContext,
    SpanSink, TempoMapRevision, TransportCommand, TransportGeneration, TransportPlan,
    TransportPlanBuilder, TransportScheduler, TransportSpan, event_engine_frame,
    event_generation_matches, media_time_ticks, span_event_window,
};

/// Returns the crate version string.
///
/// **RT contract:** RT-suitable — pure, allocation-free.
///
/// # Examples
///
/// ```
/// assert_ne!(ruviform_media_core::version(), "");
/// ```
#[must_use]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Issue #130 RT-verification lane: `AllocDisabler` swaps the lib
    /// test binary's allocator so `assert_no_alloc` catches any
    /// (de)allocation inside its measurement window. One static per
    /// crate test binary — `music::tests::rt_conversion_no_alloc`
    /// measures against this same allocator; the crate has no
    /// `tests/*.rs` integration binary.
    #[cfg(debug_assertions)]
    #[global_allocator]
    static ALLOC: assert_no_alloc::AllocDisabler = assert_no_alloc::AllocDisabler;

    fn base(numerator: u32, denominator: u32) -> TimeBase {
        TimeBase::new(numerator, denominator).unwrap()
    }

    #[test]
    fn version_is_not_empty() {
        assert_ne!(version(), "");
    }

    /// RT contract: ordering and equality are exact cross-base rational
    /// comparisons — pure integer math, no allocation.
    #[test]
    fn rt_cross_base_ordering() {
        let one_sec = MediaTime::new(1, base(1, 1));
        assert_eq!(one_sec, MediaTime::new(90_000, TimeBase::COMMON_90K));
        assert_eq!(MediaTime::ZERO, MediaTime::new(0, base(1, 48_000)));
        assert!(MediaTime::new(89_999, TimeBase::COMMON_90K) < one_sec);
        assert!(MediaTime::new(90_001, TimeBase::COMMON_90K) > one_sec);
        assert!(MediaTime::new(-1, base(1, 1)) < MediaTime::ZERO);
    }

    /// RT contract: base conversion is exact rational math — `Ok`/`Some`
    /// when representable, explicit rounding otherwise, `Err`/`None` on
    /// overflow.
    #[test]
    fn rt_convert_exact_and_rounding() {
        let one_sec = MediaTime::new(1, base(1, 1));
        assert_eq!(
            one_sec.convert_exact(TimeBase::COMMON_90K),
            Some(MediaTime::new(90_000, TimeBase::COMMON_90K))
        );

        let half = MediaTime::new(1, base(1, 2)); // 0.5 s
        let thirds = base(1, 3); // 1.5 ticks — not representable
        assert_eq!(half.convert_exact(thirds), None);
        assert_eq!(
            half.convert(thirds, Rounding::NearestEven).unwrap().value(),
            2
        );
        assert_eq!(half.convert(thirds, Rounding::Floor).unwrap().value(), 1);

        assert_eq!(
            MediaTime::new(i64::MAX, base(1, 1)).convert_exact(TimeBase::COMMON_90K),
            None
        );
    }

    /// RT contract: checked arithmetic is total — `Err` on overflow, never
    /// a panic or a silent `f64` fallback.
    #[test]
    fn rt_checked_arithmetic() {
        let t = MediaTime::new(4, base(1, 4));
        let d = MediaDuration::new(1, base(1, 1));
        assert_eq!(t.checked_add(d), Ok(MediaTime::new(8, base(1, 4))));
        assert_eq!(t.checked_sub(d), Ok(MediaTime::ZERO));
        assert_eq!(
            MediaTime::new(i64::MAX, base(1, 1)).checked_add(d),
            Err(Error::TimeOverflow)
        );
    }

    /// RT contract: drop-frame labeling is pure integer math; a frame
    /// count round-trips through `floor(t * rate)` to the expected SMPTE
    /// label.
    #[test]
    fn rt_drop_frame_roundtrip() {
        let rate = FrameRate::new(30_000, 1_001).unwrap();
        let frame_base = rate.frame_duration();
        let label = |frames: i64| {
            Timecode::from_media_time_drop(MediaTime::new(frames, frame_base), rate).unwrap()
        };

        // First frame of minute 1 -> 00:01:00:02 (labels 00/01 dropped).
        let tc = label(1_800);
        assert_eq!((tc.minutes(), tc.seconds(), tc.frames()), (1, 0, 2));
        assert!(tc.is_drop_frame());
        // Ten-minute boundary: no drop -> 00:10:00:00.
        assert_eq!(
            (
                label(17_982).minutes(),
                label(17_982).seconds(),
                label(17_982).frames()
            ),
            (10, 0, 0)
        );
        // One label hour == 107_892 real frames -> 01:00:00:00.
        assert_eq!(
            (
                label(107_892).hours(),
                label(107_892).minutes(),
                label(107_892).seconds(),
                label(107_892).frames()
            ),
            (1, 0, 0, 0)
        );
    }
}
