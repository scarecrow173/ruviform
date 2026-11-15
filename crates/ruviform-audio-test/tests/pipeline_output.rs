//! Pipeline output-leg tests (Issue #628, 3rd slice): scripted output
//! device → production `ruviform_audio::Session::open` →
//! `GraphPlan::drive` callback → captured PCM vs independent expected.
//!
//! OS-L1 deterministic evidence only — not WASAPI/WinMM, GUI, or
//! plugin-ABI evidence.

mod backend_support;

use std::sync::Arc;

use backend_support::*;
use ruviform_audio_core::device::DeviceError;
use ruviform_audio_test::backend::{ScriptedBackend, ScriptedDeviceHandle, StreamFault};
use ruviform_audio_test::oracle::PcmCompare;
use ruviform_audio_test::pcm::PcmFixture;
use ruviform_audio_test::pipeline::{
    PipelineError, PlaybackPipeline, compare_pcm_block, expected_fixture_block, fixture_graph,
};

/// Sine is transcendental — the established epsilon for this oracle.
const SINE_EPS: f32 = 1e-5;

/// A backend + output-only device whose stream does/doesn't record.
fn output_fixture(record: bool) -> (ScriptedBackend, ScriptedDeviceHandle) {
    let clock = test_clock();
    let mut backend = ScriptedBackend::new(clock);
    let mut spec = output_only_spec("pipe-out");
    if let Some(output) = spec.output.as_mut() {
        output.record_output = record;
    }
    let handle = backend.add_device(spec).unwrap();
    (backend, handle)
}

#[test]
fn session_open_pumps_fixture_pcm_matching_independent_expected() {
    let (_backend, handle) = output_fixture(true);
    let fixture = stereo_fixture();
    let graph = fixture_graph(Arc::new(fixture));
    let mut pipeline = PlaybackPipeline::open(&handle, out_config(), graph).expect("session open");

    let reports = pipeline.pump_blocks(4).expect("pumps");
    assert_eq!(reports.len(), 4);
    for (i, report) in reports.iter().enumerate() {
        assert_eq!(
            report.direction,
            ruviform_audio_test::backend::StreamDirection::Output
        );
        assert_eq!(report.callback_index, i as u64);
        assert_eq!(report.frames, 256);
        assert!(!report.invalidated);
    }

    let captured = pipeline.captured();
    assert_eq!(captured.len(), 4);
    for (i, block) in captured.iter().enumerate() {
        assert_eq!(block.start_frame, i as u64 * 256);
        assert_eq!(block.frames, 256);
        assert_eq!(block.channels, 2);
        // Independent expected: oracle reference, not the fixture's
        // own fill path the graph node used.
        let expected =
            expected_fixture_block(&fixture, block.start_frame, block.frames, block.channels);
        compare_pcm_block(block, &expected, PcmCompare::AbsEpsilon(SINE_EPS))
            .unwrap_or_else(|e| panic!("block {i} mismatch: {e}"));
    }
    pipeline.stop().expect("stop");
}

#[test]
fn open_fault_surfaces_as_session_error() {
    let (_backend, handle) = output_fixture(true);
    handle.fail_next_open_output(DeviceError::Backend("permission_denied"));
    let graph = fixture_graph(Arc::new(stereo_fixture()));
    let err = PlaybackPipeline::open(&handle, out_config(), graph).expect_err("open must fail");
    assert!(
        matches!(err, PipelineError::Session(_)),
        "expected Session error, got {err:?}"
    );
}

#[test]
fn persistent_open_error_blocks_every_session_open() {
    let (_backend, handle) = output_fixture(true);
    handle.set_open_output_error(Some(DeviceError::Backend("busy")));
    for _ in 0..2 {
        let graph = fixture_graph(Arc::new(stereo_fixture()));
        let err = PlaybackPipeline::open(&handle, out_config(), graph).expect_err("open must fail");
        assert!(matches!(err, PipelineError::Session(_)), "got {err:?}");
    }
    handle.set_open_output_error(None);
    let graph = fixture_graph(Arc::new(stereo_fixture()));
    PlaybackPipeline::open(&handle, out_config(), graph).expect("open after clear");
}

#[test]
fn xrun_fault_fires_through_production_callback_path() {
    let (_backend, handle) = output_fixture(true);
    let graph = fixture_graph(Arc::new(stereo_fixture()));
    let mut pipeline = PlaybackPipeline::open(&handle, out_config(), graph).expect("session open");

    handle.inject_output_fault(StreamFault::Error {
        at_callback: 1,
        error: DeviceError::Xrun,
    });
    let reports = pipeline.pump_blocks(3).expect("pumps");
    // The error value itself is not observable — `Session::open`
    // installs a no-op error callback (recorded finding). The honest
    // observed surface is the pump report's fired count.
    assert_eq!(reports[0].errors_fired, 0);
    assert_eq!(reports[1].errors_fired, 1);
    assert_eq!(reports[2].errors_fired, 0);
    // PCM delivery continued unaffected by the reported fault.
    assert_eq!(pipeline.captured().len(), 3);
}

#[test]
fn invalidate_fault_stops_delivery_and_pump_fails() {
    let (_backend, handle) = output_fixture(true);
    let graph = fixture_graph(Arc::new(stereo_fixture()));
    let mut pipeline = PlaybackPipeline::open(&handle, out_config(), graph).expect("session open");

    handle.inject_output_fault(StreamFault::Invalidate { at_callback: 1 });
    pipeline.pump().expect("first pump");
    let report = pipeline.pump().expect("invalidating pump still reports");
    assert!(report.invalidated);
    assert_eq!(report.errors_fired, 1); // StreamInvalidated on error cb
    let err = pipeline.pump().expect_err("dead stream must fail");
    assert!(
        matches!(err, PipelineError::Device(DeviceError::StreamInvalidated)),
        "expected StreamInvalidated, got {err:?}"
    );
}

#[test]
fn vanished_device_invalidates_open_stream() {
    let (_backend, handle) = output_fixture(true);
    let graph = fixture_graph(Arc::new(stereo_fixture()));
    let mut pipeline = PlaybackPipeline::open(&handle, out_config(), graph).expect("session open");
    pipeline.pump().expect("first pump");
    handle.vanish();
    let err = pipeline.pump().expect_err("vanished device must fail");
    assert!(matches!(err, PipelineError::Device(_)), "got {err:?}");
    handle.reappear();
}

#[test]
fn capture_disabled_records_nothing_but_pumps_continue() {
    let (_backend, handle) = output_fixture(false);
    let graph = fixture_graph(Arc::new(stereo_fixture()));
    let mut pipeline = PlaybackPipeline::open(&handle, out_config(), graph).expect("session open");
    pipeline.pump_blocks(3).expect("pumps");
    assert_eq!(pipeline.captured().len(), 0);
    assert_eq!(pipeline.reports().len(), 3);
}

#[test]
fn negotiated_format_is_reported_by_session() {
    let (_backend, handle) = output_fixture(true);
    let graph = fixture_graph(Arc::new(stereo_fixture()));
    let pipeline = PlaybackPipeline::open(&handle, out_config(), graph).expect("session open");
    let format = pipeline.session().format();
    assert_eq!(format.sample_rate.get(), 48_000);
    assert_eq!(format.layout.channels(), 2);
}

#[test]
fn empty_graph_renders_silence() {
    // A graph with no nodes leaves `out` untouched — the scripted
    // backend's scratch is pre-zeroed, so capture is exact silence.
    // This pins the "no fake fill" boundary: the pipeline does not
    // synthesize audio the graph did not produce.
    let (_backend, handle) = output_fixture(true);
    let mut pipeline = PlaybackPipeline::open(&handle, out_config(), ruviform_audio::Graph::new())
        .expect("session open");
    pipeline.pump_blocks(2).expect("pumps");
    for block in pipeline.captured() {
        assert!(block.data.iter().all(|s| *s == 0.0));
    }
}

#[test]
fn device_spec_without_output_reports_unsupported() {
    let clock = test_clock();
    let mut backend = ScriptedBackend::new(clock);
    let handle = backend.add_device(input_only_spec("in-only")).unwrap();
    let graph = fixture_graph(Arc::new(stereo_fixture()));
    let err = PlaybackPipeline::open(&handle, out_config(), graph)
        .expect_err("input-only device must refuse output open");
    assert!(matches!(err, PipelineError::Session(_)), "got {err:?}");
}

#[test]
fn pcm_fixture_non_sine_patterns_compare_exact() {
    // Exact-oracle coverage through the same pipeline: Impulse is
    // non-transcendental, so `PcmCompare::Exact` must hold bit-for-bit.
    let (_backend, handle) = output_fixture(true);
    let fixture = PcmFixture::new(
        ruviform_audio_core::SampleRate::new(48_000).unwrap(),
        2,
        ruviform_audio_test::pcm::PcmPattern::Impulse {
            period_frames: 64,
            amplitude: 0.75,
        },
    )
    .unwrap();
    let graph = fixture_graph(Arc::new(fixture));
    let mut pipeline = PlaybackPipeline::open(&handle, out_config(), graph).expect("session open");
    pipeline.pump_blocks(2).expect("pumps");
    for block in pipeline.captured() {
        let expected =
            expected_fixture_block(&fixture, block.start_frame, block.frames, block.channels);
        compare_pcm_block(&block, &expected, PcmCompare::Exact)
            .unwrap_or_else(|e| panic!("exact mismatch at frame {}: {e}", block.start_frame));
    }
}
