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
    EngineFrame, InBlockTransportSupport, LedgerKey, MidiEvent, NOTE_LEDGER_CAPACITY, Ppqn,
    ScheduledEvent, SpanContext, TempoMap, TransportPlan, TransportSpan, TransportStatus,
    event_engine_frame, event_generation_matches,
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
    ctx: &SpanContext,
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

/// Observation counters for one [`chase_span_events`] scan — the
/// honesty surface for bounded-table loss.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct ChaseStats {
    /// Plan-prefix events examined (`position <=` the chase position),
    /// capped by [`CHASE_SCAN_BUDGET`].
    pub scanned: usize,
    /// Sounding keys re-fired into `scratch`.
    pub chased: usize,
    /// Note events that could not be tracked because the fixed chase
    /// table filled — they *may* include notes sounding at the
    /// position.
    pub untracked: usize,
    /// Keys confirmed sounding that did not fit `scratch` — definitely
    /// not re-fired.
    pub truncated: usize,
    /// The scan budget cut the prefix walk short: sounding state
    /// deeper than [`CHASE_SCAN_BUDGET`] events is unknown — some
    /// sounding notes may not be re-fired.
    pub incomplete: bool,
}

/// Maximum plan-prefix events examined by one [`chase_span_events`]
/// call. The scan walks *backward* from the chase position — nearest
/// events decide sounding state — so the bound keeps the RT cost
/// proportional to the budget, not the (unbounded) plan length.
pub const CHASE_SCAN_BUDGET: usize = 2048;

/// Per-key chase state tracked backward through the plan prefix.
/// `excess_offs` counts note-offs not yet paired with an earlier
/// note-on; `uncovered` marks a note-on no later off consumed — the
/// exact `NoteLedger` "ons minus offs" sounding semantics evaluated
/// in reverse. `latest_on` is the most recent note-on (first seen
/// walking backward), the event re-fired on chase.
struct ChaseEntry {
    latest_on: Option<(MidiEvent, i64)>,
    uncovered: bool,
    excess_offs: u32,
}

/// Reconstructs note state at `ctx.position_ticks` from the plan
/// prefix and writes the chase set — offset-0 re-fires of the latest
/// `NoteOn` for every key still sounding — into `scratch`.
///
/// "Still sounding" follows `NoteLedger` semantics over plan events at
/// `position <= ctx.position_ticks` whose generation tags match `ctx`
/// ([`event_generation_matches`]): effective note-ons (including
/// re-triggers, which bump the count) minus note-offs. A key whose
/// latest on sits *at* the span position is not chased — the normal
/// event window already delivers it at offset 0. Chased events are
/// `Event::new(0, EventKind::Midi(..))` emitted in chase-table order,
/// deterministic for a given plan and position.
///
/// Non-note payloads are ignored: controller/parameter state chase is
/// a deliberately deferred contract — this chase set covers the
/// stuck-note domain only. An empty list is returned when
/// `ctx.status` is not `Playing`.
///
/// When the bounded [`NOTE_LEDGER_CAPACITY`] table fills, further
/// note events for unseen keys are skipped and counted in
/// [`ChaseStats::untracked`]. When the prefix exceeds
/// [`CHASE_SCAN_BUDGET`], the walk stops and sets
/// [`ChaseStats::incomplete`] — both are defined loss, surfaced
/// through the stats for the caller to report.
///
/// **RT contract:** RT-suitable — bounded `O(min(prefix,
/// CHASE_SCAN_BUDGET) · NOTE_LEDGER_CAPACITY)` scan, writes only into
/// `scratch`, no allocation.
pub fn chase_span_events<'a>(
    plan: &TransportPlan<EventKind>,
    ctx: &SpanContext,
    scratch: &'a mut [Event],
) -> (&'a mut [Event], ChaseStats) {
    let mut stats = ChaseStats::default();
    if ctx.status != TransportStatus::Playing {
        return (&mut scratch[..0], stats);
    }
    let prefix_end = plan
        .events
        .partition_point(|e| e.position_ticks() <= ctx.position_ticks);
    let mut table: [Option<(LedgerKey, ChaseEntry)>; NOTE_LEDGER_CAPACITY] =
        [const { None }; NOTE_LEDGER_CAPACITY];
    // Walk the prefix backward — the events nearest the position
    // dominate sounding state, so a budgeted scan loses the least
    // relevant history first.
    for ev in plan.events[..prefix_end].iter().rev() {
        if stats.scanned >= CHASE_SCAN_BUDGET {
            stats.incomplete = true;
            break;
        }
        stats.scanned += 1;
        if !event_generation_matches(ctx, ev) {
            continue;
        }
        let EventKind::Midi(midi) = ev.payload else {
            continue;
        };
        let (Some(key), Some(on)) = (midi.ledger_key(), midi.effective_note_state()) else {
            continue;
        };
        let tracked = table
            .iter_mut()
            .find_map(|slot| slot.as_mut().filter(|(k, _)| *k == key).map(|(_, e)| e));
        let entry = if let Some(entry) = tracked {
            entry
        } else {
            let Some(slot) = table.iter_mut().find(|slot| slot.is_none()) else {
                // Table full — the key's sounding state is unknowable
                // within the bound; defined loss.
                stats.untracked += 1;
                continue;
            };
            *slot = Some((
                key,
                ChaseEntry {
                    latest_on: None,
                    uncovered: false,
                    excess_offs: 0,
                },
            ));
            let Some((_, new_entry)) = slot else {
                continue;
            };
            new_entry
        };
        if on {
            if entry.latest_on.is_none() {
                entry.latest_on = Some((midi, ev.position_ticks()));
            }
            if entry.excess_offs > 0 {
                entry.excess_offs -= 1;
            } else {
                entry.uncovered = true;
            }
        } else {
            entry.excess_offs = entry.excess_offs.saturating_add(1);
        }
    }
    let mut n = 0usize;
    for slot in table.iter().flatten() {
        let (_, entry) = slot;
        let Some((last_on, last_on_ticks)) = entry.latest_on else {
            continue;
        };
        if !entry.uncovered || last_on_ticks >= ctx.position_ticks {
            continue;
        }
        if n < scratch.len() {
            scratch[n] = Event::new(0, EventKind::Midi(last_on));
            n += 1;
            stats.chased += 1;
        } else {
            stats.truncated += 1;
        }
    }
    (&mut scratch[..n], stats)
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
    fn plan_of(entries: &[(i64, u64, u32)]) -> TransportPlan<u32> {
        let mut b = ruviform_media_core::TransportPlanBuilder::new(base(), PlanGeneration::new(1));
        for &(pos, seq, p) in entries {
            b.event(MediaTime::new(pos, base()), seq, p).unwrap();
        }
        b.build()
    }

    // ------------------------------------------------------------------
    // chase_span_events
    // ------------------------------------------------------------------

    use ruviform_media_core::{MidiChannel, MidiEvent, MidiGroup, MidiKind, MidiValue, NoteKey};

    fn on(key: u8) -> EventKind {
        EventKind::Midi(MidiEvent {
            group: MidiGroup::ZERO,
            channel: MidiChannel::new(0).unwrap(),
            kind: MidiKind::NoteOn {
                key: NoteKey::new(key).unwrap(),
                velocity: MidiValue::from_u7(100),
            },
        })
    }

    fn off(key: u8) -> EventKind {
        EventKind::Midi(MidiEvent {
            group: MidiGroup::ZERO,
            channel: MidiChannel::new(0).unwrap(),
            kind: MidiKind::NoteOff {
                key: NoteKey::new(key).unwrap(),
                velocity: 0,
            },
        })
    }

    fn midi_plan_of(entries: &[(i64, u64, EventKind)]) -> TransportPlan<EventKind> {
        let mut b = ruviform_media_core::TransportPlanBuilder::new(base(), PlanGeneration::new(1));
        for (pos, seq, k) in entries {
            b.event(MediaTime::new(*pos, base()), *seq, *k).unwrap();
        }
        b.build()
    }

    fn chased_keys(chased: &[Event]) -> Vec<u8> {
        chased
            .iter()
            .filter_map(|e| match e.kind() {
                EventKind::Midi(m) => match m.kind {
                    MidiKind::NoteOn { key, .. } => Some(key.get()),
                    _ => None,
                },
                _ => None,
            })
            .collect()
    }

    #[test]
    fn chase_refires_only_still_sounding_keys() {
        // key 60: on@0 off@100 → sounding across pos 50 only.
        // key 64: on@20, never off → sounding at 80.
        let plan = midi_plan_of(&[(0, 0, on(60)), (20, 1, on(64)), (100, 2, off(60))]);
        let mut scratch = scratch();
        let (chased, stats) = chase_span_events(&plan, &ctx(80), &mut scratch);
        // Backward scan emits nearest-position keys first — deterministic
        // for a given plan and position.
        assert_eq!(chased_keys(chased), vec![64, 60]);
        assert_eq!(stats.scanned, 2);
        assert_eq!(stats.chased, 2);
        assert!(chased.iter().all(|e| e.frame_offset() == 0));
    }

    #[test]
    fn chase_skips_completed_notes_and_uses_latest_retrigger() {
        let plan = midi_plan_of(&[(0, 0, on(60)), (50, 1, off(60)), (60, 2, on(61))]);
        let mut scratch = scratch();
        let (chased, _) = chase_span_events(&plan, &ctx(80), &mut scratch);
        assert_eq!(chased_keys(chased), vec![61]);
    }

    #[test]
    fn chase_excludes_on_at_exact_position() {
        // The NoteOn at pos == position is delivered by the normal
        // span window — chasing it too would double-fire.
        let plan = midi_plan_of(&[(50, 0, on(60))]);
        let mut scratch = scratch();
        let (chased, stats) = chase_span_events(&plan, &ctx(50), &mut scratch);
        assert_eq!(chased, []);
        assert_eq!(stats.chased, 0);
    }

    #[test]
    fn chase_empty_when_not_playing() {
        let plan = midi_plan_of(&[(0, 0, on(60))]);
        let mut c = ctx(80);
        c.status = TransportStatus::Paused;
        let mut scratch = scratch();
        let (chased, _) = chase_span_events(&plan, &c, &mut scratch);
        assert_eq!(chased, []);
    }

    #[test]
    fn chase_ignores_stale_plan_generation() {
        let mut plan = midi_plan_of(&[(0, 0, on(60))]);
        plan.events[0].plan_generation = PlanGeneration::new(99);
        let mut scratch = scratch();
        let (chased, stats) = chase_span_events(&plan, &ctx(80), &mut scratch);
        assert_eq!(chased, []);
        assert_eq!(stats.scanned, 1);
        assert_eq!(stats.chased, 0);
    }

    #[test]
    fn chase_reports_scratch_truncation() {
        let plan = midi_plan_of(&[(0, 0, on(60)), (10, 1, on(64))]);
        let mut scratch = scratch();
        let (chased, stats) = chase_span_events(&plan, &ctx(80), &mut scratch[..1]);
        assert_eq!(chased.len(), 1);
        assert_eq!(stats.truncated, 1);
    }

    #[test]
    fn chase_respects_scan_budget() {
        // Prefix longer than CHASE_SCAN_BUDGET — the backward walk
        // stops at the bound, flags the loss, and still chases the
        // keys nearest the position.
        let mut entries = Vec::with_capacity(CHASE_SCAN_BUDGET + 128);
        for i in 0..(CHASE_SCAN_BUDGET + 128) {
            let pos = i64::try_from(i).unwrap();
            entries.push((pos, u64::try_from(i).unwrap(), on(60 + (i % 4) as u8)));
        }
        let plan = midi_plan_of(&entries);
        let mut scratch: [Event; NOTE_LEDGER_CAPACITY] =
            [Event::new(0, kind(&0)); NOTE_LEDGER_CAPACITY];
        let (chased, stats) = chase_span_events(&plan, &ctx(99_000), &mut scratch);
        assert_eq!(stats.scanned, CHASE_SCAN_BUDGET);
        assert!(stats.incomplete);
        // The four keys' latest ons are all within the scanned tail.
        assert_eq!(stats.chased, 4);
        assert_eq!(chased.len(), 4);
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
