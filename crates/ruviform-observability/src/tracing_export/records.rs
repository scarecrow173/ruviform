//! [`TracingRecordExporter`] — converts drained
//! [`crate::record::EventRecord`]s into `tracing` events under the
//! fixed `rt_record` schema in [`super::schema`].
//!
//! # Contract
//!
//! - **Pull, non-RT only.** [`TracingRecordExporter::emit_records`]
//!   reads a slice of [`crate::rt_bridge::ReceivedRecord`] envelopes
//!   and allocates (`String` hex fields); it must never be called from
//!   an RT writer path, and no RT path may call `tracing`. The intended
//!   flow is: host drains `RecordSink` into a bounded `Vec`, *then*
//!   calls `emit_records` — never emit inside the `drain` callback.
//! - **Read-only over authoritative state.** The exporter borrows the
//!   record slice and the [`CorrelationBook`] (`&` only — `issue`/
//!   `remint`/`expire` need `&mut` and cannot be reached). It cannot
//!   mutate records, the book, the bridge, or any Collector/Metrics/
//!   Diagnostics/Source state.
//! - **Host-owned dispatch.** Same rule as [`super::TracingExporter`]:
//!   the `tracing::Dispatch` is injected at construction; nothing
//!   installs a global default, forces a subscriber, or writes to
//!   stdout. `Dispatch::none()` simply drops events.
//! - **Correlation is classification, not relationship.** A record's
//!   `CorrelationLink` is resolved through `CorrelationBook::classify`
//!   and reported as `correlation_status`/`unlinked_reason` fields.
//!   Unlinked links never produce span or parent/child fields — they
//!   are stale handles, not edges.
//! - **Two clock vocabularies stay separate.** `ClockContext` fields
//!   (`clock_*`) carry the record's *own* native clock-domain reading
//!   verbatim; `observed_at_mono_us` is the non-RT observation instant
//!   relative to the exporter's first `emit_records` call. No
//!   conversion between them is attempted.

use std::time::Instant;

use tracing::{Dispatch, Level, dispatcher};

use crate::record::{
    ClockContext, CorrelationBook, CorrelationLink, CorrelationStatus, EventRecord,
};
use crate::rt_bridge::ReceivedRecord;

use super::schema::{
    CLOCK_STATUS_STAMPED, CLOCK_STATUS_UNSTAMPED, CORRELATION_STATUS_LINKED,
    CORRELATION_STATUS_NONE, CORRELATION_STATUS_UNLINKED, EVENT_CODE_RECORD, EVENT_VERSION, TARGET,
    clock_provenance_str, confidentiality_str, mono_us_since, record_level, unlinked_reason_str,
};
use super::trace_context::TraceContext;

/// Default cap on `rt_record` events emitted per
/// [`emit_records`](TracingRecordExporter::emit_records) call. A ring
/// drain is already bounded by bridge capacity; this cap bounds the
/// *event* fan-out independently so a large drain cannot flood a
/// subscriber.
pub const DEFAULT_MAX_EVENTS_PER_DRAIN: usize = 256;

/// Per-call accounting returned by
/// [`TracingRecordExporter::emit_records`].
///
/// Loss categories are never conflated: `cap_dropped` counts records
/// the emission cap suppressed, while `correlation_*` classifies
/// *every* record in the slice (emitted or not) so correlation health
/// stays observable even under cap pressure.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RecordEmitStats {
    /// `rt_record` events actually emitted this call.
    pub emitted: u64,
    /// Records that exceeded `max_events_per_drain` — counted, never
    /// emitted. Records carry no exporter-side state, so a capped
    /// record is simply not reported; there is no baseline to keep
    /// consistent for a later retry.
    pub cap_dropped: u64,
    /// Records whose link classified [`CorrelationStatus::Linked`].
    pub correlation_linked: u64,
    /// Records whose link classified [`CorrelationStatus::Unlinked`]
    /// (any [`crate::record::UnlinkedReason`]).
    pub correlation_unlinked: u64,
    /// Records carrying no correlation link at all — intentionally
    /// uncorrelated, distinct from unlinked.
    pub correlation_none: u64,
}

/// Converts drained [`EventRecord`]s into `tracing` events on a
/// host-injected [`Dispatch`].
///
/// Non-RT only: [`emit_records`](Self::emit_records) allocates and may
/// call into the subscriber. Construct once per drain owner and reuse
/// across drains so `observed_at_mono_us` stays on one `t0` origin.
///
/// # Examples
///
/// ```
/// use ruviform_observability::tracing_export::TracingRecordExporter;
/// use tracing::Dispatch;
///
/// // `Dispatch::none()` drops every event — safe with no subscriber
/// // installed and no global default set.
/// let exporter = TracingRecordExporter::new(Dispatch::none());
/// ```
pub struct TracingRecordExporter {
    dispatch: Dispatch,
    max_events_per_drain: u64,
    /// First `observed_at` ever seen — origin of `observed_at_mono_us`.
    t0: Option<Instant>,
}

/// The correlation classification resolved for one record — shared by
/// the stats counters and the emitted fields.
struct CorrelationFields {
    status: &'static str,
    token: u64,
    epoch: u64,
    unlinked_reason: &'static str,
}

/// The clock fields for one record — stamped values verbatim, or the
/// documented `0`/`"none"` sentinels when the record is unstamped.
struct ClockFields {
    status: &'static str,
    domain: u64,
    epoch: u64,
    position: u64,
    timebase_num: u64,
    timebase_den: u64,
    provenance: &'static str,
}

impl TracingRecordExporter {
    /// Creates an exporter emitting at most
    /// [`DEFAULT_MAX_EVENTS_PER_DRAIN`] `rt_record` events per drain.
    #[must_use]
    pub fn new(dispatch: Dispatch) -> Self {
        Self::with_max_events_per_drain(dispatch, DEFAULT_MAX_EVENTS_PER_DRAIN)
    }

    /// Creates an exporter with an explicit per-drain event cap.
    /// `max_events_per_drain == 0` emits no events (every record is
    /// `cap_dropped`).
    #[must_use]
    pub fn with_max_events_per_drain(dispatch: Dispatch, max_events_per_drain: usize) -> Self {
        Self {
            dispatch,
            max_events_per_drain: u64::try_from(max_events_per_drain).unwrap_or(u64::MAX),
            t0: None,
        }
    }

    /// The injected dispatch.
    #[must_use]
    pub fn dispatch(&self) -> &Dispatch {
        &self.dispatch
    }

    /// Emits one `rt_record` event per record in `records`, up to
    /// `max_events_per_drain`, and returns per-call accounting.
    ///
    /// `observed_at` is the non-RT instant the drain was taken; it
    /// anchors `observed_at_mono_us` (relative to the exporter's first
    /// call). `book` classifies each record's [`CorrelationLink`] —
    /// `classify` is `&self`, so issuing/expiring stays with the book's
    /// owner. `trace`, when `Some`, attaches the W3C trace fields to
    /// every emitted event; it is correlation metadata only and never
    /// authorization.
    ///
    /// Non-RT only — allocates and may run subscriber code. Never call
    /// from an RT writer path or inside a `RecordSink::drain` callback.
    #[must_use]
    pub fn emit_records(
        &mut self,
        observed_at: Instant,
        records: &[ReceivedRecord<EventRecord>],
        book: &CorrelationBook,
        trace: Option<&TraceContext>,
    ) -> RecordEmitStats {
        let mut stats = RecordEmitStats::default();
        let t0 = *self.t0.get_or_insert(observed_at);

        for received in records {
            let record = received.record();
            let header = record.header();
            let correlation = correlation_fields(header.correlation(), book);
            match correlation.status {
                CORRELATION_STATUS_LINKED => stats.correlation_linked += 1,
                CORRELATION_STATUS_UNLINKED => stats.correlation_unlinked += 1,
                _ => stats.correlation_none += 1,
            }

            if stats.emitted >= self.max_events_per_drain {
                // No exporter-side state to preserve — the record is
                // simply not reported this drain.
                stats.cap_dropped += 1;
                continue;
            }
            self.emit_record(
                record,
                received.dropped_before(),
                &correlation,
                observed_at,
                t0,
                trace,
            );
            stats.emitted += 1;
        }

        stats
    }

    /// Emits one `rt_record` event under the fixed schema. All fields
    /// are always present; inapplicable ones carry the documented
    /// `0`/`"none"` sentinels, and `trace_*` fields appear only when a
    /// valid [`TraceContext`] was supplied (as `Option` fields, `None`
    /// records nothing).
    ///
    /// `tracing::event!` callsites carry a *static* level, so the
    /// computed severity is dispatched through one callsite per level —
    /// same pattern as [`super::TracingExporter`].
    fn emit_record(
        &self,
        record: EventRecord,
        dropped_before: u64,
        correlation: &CorrelationFields,
        observed_at: Instant,
        t0: Instant,
        trace: Option<&TraceContext>,
    ) {
        let header = record.header();
        let clock = clock_fields(header.clock());
        let level = record_level(header.impact(), dropped_before);
        let trace_id = trace.map(|t| t.trace_id_hex());
        let parent_span_id = trace.map(|t| t.parent_span_id_hex());
        let trace_flags = trace.map(|t| u64::from(t.flags()));
        let trace_sampled = trace.map(|t| t.is_sampled());

        macro_rules! go {
            ($lvl:expr) => {
                dispatcher::with_default(&self.dispatch, || {
                    tracing::event!(
                        target: TARGET,
                        $lvl,
                        event_code = EVENT_CODE_RECORD,
                        event_version = EVENT_VERSION,
                        event_id = u64::from(header.event().get()),
                        record_version = u64::from(header.version()),
                        source_id = header.source().id().get(),
                        source_generation = header.source().generation().get(),
                        domain_generation = header.generation(),
                        value = record.value(),
                        dropped_before = dropped_before,
                        confidentiality = confidentiality_str(header.confidentiality()),
                        correlation_status = correlation.status,
                        correlation_token = correlation.token,
                        correlation_epoch = correlation.epoch,
                        unlinked_reason = correlation.unlinked_reason,
                        clock_status = clock.status,
                        clock_domain = clock.domain,
                        clock_epoch = clock.epoch,
                        clock_position = clock.position,
                        clock_timebase_num = clock.timebase_num,
                        clock_timebase_den = clock.timebase_den,
                        clock_provenance = clock.provenance,
                        observed_at_mono_us = mono_us_since(observed_at, t0),
                        trace_id = trace_id.as_deref(),
                        parent_span_id = parent_span_id.as_deref(),
                        trace_flags = trace_flags,
                        trace_sampled = trace_sampled,
                    );
                })
            };
        }
        match level {
            Level::DEBUG => go!(Level::DEBUG),
            Level::INFO => go!(Level::INFO),
            // `record_level` only yields DEBUG/INFO/WARN; anything else
            // fails safe upward rather than under-reporting.
            _ => go!(Level::WARN),
        }
    }
}

/// Resolves a record's [`CorrelationLink`] through the book. `None`
/// means *intentionally uncorrelated* — never conflated with an
/// unlinked (stale/expired/unknown) link. Token and epoch are emitted
/// for any present link, linked or not: they are the record's claimed
/// handle, and `correlation_status`/`unlinked_reason` carry the
/// authoritative classification.
fn correlation_fields(link: Option<CorrelationLink>, book: &CorrelationBook) -> CorrelationFields {
    let Some(link) = link else {
        return CorrelationFields {
            status: CORRELATION_STATUS_NONE,
            token: 0,
            epoch: 0,
            unlinked_reason: unlinked_reason_str(None),
        };
    };
    let token = link.token().get();
    let epoch = link.epoch();
    match book.classify(link) {
        CorrelationStatus::Linked => CorrelationFields {
            status: CORRELATION_STATUS_LINKED,
            token,
            epoch,
            unlinked_reason: unlinked_reason_str(None),
        },
        CorrelationStatus::Unlinked(reason) => CorrelationFields {
            status: CORRELATION_STATUS_UNLINKED,
            token,
            epoch,
            unlinked_reason: unlinked_reason_str(Some(reason)),
        },
    }
}

/// Resolves a record's [`ClockContext`]. `None` means *unstamped* —
/// distinct from a stamped `Estimated` provenance — and emits the
/// documented `0`/`"none"` sentinels rather than fabricating a reading.
fn clock_fields(clock: Option<ClockContext>) -> ClockFields {
    let Some(ctx) = clock else {
        return ClockFields {
            status: CLOCK_STATUS_UNSTAMPED,
            domain: 0,
            epoch: 0,
            position: 0,
            timebase_num: 0,
            timebase_den: 0,
            provenance: "none",
        };
    };
    ClockFields {
        status: CLOCK_STATUS_STAMPED,
        domain: ctx.domain().get(),
        epoch: ctx.epoch().get(),
        position: ctx.position(),
        timebase_num: u64::from(ctx.time_base().numerator()),
        timebase_den: u64::from(ctx.time_base().denominator()),
        provenance: clock_provenance_str(ctx.provenance()),
    }
}
