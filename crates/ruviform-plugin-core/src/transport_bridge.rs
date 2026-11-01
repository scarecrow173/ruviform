//! Transport span → plugin wire delivery bridge (Issue #190): maps the
//! canonical [`TransportSpan`]/[`ScheduledEvent`] contract from
//! `ruviform-media-core` onto the existing plugin wire types —
//! [`ProcessConfig`]'s `min_frames`/`max_frames`, per-call
//! [`TransportInfo`] and process-start-relative [`EventList`].
//!
//! This is the *thin* half of the contract: canonical identity, span
//! splitting, apply/receipt and budget logic live in media-core; here
//! lives only the wire conversion. One `process` call carries exactly
//! one transport context, so a logical span is cut into physical calls
//! by [`BlockContract::partition`] — never below `min_frames`, never
//! above `max_frames`.
//!
//! ## RT contract
//!
//! [`call_transport_info`], [`calls_for_span`] and
//! [`build_call_event_list`] are RT-suitable: pure integer math, no
//! allocation (the caller supplies the event scratch). No blocking or
//! I/O.

use ruviform_media_core::{
    EngineFrame, InBlockTransportSupport, Ppqn, ScheduledEvent, TempoMap, TransportSpan,
    event_engine_frame,
};

use crate::{Event, EventKind, EventList, ProcessConfig, TransportInfo};

/// Derives a [`BlockContract`] from an activated [`ProcessConfig`]. The
/// `in_block` capability is an adapter-attested input: `Unknown` and
/// `Declared` are never treated as supported — only
/// [`InBlockTransportSupport::Verified`] (proven end-to-end by an
/// adapter, e.g. CLAP `CLAP_EVENT_TRANSPORT`) may enable the in-block
/// transport-update path, which this `PoC` does not implement.
///
/// L0/L1 difference: an L0 gate rejects `frames < min_frames` inside the
/// lock, while L1 delegates `min_frames` to the child contract — the
/// caller passes the *effective* activated minimum here.
///
/// **RT contract:** non-RT — evaluated at activation/preflight.
#[must_use]
pub const fn contract_from_process_config(
    config: &ProcessConfig,
    in_block: InBlockTransportSupport,
) -> ruviform_media_core::BlockContract {
    ruviform_media_core::BlockContract::new(config.min_frames(), config.max_frames(), in_block)
}

/// The legal physical-call partition for `span` under `contract`, or
/// `None` when the span is undeliverable (no partition satisfies
/// `min_frames`/`max_frames`). Undeliverable spans must not reach a
/// plugin — the scheduler marks them `deliverable == false` and records
/// a fault.
///
/// **RT contract:** RT-suitable — pure integer math.
#[must_use]
pub fn calls_for_span(
    span: &TransportSpan,
    contract: &ruviform_media_core::BlockContract,
) -> Option<ruviform_media_core::CallPartition> {
    contract.partition(span.frames())
}

/// The [`TransportInfo`] snapshot for one physical call: status, the
/// transport position at the call's offset inside its span, the active
/// loop range, and — when a tempo map and a requested `ppqn` are given —
/// the musical-time snapshot via [`TempoMap::media_to_beat`] +
/// [`TempoMap::time_info`] (exact rational path, no fixed-PPQN
/// assumption).
///
/// **RT contract:** RT-suitable — pure integer math.
#[must_use]
pub fn call_transport_info(
    ctx: &ruviform_media_core::SpanContext,
    call_offset_frames: u32,
    tempo_map: Option<&TempoMap>,
    ppqn: Option<Ppqn>,
) -> TransportInfo {
    let position = ctx.position_at_offset(call_offset_frames);
    let mut info = TransportInfo::new(ctx.status, position);
    if let Some(range) = ctx.loop_range() {
        info = info.with_loop_range(range);
    }
    if let (Some(map), Some(ppqn)) = (tempo_map, ppqn)
        && let Ok(info_musical) = map
            .media_to_beat(position)
            .and_then(|beat| map.time_info(beat, ppqn))
    {
        info = info.with_musical(info_musical);
    }
    info
}

/// Writes the events of `events` (a span's position-matched window —
/// see `ruviform_media_core::span_event_window`) whose engine frames
/// fall inside `[call_start, call_start + call_frames)` into `scratch`,
/// converted to *process-start-relative* `frame_offset`s, and validates
/// them as an [`EventList`].
///
/// The wire contract is preserved by construction: offsets are
/// nondecreasing (events are position-ordered), `frame_offset <
/// block_frames`, and a `call_frames == 0` flush call accepts offset 0
/// only.
///
/// Returns `None` when `scratch` is too small (a delivery budget
/// violation the caller should surface, not silently drop).
///
/// **RT contract:** RT-suitable — writes only into `scratch`, no
/// allocation.
#[must_use]
pub fn build_call_event_list<'a, P>(
    span: &TransportSpan,
    events: &[ScheduledEvent<P>],
    call_start: EngineFrame,
    call_frames: u32,
    scratch: &'a mut [Event],
    kind: impl Fn(&P) -> EventKind,
) -> Option<EventList<'a>> {
    let call_end = call_start.advance(u64::from(call_frames.max(1)));
    let mut n = 0usize;
    for ev in events {
        let Some(frame) = event_engine_frame(span, ev) else {
            continue;
        };
        if call_frames == 0 {
            if frame != call_start {
                continue;
            }
        } else if frame.checked_offset(call_start).is_none()
            || frame.checked_offset(call_end).is_some()
        {
            continue;
        }
        if n >= scratch.len() {
            return None;
        }
        let offset = u32::try_from(frame.checked_offset(call_start).unwrap_or(0)).unwrap_or(0);
        scratch[n] = Event::new(offset, kind(&ev.payload));
        n += 1;
    }
    EventList::new(&scratch[..n], call_frames).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ParamEvent;
    use crate::param::{ParamId, ParamValue};
    use ruviform_media_core::{
        ClockEpochId, EngineFrame, LoopOccurrence, MediaTime, PlanGeneration, SpanContext,
        TempoMapRevision, TimeBase, TransportGeneration, TransportStatus,
    };

    fn base() -> TimeBase {
        TimeBase::sample_rate(48_000).unwrap()
    }

    #[allow(clippy::trivially_copy_pass_by_ref)] // signature fixed by fn(&T)
    fn kind(p: &u32) -> EventKind {
        EventKind::ParamValue(ParamEvent {
            param_id: ParamId(*p),
            value: ParamValue::new(1.0).unwrap(),
        })
    }

    fn scratch() -> [Event; 8] {
        [Event::new(0, kind(&0)); 8]
    }

    fn ctx(pos: i64) -> SpanContext {
        SpanContext {
            status: TransportStatus::Playing,
            position_ticks: pos,
            position_base: base(),
            loop_ticks: None,
            occurrence: LoopOccurrence::new(0),
            clock_epoch: ClockEpochId::new(0),
            transport_generation: TransportGeneration::new(0),
            plan_generation: PlanGeneration::new(1),
            tempo_revision: TempoMapRevision::new(0),
        }
    }

    fn span(start: u64, end: u64, pos: i64) -> TransportSpan {
        TransportSpan {
            start: EngineFrame::new(ClockEpochId::new(0), start),
            end: EngineFrame::new(ClockEpochId::new(0), end),
            ctx: ctx(pos),
            deliverable: true,
        }
    }

    /// Builds events through the real plan builder (position → ticks
    /// conversion + generation stamping are its job).
    fn plan_of(entries: &[(i64, u64, u32)]) -> ruviform_media_core::TransportPlan<u32> {
        let mut b = ruviform_media_core::TransportPlanBuilder::new(base(), PlanGeneration::new(1));
        for &(pos, seq, p) in entries {
            b.event(MediaTime::new(pos, base()), seq, p).unwrap();
        }
        b.build()
    }

    #[test]
    fn contract_maps_process_config() {
        let config = ProcessConfig::new(
            ruviform_audio_core::SampleRate::new(48_000).unwrap(),
            128,
            512,
        )
        .unwrap();
        let c = contract_from_process_config(&config, InBlockTransportSupport::Unknown);
        assert_eq!(c.min_frames, 128);
        assert_eq!(c.max_frames, 512);
    }

    #[test]
    fn call_transport_reports_position_at_offset() {
        let info = call_transport_info(&ctx(1000), 20, None, None);
        assert_eq!(info.status(), TransportStatus::Playing);
        assert_eq!(info.position(), MediaTime::new(1020, base()));
        assert_eq!(info.loop_range(), None);
    }

    #[test]
    fn call_transport_carries_loop_range() {
        let mut c = ctx(0);
        c.loop_ticks = Some((100, 356));
        let info = call_transport_info(&c, 0, None, None);
        let r = info.loop_range().unwrap();
        assert_eq!(r.start(), MediaTime::new(100, base()));
        assert_eq!(r.end(), MediaTime::new(356, base()));
    }

    #[test]
    fn event_list_offsets_are_process_relative() {
        // Span [100,256): event at engine frame 120 lands at offset 20
        // in a call starting at 100; event at 100 lands at 0.
        let sp = span(100, 256, 0);
        let plan = plan_of(&[(0, 1, 10), (20, 2, 20)]);
        let events = &plan.events[..];
        let mut scratch = scratch();
        let call_start = EngineFrame::new(ClockEpochId::new(0), 100);
        let list = build_call_event_list(&sp, events, call_start, 156, &mut scratch, kind).unwrap();
        let offsets: Vec<u32> = list.iter().map(Event::frame_offset).collect();
        assert_eq!(offsets, vec![0, 20]);
    }

    #[test]
    fn events_outside_call_window_are_skipped() {
        let sp = span(100, 256, 0);
        // Event at pos 156 → engine 256 → outside this call (belongs to
        // the next callback's span).
        let plan = plan_of(&[(155, 1, 1), (156, 2, 2)]);
        let events = &plan.events[..];
        let mut scratch = scratch();
        let call_start = EngineFrame::new(ClockEpochId::new(0), 100);
        let list = build_call_event_list(&sp, events, call_start, 156, &mut scratch, kind).unwrap();
        let offsets: Vec<u32> = list.iter().map(Event::frame_offset).collect();
        assert_eq!(offsets, vec![155]);
    }
}
