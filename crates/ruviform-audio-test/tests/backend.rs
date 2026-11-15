//! Contract tests for the scripted port-trait backend (Issue #628,
//! 2nd slice): enumeration, open/negotiation, manual pump delivery,
//! timestamps, PCM replay/capture, hot-unplug, and the MIDI feed.
//!
//! These prove the *contract* is implementable deterministically —
//! OS-L1 evidence, not real-OS-path evidence.

mod backend_support;

use std::sync::{Arc, Mutex};

use backend_support::*;
use ruviform_audio_core::device::{
    BufferSize, Device, DeviceEnumerator, DeviceError, DeviceId, OutputStreamConfig,
};
use ruviform_audio_core::midi_input::mock::MockBackend;
use ruviform_audio_core::midi_input::{MidiInputInfo, MidiPortCapabilities, MidiPortOrigin};
use ruviform_audio_core::{AudioFormat, ChannelLayout, SampleFormat};
use ruviform_audio_test::backend::{
    MidiFeed, ScriptedBackend, ScriptedDeviceHandle, StreamDirection, StreamFault,
};
use ruviform_audio_test::clock::VirtualClockDriver;
use ruviform_audio_test::midi::MidiCue;
use ruviform_media_core::{
    ClockDomainId, MediaTime, MidiChannel, MidiEvent, MidiGroup, MidiKind, MidiValue, NoteKey,
    TimeBase,
};

/// `(callback_nanos, playback_nanos)` observed per output callback.
type TimestampLog = Arc<Mutex<Vec<(i64, Option<i64>)>>>;

/// `(frame_position, first_sample, last_sample)` per input callback.
type DeliveryLog = Arc<Mutex<Vec<(Option<u64>, f32, f32)>>>;

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

/// A backend + driver + one dual-direction device.
fn fixture() -> (ScriptedBackend, VirtualClockDriver, ScriptedDeviceHandle) {
    let clock = test_clock();
    let mut backend = ScriptedBackend::new(Arc::clone(&clock));
    let handle = backend.add_device(device_spec("main")).unwrap();
    let driver = VirtualClockDriver::new(clock);
    (backend, driver, handle)
}

#[test]
fn enumerate_and_default_selection() {
    let clock = test_clock();
    let mut backend = ScriptedBackend::new(clock);
    let dual = backend.add_device(device_spec("dual")).unwrap();
    let out = backend.add_device(output_only_spec("out-only")).unwrap();
    let input = backend.add_device(input_only_spec("in-only")).unwrap();

    // Output-capable: dual + out-only; input-capable: dual + in-only.
    assert_eq!(backend.output_devices().unwrap().len(), 2);
    assert_eq!(backend.input_devices().unwrap().len(), 2);
    // Default output = first present output-capable ("dual").
    let default_out = backend.default_output_device().unwrap().unwrap();
    assert_eq!(default_out.name().unwrap(), "dual");
    // Default input = first present input-capable ("dual" — registered first).
    let default_in = backend.default_input_device().unwrap().unwrap();
    assert_eq!(default_in.name().unwrap(), "dual");

    // Pin defaults explicitly.
    backend.set_default_output(Some(1));
    backend.set_default_input(Some(2));
    assert_eq!(
        backend
            .default_output_device()
            .unwrap()
            .unwrap()
            .name()
            .unwrap(),
        "out-only"
    );
    assert_eq!(
        backend
            .default_input_device()
            .unwrap()
            .unwrap()
            .name()
            .unwrap(),
        "in-only"
    );

    // Vanished devices are excluded from enumeration.
    out.vanish();
    assert_eq!(backend.output_devices().unwrap().len(), 1);
    assert_eq!(backend.device_count(), 3);
    input.reappear();
    let _ = dual;
}

#[test]
fn device_describe_reports_spec() {
    let (_b, _d, handle) = fixture();
    assert_eq!(handle.name().unwrap(), "main");
    assert_eq!(
        handle.id().unwrap(),
        Some(DeviceId::new("main-id".to_owned()))
    );
    let out_format = handle.default_output_format().unwrap();
    assert_eq!(out_format.sample_rate.get(), 48_000);
    assert_eq!(out_format.layout.channels(), 2);
    assert_eq!(handle.supported_output_formats().unwrap().len(), 1);
    assert_eq!(handle.default_input_format().unwrap(), out_format);
    assert_eq!(handle.supported_input_formats().unwrap().len(), 1);
}

#[test]
fn output_stream_reports_and_pumps() {
    let clock = test_clock();
    let mut backend = ScriptedBackend::new(Arc::clone(&clock));
    let mut spec = device_spec("latency-dev");
    spec.output.as_mut().unwrap().stream.latency = Some(latency64());
    let handle = backend.add_device(spec).unwrap();
    let mut driver = VirtualClockDriver::new(clock);
    driver.advance_frames(480); // 10 ms @ 48 kHz → 10_000_000 ns

    let stream = handle
        .open_output_stream(&out_config(), Box::new(|_, _| {}), Box::new(|_| {}))
        .unwrap();
    assert_eq!(stream.format(), stereo_format());
    assert_eq!(stream.buffer_size().unwrap(), BufferSize::Fixed(256));
    assert_eq!(stream.clock_domain(), test_domain());
    assert_eq!(stream.output_latency().unwrap().unwrap().frames, 64);
    // now() tracks the virtual clock.
    assert_eq!(stream.now().unwrap().value(), 10_000_000);

    stream.start().unwrap();
    let report = handle.pump_output_next().unwrap();
    assert_eq!(report.direction, StreamDirection::Output);
    assert_eq!(report.callback_index, 0);
    assert_eq!(report.frames, 256);
    assert_eq!(report.position, 0);
    assert_eq!(report.callback_nanos, 10_000_000);
    // playback = callback + latency (64 frames @ 48 kHz = 1_333_333 ns).
    assert_eq!(report.playback_nanos, Some(10_000_000 + 1_333_333));
    assert_eq!(report.errors_fired, 0);
    assert!(!report.invalidated);

    let report2 = handle.pump_output_next().unwrap();
    assert_eq!(report2.callback_index, 1);
    assert_eq!(report2.position, 256);

    // Output capture recorded both blocks.
    let captured = handle.captured_output();
    assert_eq!(captured.len(), 2);
    assert_eq!(captured[0].frames, 256);
    assert_eq!(captured[0].start_frame, 0);
    assert_eq!(captured[1].start_frame, 256);
    assert_eq!(captured[0].channels, 2);
    assert_eq!(captured[0].sample_rate.get(), 48_000);
    assert_eq!(captured[0].data.len(), 512);
}

#[test]
fn output_callback_receives_view_and_timestamp() {
    let (_b, mut driver, handle) = fixture();
    let seen: TimestampLog = Arc::new(Mutex::new(Vec::new()));
    let seen2 = Arc::clone(&seen);
    let stream = handle
        .open_output_stream(
            &out_config(),
            Box::new(move |mut view, info| {
                assert_eq!(view.channels(), 2);
                assert_eq!(view.frames(), 256);
                for sample in view.channel_mut(0).unwrap() {
                    *sample = 0.25;
                }
                seen2.lock().unwrap().push((
                    info.timestamp.callback.value(),
                    info.timestamp.playback.map(MediaTime::value),
                ));
            }),
            Box::new(|_| {}),
        )
        .unwrap();
    stream.start().unwrap();
    driver.advance_frames(480);
    handle.pump_output_next().unwrap();

    let seen = seen.lock().unwrap();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].0, 10_000_000);
    assert_eq!(seen[0].1, None); // no latency scripted → no playback

    // The written value is what capture recorded (bit-exact — the
    // callback wrote the literal 0.25).
    let captured = handle.captured_output();
    assert!(
        captured[0].data[..256]
            .iter()
            .all(|&s| s.to_bits() == 0.25f32.to_bits())
    );
    assert!(captured[0].data[256..].iter().all(|&s| s == 0.0));
}

#[test]
fn frames_cycle_varies_delivery() {
    let mut backend = ScriptedBackend::new(test_clock());
    let mut spec = device_spec("cycle");
    spec.output.as_mut().unwrap().stream.frames_cycle = vec![128, 256];
    let handle = backend.add_device(spec).unwrap();

    let stream = handle
        .open_output_stream(&out_config(), Box::new(|_, _| {}), Box::new(|_| {}))
        .unwrap();
    assert_eq!(stream.buffer_size().unwrap(), BufferSize::Default);
    stream.start().unwrap();

    let r0 = handle.pump_output_next().unwrap();
    let r1 = handle.pump_output_next().unwrap();
    let r2 = handle.pump_output_next().unwrap();
    assert_eq!((r0.frames, r0.position), (128, 0));
    assert_eq!((r1.frames, r1.position), (256, 128));
    assert_eq!((r2.frames, r2.position), (128, 384));
}

#[test]
fn input_stream_replays_scenario() {
    let (_b, mut driver, handle) = fixture();
    let fixture_pcm = stereo_fixture();
    let delivered: DeliveryLog = Arc::new(Mutex::new(Vec::new()));
    let delivered2 = Arc::clone(&delivered);

    let stream = handle
        .open_input_stream(
            &in_config(),
            Box::new(move |view, info| {
                let ch0 = view.channel(0).unwrap();
                delivered2.lock().unwrap().push((
                    info.frame_position,
                    ch0[0],
                    ch0[view.frames() - 1],
                ));
            }),
            Box::new(|_| {}),
        )
        .unwrap();
    assert_eq!(stream.buffer_size().unwrap(), BufferSize::Fixed(256));
    assert_eq!(stream.input_latency().unwrap(), None);
    stream.start().unwrap();

    driver.advance_frames(480);
    for expected_index in 0..4u64 {
        let report = handle.pump_input_next().unwrap();
        assert_eq!(report.direction, StreamDirection::Input);
        assert_eq!(report.callback_index, expected_index);
        assert_eq!(report.frames, 256);
        assert_eq!(report.position, expected_index * 256);
        assert_eq!(report.reported_frame_position, Some(expected_index * 256));
        assert_eq!(report.gap_frames, 0);
        assert_eq!(report.errors_fired, 0);
        // capture = callback − 0 latency (no latency scripted) → equal.
        assert_eq!(report.capture_nanos, Some(report.callback_nanos));
        assert_eq!(report.callback_nanos, 10_000_000);
    }

    // Delivered samples match the fixture at absolute frame positions.
    let delivered = delivered.lock().unwrap();
    assert_eq!(delivered.len(), 4);
    for (i, &(pos, first, last)) in delivered.iter().enumerate() {
        let base = i as u64 * 256;
        assert_eq!(pos, Some(base));
        // Bit-exact: the fixture is deterministic integer-pattern math.
        assert_eq!(first.to_bits(), fixture_pcm.sample_at(0, base).to_bits());
        assert_eq!(
            last.to_bits(),
            fixture_pcm.sample_at(0, base + 255).to_bits()
        );
    }
}

#[test]
fn input_gap_reports_xrun_and_silence() {
    let mut backend = ScriptedBackend::new(test_clock());
    // Scenario covers only frames [0, 256) — pumps 1..2 are gaps.
    let mut spec = device_spec("gaps");
    spec.input = Some(input_spec(pcm_scenario(0, 256), 256));
    let handle = backend.add_device(spec).unwrap();

    let errors: Arc<Mutex<Vec<DeviceError>>> = Arc::new(Mutex::new(Vec::new()));
    let errors2 = Arc::clone(&errors);
    let stream = handle
        .open_input_stream(
            &in_config(),
            Box::new(|view, _| {
                assert!(view.channel(0).unwrap().iter().all(|&s| s.is_finite()));
            }),
            Box::new(move |e| errors2.lock().unwrap().push(e)),
        )
        .unwrap();
    stream.start().unwrap();

    let r0 = handle.pump_input_next().unwrap();
    assert_eq!(r0.gap_frames, 0);
    assert_eq!(r0.errors_fired, 0);

    let r1 = handle.pump_input_next().unwrap();
    assert_eq!(r1.position, 256);
    assert_eq!(r1.gap_frames, 256);
    assert_eq!(r1.errors_fired, 1); // Xrun
    assert_eq!(errors.lock().unwrap().as_slice(), &[DeviceError::Xrun]);
}

#[test]
fn vanish_and_reappear() {
    let (_b, _driver, handle) = fixture();
    let errors: Arc<Mutex<Vec<DeviceError>>> = Arc::new(Mutex::new(Vec::new()));
    let errors2 = Arc::clone(&errors);
    let stream = handle
        .open_output_stream(
            &out_config(),
            Box::new(|_, _| {}),
            Box::new(move |e| errors2.lock().unwrap().push(e)),
        )
        .unwrap();
    stream.start().unwrap();
    handle.pump_output_next().unwrap();

    handle.vanish();
    assert!(!handle.is_present());
    assert_eq!(handle.name(), Err(DeviceError::DeviceNotFound));
    // The slot stays "open" until the next pump latches invalidation.
    assert!(handle.is_output_open());

    // Next pump invalidates: error callback fires once, pump fails.
    assert!(matches!(
        handle.pump_output_next(),
        Err(DeviceError::StreamInvalidated)
    ));
    assert!(!handle.is_output_open());
    assert_eq!(
        errors.lock().unwrap().as_slice(),
        &[DeviceError::StreamInvalidated]
    );
    // Subsequent pumps fail without re-notifying.
    assert!(matches!(
        handle.pump_output_next(),
        Err(DeviceError::StreamInvalidated)
    ));
    assert_eq!(errors.lock().unwrap().len(), 1);

    // Reappear: enumerable + openable again (old dead slot replaced).
    handle.reappear();
    assert!(handle.is_present());
    assert!(handle.name().is_ok());
    let stream2 = handle
        .open_output_stream(&out_config(), Box::new(|_, _| {}), Box::new(|_| {}))
        .unwrap();
    stream2.start().unwrap();
    let report = handle.pump_output_next().unwrap();
    assert_eq!(report.callback_index, 0); // fresh stream, fresh index
    drop(stream);
    drop(stream2);
}

#[test]
fn open_and_busy_faults() {
    let (_b, _driver, handle) = fixture();

    // One-shot open fault consumed FIFO.
    handle.fail_next_open_output(DeviceError::DeviceBusy);
    assert!(matches!(
        handle.open_output_stream(&out_config(), Box::new(|_, _| {}), Box::new(|_| {})),
        Err(DeviceError::DeviceBusy)
    ));
    let first = handle
        .open_output_stream(&out_config(), Box::new(|_, _| {}), Box::new(|_| {}))
        .unwrap();

    // Second concurrent open → DeviceBusy.
    assert!(matches!(
        handle.open_output_stream(&out_config(), Box::new(|_, _| {}), Box::new(|_| {})),
        Err(DeviceError::DeviceBusy)
    ));
    drop(first);
    // After drop the direction re-opens.
    let second = handle
        .open_output_stream(&out_config(), Box::new(|_, _| {}), Box::new(|_| {}))
        .unwrap();
    drop(second);

    // Persistent open error (permission-denied mapping).
    handle.set_open_output_error(Some(DeviceError::Backend("permission_denied")));
    assert!(matches!(
        handle.open_output_stream(&out_config(), Box::new(|_, _| {}), Box::new(|_| {})),
        Err(DeviceError::Backend("permission_denied"))
    ));
    handle.set_open_output_error(None);
    let _third = handle
        .open_output_stream(&out_config(), Box::new(|_, _| {}), Box::new(|_| {}))
        .unwrap();
}

#[test]
fn unsupported_config_and_direction() {
    let (_b, _driver, handle) = fixture();

    // Mono is outside the declared stereo-only supported space.
    let mono = OutputStreamConfig {
        format: AudioFormat {
            layout: ChannelLayout::Mono,
            ..stereo_format()
        },
        buffer_size: BufferSize::Default,
    };
    assert!(matches!(
        handle.open_output_stream(&mono, Box::new(|_, _| {}), Box::new(|_| {})),
        Err(DeviceError::UnsupportedConfig)
    ));

    // Fixed(0) is not negotiable.
    let zero = OutputStreamConfig {
        buffer_size: BufferSize::Fixed(0),
        ..out_config()
    };
    assert!(matches!(
        handle.open_output_stream(&zero, Box::new(|_, _| {}), Box::new(|_| {})),
        Err(DeviceError::UnsupportedConfig)
    ));

    // Non-F32 sample encoding is outside the contract vocabulary.
    let s16 = OutputStreamConfig {
        format: AudioFormat {
            sample_format: SampleFormat::S16,
            ..stereo_format()
        },
        buffer_size: BufferSize::Default,
    };
    assert!(matches!(
        handle.open_output_stream(&s16, Box::new(|_, _| {}), Box::new(|_| {})),
        Err(DeviceError::UnsupportedConfig)
    ));

    // A direction the device does not declare → Unsupported.
    let mut backend = ScriptedBackend::new(test_clock());
    let out_only = backend.add_device(output_only_spec("out")).unwrap();
    assert!(matches!(
        out_only.open_input_stream(&in_config(), Box::new(|_, _| {}), Box::new(|_| {})),
        Err(DeviceError::Unsupported)
    ));
    assert_eq!(out_only.supported_input_formats().unwrap().len(), 0);
    assert_eq!(
        out_only.default_input_format(),
        Err(DeviceError::Unsupported)
    );
}

#[test]
fn start_and_stop_faults() {
    let mut backend = ScriptedBackend::new(test_clock());
    let mut spec = device_spec("startstop");
    spec.output
        .as_mut()
        .unwrap()
        .stream
        .start_faults
        .push_back(DeviceError::Xrun);
    spec.output
        .as_mut()
        .unwrap()
        .stream
        .stop_faults
        .push_back(DeviceError::Backend("stop_fail"));
    let handle = backend.add_device(spec).unwrap();

    let stream = handle
        .open_output_stream(&out_config(), Box::new(|_, _| {}), Box::new(|_| {}))
        .unwrap();
    // Pump before start → honest not-started error.
    assert!(matches!(
        handle.pump_output_next(),
        Err(DeviceError::Backend("not_started"))
    ));
    // First start consumes the scripted fault.
    assert_eq!(stream.start(), Err(DeviceError::Xrun));
    assert!(matches!(
        handle.pump_output_next(),
        Err(DeviceError::Backend("not_started"))
    ));
    // Retry succeeds.
    stream.start().unwrap();
    handle.pump_output_next().unwrap();
    // Stop consumes its fault; stream stays started.
    assert_eq!(stream.stop(), Err(DeviceError::Backend("stop_fail")));
    handle.pump_output_next().unwrap();
    stream.stop().unwrap();
    assert!(matches!(
        handle.pump_output_next(),
        Err(DeviceError::Backend("not_started"))
    ));
}

#[test]
fn spec_validation_rejects_malformed() {
    let mut backend = ScriptedBackend::new(test_clock());

    let mut bad = device_spec("bad");
    bad.name.clear();
    assert!(backend.add_device(bad).is_err());

    let mut bad = device_spec("bad");
    bad.output.as_mut().unwrap().supported_formats.clear();
    assert!(backend.add_device(bad).is_err());

    let mut bad = device_spec("bad");
    bad.output.as_mut().unwrap().stream.default_frames = 0;
    assert!(backend.add_device(bad).is_err());

    let mut bad = device_spec("bad");
    bad.output.as_mut().unwrap().stream.frames_cycle = vec![0];
    assert!(backend.add_device(bad).is_err());

    let mut bad = device_spec("bad");
    bad.output
        .as_mut()
        .unwrap()
        .stream
        .faults
        .push(StreamFault::Frames {
            at_callback: 0,
            frames: 0,
        });
    assert!(backend.add_device(bad).is_err());

    let mut bad = device_spec("bad");
    bad.output.as_mut().unwrap().supported_formats[0].channels = 0;
    assert!(backend.add_device(bad).is_err());

    // Nothing was registered by the rejected specs.
    assert_eq!(backend.device_count(), 0);
}

#[test]
fn enumerate_level_error() {
    let mut backend = ScriptedBackend::new(test_clock());
    let _h = backend.add_device(device_spec("dev")).unwrap();

    backend.set_enumerate_error(Some(DeviceError::Backend("permission_denied")));
    for result in [
        backend.output_devices().map(|_| ()),
        backend.input_devices().map(|_| ()),
        backend.default_output_device().map(|_| ()),
        backend.default_input_device().map(|_| ()),
    ] {
        assert_eq!(result, Err(DeviceError::Backend("permission_denied")));
    }
    backend.set_enumerate_error(None);
    assert_eq!(backend.output_devices().unwrap().len(), 1);
}

#[test]
fn midi_feed_orders_and_drains_by_frame() {
    let frame_base = TimeBase::new(1, 48_000).unwrap();
    let scenario = ruviform_audio_test::midi::MidiScenario::new(
        frame_base,
        vec![
            MidiCue {
                position: MediaTime::new(100, frame_base),
                source_sequence: 0,
                event: note_on(60),
            },
            MidiCue {
                position: MediaTime::new(50, frame_base),
                source_sequence: 1,
                event: note_on(62),
            },
        ],
    );
    let domain = ClockDomainId::new(11);
    let mut feed = MidiFeed::new(&scenario, domain);
    assert_eq!(feed.domain(), domain);
    assert_eq!(feed.skipped(), 0);
    assert_eq!(feed.remaining(), 2);

    let mut backend = MockBackend::new();
    let port = backend.add_port(MidiInputInfo::new(
        "mock-0".to_owned(),
        "Mock Port 0".to_owned(),
        MidiPortOrigin::External,
        MidiPortCapabilities::stamped(false, domain),
    ));

    // Nothing due before frame 50.
    assert_eq!(feed.pump_into_port(&port, 49), 0);
    assert_eq!(port.pending_len(), 0);
    // Both cues due at frame 100 (sorted: 50 then 100).
    assert_eq!(feed.pump_into_port(&port, 100), 2);
    assert_eq!(feed.remaining(), 0);
    assert_eq!(port.pending_len(), 2);
}

#[test]
fn midi_feed_skips_unrepresentable_positions() {
    let frame_base = TimeBase::new(1, 48_000).unwrap();
    // Negative position cannot convert to u64 ticks → skipped.
    let scenario = ruviform_audio_test::midi::MidiScenario::new(
        frame_base,
        vec![
            MidiCue {
                position: MediaTime::new(-5, frame_base),
                source_sequence: 0,
                event: note_on(60),
            },
            MidiCue {
                position: MediaTime::new(10, frame_base),
                source_sequence: 1,
                event: note_on(62),
            },
        ],
    );
    let mut feed = MidiFeed::new(&scenario, ClockDomainId::new(11));
    assert_eq!(feed.skipped(), 1);
    assert_eq!(feed.remaining(), 1);
    let mut out = Vec::new();
    assert_eq!(feed.drain_due(10, &mut out), 1);
    assert_eq!(out[0].stamp.ticks, 10);
    assert_eq!(out[0].stamp.domain, ClockDomainId::new(11));
}
