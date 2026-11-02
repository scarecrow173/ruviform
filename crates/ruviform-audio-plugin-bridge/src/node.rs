//! [`PluginNode`] — the RT-side adapter: wraps a `PluginEndpoint`,
//! embeds a `TransportScheduler<EventKind>` and turns each callback
//! into a bounded sequence of legal physical `process` calls.
//!
//! ## Data flow (one callback)
//!
//! 1. drain ≤ [`CTRL_DRAIN_MAX`] control messages — `request` /
//!    `install_plan` (vault-cloned first) / `set_clock` /
//!    `device_restart` are applied at callback head, `due_frames`
//!    relative to the node's own cursor.
//! 2. materialize `NodeIo` input/output port views (stack arrays — no
//!    allocation).
//! 3. `scheduler.run_callback(cursor, frames, sink)` — every
//!    `deliverable` span becomes a `BlockContract` partition of
//!    physical calls with process-start-relative `EventList`s and a
//!    `TransportInfo` snapshot; every undeliverable span becomes
//!    silence plus a [`NodeReport::UndeliverableSpan`].
//! 4. `pop_receipt`/`pop_fault` drain into the bounded report channel
//!    (full → [`NodeStatus::snapshot`]'s `report_overflows`).
//!
//! No allocation, no blocking, no `unsafe` on any of this.

use core::array;

use ruviform_audio::typed::{GraphProcessor, InputView, NodeIo, OutputView};
use ruviform_graph::{Direction, NodeSpec, PortFormat, PortRole};
use ruviform_media_core::{
    BlockContract, ClockEpochId, DeliveryPolicy, EngineFrame, ScheduleBudget, ScheduledEvent,
    SpanSink, TransportScheduler, TransportSpan,
};
use ruviform_plugin_core::{
    BufferView, BufferViewMut, BusDescriptor, BusDirection, BusRole, Event, EventKind, EventList,
    HostInput, HostOutput, ParamEvent, ParamId, ParamValue,
};
use ruviform_plugin_core::{build_call_event_list, call_transport_info, calls_for_span};
use ruviform_plugin_host::{HostError, PluginEndpoint};
use ruviform_rt::RtContext;
use ruviform_rt::{Consumer, Producer};

use crate::control::{NodeControl, NodeReport};
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

/// The sink `run_callback` feeds — holds disjoint borrows of the
/// node's fields so `PluginNode::process` satisfies the borrow checker
/// while staying allocation-free.
struct CallSink<'a, 'io> {
    callback_start: EngineFrame,
    contract: &'a BlockContract,
    endpoint: &'a mut PluginEndpoint,
    report: &'a mut Producer<NodeReport>,
    status: &'a NodeStatus,
    scratch: &'a mut [Event],
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
            let wins =
                Self::build_inputs(self.bus_in, &mut *self.in_views, self.silent, buf_off, len);
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
            let (events, overflow) = match build_call_event_list(
                span,
                events,
                call_start,
                len_u32,
                &mut *self.scratch,
                |k| *k,
            ) {
                Some(list) => (list, false),
                // Defined fail-safe: the call runs with an empty list
                // and the drop is reported, never silently lost.
                None => (EventList::empty(len_u32), true),
            };
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
    ) -> Self {
        let (bus_in, bus_out) = split_buses(buses);
        let silent_channels = bus_in
            .iter()
            .map(|b| b.descriptor.channels())
            .max()
            .unwrap_or(1)
            .min(u32::try_from(MAX_CHANNELS).unwrap_or(u32::MAX))
            as usize;
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
            event_scratch: vec![
                Event::new(
                    0,
                    EventKind::ParamValue(ParamEvent {
                        param_id: ParamId(0),
                        value: ParamValue::new(0.0).unwrap(),
                    }),
                );
                budget.max_events_per_span as usize
            ],
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
                    self.cursor = EngineFrame::new(epoch, frame);
                }
                NodeControl::DeviceRestart { base_frame } => {
                    let epoch = self.scheduler.device_restart(base_frame);
                    self.cursor = EngineFrame::new(epoch, base_frame);
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
        let mut in_views: [Option<InputView<'_>>; MAX_BUSES] = array::from_fn(|i| io.input(i));
        let mut out_views: [Option<OutputView<'_>>; MAX_BUSES] = array::from_fn(|i| io.output(i));
        let start = self.cursor;
        {
            let mut sink = CallSink {
                callback_start: start,
                contract: &self.contract,
                endpoint: &mut self.endpoint,
                report: &mut self.report_tx,
                status: &self.status,
                scratch: &mut self.event_scratch,
                in_views: &mut in_views,
                out_views: &mut out_views,
                silent: &self.silent,
                bus_in: &self.bus_in,
                bus_out: &self.bus_out,
                buffer_frames: usize::try_from(frames).unwrap_or(usize::MAX),
            };
            self.scheduler.run_callback(start, frames, &mut sink);
        }
        self.cursor = start.advance(u64::from(frames));
        self.flush_reports();
    }
}
