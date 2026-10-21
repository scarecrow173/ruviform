//! Mock-backend integration test: a *non-cpal* implementation of the
//! device boundary ports.
//!
//! The port is the contract a second backend would implement — a fake
//! `DeviceEnumerator`/`Device`/`OutputStream` that scripts its own
//! callback firings proves, without hardware, that the trait shape is
//! implementable and that `Box<dyn …>` exchange works end to end:
//! enumerate → default device → open → scripted callback → start/stop
//! → `now`.
//!
//! Everything here exercises `ruviform_audio_core::device` only; the
//! adapter crate is involved only to assert its `Host` type satisfies
//! the same `DeviceEnumerator` contract.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use ruviform_audio_core::device::{
    BufferSize, DEVICE_CLOCK_TIMEBASE, Device, DeviceEnumerator, DeviceError, DeviceId,
    DeviceResult, OutputCallbackInfo, OutputDataCallback, OutputErrorCallback, OutputStream,
    OutputStreamConfig, OutputTimestamp, SupportedBufferSize, SupportedOutputFormat,
};
use ruviform_audio_core::{
    AudioBufferViewMut, AudioFormat, ChannelLayout, SampleFormat, SampleRate,
};
use ruviform_media_core::MediaTime;

/// A recorded callback firing: the view shape and the timestamp the
/// fake backend delivered.
#[derive(Debug)]
struct Firing {
    channels: usize,
    frames: usize,
    callback_nanos: i64,
    playback_nanos: Option<i64>,
}

/// Observations the test collects — pushed by the fake stream (firings
/// and error deliveries) and by the user-side error callback the test
/// registers at open. Locking is legal here: the fake's "callback"
/// fires on the test thread, not an RT thread.
#[derive(Debug, Default)]
struct Observed {
    firings: Vec<Firing>,
    errors: Vec<DeviceError>,
}

/// `start`/`stop`/`now` bookkeeping.
#[derive(Debug, Default)]
struct Controls {
    started: bool,
    now_calls: usize,
}

/// What `start()` scripts: the per-firing frame counts plus the errors
/// delivered to the error callback afterwards.
struct FakeScript {
    /// Frames per data-callback firing.
    data: Vec<usize>,
    /// Errors fired through `OutputErrorCallback` on `start()`.
    errors: Vec<DeviceError>,
}

/// State shared between the fake device and each stream it opens.
struct Shared {
    data_callback: Mutex<Option<OutputDataCallback>>,
    error_callback: Mutex<Option<OutputErrorCallback>>,
    observed: Mutex<Observed>,
    controls: Mutex<Controls>,
    script: FakeScript,
}

/// A fake output stream: `start()` triggers the scripted data-callback
/// firings against stack-allocated scratch — mirroring how a real
/// backend fires from its own thread, minus the thread.
struct FakeStream {
    format: AudioFormat,
    shared: Arc<Shared>,
}

impl OutputStream for FakeStream {
    fn start(&self) -> DeviceResult<()> {
        self.shared.controls.lock().unwrap().started = true;

        let mut guard = self.shared.data_callback.lock().unwrap();
        let Some(callback) = guard.as_mut() else {
            return Err(DeviceError::StreamInvalidated);
        };
        for &frames in &self.shared.script.data {
            let channels = usize::try_from(self.format.layout.channels()).unwrap_or(usize::MAX);
            // Fire against a bounded scratch the way a real backend
            // hands over its own buffer.
            let mut scratch = vec![0.0_f32; channels * frames];
            let info = OutputCallbackInfo {
                timestamp: OutputTimestamp {
                    callback: MediaTime::new(1_000_000_000, DEVICE_CLOCK_TIMEBASE),
                    playback: Some(MediaTime::new(1_005_000_000, DEVICE_CLOCK_TIMEBASE)),
                },
            };
            callback(
                AudioBufferViewMut::try_new(&mut scratch, channels, frames)
                    .map_err(|_| DeviceError::UnsupportedConfig)?,
                info,
            );
            self.shared.observed.lock().unwrap().firings.push(Firing {
                channels,
                frames,
                callback_nanos: info.timestamp.callback.value(),
                playback_nanos: info.timestamp.playback.map(MediaTime::value),
            });
        }

        // Scripted error deliveries — e.g. an xrun notification.
        let mut guard = self.shared.error_callback.lock().unwrap();
        if let Some(error_callback) = guard.as_mut() {
            for &error in &self.shared.script.errors {
                error_callback(error);
            }
        }
        Ok(())
    }

    fn stop(&self) -> DeviceResult<()> {
        self.shared.controls.lock().unwrap().started = false;
        Ok(())
    }

    fn format(&self) -> AudioFormat {
        self.format
    }

    fn buffer_size(&self) -> DeviceResult<BufferSize> {
        Ok(BufferSize::Fixed(480))
    }

    fn now(&self) -> DeviceResult<MediaTime> {
        self.shared.controls.lock().unwrap().now_calls += 1;
        Ok(MediaTime::new(2_000_000_000, DEVICE_CLOCK_TIMEBASE))
    }
}

/// A fake device: fixed name/id/formats; `open_output_stream` keeps
/// the registered callbacks for scripted firing.
struct FakeDevice {
    name: &'static str,
    shared: Arc<Shared>,
}

impl Device for FakeDevice {
    fn name(&self) -> DeviceResult<String> {
        Ok(self.name.to_owned())
    }

    fn id(&self) -> DeviceResult<Option<DeviceId>> {
        Ok(Some(DeviceId::new(format!("fake:{}", self.name))))
    }

    fn default_output_format(&self) -> DeviceResult<AudioFormat> {
        Ok(AudioFormat {
            sample_rate: SampleRate::new(48_000).unwrap(),
            layout: ChannelLayout::Stereo,
            sample_format: SampleFormat::F32,
        })
    }

    fn supported_output_formats(&self) -> DeviceResult<Vec<SupportedOutputFormat>> {
        Ok(vec![SupportedOutputFormat {
            channels: 2,
            min_sample_rate: 44_100,
            max_sample_rate: 48_000,
            buffer_size: SupportedBufferSize::Range { min: 32, max: 1024 },
            sample_format: SampleFormat::F32,
        }])
    }

    fn open_output_stream(
        &self,
        config: &OutputStreamConfig,
        data_callback: OutputDataCallback,
        error_callback: OutputErrorCallback,
    ) -> DeviceResult<Box<dyn OutputStream>> {
        if config.format.sample_format != SampleFormat::F32 {
            return Err(DeviceError::UnsupportedConfig);
        }
        *self.shared.data_callback.lock().unwrap() = Some(data_callback);
        *self.shared.error_callback.lock().unwrap() = Some(error_callback);
        Ok(Box::new(FakeStream {
            format: config.format,
            shared: Arc::clone(&self.shared),
        }))
    }
}

/// A fake enumerator handing out fresh `FakeDevice`s that share the
/// same observation sink.
struct FakeEnumerator {
    shared: Arc<Shared>,
}

impl DeviceEnumerator for FakeEnumerator {
    fn output_devices(&self) -> DeviceResult<Vec<Box<dyn Device>>> {
        Ok(vec![Box::new(FakeDevice {
            name: "fake-out-0",
            shared: Arc::clone(&self.shared),
        })])
    }

    fn default_output_device(&self) -> DeviceResult<Option<Box<dyn Device>>> {
        Ok(self.output_devices()?.into_iter().next())
    }
}

fn fake_enumerator(script: FakeScript) -> (FakeEnumerator, Arc<Shared>) {
    let shared = Arc::new(Shared {
        data_callback: Mutex::new(None),
        error_callback: Mutex::new(None),
        observed: Mutex::new(Observed::default()),
        controls: Mutex::new(Controls::default()),
        script,
    });
    let enumerator = FakeEnumerator {
        shared: Arc::clone(&shared),
    };
    (enumerator, shared)
}

#[test]
fn ports_are_object_safe_and_send() {
    // Compile-time proof: `Box<dyn>` exchange and `Send` through the
    // trait objects.
    fn assert_send<T: Send>(_: &T) {}
    fn assert_enumerator(_: &dyn DeviceEnumerator) {}
    fn assert_device(_: &dyn Device) {}
    fn assert_stream(_: &dyn OutputStream) {}

    let (enumerator, _) = fake_enumerator(FakeScript {
        data: vec![],
        errors: vec![],
    });
    assert_enumerator(&enumerator);

    let device = enumerator.default_output_device().unwrap().unwrap();
    assert_device(&*device);
    assert_send(&device);

    let config = OutputStreamConfig {
        format: device.default_output_format().unwrap(),
        buffer_size: BufferSize::Default,
    };
    let stream = device
        .open_output_stream(&config, Box::new(|_, _| {}), Box::new(|_| {}))
        .unwrap();
    assert_stream(&*stream);
    assert_send(&stream);
}

#[test]
fn closures_coerce_onto_the_hrtb_callback_types() {
    // A plain `FnMut + Send + 'static` closure coerces onto
    // `for<'a> FnMut(AudioBufferViewMut<'a, f32>, …)` — the HRTB
    // inference the port relies on.
    let mut data_cb: OutputDataCallback = Box::new(
        |mut out: AudioBufferViewMut<'_, f32>, _info: OutputCallbackInfo| {
            for channel in 0..out.channels() {
                out.channel_mut(channel).unwrap().fill(0.0);
            }
        },
    );
    let mut err_cb: OutputErrorCallback = Box::new(|_e: DeviceError| {});
    let mut scratch = [0.0_f32; 8];
    let info = OutputCallbackInfo {
        timestamp: OutputTimestamp {
            callback: MediaTime::new(0, DEVICE_CLOCK_TIMEBASE),
            playback: None,
        },
    };
    data_cb(
        AudioBufferViewMut::try_new(&mut scratch, 2, 4).unwrap(),
        info,
    );
    err_cb(DeviceError::Xrun);
}

#[test]
fn full_session_flow_over_boxed_trait_objects() {
    let (enumerator, shared) = fake_enumerator(FakeScript {
        data: vec![4, 2],
        errors: vec![DeviceError::Xrun],
    });

    // 1. Enumerate → exactly one output device, all through
    //    `Box<dyn Device>`.
    let devices: Vec<Box<dyn Device>> = enumerator.output_devices().unwrap();
    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0].name().unwrap(), "fake-out-0");
    assert_eq!(
        devices[0].id().unwrap().unwrap(),
        DeviceId::new("fake:fake-out-0".to_owned()),
    );
    let default_device: Box<dyn Device> = enumerator
        .default_output_device()
        .unwrap()
        .expect("fake always reports a default");

    // 2. Default + supported formats.
    let format = default_device.default_output_format().unwrap();
    assert_eq!(format.sample_format, SampleFormat::F32);
    let supported = default_device.supported_output_formats().unwrap();
    assert_eq!(supported.len(), 1);
    assert_eq!(supported[0].channels, 2);

    // 3. Open with scripted callbacks — the user callback counts the
    //    frames it is asked to fill; the error callback records into
    //    the shared observation sink.
    let wrote = Arc::new(AtomicUsize::new(0));
    let errors_sink = Arc::clone(&shared);
    let stream: Box<dyn OutputStream> = default_device
        .open_output_stream(
            &OutputStreamConfig {
                format,
                buffer_size: BufferSize::Fixed(480),
            },
            {
                let wrote = Arc::clone(&wrote);
                Box::new(move |mut out: AudioBufferViewMut<'_, f32>, _info| {
                    wrote.fetch_add(out.frames(), Ordering::Relaxed);
                    for channel in 0..out.channels() {
                        out.channel_mut(channel).unwrap().fill(0.25);
                    }
                })
            },
            Box::new(move |error: DeviceError| {
                errors_sink.observed.lock().unwrap().errors.push(error);
            }),
        )
        .unwrap();
    assert_eq!(stream.format(), format);
    assert_eq!(stream.buffer_size().unwrap(), BufferSize::Fixed(480));

    // 4. start → scripted firings → stop.
    stream.start().unwrap();
    assert!(shared.controls.lock().unwrap().started);
    stream.stop().unwrap();
    assert!(!shared.controls.lock().unwrap().started);

    // 5. `now` through the trait object.
    let now = stream.now().unwrap();
    assert_eq!(now.value(), 2_000_000_000);
    assert_eq!(shared.controls.lock().unwrap().now_calls, 1);

    // 6. Observations: two scripted firings with the requested shapes
    //    and timestamps; the scripted xrun reached the error
    //    callback; the user callback covered every frame.
    {
        let observed = shared.observed.lock().unwrap();
        assert_eq!(
            observed
                .firings
                .iter()
                .map(|f| (f.channels, f.frames))
                .collect::<Vec<_>>(),
            vec![(2, 4), (2, 2)],
        );
        assert!(
            observed
                .firings
                .iter()
                .all(|f| f.callback_nanos == 1_000_000_000),
        );
        assert!(
            observed
                .firings
                .iter()
                .all(|f| f.playback_nanos == Some(1_005_000_000)),
        );
        assert_eq!(observed.errors, vec![DeviceError::Xrun]);
    }
    assert_eq!(wrote.load(Ordering::Relaxed), 6);
}

#[test]
fn adapter_host_satisfies_the_same_enumerator_contract() {
    // The cpal-backed `Host` implements the same port — no hardware
    // touched to prove the shape.
    fn assert_enumerator(_: &dyn DeviceEnumerator) {}
    assert_enumerator(&ruviform_audio_device::default_host());
}
