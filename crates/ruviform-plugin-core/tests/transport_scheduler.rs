//! Mock Driver `PoC` for the Issue #190 generic transport span scheduler:
//! drives `TransportScheduler` callbacks through the plugin-core
//! transport bridge into a recording `PluginProcessor` and asserts the
//! Issue #190 / Synvelis §14 acceptance matrix (TT-006..030 scope).

use std::sync::Arc;

use ruviform_media_core::{
    ApplyDegradation, ApplyStatus, BlockContract, DeliveryPolicy, EngineFrame, FaultKind,
    InBlockTransportSupport, LoopOccurrence, MediaDuration, MediaRange, MediaTime, RejectReason,
    ScheduleBudget, ScheduledEvent, SpanSink, TempoMap, TimeBase, TransportCommand,
    TransportPlanBuilder, TransportScheduler, TransportSpan, TransportStatus,
};
use ruviform_plugin_core::{
    BufferView, BufferViewMut, Event, EventKind, EventList, ParamEvent, ParamId, ParamValue,
    PluginProcessor, PrepareError, ProcessConfig, ProcessContext, ProcessResult, TransportInfo,
    build_call_event_list, call_transport_info, calls_for_span, contract_from_process_config,
};

fn base() -> TimeBase {
    TimeBase::sample_rate(48_000).unwrap()
}

fn t(v: i64) -> MediaTime {
    MediaTime::new(v, base())
}

fn range(s: i64, e: i64) -> MediaRange {
    MediaRange::new(t(s), MediaDuration::new(e - s, base()))
}

#[allow(clippy::trivially_copy_pass_by_ref)] // signature fixed by fn(&T)
fn kind(p: &u32) -> EventKind {
    EventKind::ParamValue(ParamEvent {
        param_id: ParamId(*p),
        value: ParamValue::new(1.0).unwrap(),
    })
}

/// One recorded physical `process` call.
#[derive(Debug)]
struct RecordedCall {
    frames: u32,
    status: TransportStatus,
    position: MediaTime,
    loop_range: Option<MediaRange>,
    occurrence: LoopOccurrence,
    events: Vec<(u32, u32)>, // (frame_offset, param_id payload)
}

struct In<'a>(&'a [f32]);
impl BufferView for In<'_> {
    fn channels(&self) -> usize {
        1
    }
    fn frames(&self) -> usize {
        self.0.len()
    }
    fn channel(&self, index: usize) -> Option<&[f32]> {
        (index == 0).then_some(self.0)
    }
}

struct Out<'a>(&'a mut [f32]);
impl BufferView for Out<'_> {
    fn channels(&self) -> usize {
        1
    }
    fn frames(&self) -> usize {
        self.0.len()
    }
    fn channel(&self, index: usize) -> Option<&[f32]> {
        (index == 0).then_some(&*self.0)
    }
}
impl BufferViewMut for Out<'_> {
    fn channel_mut(&mut self, index: usize) -> Option<&mut [f32]> {
        (index == 0).then_some(&mut *self.0)
    }
}

struct RecordingProcessor {
    calls: Vec<RecordedCall>,
    prepared: Option<ProcessConfig>,
}

impl RecordingProcessor {
    fn new() -> Self {
        Self {
            calls: Vec::new(),
            prepared: None,
        }
    }
}

impl PluginProcessor for RecordingProcessor {
    fn prepare(
        &mut self,
        config: &ProcessConfig,
        _context: &mut ProcessContext,
    ) -> Result<(), PrepareError> {
        self.prepared = Some(*config);
        Ok(())
    }

    fn process<I, O>(
        &mut self,
        inputs: &[I],
        outputs: &mut [O],
        events: EventList<'_>,
        transport: Option<TransportInfo>,
        _context: &mut ProcessContext,
    ) -> ProcessResult
    where
        I: BufferView,
        O: BufferViewMut,
    {
        let frames = outputs.first().map_or(0, BufferView::frames);
        let _ = inputs;
        let info = transport.expect("mock driver always supplies transport");
        let mut evs = Vec::new();
        for e in events.iter() {
            let EventKind::ParamValue(p) = e.kind() else {
                panic!("unexpected event kind")
            };
            evs.push((e.frame_offset(), p.param_id.0));
        }
        self.calls.push(RecordedCall {
            frames: u32::try_from(frames).unwrap_or(u32::MAX),
            status: info.status(),
            position: info.position(),
            loop_range: info.loop_range(),
            occurrence: LoopOccurrence::new(0),
            events: evs,
        });
        ProcessResult::CONTINUE
    }
}

/// The span→physical-call sink: converts each logical span into legal
/// `process` calls — undeliverable spans are recorded, never called.
struct DriverSink<'a> {
    proc: &'a mut RecordingProcessor,
    contract: BlockContract,
    tempo_map: Option<&'a TempoMap>,
    ppqn: Option<ruviform_media_core::Ppqn>,
    event_scratch: &'a mut [Event],
    buffer: &'a mut [f32],
    skipped: &'a mut Vec<(u64, u64)>,
    span_ctx: &'a mut Vec<ruviform_media_core::SpanContext>,
}

impl SpanSink<u32> for DriverSink<'_> {
    fn span(&mut self, span: &TransportSpan, events: &[ScheduledEvent<u32>]) {
        self.span_ctx.push(span.ctx);
        if !span.deliverable {
            self.skipped.push((span.start.frame(), span.end.frame()));
            return;
        }
        let Some(part) = calls_for_span(span, &self.contract) else {
            self.skipped.push((span.start.frame(), span.end.frame()));
            return;
        };
        for i in 0..part.chunks() {
            let (off, frames) = part.chunk(i);
            let call_start = span.start.advance(u64::from(off));
            let info = call_transport_info(&span.ctx, off, self.tempo_map, self.ppqn);
            let Some(list) =
                build_call_event_list(span, events, call_start, frames, self.event_scratch, kind)
            else {
                self.skipped.push((span.start.frame(), span.end.frame()));
                return;
            };
            let mut out = Out(&mut self.buffer[..frames as usize]);
            let inn = In(&[]);
            let mut ctx = ProcessContext::new();
            self.proc.process(
                &[inn],
                std::slice::from_mut(&mut out),
                list,
                Some(info),
                &mut ctx,
            );
            let recorded = self.proc.calls.last_mut().unwrap();
            recorded.occurrence = span.ctx.occurrence;
        }
    }
}

struct MockDriver {
    sched: TransportScheduler<u32>,
    contract: BlockContract,
    proc: RecordingProcessor,
    tempo_map: Option<Arc<TempoMap>>,
    ppqn: Option<ruviform_media_core::Ppqn>,
    event_scratch: Vec<Event>,
    buffer: Vec<f32>,
    skipped: Vec<(u64, u64)>,
    span_ctx: Vec<ruviform_media_core::SpanContext>,
    next_frame: u64,
}

impl MockDriver {
    fn new(min_frames: u32, max_frames: u32, policy: DeliveryPolicy) -> Self {
        let config = ProcessConfig::new(
            ruviform_audio_core::SampleRate::new(48_000).unwrap(),
            min_frames,
            max_frames,
        )
        .unwrap();
        let contract = contract_from_process_config(&config, InBlockTransportSupport::Unknown);
        let mut proc = RecordingProcessor::new();
        proc.prepare(&config, &mut ProcessContext::new()).unwrap();
        Self {
            sched: TransportScheduler::new(base(), contract, policy, ScheduleBudget::default()),
            contract,
            proc,
            tempo_map: None,
            ppqn: None,
            event_scratch: vec![Event::new(0, kind(&0)); 512],
            buffer: vec![0.0; 8192],
            skipped: Vec::new(),
            span_ctx: Vec::new(),
            next_frame: 0,
        }
    }

    fn epoch_frame(&self, f: u64) -> EngineFrame {
        EngineFrame::new(self.sched.epoch(), f)
    }

    fn drive(&mut self, frames: u32) {
        let start = self.epoch_frame(self.next_frame);
        self.next_frame += u64::from(frames);
        let mut sink = DriverSink {
            proc: &mut self.proc,
            contract: self.contract,
            tempo_map: self.tempo_map.as_deref(),
            ppqn: self.ppqn,
            event_scratch: &mut self.event_scratch,
            buffer: &mut self.buffer,
            skipped: &mut self.skipped,
            span_ctx: &mut self.span_ctx,
        };
        self.sched.run_callback(start, frames, &mut sink);
    }

    fn build_plan(
        &mut self,
        entries: &[(i64, u64, u32)],
    ) -> Arc<ruviform_media_core::TransportPlan<u32>> {
        let mut b = self.sched.plan_builder();
        for &(pos, seq, p) in entries {
            b.event(t(pos), seq, p).unwrap();
        }
        Arc::new(b.build())
    }
}

// TT-008 / AC-1: Requested → Pending → Applied + actual applied frame.
#[test]
fn tt008_command_receipt_lifecycle() {
    let mut d = MockDriver::new(1, 512, DeliveryPolicy::Exact);
    let play = d
        .sched
        .request(TransportCommand::Play, d.epoch_frame(0))
        .unwrap();
    let seek = d
        .sched
        .request(TransportCommand::Seek(t(1000)), d.epoch_frame(200))
        .unwrap();
    let pause = d
        .sched
        .request(TransportCommand::Pause, d.epoch_frame(300))
        .unwrap();
    d.drive(256);
    d.drive(256);
    let r = d.sched.drain_receipts();
    let play_statuses: Vec<ApplyStatus> = r
        .iter()
        .filter(|x| x.id == play)
        .map(|x| x.status)
        .collect();
    assert_eq!(
        play_statuses,
        vec![
            ApplyStatus::Requested,
            ApplyStatus::Pending,
            ApplyStatus::Applied
        ]
    );
    let seek_applied = r
        .iter()
        .find(|x| x.id == seek && x.status == ApplyStatus::Applied)
        .unwrap();
    assert_eq!(seek_applied.applied_frame, Some(d.epoch_frame(200)));
    let pause_applied = r
        .iter()
        .find(|x| x.id == pause && x.status == ApplyStatus::Applied)
        .unwrap();
    assert_eq!(pause_applied.applied_frame, Some(d.epoch_frame(300)));
    // Second callback is paused: position frozen at 1000.
    let last = d.proc.calls.last().unwrap();
    assert_eq!(last.status, TransportStatus::Paused);
}

// TT-006/TT-025 / AC-2: 256-frame callback, loop wrap at frame 100 →
// [0,100) + [100,256), each call gets its own transport position.
#[test]
fn tt025_loop_wrap_splits_and_transports() {
    let mut d = MockDriver::new(1, 512, DeliveryPolicy::Exact);
    d.sched
        .request(TransportCommand::Play, d.epoch_frame(0))
        .unwrap();
    d.sched
        .request(TransportCommand::Seek(t(100)), d.epoch_frame(0))
        .unwrap();
    d.sched
        .request(TransportCommand::SetLoop(range(0, 200)), d.epoch_frame(0))
        .unwrap();
    d.drive(256);
    assert_eq!(d.proc.calls.len(), 2);
    assert_eq!(d.proc.calls[0].frames, 100);
    assert_eq!(d.proc.calls[0].position, t(100));
    assert_eq!(d.proc.calls[1].frames, 156);
    assert_eq!(d.proc.calls[1].position, t(0));
    assert_eq!(d.proc.calls[1].loop_range, Some(range(0, 200)));
    assert_eq!(d.proc.calls[1].occurrence, LoopOccurrence::new(1));
    assert_eq!(d.skipped, [] as [(u64, u64); 0]);
}

// TT-026a / AC-3: min_frames=128 — the 100-frame call must NEVER happen.
// Exact: wrap produces a defined fault + undeliverable span; a command
// apply at 100 is rejected.
#[test]
fn tt026_min_frames_exact_never_calls_under_min() {
    let mut d = MockDriver::new(128, 512, DeliveryPolicy::Exact);
    d.sched
        .request(TransportCommand::Play, d.epoch_frame(0))
        .unwrap();
    // Command apply at an illegal boundary: preflight rejects.
    let pf = d.sched.preflight(
        TransportCommand::Seek(t(9)),
        d.epoch_frame(100),
        d.epoch_frame(0),
        256,
    );
    assert_eq!(pf, Err(RejectReason::UnsupportedExactTransport));
    // Requested anyway → RT rejects it, last-good state preserved.
    let seek = d
        .sched
        .request(TransportCommand::Seek(t(9)), d.epoch_frame(100))
        .unwrap();
    d.sched
        .request(TransportCommand::SetLoop(range(0, 100)), d.epoch_frame(0))
        .unwrap();
    d.drive(256);
    // No call may violate min_frames.
    assert!(d.proc.calls.iter().all(|c| c.frames >= 128));
    // Seek was rejected, not applied.
    let r = d.sched.drain_receipts();
    let seek_rej = r
        .iter()
        .find(|x| x.id == seek && x.status == ApplyStatus::Rejected)
        .unwrap();
    assert_eq!(
        seek_rej.reason,
        Some(RejectReason::UnsupportedExactTransport)
    );
    // The intrinsic wrap produced a defined fault + undeliverable span.
    let faults = d.sched.drain_faults();
    assert!(
        faults
            .iter()
            .any(|f| f.kind == FaultKind::UnrepresentableLoopWrap)
    );
    assert_ne!(d.skipped, [] as [(u64, u64); 0]);
}

// TT-026b / AC-3, AC-11: explicit Degraded defers the apply to the next
// legal boundary; receipt records requested vs actual frame.
#[test]
fn tt026_degraded_defers_to_legal_boundary() {
    let mut d = MockDriver::new(128, 512, DeliveryPolicy::Degraded);
    d.sched
        .request(TransportCommand::Play, d.epoch_frame(0))
        .unwrap();
    let seek = d
        .sched
        .request(TransportCommand::Seek(t(9)), d.epoch_frame(100))
        .unwrap();
    d.drive(256);
    assert_eq!(d.proc.calls.len(), 2);
    assert_eq!(d.proc.calls[0].frames, 128);
    assert_eq!(d.proc.calls[1].frames, 128);
    assert!(d.proc.calls.iter().all(|c| c.frames >= 128));
    let r = d.sched.drain_receipts();
    let applied = r
        .iter()
        .find(|x| x.id == seek && x.status == ApplyStatus::Applied)
        .unwrap();
    assert_eq!(applied.applied_frame, Some(d.epoch_frame(128)));
    assert_eq!(
        applied.degradation,
        Some(ApplyDegradation::DeferredApply {
            requested: d.epoch_frame(100),
            applied: d.epoch_frame(128),
        })
    );
}

// TT-027 / AC-10: unproven in-block transport support (Unknown and
// Declared alike) must not be treated as supported — exact still
// rejects; only Verified could enable the in-block path (not in PoC).
#[test]
fn tt027_in_block_unproven_not_treated_as_supported() {
    for cap in [
        InBlockTransportSupport::Unknown,
        InBlockTransportSupport::Declared,
    ] {
        let contract = BlockContract::new(128, 512, cap);
        let mut sched: TransportScheduler<u32> = TransportScheduler::new(
            base(),
            contract,
            DeliveryPolicy::Exact,
            ScheduleBudget::default(),
        );
        sched
            .request(TransportCommand::Play, EngineFrame::new(sched.epoch(), 0))
            .unwrap();
        let pf = sched.preflight(
            TransportCommand::Seek(t(1)),
            EngineFrame::new(sched.epoch(), 100),
            EngineFrame::new(sched.epoch(), 0),
            256,
        );
        assert_eq!(pf, Err(RejectReason::UnsupportedExactTransport));
    }
}

// TT-028 / AC-4: events at callback frames 100/120/256 land at
// process-relative offsets 0/20/next-callback-0 — no dup, no loss.
#[test]
fn tt028_event_process_relative_offsets() {
    let mut d = MockDriver::new(1, 512, DeliveryPolicy::Exact);
    d.sched
        .request(TransportCommand::Play, d.epoch_frame(0))
        .unwrap();
    d.sched
        .request(TransportCommand::Seek(t(100)), d.epoch_frame(0))
        .unwrap();
    d.sched
        .request(TransportCommand::SetLoop(range(0, 200)), d.epoch_frame(0))
        .unwrap();
    // positions: 0 → engine 100 (call2 offset 0); 20 → engine 120
    // (offset 20); 156 → engine 256 (next callback offset 0).
    let plan = d.build_plan(&[(0, 1, 42), (20, 2, 43), (156, 3, 44)]);
    d.sched.install_plan(plan, d.epoch_frame(0)).unwrap();
    d.drive(256);
    d.drive(256);
    // Loop [0,200) starting at pos 100: cb1 = [0,100) pos[100,200) +
    // [100,256) pos[0,156); cb2 = [256,300) pos[156,200) +
    // [300,500) pos[0,200) + [500,512) pos[0,12). Absolute-position
    // events fire once per occurrence: pos0 → engine 100/300/500
    // (offset 0 each), pos20 → 120/320 (offset 20), pos156 →
    // 56/256/456 (offsets 56/0/156).
    let evs: Vec<(u32, u32)> = d.proc.calls.iter().flat_map(|c| c.events.clone()).collect();
    assert_eq!(evs.iter().filter(|e| e.1 == 42).count(), 3);
    assert!(evs.contains(&(0, 42)));
    assert!(evs.contains(&(20, 43)));
    // pos156 fires at process-relative offsets 56, 0, 156 — never
    // duplicated inside one call/occurrence.
    let e44: Vec<u32> = evs.iter().filter(|e| e.1 == 44).map(|e| e.0).collect();
    assert_eq!(e44, vec![56, 0, 156]);
}

// TT-007/TT-029 / AC-5: multi-occurrence loop — the same source event
// fires exactly once per occurrence, never duplicated, never lost.
#[test]
fn tt029_loop_occurrence_distinguishes_deliveries() {
    let mut d = MockDriver::new(1, 512, DeliveryPolicy::Exact);
    d.sched
        .request(TransportCommand::Play, d.epoch_frame(0))
        .unwrap();
    d.sched
        .request(TransportCommand::Seek(t(100)), d.epoch_frame(0))
        .unwrap();
    d.sched
        .request(TransportCommand::SetLoop(range(0, 200)), d.epoch_frame(0))
        .unwrap();
    let plan = d.build_plan(&[(50, 1, 7)]);
    d.sched.install_plan(plan, d.epoch_frame(0)).unwrap();
    for _ in 0..4 {
        d.drive(256);
    }
    // pos 50 fires once per occurrence: wraps land at engine 300/500/
    // 700/900, so deliveries are at engine 150 (occ1), 350, 550, 750,
    // 950 — exactly once each, in distinct occurrences.
    let occs: Vec<LoopOccurrence> = d
        .proc
        .calls
        .iter()
        .filter(|c| c.events.iter().any(|&(_, p)| p == 7))
        .map(|c| c.occurrence)
        .collect();
    assert_eq!(occs.len(), 5);
    assert_eq!(
        occs.iter().collect::<std::collections::HashSet<_>>().len(),
        occs.len(),
        "one delivery per occurrence, each occurrence distinguishable"
    );
    assert!(occs.contains(&LoopOccurrence::new(1)));
    assert!(occs.contains(&LoopOccurrence::new(2)));
    assert!(occs.contains(&LoopOccurrence::new(3)));
    // Total deliveries of event 7 across 4 callbacks = 5 occurrences.
    let total: usize = d
        .proc
        .calls
        .iter()
        .map(|c| c.events.iter().filter(|&&(_, p)| p == 7).count())
        .sum();
    assert_eq!(total, 5);
    // And no call ever delivered the same source event twice.
    for c in &d.proc.calls {
        assert!(c.events.iter().filter(|&&(_, p)| p == 7).count() <= 1);
    }
}

// T-010 / AC-4: same-frame events keep explicit causal source order.
#[test]
fn tt010_same_frame_source_order() {
    let mut d = MockDriver::new(1, 512, DeliveryPolicy::Exact);
    d.sched
        .request(TransportCommand::Play, d.epoch_frame(0))
        .unwrap();
    let plan = d.build_plan(&[(100, 3, 30), (100, 1, 10), (100, 2, 20)]);
    d.sched.install_plan(plan, d.epoch_frame(0)).unwrap();
    d.drive(256);
    let evs: Vec<(u32, u32)> = d.proc.calls[0].events.clone();
    assert_eq!(
        evs,
        vec![(100, 10), (100, 20), (100, 30)],
        "stable source_sequence order at one frame"
    );
}

// AC-6: stale generation events are not delivered — defined fault only.
#[test]
fn stale_plan_generation_event_not_delivered() {
    let mut d = MockDriver::new(1, 512, DeliveryPolicy::Exact);
    d.sched
        .request(TransportCommand::Play, d.epoch_frame(0))
        .unwrap();
    let tg = d.sched.transport_generation();
    let mut b = TransportPlanBuilder::new(base(), d.sched.next_plan_generation());
    b.bound_event(
        t(100),
        1,
        ruviform_media_core::TransportGeneration::new(tg.get() + 7),
        5,
    )
    .unwrap();
    let plan = Arc::new(b.build());
    d.sched.install_plan(plan, d.epoch_frame(0)).unwrap();
    d.drive(256);
    assert!(
        d.proc
            .calls
            .iter()
            .all(|c| c.events.iter().all(|&(_, p)| p != 5))
    );
    let faults = d.sched.drain_faults();
    assert!(
        faults
            .iter()
            .any(|f| f.kind == FaultKind::StaleEventGeneration)
    );
}

// TT-009 / AC-7: device restart (clock epoch) and transport seek
// (transport generation) are observable as distinct changes.
#[test]
fn tt009_restart_vs_seek_epoch_generation() {
    let mut d = MockDriver::new(1, 512, DeliveryPolicy::Exact);
    let e0 = d.sched.epoch();
    let tg0 = d.sched.transport_generation();
    d.sched
        .request(TransportCommand::Play, d.epoch_frame(0))
        .unwrap();
    d.next_frame = 0;
    let e1 = d.sched.device_restart(0);
    assert_ne!(e0, e1);
    assert_eq!(d.sched.epoch(), e1);
    // Seek in the new epoch: tg changes, epoch does not.
    d.next_frame = 0;
    d.sched
        .request(TransportCommand::Seek(t(5)), EngineFrame::new(e1, 10))
        .unwrap();
    d.drive(256);
    assert_eq!(d.sched.epoch(), e1);
    assert_ne!(d.sched.transport_generation(), tg0);
    let r = d.sched.drain_receipts();
    let seek_applied = r.iter().find(|x| x.status == ApplyStatus::Applied).unwrap();
    assert_eq!(seek_applied.clock_epoch, e1);
}

// TT-012 / AC-8: span budget overflow is a defined fault, not unbounded
// splitting.
#[test]
fn tt012_span_budget_overflow_faults() {
    let mut d = MockDriver::new(1, 512, DeliveryPolicy::Exact);
    d.sched = TransportScheduler::new(
        base(),
        d.contract,
        DeliveryPolicy::Exact,
        ScheduleBudget::new(2, 256, 16, 4096),
    );
    d.sched
        .request(TransportCommand::Play, d.epoch_frame(0))
        .unwrap();
    // Loop len 50 → many wraps per 256 callback.
    d.sched
        .request(TransportCommand::SetLoop(range(0, 50)), d.epoch_frame(0))
        .unwrap();
    d.drive(256);
    let faults = d.sched.drain_faults();
    assert!(
        faults
            .iter()
            .any(|f| f.kind == FaultKind::SpanBudgetExceeded)
    );
}

// T-029b / AC-9: zero-frame flush keeps the wire contract — offset 0 is
// legal only for block_frames == 0.
#[test]
fn tt029_zero_frame_flush_contract() {
    let mut proc = RecordingProcessor::new();
    let config = ProcessConfig::new(
        ruviform_audio_core::SampleRate::new(48_000).unwrap(),
        1,
        512,
    )
    .unwrap();
    proc.prepare(&config, &mut ProcessContext::new()).unwrap();
    let ev = Event::new(0, kind(&1));
    let evs = [ev];
    let list = EventList::new(&evs, 0).unwrap();
    let mut ctx = ProcessContext::new();
    let mut out = Out(&mut []);
    let info = TransportInfo::new(TransportStatus::Playing, t(0));
    proc.process::<In, Out>(
        &[],
        std::slice::from_mut(&mut out),
        list,
        Some(info),
        &mut ctx,
    );
    assert_eq!(proc.calls[0].frames, 0);
    assert_eq!(proc.calls[0].events, vec![(0, 1)]);
    // A nonzero offset in a zero-frame block is rejected by the wire contract.
    let bad = [Event::new(4, kind(&2))];
    assert!(EventList::new(&bad, 0).is_err());
}

// T-029c / AC-10: L0 min-gate vs L1 delegated minimum — the effective
// activated min is what the contract enforces.
#[test]
fn tt029_l0_l1_min_frames_difference() {
    let l0 = ProcessConfig::new(
        ruviform_audio_core::SampleRate::new(48_000).unwrap(),
        128,
        512,
    )
    .unwrap();
    let c0 = contract_from_process_config(&l0, InBlockTransportSupport::Unknown);
    assert!(!c0.legal_span_len(100));
    // L1-style child-delegated contract exposes min_frames = 0 → the
    // same span becomes legal under the child contract.
    let c1 = BlockContract::new(0, 512, InBlockTransportSupport::Unknown);
    assert!(c1.legal_span_len(100));
}

// TT-030 / AC-11: Offline Exact never silently quantizes; Live Degraded
// only when explicitly selected.
#[test]
fn tt030_offline_exact_vs_live_degraded() {
    // Exact (offline path): illegal apply boundary → reject, no delivery.
    let mut exact = MockDriver::new(128, 512, DeliveryPolicy::Exact);
    exact
        .sched
        .request(TransportCommand::Play, exact.epoch_frame(0))
        .unwrap();
    let seek = exact
        .sched
        .request(TransportCommand::Seek(t(3)), exact.epoch_frame(100))
        .unwrap();
    exact.drive(256);
    let r = exact.sched.drain_receipts();
    assert!(
        r.iter()
            .any(|x| x.id == seek && x.status == ApplyStatus::Rejected)
    );
    // Position unchanged — no silent quantize to a legal frame.
    assert_eq!(exact.proc.calls.last().unwrap().position, t(0));
    // Degraded (live, explicitly selected): deferred apply is recorded.
    let mut deg = MockDriver::new(128, 512, DeliveryPolicy::Degraded);
    deg.sched
        .request(TransportCommand::Play, deg.epoch_frame(0))
        .unwrap();
    let seek = deg
        .sched
        .request(TransportCommand::Seek(t(3)), deg.epoch_frame(100))
        .unwrap();
    deg.drive(256);
    let r = deg.sched.drain_receipts();
    let applied = r
        .iter()
        .find(|x| x.id == seek && x.status == ApplyStatus::Applied)
        .unwrap();
    assert!(applied.degradation.is_some());
    assert_eq!(deg.proc.calls.last().unwrap().position, t(3));
}
