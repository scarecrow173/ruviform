//! `ruviform-media-core` — shared media value types for the Ruviform media
//! foundation: rational time ([`TimeBase`], [`MediaTime`],
//! [`MediaDuration`], [`MediaRange`], [`Rounding`]), container-level stream
//! descriptors ([`StreamId`], [`StreamKind`], [`StreamDescriptor`]), the
//! media-wide [`Error`]/[`Result`] types, display-side [`Timecode`]
//! conversion (including SMPTE drop-frame labeling) and transport state
//! ([`TransportStatus`], [`TransportSnapshot`]).
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
//! the guaranteed scope is defined per API.
//!
//! Contract source: `docs/designs/ruviform-architecture-design-baseline.md`
//! (crate table A.2, "時間モデル") and
//! `docs/adr/realtime-contract-and-verification-policy.md`.

// The public names and shapes below are fixed API decisions of the data
// model: they deliberately echo their module (`stream::StreamId`,
// `time::MediaTime`, …) while being re-exported at the crate root, and as
// a `publish = false` workspace-internal value-type crate this API relies
// on exhaustive matching and literal construction rather than
// `#[non_exhaustive]` forward-compat guards.
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::exhaustive_enums)]
#![allow(clippy::exhaustive_structs)]

mod error;
mod stream;
mod time;
mod timecode;
mod transport;

pub use error::{Error, Result};
pub use stream::{StreamDescriptor, StreamId, StreamKind};
pub use time::{MediaDuration, MediaRange, MediaTime, Rounding, TimeBase};
pub use timecode::Timecode;
pub use transport::{TransportSnapshot, TransportStatus};

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
        let rate = base(30_000, 1_001);
        let frame_base = base(1_001, 30_000);
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
