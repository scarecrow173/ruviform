//! [`PluginNode`] — the RT-side adapter: wraps a `PluginEndpoint`,
//! embeds a `TransportScheduler<EventKind>` and turns each callback
//! into a bounded sequence of legal physical `process` calls.
//!
//! ## Data flow (one callback)
//!
//! 1. drain ≤ [`CTRL_DRAIN_MAX`] control messages — `request` /
//!    `install_plan` (vault-cloned first) / `set_clock` /
//!    `device_restart` / `set_clock_anchor` are applied at callback
//!    head, `due_frames` relative to the node's own cursor.
//! 2. pump the live-input channel into the [`InputStage`] (bounded
//!    `try_pop`s — a full `pending` leaves events in the channel).
//! 3. materialize `NodeIo` input/output port views (stack arrays — no
//!    allocation).
//! 4. `scheduler.run_callback(cursor, frames, sink)` — every
//!    `deliverable` span becomes a `BlockContract` partition of
//!    physical calls; each call merges `release ++ chase ++ plan ++
//!    live` into one process-start-relative `EventList` plus a
//!    `TransportInfo` snapshot; every undeliverable span becomes
//!    silence plus a [`NodeReport::UndeliverableSpan`]. A changed span
//!    context (epoch / generations / loop occurrence) marks a
//!    discontinuity: the delivered-note ledger releases at the span's
//!    first call and [`chase_span_events`] re-fires notes spanning the
//!    new position.
//! 5. `pop_receipt`/`pop_fault` drain into the bounded report channel
//!    (full → [`NodeStatus::snapshot`]'s `report_overflows`).
//!
//! No allocation, no blocking, no `unsafe` on any of this.

use core::array;

use ruviform_audio::typed::{GraphProcessor, InputView, NodeIo, OutputView};
use ruviform_audio_core::midi_input::TimedMidiInput;
use ruviform_graph::{Direction, NodeSpec, PortFormat, PortRole};
use ruviform_media_core::{
    BlockContract, ClockEpochId, DeliveryPolicy, EngineFrame, LoopOccurrence, MidiEvent,
    NOTE_LEDGER_CAPACITY, NoteLedger, PlanGeneration, ScheduleBudget, ScheduledEvent, SpanSink,
    TransportGeneration, TransportPlan, TransportScheduler, TransportSpan,
};
use ruviform_plugin_core::{
    BufferView, BufferViewMut, BusDescriptor, BusDirection, BusRole, Event, EventKind, EventList,
    HostInput, HostOutput, ParamEvent, ParamId, ParamValue,
};
use ruviform_plugin_core::{
    build_call_event_list, call_transport_info, calls_for_span, chase_span_events,
    merge_call_events, sort_events_by_frame_offset,
};
use ruviform_plugin_host::{HostError, PluginEndpoint};
use ruviform_rt::RtContext;
use ruviform_rt::{Consumer, Producer};

use crate::control::{NodeControl, NodeReport};
use crate::input::InputStage;
use crate::status::{Counter, NodeStatus};
use crate::vault::PlanVault;

/// Maximum buses of one direction the node can present — matches the
/// wire bound (`WIRE_MAX_CHANNELS`/`WIRE_MAX_FRAMES` family).
pub const MAX_BUSES: usize = 16;
/// Maximum channels per bus in the windowed views.
pub const MAX_CHANNELS: usize = 16;
/// Control messages applied per callback — bounded drain.
pub const CTRL_DRAIN_MAX: usize = 32;
/// Plan installs kept alive in the vault.
pub const PLAN_VAULT_CAPACITY: usize = 64;
/// Default control/report channel capacities.
pub const CONTROL_CHANNEL_CAPACITY: usize = 64;
/// Report channel capacity.
pub const REPORT_CHANNEL_CAPACITY: usize = 256;

/// An erased per-bus input view over a `[buf_off, buf_off+frames)`
/// window of one port — stack-built, implements [`BufferView`].
#[derive(Default)]
pub struct WindowedIn<'a> {
    channels: [Option<&'a [f32]>; MAX_CHANNELS],
    count: usize,
    frames: usize,
}

impl BufferView for WindowedIn<'_> {
    fn channels(&self) -> usize {
        self.count
    }
    fn frames(&self) -> usize {
        self.frames
    }
    fn channel(&self, index: usize) -> Option<&[f32]> {
        self.channels.get(index).copied().flatten()
    }
}

/// An erased per-bus output view — the mutable counterpart of
/// [`WindowedIn`]. Holds the port view lazily: `channel_mut` reborrows
/// and re-windows on each call (the trait's sequential-borrow
/// semantics — a caller may only hold one channel's `&mut` at a
/// time). An unconnected port reports `None` for every channel —
/// the plugin sees the declared channel count but no writable plane,
/// the same answer an absent graph connection gives.
#[derive(Default)]
pub struct WindowedOut<'v, 'io: 'v> {
    view: Option<&'v mut OutputView<'io>>,
    off: usize,
    len: usize,
    count: usize,
}

impl BufferView for WindowedOut<'_, '_> {
    fn channels(&self) -> usize {
        self.count
    }
    fn frames(&self) -> usize {
        self.len
    }
    fn channel(&self, _index: usize) -> Option<&[f32]> {
        // `OutputView` exposes no shared read — output planes are
        // write-only to the plugin, so `None` is the honest answer.
        None
    }
}

impl BufferViewMut for WindowedOut<'_, '_> {
    fn channel_mut(&mut self, index: usize) -> Option<&mut [f32]> {
        let plane = self.view.as_mut()?.channel_mut(index)?;
        if self.off + self.len <= plane.len() {
            Some(&mut plane[self.off..self.off + self.len])
        } else {
            None
        }
    }
}

/// Per-direction bus metadata the sink consults each call.
#[derive(Copy, Clone, Debug)]
pub(crate) struct BusMeta {
    /// The declared descriptor (direction/layout/role/enabled).
    pub(crate) descriptor: BusDescriptor,
}

/// Splits a declared bus table into per-direction metadata (declared
/// order preserved — `NodeIo::input(i)`/`output(i)` is the i-th
/// same-direction port).
#[must_use]
pub(crate) fn split_buses(declared: &[BusDescriptor]) -> (Vec<BusMeta>, Vec<BusMeta>) {
    let mut ins = Vec::new();
    let mut outs = Vec::new();
    for d in declared {
        let meta = BusMeta { descriptor: *d };
        match d.direction {
            BusDirection::Input => ins.push(meta),
            _ => outs.push(meta),
        }
    }
    (ins, outs)
}

/// Reconstructs the declared bus table (flattened: inputs then
/// outputs, declaration order inside each direction) from a compiled
/// [`NodeSpec`]'s ports — the factory's configuration-change probe.
/// Non-audio ports are skipped (the bridge only declares audio ports).
///
/// **RT contract:** non-RT only — allocates.
#[must_use]
pub fn buses_from_specs(spec: &NodeSpec) -> Vec<BusDescriptor> {
    let mut out = Vec::with_capacity(spec.ports().len());
    for want_input in [true, false] {
        for port in spec.ports() {
            let is_input = matches!(port.direction(), Direction::Input);
            if is_input != want_input {
                continue;
            }
            let Some(layout) = (match port.format() {
                PortFormat::Audio(f) => Some(f.layout),
                _ => None,
            }) else {
                continue;
            };
            out.push(BusDescriptor {
                direction: if is_input {
                    BusDirection::Input
                } else {
                    BusDirection::Output
                },
                layout,
                name: "",
                role: match port.role() {
                    PortRole::Main => BusRole::Main,
                    PortRole::Sidechain => BusRole::Sidechain,
                    PortRole::Send => BusRole::Send,
                    PortRole::Return => BusRole::Return,
                    _ => BusRole::Aux,
                },
                enabled: matches!(port.activation(), ruviform_graph::Activation::Enabled),
            });
        }
    }
    out
}

/// The span-context identity whose change marks a transport
/// discontinuity — clock epoch, live transport/plan generations and
/// loop occurrence. Position is deliberately excluded: continuous
/// playback advances it every span without discontinuing note state.
type SpanCtxKey = (
    ClockEpochId,
    TransportGeneration,
    PlanGeneration,
    LoopOccurrence,
);

/// The sink `run_callback` feeds — holds disjoint borrows of the
/// node's fields so `PluginNode::process` satisfies the borrow checker
/// while staying allocation-free.
struct CallSink<'a, 'io> {
    callback_start: EngineFrame,
    contract: &'a BlockContract,
    endpoint: &'a mut PluginEndpoint,
    report: &'a mut Producer<NodeReport>,
    status: &'a NodeStatus,
    /// Merge output — the merged list becomes the call's `EventList`.
    scratch: &'a mut [Event],
    /// Per-call plan-window scratch (`build_call_event_list` output).
    plan_scratch: &'a mut [Event],
    /// Per-call live-input scratch (`collect_block` output).
    live_scratch: &'a mut [Event],
    /// Stable-sort scratch for the live list.
    sort_scratch: &'a mut [Event],
    /// `NoteLedger::release_all` emission scratch (`MidiEvent`s).
    release_midi: &'a mut [MidiEvent],
    /// Released note-offs wrapped as offset-0 `Event`s.
    release_events: &'a mut [Event],
    /// Chase emission scratch (`chase_span_events` output).
    chase_scratch: &'a mut [Event],
    /// Merge-output provenance scratch — each emitted event's source
    /// index (the note-off-priority collision pass's input).
    src_scratch: &'a mut [u8],
    /// `NoteLedger::drain_evicted` emission scratch (`MidiEvent`s) —
    /// synthetic offs owed by capacity-evicted keys.
    evict_midi: &'a mut [MidiEvent],
    /// Evicted-key offs wrapped as offset-0 `Event`s.
    evict_events: &'a mut [Event],
    /// The node's live-input stage, when attached with input.
    input: Option<&'a mut InputStage>,
    /// Delivered-note tracker — fed from the merged list each call,
    /// released on transport discontinuity.
    ledger: &'a mut NoteLedger,
    /// The active plan for span chase — snapshotted before
    /// `run_callback`; a mid-callback install leaves it conservatively
    /// stale (generation gating then yields an empty chase set).
    plan: Option<&'a TransportPlan<EventKind>>,
    /// Last *deliverable* span's context identity plus the position
    /// its successor was expected to start at — the discontinuity
    /// detector's memory, persisted across callbacks. Identity alone
    /// misses same-generation position jumps (`SetClock` rebases the
    /// frame cursor without touching a generation), so continuity
    /// also requires the derived position to match the expected
    /// advance. Undeliverable spans leave it alone so the change
    /// still registers at the next deliverable span.
    last_ctx: &'a mut Option<(SpanCtxKey, i64)>,
    in_views: &'a mut [Option<InputView<'io>>],
    out_views: &'a mut [Option<OutputView<'io>>],
    silent: &'a [Vec<f32>],
    bus_in: &'a [BusMeta],
    bus_out: &'a [BusMeta],
    /// The callback's frame count — a call window past it exceeds the
    /// `NodeIo` buffers, so it is silenced and reported instead of
    /// reaching the plugin.
    buffer_frames: usize,
}

impl<'io> CallSink<'_, 'io> {
    fn emit(&mut self, report: NodeReport) {
        if self.report.try_push(report).is_err() {
            self.status.bump(Counter::ReportOverflow);
        }
    }

    /// Zeroes `[off, off+len)` on every output port view and the silent
    /// fallback — the defined "plugin did not run" output.
    fn silence_outputs(&mut self, off: usize, len: usize) {
        for view in self.out_views.iter_mut() {
            let Some(view) = view else { continue };
            for c in 0..view.channels() {
                if let Some(plane) = view.channel_mut(c) {
                    let end = (off + len).min(plane.len());
                    if off < end {
                        plane[off..end].fill(0.0);
                    }
                }
            }
        }
    }

    /// Builds the erased per-bus input views for `[buf_off, buf_off+len)`.
    /// Unconnected ports and disabled buses get the silent slab /
    /// zero-channel empty views respectively.
    fn build_inputs<'v>(
        bus_in: &[BusMeta],
        views: &'v mut [Option<InputView<'io>>],
        silent: &'v [Vec<f32>],
        buf_off: usize,
        len: usize,
    ) -> [WindowedIn<'v>; MAX_BUSES] {
        // `iter_mut` yields each `Option` exactly once — disjoint `&mut`
        // borrows the borrow checker can prove (indexing could not).
        let mut it = views.iter_mut();
        array::from_fn(|i| {
            let Some(bus) = bus_in.get(i) else {
                let _ = it.next();
                return WindowedIn::default();
            };
            if !bus.descriptor.enabled {
                let _ = it.next();
                return WindowedIn::default();
            }
            // `Option<&'v InputView>` is `Copy` — each channel call
            // reborrows the same shared view.
            let view = it.next().and_then(Option::as_mut).map(|v| &*v);
            let channels =
                bus.descriptor
                    .channels()
                    .min(u32::try_from(MAX_CHANNELS).unwrap_or(u32::MAX)) as usize;
            let mut win = WindowedIn {
                count: channels,
                frames: len,
                ..WindowedIn::default()
            };
            for c in 0..channels {
                let plane: &[f32] = match view.and_then(|v| v.channel(c)) {
                    Some(plane) if buf_off + len <= plane.len() => &plane[buf_off..buf_off + len],
                    _ => silent.get(c).map_or(&[][..], |s| &s[..len.min(s.len())]),
                };
                win.channels[c] = Some(plane);
            }
            win
        })
    }

    /// Builds the erased per-bus output views for `[buf_off, buf_off+len)`.
    fn build_outputs<'v>(
        bus_out: &[BusMeta],
        views: &'v mut [Option<OutputView<'io>>],
        buf_off: usize,
        len: usize,
    ) -> [WindowedOut<'v, 'io>; MAX_BUSES] {
        let mut it = views.iter_mut();
        array::from_fn(|i| {
            let Some(bus) = bus_out.get(i) else {
                let _ = it.next();
                return WindowedOut::default();
            };
            if !bus.descriptor.enabled {
                let _ = it.next();
                return WindowedOut::default();
            }
            let view = match it.next() {
                Some(Some(view)) => Some(view),
                _ => None,
            };
            WindowedOut {
                view,
                off: buf_off,
                len,
                count: bus
                    .descriptor
                    .channels()
                    .min(u32::try_from(MAX_CHANNELS).unwrap_or(u32::MAX))
                    as usize,
            }
        })
    }
}

impl SpanSink<EventKind> for CallSink<'_, '_> {
    fn span(&mut self, span: &TransportSpan, events: &[ScheduledEvent<EventKind>]) {
        let span_start = match span.start.checked_offset(self.callback_start) {
            Some(off) => usize::try_from(off).unwrap_or(usize::MAX),
            // A span outside the callback window is a scheduler bug —
            // do not touch buffers; the scheduler already recorded a
            // fault.
            None => return,
        };
        let span_frames = usize::try_from(span.frames()).unwrap_or(usize::MAX);
        if !span.deliverable {
            self.silence_outputs(span_start, span_frames);
            self.status.bump(Counter::UndeliverableSpan);
            self.emit(NodeReport::UndeliverableSpan {
                start: span.start.frame(),
                end: span.end.frame(),
            });
            return;
        }
        let Some(partition) = calls_for_span(span, self.contract) else {
            // Defensive — `deliverable` was computed against the same
            // contract; treat as undeliverable rather than panic.
            self.silence_outputs(span_start, span_frames);
            self.status.bump(Counter::UndeliverableSpan);
            self.emit(NodeReport::UndeliverableSpan {
                start: span.start.frame(),
                end: span.end.frame(),
            });
            return;
        };
        // Transport discontinuity: a changed context identity means
        // note state delivered so far no longer describes the new
        // position — release every tracked note at the span's first
        // call, then chase the plan prefix for notes spanning the new
        // position so a seek/loop entry mid-note re-fires it.
        let (release_n, chase_n) = self.prepare_discontinuity(&span.ctx, span.frames());
        for chunk in 0..partition.chunks() {
            let (rel, len_u32) = partition.chunk(chunk);
            let buf_off = span_start + usize::try_from(rel).unwrap_or(usize::MAX);
            let len = usize::try_from(len_u32).unwrap_or(usize::MAX);
            if buf_off + len > self.buffer_frames {
                // A physical call window past the attached bound must
                // never reach the plugin — silence + auditable fault.
                self.silence_outputs(buf_off, len);
                self.status.bump(Counter::UndeliverableSpan);
                self.emit(NodeReport::UndeliverableSpan {
                    start: span.start.advance(u64::from(rel)).frame(),
                    end: span
                        .start
                        .advance(u64::from(rel) + u64::from(len_u32))
                        .frame(),
                });
                continue;
            }
            let (rel_n, ch_n) = if chunk == 0 {
                (release_n, chase_n)
            } else {
                (0, 0)
            };
            self.run_call(span, events, rel, len_u32, buf_off, len, rel_n, ch_n);
        }
    }
}

impl CallSink<'_, '_> {
    /// The transport-discontinuity pre-pass for a deliverable span:
    /// detects a changed [`SpanCtxKey`] or a position jump the key
    /// cannot see (`SetClock` rebase), releases every ledger-tracked
    /// note into `release_events` and chases the plan prefix into
    /// `chase_scratch`. Returns the emitted counts — the caller merges
    /// them as offset-0 sources at the span's first call.
    ///
    /// **RT contract:** RT-suitable — bounded writes, no allocation.
    fn prepare_discontinuity(
        &mut self,
        ctx: &ruviform_media_core::SpanContext,
        span_frames: u32,
    ) -> (usize, usize) {
        let ctx_key: SpanCtxKey = (
            ctx.clock_epoch,
            ctx.transport_generation,
            ctx.plan_generation,
            ctx.occurrence,
        );
        // The next continuous span must start where this span's
        // position ends — `position_at_offset` freezes for non-Playing
        // spans, so a paused run chains `position_ticks` unchanged.
        let expected = ctx.position_at_offset(span_frames).value();
        let continuous = matches!(
            *self.last_ctx,
            Some((key, pos)) if key == ctx_key && pos == ctx.position_ticks
        );
        *self.last_ctx = Some((ctx_key, expected));
        if continuous {
            return (0, 0);
        }
        let emitted = self.ledger.release_all(&mut *self.release_midi);
        let release_n = emitted.min(self.release_events.len());
        for (dst, midi) in self
            .release_events
            .iter_mut()
            .zip(&self.release_midi[..release_n])
        {
            *dst = Event::new(0, EventKind::Midi(*midi));
        }
        self.status
            .bump_by(Counter::NotesReleased, release_n as u64);
        // Emissions the release scratch could not hold are lost — the
        // ledger was still cleared (the fail-safe must not retain
        // phantom notes); count with chase loss.
        let mut overflow = emitted - release_n;
        let mut chase_n = 0usize;
        // Chase only when the snapshot IS the live plan — a
        // mid-callback install leaves the snapshot stale, and chasing
        // it would re-fire a retired plan's notes.
        if let Some(plan) = self.plan
            && plan.generation == ctx.plan_generation
        {
            let (chased, stats) = chase_span_events(plan, ctx, &mut *self.chase_scratch);
            chase_n = chased.len();
            overflow += stats.truncated + stats.untracked + usize::from(stats.incomplete);
            self.status
                .bump_by(Counter::NotesChased, stats.chased as u64);
        }
        if overflow > 0 {
            self.status.bump(Counter::ChaseOverflow);
            self.emit(NodeReport::ChaseOverflow {
                dropped: u64::try_from(overflow).unwrap_or(u64::MAX),
            });
        }
        (release_n, chase_n)
    }

    /// Drains the ledger's pending evicted-key synthetic offs into
    /// `evict_events` as offset-0 events — owed releases delivered
    /// ahead of every other source so a same-call re-attack lands
    /// after the release. Returns the emitted count.
    ///
    /// **RT contract:** RT-suitable — bounded writes, no allocation.
    fn drain_evicted(
        ledger: &mut NoteLedger,
        evict_midi: &mut [MidiEvent],
        evict_events: &mut [Event],
    ) -> usize {
        let written = ledger.drain_evicted(evict_midi);
        let n = written.min(evict_events.len());
        for (dst, midi) in evict_events.iter_mut().zip(&evict_midi[..n]) {
            *dst = Event::new(0, EventKind::Midi(*midi));
        }
        n
    }

    /// One physical `process` call: merges
    /// `evict ++ release ++ chase ++ plan ++ live` into the call's `EventList`
    /// (each source already offset-sorted; release/chase ride only the
    /// span's first call), feeds the delivered-note ledger with
    /// exactly what the plugin receives, then runs the endpoint.
    /// Every defined failure stays bounded, counted and reported.
    ///
    /// **RT contract:** RT-suitable — bounded work, no allocation.
    #[allow(clippy::too_many_arguments)]
    fn run_call(
        &mut self,
        span: &TransportSpan,
        events: &[ScheduledEvent<EventKind>],
        rel: u32,
        len_u32: u32,
        buf_off: usize,
        len: usize,
        release_n: usize,
        chase_n: usize,
    ) {
        let wins = Self::build_inputs(self.bus_in, &mut *self.in_views, self.silent, buf_off, len);
        let mut wouts = Self::build_outputs(self.bus_out, &mut *self.out_views, buf_off, len);
        // `iter()`/`iter_mut()` hand out disjoint element borrows;
        // both iterators yield exactly `MAX_BUSES` items.
        let mut in_it = wins.iter();
        let hins: [HostInput<'_>; MAX_BUSES] = array::from_fn(|_| {
            let Some(w) = in_it.next() else {
                unreachable!("iter len equals array len")
            };
            HostInput(w as &dyn BufferView)
        });
        let mut out_it = wouts.iter_mut();
        let mut houts: [HostOutput<'_>; MAX_BUSES] = array::from_fn(|_| {
            let Some(w) = out_it.next() else {
                unreachable!("iter len equals array len")
            };
            HostOutput(w as &mut dyn BufferViewMut)
        });
        let call_start = span.start.advance(u64::from(rel));
        let evict_n =
            Self::drain_evicted(self.ledger, &mut *self.evict_midi, &mut *self.evict_events);
        // Merge sources, each already sorted by `frame_offset`:
        // `[evict, release, chase, plan, live]` — owed teardown first
        // (ledger evictions, then the old context's release), then the
        // chase re-fires, the scheduled window, and live input.
        // Same-key same-offset collisions are normalized by the
        // merge's cross-source note-off priority.
        let (plan_list, mut overflow) = match build_call_event_list(
            span,
            events,
            call_start,
            len_u32,
            &mut *self.plan_scratch,
            |k| *k,
        ) {
            Some(list) => (list, false),
            // Defined fail-safe: the call runs with an empty list
            // and the drop is reported, never silently lost.
            None => (EventList::empty(len_u32), true),
        };
        let live: &[Event] = match self.input.as_deref_mut() {
            Some(stage) => {
                let (collected, _stats) =
                    stage.collect_block(call_start, len_u32, call_start, &mut *self.live_scratch);
                sort_events_by_frame_offset(collected, &mut *self.sort_scratch);
                collected
            }
            None => &[],
        };
        let sources = [
            &self.evict_events[..evict_n],
            &self.release_events[..release_n],
            &self.chase_scratch[..chase_n],
            plan_list.as_slice(),
            live,
        ];
        let merged = if let Ok(merged) =
            merge_call_events(&sources, &mut *self.scratch, &mut *self.src_scratch)
        {
            merged
        } else {
            // Bounded-scratch loss — report, deliver an empty list
            // rather than a partial merge.
            overflow = true;
            &mut self.scratch[..0]
        };
        let events = if let Ok(list) = EventList::new(merged, len_u32) {
            list
        } else {
            // The merge's sources were all validated sorted — treat a
            // rejection here as the same defined fail-safe.
            overflow = true;
            EventList::empty(len_u32)
        };
        // Feed the delivered-note ledger exactly what the plugin is
        // about to receive — a fall-back empty list feeds nothing, so
        // the ledger never claims undelivered notes.
        let mut ledger_overflow = false;
        for ev in events.iter() {
            if let EventKind::Midi(midi) = ev.kind() {
                self.ledger.feed(&midi);
                if self.ledger.take_overflow() {
                    ledger_overflow = true;
                }
            }
        }
        let transport = call_transport_info(&span.ctx, rel, None, None);
        self.status.bump(Counter::PhysicalCall);
        let result = self.endpoint.process(
            &hins[..self.bus_in.len()],
            &mut houts[..self.bus_out.len()],
            events,
            Some(transport),
        );
        // The `self.in_views`/`out_views` window borrows end with
        // `hins`/`houts` above — self is free again for reports.
        if overflow {
            self.status.bump(Counter::EventScratchOverflow);
            self.emit(NodeReport::EventScratchOverflow);
        }
        if ledger_overflow {
            self.status.bump(Counter::LedgerOverflow);
            self.emit(NodeReport::LedgerOverflow);
        }
        if let Err(err) = result {
            self.silence_outputs(buf_off, len);
            self.status.bump(Counter::EndpointError);
            self.emit(NodeReport::EndpointError {
                kind: match err {
                    HostError::Lifecycle(_) => 0,
                    HostError::Prepare(_) => 1,
                    HostError::EndpointUnavailable => 2,
                    _ => 3,
                },
            });
        }
    }
}

/// The RT-side adapter node — a `GraphProcessor` the typed runtime
/// installs; its non-RT twin is [`crate::PluginNodeControl`].
///
/// Build via [`crate::attach`] or the crate-internal
/// [`PluginNode::new`]; never construct the fields by hand.
pub struct PluginNode {
    endpoint: PluginEndpoint,
    scheduler: TransportScheduler<EventKind>,
    ctrl_rx: Consumer<NodeControl>,
    report_tx: Producer<NodeReport>,
    status: std::sync::Arc<NodeStatus>,
    cursor: EngineFrame,
    contract: BlockContract,
    bus_in: Vec<BusMeta>,
    bus_out: Vec<BusMeta>,
    event_scratch: Vec<Event>,
    event_src_scratch: Vec<u8>,
    plan_scratch: Vec<Event>,
    live_scratch: Vec<Event>,
    sort_scratch: Vec<Event>,
    release_midi: Vec<MidiEvent>,
    release_events: Vec<Event>,
    evict_midi: Vec<MidiEvent>,
    evict_events: Vec<Event>,
    chase_scratch: Vec<Event>,
    /// Live-input stage — `Some` when attached with
    /// `AttachConfig::input`; pumped at callback head and merged into
    /// every physical call.
    input_stage: Option<InputStage>,
    /// The RT side of the input handoff channel, drained into
    /// `input_stage` by [`InputStage::pump`].
    input_rx: Option<Consumer<TimedMidiInput>>,
    /// Delivered-note tracker across every event source (plan, live,
    /// chase) — the release authority on transport discontinuity.
    ledger: NoteLedger,
    /// Last deliverable span's context identity and expected successor
    /// position (discontinuity detection memory, carried across
    /// callbacks).
    last_ctx: Option<(SpanCtxKey, i64)>,
    silent: Vec<Vec<f32>>,
    vault: PlanVault,
}

impl PluginNode {
    /// Crate-internal constructor — [`crate::attach`] is the public
    /// surface. `buses` is the consumer-declared table (flattened
    /// plugin-facing order); `bus_in`/`bus_out` are its split views;
    /// `scratch_frames` is `bounds.max_frames` (the silent slab length).
    ///
    /// **RT contract:** non-RT — allocates scratch/ledgers.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        endpoint: PluginEndpoint,
        buses: &[BusDescriptor],
        contract: BlockContract,
        policy: DeliveryPolicy,
        budget: ScheduleBudget,
        frame_base: ruviform_media_core::TimeBase,
        epoch: ClockEpochId,
        ctrl_rx: Consumer<NodeControl>,
        report_tx: Producer<NodeReport>,
        status: std::sync::Arc<NodeStatus>,
        scratch_frames: usize,
        input_stage: Option<InputStage>,
        input_rx: Option<Consumer<TimedMidiInput>>,
    ) -> Self {
        let (bus_in, bus_out) = split_buses(buses);
        let silent_channels = bus_in
            .iter()
            .map(|b| b.descriptor.channels())
            .max()
            .unwrap_or(1)
            .min(u32::try_from(MAX_CHANNELS).unwrap_or(u32::MAX))
            as usize;
        let event_seed = Event::new(
            0,
            EventKind::ParamValue(ParamEvent {
                param_id: ParamId(0),
                value: ParamValue::new(0.0).unwrap(),
            }),
        );
        let midi_seed = MidiEvent {
            group: ruviform_media_core::MidiGroup::ZERO,
            channel: ruviform_media_core::MidiChannel::new(0).unwrap(),
            kind: ruviform_media_core::MidiKind::NoteOff {
                key: ruviform_media_core::NoteKey::new(60).unwrap(),
                velocity: 0,
            },
        };
        let per_span = budget.max_events_per_span as usize;
        // The merge output holds the worst-case union of every
        // bounded source: plan window + live input (each
        // `max_events_per_span`) plus evict + release + chase (each
        // `NOTE_LEDGER_CAPACITY`).
        let merge_capacity = 2 * per_span + 3 * NOTE_LEDGER_CAPACITY;
        Self {
            endpoint,
            scheduler: TransportScheduler::new(frame_base, contract, policy, budget),
            ctrl_rx,
            report_tx,
            status,
            cursor: EngineFrame::new(epoch, 0),
            contract,
            bus_in,
            bus_out,
            event_scratch: vec![event_seed; merge_capacity],
            event_src_scratch: vec![0; merge_capacity],
            plan_scratch: vec![event_seed; per_span],
            live_scratch: vec![event_seed; per_span],
            sort_scratch: vec![event_seed; per_span],
            release_midi: vec![midi_seed; NOTE_LEDGER_CAPACITY],
            release_events: vec![event_seed; NOTE_LEDGER_CAPACITY],
            evict_midi: vec![midi_seed; NOTE_LEDGER_CAPACITY],
            evict_events: vec![event_seed; NOTE_LEDGER_CAPACITY],
            chase_scratch: vec![event_seed; NOTE_LEDGER_CAPACITY],
            input_stage,
            input_rx,
            ledger: NoteLedger::new(),
            last_ctx: None,
            silent: (0..silent_channels.max(1))
                .map(|_| vec![0.0; scratch_frames])
                .collect(),
            vault: PlanVault::new(PLAN_VAULT_CAPACITY),
        }
    }

    /// Bounded control drain at callback head — `due_frames` are
    /// relative to the cursor *as it was before this callback*;
    /// `install_plan` vault-clones the `Arc` before staging.
    fn apply_controls(&mut self) {
        for _ in 0..CTRL_DRAIN_MAX {
            let Ok(ctrl) = self.ctrl_rx.try_pop() else {
                break;
            };
            match ctrl {
                NodeControl::Request { cmd, due_frames } => {
                    let due = self.cursor.advance(due_frames);
                    let _ = self.scheduler.request(cmd, due);
                }
                NodeControl::InstallPlan { plan, due_frames } => {
                    if self.vault.install(plan.clone()) {
                        let due = self.cursor.advance(due_frames);
                        let _ = self.scheduler.install_plan(plan, due);
                    } else {
                        self.status.bump(Counter::VaultOverflow);
                        if self.report_tx.try_push(NodeReport::VaultExceeded).is_err() {
                            self.status.bump(Counter::ReportOverflow);
                        }
                    }
                }
                NodeControl::SetClock { epoch, frame } => {
                    // The scheduler rejects every callback whose start
                    // epoch differs from its own — a foreign-epoch
                    // cursor move would stall output indefinitely.
                    // Same-epoch moves stay legal (the position
                    // discontinuity is caught by span-ctx tracking);
                    // epoch rebases go through `DeviceRestart`.
                    if epoch == self.scheduler.epoch() {
                        self.cursor = EngineFrame::new(epoch, frame);
                    } else {
                        self.status.bump(Counter::StaleEpochControl);
                        if self
                            .report_tx
                            .try_push(NodeReport::StaleEpochControl)
                            .is_err()
                        {
                            self.status.bump(Counter::ReportOverflow);
                        }
                    }
                }
                NodeControl::DeviceRestart { base_frame } => {
                    let epoch = self.scheduler.device_restart(base_frame);
                    self.cursor = EngineFrame::new(epoch, base_frame);
                }
                NodeControl::SetClockAnchor { anchor } => {
                    // Defined no-op when the node has no input leg.
                    if let Some(stage) = &mut self.input_stage {
                        stage.publish_anchor(anchor);
                    }
                }
            }
        }
    }

    /// Moves scheduler receipts/faults into the bounded report channel.
    fn flush_reports(&mut self) {
        for _ in 0..REPORT_CHANNEL_CAPACITY {
            let mut drained = false;
            if let Some(r) = self.scheduler.pop_receipt() {
                drained = true;
                if self.report_tx.try_push(NodeReport::Receipt(r)).is_err() {
                    self.status.bump(Counter::ReportOverflow);
                }
            }
            if let Some(f) = self.scheduler.pop_fault() {
                drained = true;
                if self.report_tx.try_push(NodeReport::Fault(f)).is_err() {
                    self.status.bump(Counter::ReportOverflow);
                }
            }
            if !drained {
                break;
            }
        }
    }
}

impl GraphProcessor for PluginNode {
    fn latency(&self) -> u32 {
        self.endpoint.latency_frames()
    }

    fn process(&mut self, io: &mut NodeIo<'_>, _ctx: &RtContext) {
        let Ok(frames) = u32::try_from(io.frames()) else {
            return;
        };
        self.apply_controls();
        // Move queued live events into the stage before spans run —
        // anything that arrives during the callback waits for the
        // next one (channel backpressure, bounded).
        if let (Some(stage), Some(rx)) = (&mut self.input_stage, &mut self.input_rx) {
            stage.pump(rx);
        }
        let mut in_views: [Option<InputView<'_>>; MAX_BUSES] = array::from_fn(|i| io.input(i));
        let mut out_views: [Option<OutputView<'_>>; MAX_BUSES] = array::from_fn(|i| io.output(i));
        let start = self.cursor;
        // Snapshot the active plan for span chase — an `Arc` refcount
        // bump only. The vault already holds a reference, so this
        // clone's drop at callback end can never run the plan
        // destructor on the RT path.
        let plan = self.scheduler.plan().cloned();
        {
            let mut sink = CallSink {
                callback_start: start,
                contract: &self.contract,
                endpoint: &mut self.endpoint,
                report: &mut self.report_tx,
                status: &self.status,
                scratch: &mut self.event_scratch,
                plan_scratch: &mut self.plan_scratch,
                live_scratch: &mut self.live_scratch,
                sort_scratch: &mut self.sort_scratch,
                release_midi: &mut self.release_midi,
                release_events: &mut self.release_events,
                src_scratch: &mut self.event_src_scratch,
                evict_midi: &mut self.evict_midi,
                evict_events: &mut self.evict_events,
                chase_scratch: &mut self.chase_scratch,
                input: self.input_stage.as_mut(),
                ledger: &mut self.ledger,
                plan: plan.as_deref(),
                last_ctx: &mut self.last_ctx,
                in_views: &mut in_views,
                out_views: &mut out_views,
                silent: &self.silent,
                bus_in: &self.bus_in,
                bus_out: &self.bus_out,
                buffer_frames: usize::try_from(frames).unwrap_or(usize::MAX),
            };
            self.scheduler.run_callback(start, frames, &mut sink);
        }
        // Mirror live-input counters for the control surface.
        if let Some(stage) = &self.input_stage {
            self.status.set_input(stage.counters());
        }
        self.cursor = start.advance(u64::from(frames));
        self.flush_reports();
    }
}
