//! Fault-injection tests for the scripted port-trait backend (Issue
//! #628, 2nd slice): callback-indexed `StreamFault`s, mid-stream
//! injection, invalidation/reconnect, and Expected-vs-Observed
//! matching between scripted faults and delivered errors.

mod backend_support;

use std::sync::{Arc, Mutex};

use backend_support::*;
use ruviform_audio_core::device::{Device, DeviceError};
use ruviform_audio_test::backend::StreamFault;
use ruviform_audio_test::backend::{ScriptedBackend, ScriptedDeviceHandle};

fn errors() -> (
    Arc<Mutex<Vec<DeviceError>>>,
    impl FnMut(DeviceError) + Send + 'static,
) {
    let vec = Arc::new(Mutex::new(Vec::new()));
    let sink = {
        let vec = Arc::clone(&vec);
        move |e: DeviceError| vec.lock().unwrap().push(e)
    };
    (vec, sink)
}

/// Backend + handle whose output spec carries `faults`.
fn output_fault_fixture(faults: Vec<StreamFault>) -> ScriptedDeviceHandle {
    let mut backend = ScriptedBackend::new(test_clock());
    let mut spec = device_spec("faults");
    spec.output.as_mut().unwrap().stream.faults = faults;
    backend.add_device(spec).unwrap()
}

/// Backend + handle whose input spec carries `faults`.
fn input_fault_fixture(faults: Vec<StreamFault>) -> ScriptedDeviceHandle {
    let mut backend = ScriptedBackend::new(test_clock());
    let mut spec = device_spec("faults");
    spec.input.as_mut().unwrap().stream.faults = faults;
    backend.add_device(spec).unwrap()
}

#[test]
fn error_fault_fires_on_matching_callback_only() {
    let handle = output_fault_fixture(vec![
        StreamFault::Error {
            at_callback: 1,
            error: DeviceError::Xrun,
        },
        StreamFault::Error {
            at_callback: 1,
            error: DeviceError::Backend("queue_full"),
        },
    ]);
    let (errs, err_cb) = errors();
    let stream = handle
        .open_output_stream(&out_config(), Box::new(|_, _| {}), Box::new(err_cb))
        .unwrap();
    stream.start().unwrap();

    let r0 = handle.pump_output_next().unwrap();
    assert_eq!(r0.errors_fired, 0);
    let r1 = handle.pump_output_next().unwrap();
    assert_eq!(r1.errors_fired, 2);
    assert_eq!(
        errs.lock().unwrap().as_slice(),
        &[DeviceError::Xrun, DeviceError::Backend("queue_full")]
    );
    // The fault is index-bound: index 2 does not refire.
    let r2 = handle.pump_output_next().unwrap();
    assert_eq!(r2.errors_fired, 0);
    assert_eq!(errs.lock().unwrap().len(), 2);
}

#[test]
fn frames_fault_overrides_delivery() {
    let handle = output_fault_fixture(vec![StreamFault::Frames {
        at_callback: 0,
        frames: 100,
    }]);
    let stream = handle
        .open_output_stream(&out_config(), Box::new(|_, _| {}), Box::new(|_| {}))
        .unwrap();
    stream.start().unwrap();

    let r0 = handle.pump_output_next().unwrap();
    assert_eq!(r0.frames, 100);
    assert_eq!(r0.position, 0);
    // Position advances by the *delivered* count.
    let r1 = handle.pump_output_next().unwrap();
    assert_eq!(r1.frames, 256);
    assert_eq!(r1.position, 100);
    // Capture recorded the overridden block.
    let captured = handle.captured_output();
    assert_eq!(captured[0].frames, 100);
    assert_eq!(captured[0].data.len(), 200);
}

#[test]
fn frame_position_and_capture_faults_on_input() {
    let handle = input_fault_fixture(vec![
        StreamFault::Error {
            at_callback: 1,
            error: DeviceError::ClockDiscontinuity,
        },
        StreamFault::FramePosition {
            at_callback: 1,
            position: 4096,
        },
        StreamFault::CaptureNone { at_callback: 0 },
    ]);
    let (errs, err_cb) = errors();
    let stream = handle
        .open_input_stream(&in_config(), Box::new(|_, _| {}), Box::new(err_cb))
        .unwrap();
    stream.start().unwrap();

    // Capture suppressed at index 0.
    let r0 = handle.pump_input_next().unwrap();
    assert_eq!(r0.capture_nanos, None);
    assert_eq!(r0.reported_frame_position, Some(0));

    // Index 1: discontinuity error + position jump to 4096 (the
    // scenario covers 0..65536, so no gap is reported).
    let r1 = handle.pump_input_next().unwrap();
    assert_eq!(r1.position, 4096);
    assert_eq!(r1.reported_frame_position, Some(4096));
    assert_eq!(r1.errors_fired, 1);
    assert_eq!(
        errs.lock().unwrap().as_slice(),
        &[DeviceError::ClockDiscontinuity]
    );

    // The jump persists — index 2 continues from 4096 + 256.
    let r2 = handle.pump_input_next().unwrap();
    assert_eq!(r2.position, 4352);
    // Capture reporting resumes (fault was index-bound).
    assert_eq!(r2.capture_nanos, Some(r2.callback_nanos));
}

#[test]
fn invalidate_fault_kills_stream_and_allows_reopen() {
    let handle = output_fault_fixture(vec![StreamFault::Invalidate { at_callback: 1 }]);
    let (errs, err_cb) = errors();
    let stream = handle
        .open_output_stream(&out_config(), Box::new(|_, _| {}), Box::new(err_cb))
        .unwrap();
    stream.start().unwrap();

    handle.pump_output_next().unwrap();
    let r1 = handle.pump_output_next().unwrap();
    assert!(r1.invalidated);
    assert_eq!(r1.errors_fired, 1);
    assert_eq!(
        errs.lock().unwrap().as_slice(),
        &[DeviceError::StreamInvalidated]
    );
    assert!(!handle.is_output_open());

    // Dead stream refuses further pumps; the trait object reports too.
    assert!(matches!(
        handle.pump_output_next(),
        Err(DeviceError::StreamInvalidated)
    ));
    assert_eq!(stream.start(), Err(DeviceError::StreamInvalidated));

    // Reconnect: drop the dead stream, open a fresh one.
    drop(stream);
    let stream2 = handle
        .open_output_stream(&out_config(), Box::new(|_, _| {}), Box::new(|_| {}))
        .unwrap();
    stream2.start().unwrap();
    let r = handle.pump_output_next().unwrap();
    assert_eq!(r.callback_index, 0);
}

#[test]
fn inject_fault_mid_stream_reaches_live_core() {
    let handle = output_fault_fixture(vec![]);
    let (errs, err_cb) = errors();
    let stream = handle
        .open_output_stream(&out_config(), Box::new(|_, _| {}), Box::new(err_cb))
        .unwrap();
    stream.start().unwrap();
    handle.pump_output_next().unwrap();

    // Inject after open — must reach the *live* stream's fault list,
    // not only future opens.
    handle.inject_output_fault(StreamFault::Error {
        at_callback: 1,
        error: DeviceError::Xrun,
    });
    let r1 = handle.pump_output_next().unwrap();
    assert_eq!(r1.errors_fired, 1);
    assert_eq!(errs.lock().unwrap().as_slice(), &[DeviceError::Xrun]);
}

#[test]
fn inject_input_fault_mid_stream() {
    let handle = input_fault_fixture(vec![]);
    let (errs, err_cb) = errors();
    let stream = handle
        .open_input_stream(&in_config(), Box::new(|_, _| {}), Box::new(err_cb))
        .unwrap();
    stream.start().unwrap();
    handle.pump_input_next().unwrap();

    handle.inject_input_fault(StreamFault::Frames {
        at_callback: 1,
        frames: 64,
    });
    let r1 = handle.pump_input_next().unwrap();
    assert_eq!(r1.frames, 64);
    // Position advanced by the overridden count.
    let r2 = handle.pump_input_next().unwrap();
    assert_eq!(r2.position, 256 + 64);
    assert_eq!(errs.lock().unwrap().len(), 0);
}

#[test]
fn vanish_mid_input_stream_invalidates_once() {
    let mut backend = ScriptedBackend::new(test_clock());
    let handle = backend.add_device(device_spec("vanish-in")).unwrap();
    let (errs, err_cb) = errors();
    let stream = handle
        .open_input_stream(&in_config(), Box::new(|_, _| {}), Box::new(err_cb))
        .unwrap();
    stream.start().unwrap();
    handle.pump_input_next().unwrap();

    handle.vanish();
    assert!(matches!(
        handle.pump_input_next(),
        Err(DeviceError::StreamInvalidated)
    ));
    assert_eq!(
        errs.lock().unwrap().as_slice(),
        &[DeviceError::StreamInvalidated]
    );
    assert!(!handle.is_input_open());
    // Device-level methods also report not-found while vanished.
    assert_eq!(
        handle.default_input_format(),
        Err(DeviceError::DeviceNotFound)
    );
}

#[test]
fn open_fault_persistent_and_once_on_input() {
    let handle = input_fault_fixture(vec![]);
    handle.fail_next_open_input(DeviceError::DeviceBusy);
    assert!(matches!(
        handle.open_input_stream(&in_config(), Box::new(|_, _| {}), Box::new(|_| {})),
        Err(DeviceError::DeviceBusy)
    ));
    let stream = handle
        .open_input_stream(&in_config(), Box::new(|_, _| {}), Box::new(|_| {}))
        .unwrap();
    drop(stream);

    handle.set_open_input_error(Some(DeviceError::Backend("permission_denied")));
    assert!(matches!(
        handle.open_input_stream(&in_config(), Box::new(|_, _| {}), Box::new(|_| {})),
        Err(DeviceError::Backend("permission_denied"))
    ));
}

#[test]
fn clock_domain_and_per_device_clock() {
    // Two devices on different VirtualClocks → different domains.
    let clock_a = test_clock(); // epoch 7
    let mut clock_b_cfg = ruviform_audio_test::clock::VirtualClockConfig {
        sample_rate: 48_000,
        epoch: 99,
        offset_ticks: 0,
        drift_ppm: 0,
        jitter: ruviform_audio_test::clock::JitterSpec::DISABLED,
    };
    let _ = &mut clock_b_cfg;
    let clock_b = Arc::new(ruviform_audio_test::clock::VirtualClock::new(clock_b_cfg).unwrap());

    let mut backend = ScriptedBackend::new(clock_a);
    let mut spec_a = device_spec("a");
    spec_a.clock = None; // backend clock (epoch 7)
    let handle_a = backend.add_device(spec_a).unwrap();

    let mut spec_b = device_spec("b");
    spec_b.clock = Some(clock_b);
    let handle_b = backend.add_device(spec_b).unwrap();

    let stream_a = handle_a
        .open_output_stream(&out_config(), Box::new(|_, _| {}), Box::new(|_| {}))
        .unwrap();
    let stream_b = handle_b
        .open_output_stream(&out_config(), Box::new(|_, _| {}), Box::new(|_| {}))
        .unwrap();

    assert_eq!(stream_a.clock_domain(), test_domain());
    assert_eq!(
        stream_b.clock_domain(),
        ruviform_media_core::ClockDomainId::new(99)
    );
    assert_ne!(stream_a.clock_domain(), stream_b.clock_domain());
}
