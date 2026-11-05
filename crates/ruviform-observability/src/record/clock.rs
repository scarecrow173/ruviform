//! Clock context — the bounded clock-domain stamp a record carries.
//!
//! A [`ClockContext`] composes `ruviform-media-core`'s single clock
//! vocabulary: an [`InputStamp`] (domain + ticks + timebase + provenance)
//! plus the [`ClockEpochId`] of the domain incarnation the stamp belongs
//! to. Reusing that vocabulary keeps a record's clock reading in the same
//! identity space as device stamps and `EngineFrame`s — this crate never
//! defines a second clock vocabulary.
//!
//! **Conversion is intentionally absent.** A `ClockContext` asserts what
//! was true when the record was written; it is not a wall-clock or
//! monotonic timestamp and offers no API that converts or fabricates one
//! after the fact. The only stamp→frame conversion authority is
//! `ruviform_media_core::map_stamp_to_frame` (with its [`ClockAnchor`]
//! calibration and `MapRejection` for the unconvertible case).
//!
//! [`ClockAnchor`]: ruviform_media_core::ClockAnchor

use ruviform_media_core::{ClockDomainId, ClockEpochId, InputStamp, TimeBase, TimestampProvenance};

/// A bounded clock-domain stamp captured at record time.
///
/// `stamp` carries the *native* reading — which clock domain produced it
/// ([`ClockDomainId`]), the position in that domain (`ticks`), the
/// timebase those ticks are denominated in ([`TimeBase`], e.g. a sample
/// rate), and [`TimestampProvenance`] — how much of the reading is a
/// carried fact versus a derived or estimated value. `epoch`
/// distinguishes incarnations of that clock domain: stamps from
/// different epochs are never comparable.
///
/// Records carry `Option<ClockContext>`: `None` means *unstamped* — the
/// record legitimately has no clock reading — which must not be conflated
/// with `Some` carrying [`TimestampProvenance::Estimated`] (a reading
/// that exists but is an estimate).
///
/// **RT contract:** RT-suitable — `Copy` value type, `const`, pure.
///
/// # Examples
///
/// ```
/// use ruviform_media_core::{
///     ClockDomainId, ClockEpochId, InputStamp, TimeBase, TimestampProvenance,
/// };
/// use ruviform_observability::record::ClockContext;
///
/// let ctx = ClockContext::new(
///     InputStamp {
///         ticks: 96_000,
///         base: TimeBase::new(1, 48_000).unwrap(),
///         domain: ClockDomainId::SAMPLE_CLOCK,
///         provenance: TimestampProvenance::Carried,
///     },
///     ClockEpochId::new(3),
/// );
/// assert_eq!(ctx.domain(), ClockDomainId::SAMPLE_CLOCK);
/// assert_eq!(ctx.position(), 96_000);
/// assert_eq!(ctx.epoch(), ClockEpochId::new(3));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClockContext {
    stamp: InputStamp,
    epoch: ClockEpochId,
}

impl ClockContext {
    /// Composes a clock context from the native `stamp` and the `epoch`
    /// of the clock domain incarnation it belongs to.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn new(stamp: InputStamp, epoch: ClockEpochId) -> Self {
        Self { stamp, epoch }
    }

    /// The native clock-domain stamp (domain + ticks + base +
    /// provenance).
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn stamp(self) -> InputStamp {
        self.stamp
    }

    /// The clock-domain incarnation this stamp belongs to. Stamps from
    /// different epochs are never comparable.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn epoch(self) -> ClockEpochId {
        self.epoch
    }

    /// The clock domain that produced the stamp —
    /// `stamp().domain`.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn domain(self) -> ClockDomainId {
        self.stamp.domain
    }

    /// The tick position in `domain`'s clock — `stamp().ticks`.
    /// For a sample clock this is the sample/frame position.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn position(self) -> u64 {
        self.stamp.ticks
    }

    /// The timebase `position` is denominated in — `stamp().base`
    /// (e.g. `1/48000` for a 48 kHz sample clock).
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn time_base(self) -> TimeBase {
        self.stamp.base
    }

    /// How much of the stamp is a carried fact versus an estimate —
    /// `stamp().provenance`.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn provenance(self) -> TimestampProvenance {
        self.stamp.provenance
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruviform_media_core::{
        ClockDomainId, ClockEpochId, InputStamp, TimeBase, TimestampProvenance,
    };

    fn stamp(ticks: u64, domain: ClockDomainId, provenance: TimestampProvenance) -> InputStamp {
        InputStamp {
            ticks,
            base: TimeBase::new(1, 48_000).unwrap(),
            domain,
            provenance,
        }
    }

    #[test]
    fn clock_context_preserves_domain_epoch_position_base_provenance() {
        let ctx = ClockContext::new(
            stamp(
                48_000,
                ClockDomainId::SAMPLE_CLOCK,
                TimestampProvenance::Carried,
            ),
            ClockEpochId::new(7),
        );
        assert_eq!(ctx.domain(), ClockDomainId::SAMPLE_CLOCK);
        assert_eq!(ctx.epoch(), ClockEpochId::new(7));
        assert_eq!(ctx.position(), 48_000);
        assert_eq!(ctx.time_base(), TimeBase::new(1, 48_000).unwrap());
        assert_eq!(ctx.provenance(), TimestampProvenance::Carried);
        assert_eq!(ctx.stamp().ticks, 48_000);
    }

    #[test]
    fn clock_context_distinguishes_domains_epochs_and_provenance() {
        let carried = ClockContext::new(
            stamp(0, ClockDomainId::SAMPLE_CLOCK, TimestampProvenance::Carried),
            ClockEpochId::new(1),
        );
        let other_domain = ClockContext::new(
            stamp(0, ClockDomainId::MONOTONIC, TimestampProvenance::Carried),
            ClockEpochId::new(1),
        );
        let other_epoch = ClockContext::new(
            stamp(0, ClockDomainId::SAMPLE_CLOCK, TimestampProvenance::Carried),
            ClockEpochId::new(2),
        );
        let estimated = ClockContext::new(
            stamp(
                0,
                ClockDomainId::SAMPLE_CLOCK,
                TimestampProvenance::Estimated,
            ),
            ClockEpochId::new(1),
        );
        assert_ne!(carried, other_domain);
        assert_ne!(carried, other_epoch);
        assert_ne!(carried, estimated);
    }

    #[test]
    fn unstamped_is_distinct_from_estimated() {
        let unstamped: Option<ClockContext> = None;
        let estimated = Some(ClockContext::new(
            stamp(0, ClockDomainId::UNKNOWN, TimestampProvenance::Estimated),
            ClockEpochId::new(1),
        ));
        assert_ne!(unstamped, estimated);
    }

    #[test]
    fn rt_clock_context_ops_allocate_nothing() {
        use assert_no_alloc::assert_no_alloc;

        let ctx = ClockContext::new(
            stamp(1, ClockDomainId::SAMPLE_CLOCK, TimestampProvenance::Carried),
            ClockEpochId::new(1),
        );
        assert_no_alloc(|| {
            let _ = ctx.stamp();
            let _ = ctx.epoch();
            let _ = ctx.domain();
            let _ = ctx.position();
            let _ = ctx.time_base();
            let _ = ctx.provenance();
        });
    }
}
