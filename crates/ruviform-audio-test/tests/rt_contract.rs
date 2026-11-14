//! RT contract tests (`rt_` prefix — collected by `mise run test-rt`
//! via `cargo test --workspace rt_`).
//!
//! Proves the Issue #628 RT surface is allocation-free:
//! clock reads, PCM `fill`, and the scheduler `run_callback` +
//! `TraceSink` delivery path.

use std::sync::Arc;

use ruviform_audio_core::SampleRate;
use ruviform_audio_test::clock::{
    JitterSpec, VirtualClock, VirtualClockConfig, VirtualClockDriver,
};
use ruviform_audio_test::midi::TraceSink;
use ruviform_audio_test::pcm::{PcmFixture, PcmPattern};
use ruviform_media_core::{
    BlockContract, ClockEpochId, DeliveryPolicy, EngineFrame, InBlockTransportSupport,
    LoopOccurrence, MediaTime, MidiChannel, MidiEvent, MidiGroup, MidiKind, MidiValue, NoteKey,
    PlanGeneration, ScheduleBudget, ScheduledEvent, SpanContext, SpanSink, TempoMapRevision,
    TimeBase, TransportCommand, TransportGeneration, TransportPlanBuilder, TransportScheduler,
    TransportSpan, TransportStatus,
};

/// `assert_no_alloc` disabler for this test binary — any (de)allocation
/// inside a measurement window aborts, which is the failure signal.
#[cfg(debug_assertions)]
#[global_allocator]
static ALLOC: assert_no_alloc::AllocDisabler = assert_no_alloc::AllocDisabler;

fn note_on(key: u8) -> MidiEvent {
    MidiEvent {
        group: MidiGroup::ZERO,
        channel: MidiChannel::new(0).unwrap(),
        kind: MidiKind::NoteOn {
            key: NoteKey::new(key).unwrap(),
            velocity: MidiValue::from_u7(100),
        },
    }
}

#[test]
fn rt_virtual_clock_reads_do_not_alloc() {
    let clock = Arc::new(
        VirtualClock::new(VirtualClockConfig {
            sample_rate: 48_000,
            epoch: 1,
            offset_ticks: 0,
            drift_ppm: 500,
            jitter: JitterSpec {
                seed: 3,
                period_frames: 32,
                amplitude_ticks: 1,
            },
        })
        .unwrap(),
    );
    let mut driver = VirtualClockDriver::new(Arc::clone(&clock));
    driver.advance_frames(1_000);
    let shared = Arc::clone(&clock);
    assert_no_alloc::assert_no_alloc(|| {
        std::hint::black_box(shared.tick());
        std::hint::black_box(shared.monotonic_nanos());
        std::hint::black_box(shared.timebase());
        std::hint::black_box(shared.epoch());
        std::hint::black_box(shared.config().tick_at(77_777));
        std::hint::black_box(shared.config().monotonic_nanos_at(77_777));
    });
}

#[test]
fn rt_pcm_fill_does_not_alloc() {
    let fixture = PcmFixture::new(
        SampleRate::new(48_000).unwrap(),
        2,
        PcmPattern::Sweep {
            f0_hz: 100.0,
            f1_hz: 4_000.0,
            duration_frames: 8_192,
            amplitude: 0.5,
        },
    )
    .unwrap();
    let mut buf = vec![0.0f32; 2 * 256];
    assert_no_alloc::assert_no_alloc(|| {
        fixture.fill(1_000, 256, &mut buf).unwrap();
        std::hint::black_box(fixture.sample_at(1, 5_000));
        fixture.fill_channel(0, 9, &mut buf[..256]);
        std::hint::black_box(());
    });
    // sanity: the window actually wrote non-zero data
    assert!(buf.iter().any(|s| *s != 0.0));
}

#[test]
fn rt_scheduler_callback_and_trace_sink_do_not_alloc() {
    let base = TimeBase::sample_rate(48_000).unwrap();
    // Build plan + scheduler OUTSIDE the measured window.
    let mut builder = TransportPlanBuilder::<MidiEvent>::new(base, PlanGeneration::new(1));
    for i in 0..32u64 {
        builder
            .event(
                MediaTime::new(i64::try_from(64 * i + 10).unwrap(), base),
                i,
                note_on(60 + (i % 12) as u8),
            )
            .unwrap();
    }
    let plan = Arc::new(builder.build());
    let mut scheduler = TransportScheduler::<MidiEvent>::new(
        base,
        BlockContract::new(1, 4_096, InBlockTransportSupport::Unknown),
        DeliveryPolicy::Exact,
        ScheduleBudget::new(64, 1_024, 16, 0),
    );
    let epoch = scheduler.epoch();
    scheduler
        .install_plan(Arc::clone(&plan), EngineFrame::new(epoch, 0))
        .unwrap();
    scheduler
        .request(TransportCommand::Play, EngineFrame::new(epoch, 0))
        .unwrap();

    // Sink preallocated for the whole trace: 32 events, one span each.
    let mut sink = TraceSink::with_capacity(128);
    assert_no_alloc::assert_no_alloc(|| {
        for block in 0..32u64 {
            scheduler.run_callback(EngineFrame::new(epoch, block * 64), 64, &mut sink);
        }
    });
    assert_eq!(sink.entries.len(), 32);
    assert_eq!(sink.undeliverable_spans, 0);
}

#[test]
fn rt_trace_sink_span_direct_does_not_alloc() {
    // Direct SpanSink::span invocation — the sink's own contract.
    let base = TimeBase::sample_rate(48_000).unwrap();
    let epoch = ClockEpochId::new(0);
    let span = TransportSpan {
        start: EngineFrame::new(epoch, 0),
        end: EngineFrame::new(epoch, 64),
        ctx: SpanContext {
            status: TransportStatus::Playing,
            position_ticks: 0,
            position_base: base,
            loop_ticks: None,
            occurrence: LoopOccurrence::new(0),
            clock_epoch: epoch,
            transport_generation: TransportGeneration::new(0),
            // `ScheduledEvent::new` stamps generation 0 / ticks 0 —
            // matching ctx keeps `event_engine_frame` on the delivery
            // path (offset 0 → delivers at span start).
            plan_generation: PlanGeneration::new(0),
            tempo_revision: TempoMapRevision::new(0),
        },
        deliverable: true,
    };
    let event = ScheduledEvent::new(MediaTime::new(10, base), 0, note_on(60));
    let events = [event];
    let mut sink = TraceSink::with_capacity(8);
    assert_no_alloc::assert_no_alloc(|| {
        SpanSink::span(&mut sink, &span, &events);
    });
    assert_eq!(sink.entries.len(), 1);
    assert_eq!(sink.entries[0].frame.frame(), 0);
    assert_eq!(sink.undeliverable_spans, 0);
}
