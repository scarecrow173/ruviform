//! Event record — the generic fixed-size record shape for the RT bridge.
//!
//! [`EventRecord`] is `Copy + Send + 'static`, so it can travel
//! [`crate::rt_bridge::bridge`] directly: a [`RecordHeader`] of metadata
//! (event identity, source identity, domain generation, correlation
//! link, clock context, confidentiality, reporting impact) plus a `u64`
//! payload and a reserved extension field.
//!
//! The shape is the confidentiality boundary: a record can carry a scalar
//! and metadata, never a buffer — raw PCM/signal payload does not fit
//! this shape and must not be forced through it.

use crate::record::{ClockContext, CorrelationLink};
use crate::source::{Confidentiality, ReportingImpact, SourceIdentity};

/// Fixed identifier of a record's event kind.
///
/// Domains allocate their own event-id space; `0` is a valid id. The id
/// names *what happened* — payload semantics are pinned by the pair
/// `(event, version)`.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EventId(u32);

impl EventId {
    /// Wraps `raw` as an [`EventId`].
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    /// The raw 32-bit value.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// The metadata every record carries — fixed size, `Copy`.
///
/// - `event`/`version`: what the record is and which payload semantics
///   the `value` field follows.
/// - `source`: the [`SourceIdentity`] of the registration that produced
///   the record (OBS-02 registration identity — not a correlation
///   token).
/// - `generation`: a *domain-declared* generation the record was
///   produced under (e.g. a plan or session generation). `0` means "no
///   generation asserted". Distinct from [`source::SourceGeneration`]
///   (registration re-mint count) and from [`ClockContext::epoch`]
///   (clock-domain incarnation).
/// - `correlation`: `Some` [`CorrelationLink`] when the record is meant
///   to correlate with others; `None` means *intentionally uncorrelated*
///   — not the same as an unlinked (stale/expired) link.
/// - `clock`: `Some` [`ClockContext`] when a clock-domain stamp was
///   captured; `None` means *unstamped* — not the same as an
///   `Estimated` provenance stamp.
/// - `confidentiality`/`impact`: the record's own classification, reusing
///   the OBS-02 descriptor enums.
///
/// [`source::SourceGeneration`]: crate::source::SourceGeneration
/// **RT contract:** RT-suitable — `Copy` value type; every accessor is
/// `const`, pure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecordHeader {
    event: EventId,
    version: u16,
    source: SourceIdentity,
    generation: u64,
    correlation: Option<CorrelationLink>,
    clock: Option<ClockContext>,
    confidentiality: Confidentiality,
    impact: ReportingImpact,
}

impl RecordHeader {
    /// A header with the minimal required identity — `generation = 0`
    /// (no assertion), uncorrelated, unstamped,
    /// [`Confidentiality::Internal`], [`ReportingImpact::Normal`]. Refine
    /// with the `with_*` setters.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn new(event: EventId, version: u16, source: SourceIdentity) -> Self {
        Self {
            event,
            version,
            source,
            generation: 0,
            correlation: None,
            clock: None,
            confidentiality: Confidentiality::Internal,
            impact: ReportingImpact::Normal,
        }
    }

    /// Asserts the domain generation the record was produced under.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn with_generation(mut self, generation: u64) -> Self {
        self.generation = generation;
        self
    }

    /// Attaches a correlation link (`Some`) or marks the record
    /// intentionally uncorrelated (`None`).
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn with_correlation(mut self, correlation: Option<CorrelationLink>) -> Self {
        self.correlation = correlation;
        self
    }

    /// Attaches a clock-domain stamp (`Some`) or marks the record
    /// unstamped (`None`).
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn with_clock(mut self, clock: Option<ClockContext>) -> Self {
        self.clock = clock;
        self
    }

    /// Sets the record's confidentiality classification.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn with_confidentiality(mut self, confidentiality: Confidentiality) -> Self {
        self.confidentiality = confidentiality;
        self
    }

    /// Sets the record's reporting impact.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn with_impact(mut self, impact: ReportingImpact) -> Self {
        self.impact = impact;
        self
    }

    /// The event kind — **RT contract:** RT-suitable.
    #[must_use]
    pub const fn event(self) -> EventId {
        self.event
    }

    /// The payload-semantics version — **RT contract:** RT-suitable.
    #[must_use]
    pub const fn version(self) -> u16 {
        self.version
    }

    /// The registration identity that produced the record — **RT
    /// contract:** RT-suitable.
    #[must_use]
    pub const fn source(self) -> SourceIdentity {
        self.source
    }

    /// The domain-declared generation (`0` = none asserted) — **RT
    /// contract:** RT-suitable.
    #[must_use]
    pub const fn generation(self) -> u64 {
        self.generation
    }

    /// The correlation link, or `None` when intentionally uncorrelated —
    /// **RT contract:** RT-suitable.
    #[must_use]
    pub const fn correlation(self) -> Option<CorrelationLink> {
        self.correlation
    }

    /// The clock-domain stamp, or `None` when unstamped — **RT
    /// contract:** RT-suitable.
    #[must_use]
    pub const fn clock(self) -> Option<ClockContext> {
        self.clock
    }

    /// The record's confidentiality classification — **RT contract:**
    /// RT-suitable.
    #[must_use]
    pub const fn confidentiality(self) -> Confidentiality {
        self.confidentiality
    }

    /// The record's reporting impact — **RT contract:** RT-suitable.
    #[must_use]
    pub const fn impact(self) -> ReportingImpact {
        self.impact
    }
}

/// The generic fixed-size record — [`RecordHeader`] + `u64` payload +
/// reserved space.
///
/// `reserved` is `[0; 2]` in this revision: extension space for a future
/// revision, whose semantics will be fixed then — producers must not
/// write to it and consumers must not interpret it.
///
/// **RT contract:** RT-suitable — `Copy` value type; construction and
/// accessors are `const`, pure.
///
/// # Examples
///
/// ```
/// use ruviform_observability::record::{EventId, EventRecord, RecordHeader};
/// use ruviform_observability::source::{SourceGeneration, SourceId, SourceIdentity};
///
/// let source = SourceIdentity::new(SourceId::new(1).unwrap(), SourceGeneration::new(0));
/// let header = RecordHeader::new(EventId::new(7), 1, source).with_generation(3);
/// let record = EventRecord::new(header, 480);
/// assert_eq!(record.header().event(), EventId::new(7));
/// assert_eq!(record.header().generation(), 3);
/// assert_eq!(record.value(), 480);
/// assert_eq!(record.reserved(), [0; 2]);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EventRecord {
    header: RecordHeader,
    value: u64,
    reserved: [u64; 2],
}

impl EventRecord {
    /// A record from `header` and scalar `value`; `reserved` starts at
    /// `[0; 2]`.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn new(header: RecordHeader, value: u64) -> Self {
        Self {
            header,
            value,
            reserved: [0; 2],
        }
    }

    /// The record's metadata header — **RT contract:** RT-suitable.
    #[must_use]
    pub const fn header(self) -> RecordHeader {
        self.header
    }

    /// The scalar payload — **RT contract:** RT-suitable.
    #[must_use]
    pub const fn value(self) -> u64 {
        self.value
    }

    /// The reserved extension field — always `[0; 2]` in this revision;
    /// must not be interpreted.
    ///
    /// **RT contract:** RT-suitable.
    #[must_use]
    pub const fn reserved(self) -> [u64; 2] {
        self.reserved
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::{CorrelationLink, CorrelationToken};
    use crate::source::{SourceGeneration, SourceId};
    use ruviform_media_core::{
        ClockDomainId, ClockEpochId, InputStamp, TimeBase, TimestampProvenance,
    };

    fn source() -> SourceIdentity {
        SourceIdentity::new(SourceId::new(1).unwrap(), SourceGeneration::new(2))
    }

    #[test]
    fn header_defaults_and_builders() {
        let header = RecordHeader::new(EventId::new(5), 1, source());
        assert_eq!(header.event(), EventId::new(5));
        assert_eq!(header.version(), 1);
        assert_eq!(header.source(), source());
        assert_eq!(header.generation(), 0);
        assert_eq!(header.correlation(), None);
        assert_eq!(header.clock(), None);
        assert_eq!(header.confidentiality(), Confidentiality::Internal);
        assert_eq!(header.impact(), ReportingImpact::Normal);

        let clock = ClockContext::new(
            InputStamp {
                ticks: 1,
                base: TimeBase::new(1, 48_000).unwrap(),
                domain: ClockDomainId::SAMPLE_CLOCK,
                provenance: TimestampProvenance::Carried,
            },
            ClockEpochId::new(1),
        );
        let link = CorrelationLink::new(CorrelationToken::new(9).unwrap(), 0);
        let header = header
            .with_generation(4)
            .with_correlation(Some(link))
            .with_clock(Some(clock))
            .with_confidentiality(Confidentiality::Public)
            .with_impact(ReportingImpact::High);
        assert_eq!(header.generation(), 4);
        assert_eq!(header.correlation(), Some(link));
        assert_eq!(header.clock(), Some(clock));
        assert_eq!(header.confidentiality(), Confidentiality::Public);
        assert_eq!(header.impact(), ReportingImpact::High);
    }

    #[test]
    fn event_record_shape_and_reserved() {
        let header = RecordHeader::new(EventId::new(7), 1, source());
        let record = EventRecord::new(header, 480);
        assert_eq!(record.header(), header);
        assert_eq!(record.value(), 480);
        assert_eq!(record.reserved(), [0; 2]);
        // The record stays a small fixed-size `Copy` value — bounded
        // enough to sit in the bridge's per-slot envelope.
        assert!(size_of::<EventRecord>() <= 128);
        let copied = record;
        assert_eq!(copied, record);
    }

    #[test]
    fn distinct_headers_distinguish_records() {
        let a = EventRecord::new(RecordHeader::new(EventId::new(1), 1, source()), 0);
        let b = EventRecord::new(
            RecordHeader::new(EventId::new(1), 1, source()).with_generation(9),
            0,
        );
        assert_ne!(a, b);
    }

    #[test]
    fn rt_record_ops_allocate_nothing() {
        use assert_no_alloc::assert_no_alloc;

        let header = RecordHeader::new(EventId::new(1), 1, source())
            .with_generation(2)
            .with_correlation(None);
        let record = EventRecord::new(header, 1);
        assert_no_alloc(|| {
            let _ = RecordHeader::new(EventId::new(2), 1, source())
                .with_clock(None)
                .with_impact(ReportingImpact::Low);
            let _ = record.header();
            let _ = record.value();
            let _ = record.reserved();
            let _ = header.event();
            let _ = header.version();
            let _ = header.source();
            let _ = header.generation();
            let _ = header.correlation();
            let _ = header.clock();
            let _ = header.confidentiality();
            let _ = header.impact();
        });
    }
}
