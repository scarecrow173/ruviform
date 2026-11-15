//! Pipeline input-leg tests (Issue #628, 3rd slice): scripted input
//! stream → bounded capture → production typed runtime
//! (`input_writer` → `process` → `output_reader`) → observed PCM vs
//! independent expected.
//!
//! OS-L1 deterministic evidence only — not real-device, duplex, or
//! OS-latency evidence.

mod backend_support;

use backend_support::*;
use ruviform_audio_core::SampleRate;
use ruviform_audio_core::device::DeviceError;
use ruviform_audio_test::backend::{ScriptedBackend, ScriptedDeviceHandle, StreamFault};
use ruviform_audio_test::oracle::PcmCompare;
use ruviform_audio_test::pcm::{PcmFixture, PcmPattern, PcmScenario, PcmSegment};
use ruviform_audio_test::pipeline::{
    InputPipeline, PipelineError, compare_planar, expected_block, pipeline_compiler_config,
    scale_expected,
};

/// Sine is transcendental — the established epsilon for this oracle.
const SINE_EPS: f32 = 1e-5;
const GAIN: f32 = 0.5;

/// Backend + input-only device replaying `pcm` at 256 frames/callback.
fn input_fixture(pcm: PcmScenario) -> (ScriptedBackend, ScriptedDeviceHandle) {
    let clock = test_clock();
    let mut backend = ScriptedBackend::new(clock);
    let mut spec = input_only_spec("pipe-in");
    if let Some(input) = spec.input.as_mut() {
        input.pcm = pcm;
    }
    let handle = backend.add_device(spec).unwrap();
    (backend, handle)
}

fn open_pipeline(handle: &ScriptedDeviceHandle) -> InputPipeline {
    InputPipeline::open(handle, in_config(), GAIN, pipeline_compiler_config(1024), 4)
        .expect("input pipeline open")
}

/// Expected *output* for one observed block: scenario PCM at the
/// reported absolute position, scaled by gain — all through the
/// independent oracle, never the node under test.
fn expected_output(
    scenario: &PcmScenario,
    start_frame: u64,
    frames: u32,
    channels: u16,
) -> Vec<Vec<f32>> {
    scale_expected(
        &expected_block(scenario, start_frame, frames, channels),
        GAIN,
    )
}

#[test]
fn captured_input_flows_through_typed_runtime_with_gain() {
    let scenario = pcm_scenario(0, 65_536);
    let (_backend, handle) = input_fixture(scenario.clone());
    let mut pipeline = open_pipeline(&handle);

    let reports = pipeline.pump_blocks(4).expect("pumps");
    assert_eq!(reports.len(), 4);
    for (i, report) in reports.iter().enumerate() {
        assert_eq!(
            report.direction,
            ruviform_audio_test::backend::StreamDirection::Input
        );
        assert_eq!(report.callback_index, i as u64);
        assert_eq!(report.frames, 256);
        assert_eq!(report.gap_frames, 0);
        assert_eq!(report.errors_fired, 0);
    }

    let blocks = pipeline.observed_blocks();
    assert_eq!(blocks.len(), 4);
    for (i, block) in blocks.iter().enumerate() {
        let start = block
            .start_frame
            .expect("scripted backend reports frame_position");
        assert_eq!(start, i as u64 * 256, "block {i} position");
        assert_eq!(block.frames, 256);
        assert_eq!(block.channels, 2);
        assert!(block.capture.is_some(), "capture timestamp observed");
        assert_eq!(block.clock_domain, test_domain());
        if i > 0 {
            assert!(
                blocks[i].callback_nanos >= blocks[i - 1].callback_nanos,
                "callback timestamps are monotonic"
            );
        }

        let data = pipeline.observed_block_data(i).expect("observed pcm slice");
        let expected = expected_output(&scenario, start, block.frames, block.channels);
        compare_planar(
            data,
            block.frames,
            block.channels,
            &expected,
            PcmCompare::AbsEpsilon(SINE_EPS),
        )
        .unwrap_or_else(|e| panic!("block {i} (frame {start}) mismatch: {e}"));
    }

    let counters = pipeline.capture_counters();
    assert_eq!(counters.delivered, 4);
    assert_eq!(counters.starved, 0);
    assert_eq!(counters.rejected_shape, 0);
    assert_eq!(counters.overflow, 0);
    assert_eq!(pipeline.errors().len(), 0);
    pipeline.stop().expect("stop");
}

#[test]
fn scenario_gap_delivers_silence_and_reports_xrun() {
    // Coverage [0,256) ∪ [512,768): block 1 ([256,512)) is entirely
    // a gap — the backend must deliver silence AND fire Xrun, and the
    // observed output must equal gain * silence = silence.
    let fixture = stereo_fixture();
    let scenario = PcmScenario::new(vec![
        PcmSegment {
            frame_start: 0,
            frame_len: 256,
            fixture,
        },
        PcmSegment {
            frame_start: 512,
            frame_len: 256,
            fixture,
        },
    ])
    .unwrap();
    let (_backend, handle) = input_fixture(scenario.clone());
    let mut pipeline = open_pipeline(&handle);

    let reports = pipeline.pump_blocks(3).expect("pumps");
    assert_eq!(reports[1].gap_frames, 256);
    assert_eq!(reports[1].errors_fired, 1); // Xrun on the error callback
    assert!(
        pipeline
            .errors()
            .iter()
            .any(|e| matches!(e, DeviceError::Xrun))
    );

    // All three blocks still processed: gap block output is exact 0.
    let blocks = pipeline.observed_blocks();
    assert_eq!(blocks.len(), 3);
    for (i, block) in blocks.iter().enumerate() {
        let start = block.start_frame.unwrap();
        let data = pipeline.observed_block_data(i).unwrap();
        let expected = expected_output(&scenario, start, block.frames, block.channels);
        compare_planar(
            data,
            block.frames,
            block.channels,
            &expected,
            PcmCompare::AbsEpsilon(SINE_EPS),
        )
        .unwrap_or_else(|e| panic!("gap-scenario block {i} mismatch: {e}"));
    }
}

#[test]
fn open_fault_surfaces_device_error() {
    let (_backend, handle) = input_fixture(pcm_scenario(0, 65_536));
    handle.fail_next_open_input(DeviceError::Backend("permission_denied"));
    let err = InputPipeline::open(
        &handle,
        in_config(),
        GAIN,
        pipeline_compiler_config(1024),
        4,
    )
    .expect_err("open must fail");
    assert!(
        matches!(
            err,
            PipelineError::Device(DeviceError::Backend("permission_denied"))
        ),
        "expected device error, got {err:?}"
    );
}

#[test]
fn persistent_open_error_blocks_every_open() {
    let (_backend, handle) = input_fixture(pcm_scenario(0, 65_536));
    handle.set_open_input_error(Some(DeviceError::RealtimeDenied));
    let err = InputPipeline::open(
        &handle,
        in_config(),
        GAIN,
        pipeline_compiler_config(1024),
        4,
    )
    .expect_err("open must fail");
    assert!(matches!(
        err,
        PipelineError::Device(DeviceError::RealtimeDenied)
    ));
}

#[test]
fn scripted_error_fault_is_drained_into_errors() {
    let (_backend, handle) = input_fixture(pcm_scenario(0, 65_536));
    let mut pipeline = open_pipeline(&handle);
    handle.inject_input_fault(StreamFault::Error {
        at_callback: 0,
        error: DeviceError::ClockDiscontinuity,
    });
    pipeline.pump_block().expect("pump");
    assert!(
        pipeline
            .errors()
            .iter()
            .any(|e| matches!(e, DeviceError::ClockDiscontinuity))
    );
}

#[test]
fn frames_fault_changes_block_size_and_output_still_matches() {
    let scenario = pcm_scenario(0, 65_536);
    let (_backend, handle) = input_fixture(scenario.clone());
    let mut pipeline = open_pipeline(&handle);
    pipeline.pump_block().expect("first");
    handle.inject_input_fault(StreamFault::Frames {
        at_callback: 1,
        frames: 128,
    });
    pipeline.pump_block().expect("second");

    let blocks = pipeline.observed_blocks();
    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[0].frames, 256);
    assert_eq!(blocks[1].frames, 128);
    for (i, block) in blocks.iter().enumerate() {
        let start = block.start_frame.unwrap();
        let data = pipeline.observed_block_data(i).unwrap();
        let expected = expected_output(&scenario, start, block.frames, block.channels);
        compare_planar(
            data,
            block.frames,
            block.channels,
            &expected,
            PcmCompare::AbsEpsilon(SINE_EPS),
        )
        .unwrap_or_else(|e| panic!("frames-fault block {i} mismatch: {e}"));
    }
}

#[test]
fn invalidate_fault_kills_stream_and_next_pump_fails() {
    let (_backend, handle) = input_fixture(pcm_scenario(0, 65_536));
    let mut pipeline = open_pipeline(&handle);
    handle.inject_input_fault(StreamFault::Invalidate { at_callback: 1 });
    pipeline.pump_block().expect("first");
    let report = pipeline.pump_block().expect("invalidating pump reports");
    assert!(report.invalidated);
    let err = pipeline.pump_block().expect_err("dead stream must fail");
    assert!(
        matches!(err, PipelineError::Device(DeviceError::StreamInvalidated)),
        "expected StreamInvalidated, got {err:?}"
    );
    // The invalidation error itself reached the error callback.
    assert!(
        pipeline
            .errors()
            .iter()
            .any(|e| matches!(e, DeviceError::StreamInvalidated))
    );
}

#[test]
fn frame_position_fault_is_observed_verbatim() {
    let scenario = pcm_scenario(0, 65_536);
    let (_backend, handle) = input_fixture(scenario.clone());
    let mut pipeline = open_pipeline(&handle);
    pipeline.pump_block().expect("first"); // position 0
    handle.inject_input_fault(StreamFault::FramePosition {
        at_callback: 1,
        position: 4096,
    });
    pipeline.pump_block().expect("second");
    let blocks = pipeline.observed_blocks();
    assert_eq!(blocks[0].start_frame, Some(0));
    // The discontinuity is *reported*, not smoothed — the observed
    // start_frame jumps to the scripted position.
    assert_eq!(blocks[1].start_frame, Some(4096));
    // PCM follows the jumped window: expected is computed at the
    // *reported* position, matching what the backend actually played.
    let data = pipeline.observed_block_data(1).unwrap();
    let expected = expected_output(&scenario, 4096, blocks[1].frames, blocks[1].channels);
    compare_planar(
        data,
        blocks[1].frames,
        blocks[1].channels,
        &expected,
        PcmCompare::AbsEpsilon(SINE_EPS),
    )
    .expect("post-jump block matches scenario at jumped position");
}

#[test]
fn capture_none_fault_still_processes_pcm() {
    let scenario = pcm_scenario(0, 65_536);
    let (_backend, handle) = input_fixture(scenario);
    let mut pipeline = open_pipeline(&handle);
    handle.inject_input_fault(StreamFault::CaptureNone { at_callback: 0 });
    pipeline.pump_block().expect("pump");
    let blocks = pipeline.observed_blocks();
    assert_eq!(blocks.len(), 1);
    // capture: None is preserved as None — never synthesized, and
    // the block still processed (PCM flowed through the runtime).
    assert!(blocks[0].capture.is_none());
    assert_eq!(blocks[0].frames, 256);
}

#[test]
fn capture_pool_exhaustion_counts_starved_loss() {
    // Queue-full at the capture handoff: firing callbacks through the
    // exposed handle without draining fills the preallocated pool —
    // further callbacks starve and are counted, never silently
    // dropped. The next `pump_block` still drains and processes what
    // was captured.
    let (_backend, handle) = input_fixture(pcm_scenario(0, 65_536));
    let mut pipeline = open_pipeline(&handle);
    for _ in 0..5 {
        pipeline.handle().pump_input_next().expect("direct pump");
    }
    let counters = pipeline.capture_counters();
    assert_eq!(counters.delivered, 4);
    assert_eq!(counters.starved, 1);

    pipeline.pump_block().expect("drain pump");
    let counters = pipeline.capture_counters();
    // The in-pump callback also found the pool full — drain happens
    // after the callback — so starvation is counted once more; the
    // four captured blocks still processed normally.
    assert_eq!(counters.starved, 2);
    assert_eq!(counters.delivered, 4);
    assert_eq!(pipeline.observed_blocks().len(), 4);
    assert_eq!(pipeline.errors().len(), 0);
}

#[test]
fn unsupported_config_rejected_before_stream_opens() {
    // Channel count the device cannot serve → production
    // `UnsupportedConfig` at `open_input_stream`, wrapped as
    // PipelineError::Device — no stream, no capture.
    let (_backend, handle) = input_fixture(pcm_scenario(0, 65_536));
    let mut config = in_config();
    config.format.layout = ruviform_audio_core::ChannelLayout::Mono;
    let err = InputPipeline::open(&handle, config, GAIN, pipeline_compiler_config(1024), 4)
        .expect_err("mono config must fail");
    assert!(matches!(err, PipelineError::Device(_)), "got {err:?}");
    assert!(!handle.is_input_open());
}

#[test]
fn gain_zero_outputs_digital_silence() {
    let scenario = pcm_scenario(0, 4_096);
    let (_backend, handle) = input_fixture(scenario);
    let mut pipeline =
        InputPipeline::open(&handle, in_config(), 0.0, pipeline_compiler_config(1024), 4)
            .expect("open");
    pipeline.pump_blocks(2).expect("pumps");
    for i in 0..2 {
        let data = pipeline.observed_block_data(i).unwrap();
        assert!(data.iter().all(|s| *s == 0.0), "gain=0 must be silence");
    }
}

#[test]
fn non_f32_fixture_rate_mismatch_is_device_error() {
    // Scenario fixtures at a different sample rate than the opened
    // stream → scripted backend rejects at open (UnsupportedConfig) —
    // the pipeline surfaces it as Device, not a silent resample.
    let wrong_rate = PcmFixture::new(
        SampleRate::new(44_100).unwrap(),
        2,
        PcmPattern::Constant { value: 0.1 },
    )
    .unwrap();
    let scenario = PcmScenario::new(vec![PcmSegment {
        frame_start: 0,
        frame_len: 1_024,
        fixture: wrong_rate,
    }])
    .unwrap();
    let (_backend, handle) = input_fixture(scenario);
    let err = InputPipeline::open(
        &handle,
        in_config(),
        GAIN,
        pipeline_compiler_config(1024),
        4,
    )
    .expect_err("rate mismatch must fail");
    assert!(matches!(err, PipelineError::Device(_)), "got {err:?}");
}
