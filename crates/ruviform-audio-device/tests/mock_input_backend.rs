//! Mock-input-backend contract `PoC` (issue #187): a *non-cpal*
//! implementation of the input direction of the device boundary ports,
//! plus the deterministic capture → bounded-SPSC-handoff pattern a
//! recording engine performs inside the RT callback.
//!
//! The fake scripts deterministic capture blocks (planar samples,
//! callback/capture instants, frame positions, clock domain) and
//! scripted errors, proving without hardware that the `InputStream`/
//! `InputDataCallback`/`InputCallbackInfo` contract is implementable
//! and that `Box<dyn …>` exchange works end to end. The handoff half
//! proves the RT rules a real consumer must keep: pre-allocated block
//! pool, bounded copy, `try_push` delivery, observable (counted) loss
//! under queue pressure — never a block or silent drop.
//!
//! The `rt_`-prefixed tests run the callback→handoff path inside an
//! `assert_no_alloc` window (selected by `mise run test-rt`).

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use ruviform_audio_core::device::{
    BufferSize, DEVICE_CLOCK_TIMEBASE, Device, DeviceEnumerator, DeviceError, DeviceId,
    DeviceResult, InputCallbackInfo, InputDataCallback, InputErrorCallback, InputStream,
    InputStreamConfig, InputTimestamp, LatencyCoverage, OutputDataCallback, OutputErrorCallback,
    OutputStream, OutputStreamConfig, StreamLatency, SupportedBufferSize, SupportedInputFormat,
    SupportedOutputFormat, same_clock_domain,
};
use ruviform_audio_core::{AudioBufferView, AudioFormat, ChannelLayout, SampleFormat, SampleRate};
use ruviform_media_core::{
    ClockDomainId, MediaTime, MediaTimestamp, TimeBase, TimestampProvenance,
};

/// `assert_no_alloc` disabler for this test binary — any (de)allocation
/// inside a measurement window aborts, which is the failure signal.
#[cfg(debug_assertions)]
#[global_allocator]
static ALLOC: assert_no_alloc::AllocDisabler = assert_no_alloc::AllocDisabler;

/// Deterministic sample value for channel `ch`, frame `f` of a scripted
/// block — distinct per (ch, f) so a mixed-up copy is visible.
#[allow(clippy::cast_precision_loss)] // test fixture: exactness unnecessary
fn scripted_sample(ch: usize, f: usize) -> f32 {
    (ch * 10_000 + f) as f32 * 0.001
}

/// One scripted capture block the fake stream delivers per callback
/// firing.
#[derive(Debug)]
struct Block {
    /// Frames in this block.
    frames: usize,
    /// Device-clock callback instant (nanoseconds).
    callback_nanos: i64,
    /// Device-clock capture (ADC, frame 0) instant + provenance —
    /// `None` means "backend cannot report" (`capture: None`).
    capture: Option<(i64, TimestampProvenance)>,
    /// Capture-clock frame position of frame 0 — `None` means the
    /// backend does not report a position.
    frame_position: Option<u64>,
}

/// What `start()` replays: the capture blocks plus the errors the
/// stream then reports.
struct InputScript {
    /// Planar channel count (>= 2 for the multi-track scenario).
    channels: usize,
    blocks: Vec<Block>,
    errors: Vec<DeviceError>,
    /// The clock domain every block's timestamps belong to.
    clock_domain: ClockDomainId,
    /// The `input_latency` report.
    latency: Option<StreamLatency>,
    /// Device-clock `now` value.
    now_nanos: i64,
    /// `open_input_stream` returns this error instead of a stream
    /// (e.g. `Unsupported`, `UnsupportedConfig`).
    fail_open: Option<DeviceError>,
}

impl Default for InputScript {
    fn default() -> Self {
        Self {
            channels: 2,
            blocks: Vec::new(),
            errors: Vec::new(),
            clock_domain: ClockDomainId::MONOTONIC,
            latency: None,
            now_nanos: 2_000_000_000,
            fail_open: None,
        }
    }
}

/// A recorded callback firing, for test-side assertions.
#[derive(Debug)]
struct Firing {
    channels: usize,
    frames: usize,
    callback_nanos: i64,
    capture_nanos: Option<i64>,
    frame_position: Option<u64>,
    clock_domain: ClockDomainId,
}

/// Observations the test collects — pushed by the fake stream
/// (firings) or by the test-registered callbacks. Locking is
/// legal here: the fake fires on the test thread, not an RT thread.
#[derive(Debug, Default)]
struct Observed {
    firings: Vec<Firing>,
}

/// `start`/`stop`/`now` bookkeeping.
#[derive(Debug, Default)]
struct Controls {
    started: bool,
    now_calls: usize,
}

/// State shared between the fake device and each stream it opens.
struct Shared {
    input_data_callback: Mutex<Option<InputDataCallback>>,
    input_error_callback: Mutex<Option<InputErrorCallback>>,
    observed: Mutex<Observed>,
    controls: Mutex<Controls>,
    script: InputScript,
    format: AudioFormat,
}

/// A fake input stream: `start()` replays the scripted blocks through
/// the registered callback — mirroring a real backend firing from its
/// RT thread, minus the thread.
struct FakeInputStream {
    format: AudioFormat,
    shared: Arc<Shared>,
}

impl InputStream for FakeInputStream {
    fn start(&self) -> DeviceResult<()> {
        let mut guard = self.shared.input_data_callback.lock().unwrap();
        let Some(callback) = guard.as_mut() else {
            return Err(DeviceError::StreamInvalidated);
        };
        self.shared.controls.lock().unwrap().started = true;
        let channels = self.shared.script.channels;
        for (index, block) in self.shared.script.blocks.iter().enumerate() {
            // Bounded planar scratch the way a real backend hands over
            // its own captured buffer — deterministic sample pattern.
            let mut scratch = vec![0.0_f32; channels * block.frames];
            for ch in 0..channels {
                for f in 0..block.frames {
                    scratch[ch * block.frames + f] = scripted_sample(ch, index * 1_000 + f);
                }
            }
            let info = InputCallbackInfo {
                timestamp: InputTimestamp {
                    callback: MediaTime::new(block.callback_nanos, DEVICE_CLOCK_TIMEBASE),
                    capture: block.capture.map(|(nanos, provenance)| MediaTimestamp {
                        time: MediaTime::new(nanos, DEVICE_CLOCK_TIMEBASE),
                        provenance,
                    }),
                },
                clock_domain: self.shared.script.clock_domain,
                frame_position: block.frame_position,
            };
            callback(
                AudioBufferView::try_new(&scratch, channels, block.frames)
                    .map_err(|_| DeviceError::UnsupportedConfig)?,
                info,
            );
            self.shared.observed.lock().unwrap().firings.push(Firing {
                channels,
                frames: block.frames,
                callback_nanos: block.callback_nanos,
                capture_nanos: block.capture.map(|(nanos, _)| nanos),
                frame_position: block.frame_position,
                clock_domain: self.shared.script.clock_domain,
            });
        }

        let mut guard = self.shared.input_error_callback.lock().unwrap();
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
        Ok(MediaTime::new(
            self.shared.script.now_nanos,
            DEVICE_CLOCK_TIMEBASE,
        ))
    }

    fn clock_domain(&self) -> ClockDomainId {
        self.shared.script.clock_domain
    }

    fn input_latency(&self) -> DeviceResult<Option<StreamLatency>> {
        Ok(self.shared.script.latency)
    }
}

/// A fake device: input-capable, output-incapable — the honest
/// directional-failure surface the contract defines.
struct FakeInputDevice {
    name: &'static str,
    shared: Arc<Shared>,
}

impl Device for FakeInputDevice {
    fn name(&self) -> DeviceResult<String> {
        Ok(self.name.to_owned())
    }

    fn id(&self) -> DeviceResult<Option<DeviceId>> {
        Ok(Some(DeviceId::new(format!("fake:{}", self.name))))
    }

    fn default_output_format(&self) -> DeviceResult<AudioFormat> {
        Err(DeviceError::Unsupported)
    }

    fn supported_output_formats(&self) -> DeviceResult<Vec<SupportedOutputFormat>> {
        // No output capability — empty, not an error.
        Ok(Vec::new())
    }

    fn open_output_stream(
        &self,
        _config: &OutputStreamConfig,
        _data_callback: OutputDataCallback,
        _error_callback: OutputErrorCallback,
    ) -> DeviceResult<Box<dyn OutputStream>> {
        Err(DeviceError::Unsupported)
    }

    fn default_input_format(&self) -> DeviceResult<AudioFormat> {
        Ok(self.shared.format)
    }

    fn supported_input_formats(&self) -> DeviceResult<Vec<SupportedInputFormat>> {
        Ok(vec![SupportedInputFormat {
            channels: u32::try_from(self.shared.script.channels).unwrap_or(u32::MAX),
            min_sample_rate: 44_100,
            max_sample_rate: 48_000,
            buffer_size: SupportedBufferSize::Range { min: 32, max: 1024 },
            sample_format: SampleFormat::F32,
        }])
    }

    fn open_input_stream(
        &self,
        config: &InputStreamConfig,
        data_callback: InputDataCallback,
        error_callback: InputErrorCallback,
    ) -> DeviceResult<Box<dyn InputStream>> {
        if let Some(error) = self.shared.script.fail_open {
            return Err(error);
        }
        if config.format.sample_format != SampleFormat::F32 {
            return Err(DeviceError::UnsupportedConfig);
        }
        *self.shared.input_data_callback.lock().unwrap() = Some(data_callback);
        *self.shared.input_error_callback.lock().unwrap() = Some(error_callback);
        Ok(Box::new(FakeInputStream {
            format: config.format,
            shared: Arc::clone(&self.shared),
        }))
    }
}

/// A fake enumerator handing out `FakeInputDevice`s that share the
/// same observation sink.
struct FakeInputEnumerator {
    shared: Arc<Shared>,
}

impl DeviceEnumerator for FakeInputEnumerator {
    fn output_devices(&self) -> DeviceResult<Vec<Box<dyn Device>>> {
        // Input-only fake host — none exist.
        Ok(Vec::new())
    }

    fn default_output_device(&self) -> DeviceResult<Option<Box<dyn Device>>> {
        Ok(None)
    }

    fn input_devices(&self) -> DeviceResult<Vec<Box<dyn Device>>> {
        Ok(vec![Box::new(FakeInputDevice {
            name: "fake-in-0",
            shared: Arc::clone(&self.shared),
        })])
    }

    fn default_input_device(&self) -> DeviceResult<Option<Box<dyn Device>>> {
        Ok(self.input_devices()?.into_iter().next())
    }
}

fn stereo_format() -> AudioFormat {
    AudioFormat {
        sample_rate: SampleRate::new(48_000).unwrap(),
        layout: ChannelLayout::Stereo,
        sample_format: SampleFormat::F32,
    }
}

fn fake_input_enumerator(script: InputScript) -> (FakeInputEnumerator, Arc<Shared>) {
    let shared = Arc::new(Shared {
        input_data_callback: Mutex::new(None),
        input_error_callback: Mutex::new(None),
        observed: Mutex::new(Observed::default()),
        controls: Mutex::new(Controls::default()),
        script,
        format: stereo_format(),
    });
    let enumerator = FakeInputEnumerator {
        shared: Arc::clone(&shared),
    };
    (enumerator, shared)
}

/// A pre-allocated handoff block — the recording-side capture record
/// the RT callback fills and pushes. `samples` is allocated once, at
/// pool construction on the non-RT path; `copy_from` is the bounded
/// RT-suitable fill.
#[derive(Debug)]
struct CaptureBlock {
    samples: Vec<f32>,
    channels: usize,
    frames: usize,
    callback_nanos: i64,
    capture: Option<MediaTimestamp>,
    frame_position: Option<u64>,
    clock_domain: ClockDomainId,
}

impl CaptureBlock {
    /// Pool construction — non-RT only (allocates).
    fn with_capacity(channels: usize, max_frames: usize) -> Self {
        Self {
            samples: vec![0.0; channels * max_frames],
            channels,
            frames: 0,
            callback_nanos: 0,
            capture: None,
            frame_position: None,
            clock_domain: ClockDomainId::UNKNOWN,
        }
    }

    /// RT-suitable fill: bounded copy of the callback view's planar
    /// data plus the record's metadata. A contractually-legal oversized
    /// delivery (`input.frames()` may exceed the negotiated estimate)
    /// is truncated to this block's frame capacity and the overflow
    /// returned for the caller to count as observable loss — never a
    /// panic on the RT path.
    fn copy_from(&mut self, input: &AudioBufferView<'_, f32>, info: &InputCallbackInfo) -> u64 {
        let max_frames = self.samples.len() / input.channels();
        let frames = input.frames().min(max_frames);
        let count = input.channels() * frames;
        self.samples[..count].copy_from_slice(&input.data()[..count]);
        self.channels = input.channels();
        self.frames = frames;
        self.callback_nanos = info.timestamp.callback.value();
        self.capture = info.timestamp.capture;
        self.frame_position = info.frame_position;
        self.clock_domain = info.clock_domain;
        (input.frames() - frames) as u64
    }
}

/// The recording-side handoff state owned by the data callback —
/// pre-allocated block endpoints plus the observable-loss counters.
/// Atomics keep loss visible without a lock on the RT path.
struct Handoff {
    /// Pool → callback: `try_pop` a free block (empty → `starved`).
    free: ruviform_rt::Consumer<CaptureBlock>,
    /// Callback → drain: `try_push` a filled block (full → hold in
    /// `working`, retry next callback — backpressure, not loss).
    filled: ruviform_rt::Producer<CaptureBlock>,
    /// A block held because `filled` was full — retried first next
    /// callback before a new block is read.
    working: Option<CaptureBlock>,
    /// Frames dropped because no free block or no filled slot existed —
    /// observable loss, never silent.
    dropped_frames: Arc<AtomicU64>,
    /// Free-pool exhaustion events.
    starved: Arc<AtomicU64>,
    /// Frames currently held in `working` under backpressure — the
    /// in-flight remainder stays observable even after the stream
    /// stops (a nonzero value at close is a visible undrained tail,
    /// not silent loss).
    in_flight: Arc<AtomicU64>,
}

impl Handoff {
    /// One callback firing: the contract the RT section must keep —
    /// bounded copy, `try_push`/`try_pop` only, loss counted.
    fn deliver(&mut self, input: &AudioBufferView<'_, f32>, info: &InputCallbackInfo) {
        // Flush a held block first — its delivery predates this
        // block's capture.
        if let Some(block) = self.working.take() {
            let held = block.frames as u64;
            match self.filled.try_push(block) {
                Ok(()) => {
                    self.in_flight.fetch_sub(held, Ordering::Relaxed);
                }
                Err(full) => {
                    self.working = Some(full.into_inner());
                    // The queue is still full: this block's capture
                    // cannot be stored — observable loss.
                    self.dropped_frames
                        .fetch_add(input.frames() as u64, Ordering::Relaxed);
                    return;
                }
            }
        }
        let Ok(mut block) = self.free.try_pop() else {
            self.starved.fetch_add(1, Ordering::Relaxed);
            self.dropped_frames
                .fetch_add(input.frames() as u64, Ordering::Relaxed);
            return;
        };
        let overflow = block.copy_from(input, info);
        if overflow > 0 {
            // Oversized delivery: the excess frames did not fit the
            // pool block — observable loss, not a panic.
            self.dropped_frames.fetch_add(overflow, Ordering::Relaxed);
        }
        let held = block.frames as u64;
        if let Err(full) = self.filled.try_push(block) {
            // Hold for retry next callback — the block is not lost,
            // but its delivery is delayed by backpressure and stays
            // observable via `in_flight`.
            self.working = Some(full.into_inner());
            self.in_flight.fetch_add(held, Ordering::Relaxed);
        }
    }
}

/// Builds a data callback owning a `Handoff` — the engine-side pattern.
fn handoff_callback(handoff: Handoff) -> InputDataCallback {
    let mut handoff = handoff;
    Box::new(
        move |input: AudioBufferView<'_, f32>, info: InputCallbackInfo| {
            handoff.deliver(&input, &info);
        },
    )
}

#[allow(clippy::type_complexity)] // test rig: tuple of endpoints + counters
fn handoff(
    pool: usize,
    filled_capacity: usize,
) -> (
    Handoff,
    ruviform_rt::Producer<CaptureBlock>,
    ruviform_rt::Consumer<CaptureBlock>,
    Arc<AtomicU64>,
    Arc<AtomicU64>,
    Arc<AtomicU64>,
) {
    let (free_tx, free_rx) = ruviform_rt::channel::<CaptureBlock>(pool.max(1));
    let (filled_tx, filled_rx) = ruviform_rt::channel::<CaptureBlock>(filled_capacity.max(1));
    let dropped = Arc::new(AtomicU64::new(0));
    let starved = Arc::new(AtomicU64::new(0));
    let in_flight = Arc::new(AtomicU64::new(0));
    let handoff = Handoff {
        free: free_rx,
        filled: filled_tx,
        working: None,
        dropped_frames: Arc::clone(&dropped),
        starved: Arc::clone(&starved),
        in_flight: Arc::clone(&in_flight),
    };
    (handoff, free_tx, filled_rx, dropped, starved, in_flight)
}

fn block(frames: usize, pos: u64, callback_nanos: i64) -> Block {
    Block {
        frames,
        callback_nanos,
        capture: Some((callback_nanos - 2_000_000, TimestampProvenance::Carried)),
        frame_position: Some(pos),
    }
}

#[test]
fn input_ports_are_object_safe_and_send() {
    fn assert_send<T: Send>(_: &T) {}
    fn assert_enumerator(_: &dyn DeviceEnumerator) {}
    fn assert_device(_: &dyn Device) {}
    fn assert_stream(_: &dyn InputStream) {}

    let (enumerator, _) = fake_input_enumerator(InputScript::default());
    assert_enumerator(&enumerator);
    let device = enumerator.default_input_device().unwrap().unwrap();
    assert_device(&*device);
    assert_send(&device);

    let config = InputStreamConfig {
        format: device.default_input_format().unwrap(),
        buffer_size: BufferSize::Default,
    };
    let stream = device
        .open_input_stream(&config, Box::new(|_, _| {}), Box::new(|_| {}))
        .unwrap();
    assert_stream(&*stream);
    assert_send(&stream);
}

#[test]
fn input_closures_coerce_onto_the_hrtb_callback_types() {
    let mut data_cb: InputDataCallback = Box::new(
        |input: AudioBufferView<'_, f32>, _info: InputCallbackInfo| {
            assert!(input.channels() >= 1);
        },
    );
    let mut err_cb: InputErrorCallback = Box::new(|_e: DeviceError| {});
    let slab = [0.25_f32; 8];
    let info = InputCallbackInfo {
        timestamp: InputTimestamp {
            callback: MediaTime::new(0, DEVICE_CLOCK_TIMEBASE),
            capture: None,
        },
        clock_domain: ClockDomainId::MONOTONIC,
        frame_position: Some(0),
    };
    data_cb(AudioBufferView::try_new(&slab, 2, 4).unwrap(), info);
    err_cb(DeviceError::ClockDiscontinuity);
}

#[test]
fn deterministic_capture_blocks_carry_metadata() {
    let (enumerator, shared) = fake_input_enumerator(InputScript {
        blocks: vec![block(4, 0, 1_000_000_000), block(8, 4, 1_005_000_000)],
        latency: Some(StreamLatency {
            frames: 96,
            coverage: LatencyCoverage::HardwareAndBuffering,
            provenance: TimestampProvenance::Carried,
        }),
        ..InputScript::default()
    });
    let device = enumerator.default_input_device().unwrap().unwrap();

    let format = device.default_input_format().unwrap();
    assert_eq!(format.sample_format, SampleFormat::F32);
    let supported = device.supported_input_formats().unwrap();
    assert_eq!(supported.len(), 1);
    assert_eq!(supported[0].channels, 2);

    // Consumer-side record of what the callback actually receives —
    // the delivered `info` must equal the script, not just the fake's
    // own bookkeeping.
    let delivered = Arc::new(Mutex::new(Vec::new()));
    let delivered_sink = Arc::clone(&delivered);
    let stream = device
        .open_input_stream(
            &InputStreamConfig {
                format,
                buffer_size: BufferSize::Fixed(480),
            },
            Box::new(move |_input, info| {
                delivered_sink.lock().unwrap().push((
                    info.timestamp.callback.value(),
                    info.timestamp.capture.map(|c| c.time.value()),
                    info.frame_position,
                    info.clock_domain,
                ));
            }),
            Box::new(|_| {}),
        )
        .unwrap();
    assert_eq!(stream.format(), format);
    assert_eq!(stream.buffer_size().unwrap(), BufferSize::Fixed(480));
    assert_eq!(stream.clock_domain(), ClockDomainId::MONOTONIC);
    assert_eq!(
        stream.input_latency().unwrap(),
        Some(StreamLatency {
            frames: 96,
            coverage: LatencyCoverage::HardwareAndBuffering,
            provenance: TimestampProvenance::Carried,
        }),
    );

    stream.start().unwrap();
    assert!(shared.controls.lock().unwrap().started);
    stream.stop().unwrap();
    assert!(!shared.controls.lock().unwrap().started);
    assert_eq!(stream.now().unwrap().value(), 2_000_000_000);

    let observed = shared.observed.lock().unwrap();
    assert_eq!(observed.firings.len(), 2);
    assert_eq!(
        observed
            .firings
            .iter()
            .map(|f| (f.channels, f.frames))
            .collect::<Vec<_>>(),
        vec![(2, 4), (2, 8)],
    );
    // Frame positions are non-decreasing and advance by `frames`
    // while no loss occurs.
    assert_eq!(observed.firings[0].frame_position, Some(0));
    assert_eq!(observed.firings[1].frame_position, Some(4));
    // `capture` is earlier than `callback` by the scripted latency and
    // was not conflated with the callback instant.
    assert_eq!(observed.firings[0].capture_nanos, Some(998_000_000));
    assert_eq!(observed.firings[0].callback_nanos, 1_000_000_000);
    assert!(
        observed
            .firings
            .iter()
            .all(|f| f.clock_domain == ClockDomainId::MONOTONIC),
    );
    drop(observed);

    // The delivered callback info equals the script end to end.
    assert_eq!(
        *delivered.lock().unwrap(),
        vec![
            (
                1_000_000_000,
                Some(998_000_000),
                Some(0),
                ClockDomainId::MONOTONIC
            ),
            (
                1_005_000_000,
                Some(1_003_000_000),
                Some(4),
                ClockDomainId::MONOTONIC
            ),
        ],
    );
}

#[test]
fn capture_stamp_bridges_to_input_stamp_vocabulary() {
    let info = InputCallbackInfo {
        timestamp: InputTimestamp {
            callback: MediaTime::new(10_000_000, DEVICE_CLOCK_TIMEBASE),
            capture: Some(MediaTimestamp {
                time: MediaTime::new(8_000_000, DEVICE_CLOCK_TIMEBASE),
                provenance: TimestampProvenance::Carried,
            }),
        },
        clock_domain: ClockDomainId::MONOTONIC,
        frame_position: Some(512),
    };
    let stamp = info.capture_stamp().unwrap();
    assert_eq!(stamp.ticks, 8_000_000);
    assert_eq!(stamp.base, DEVICE_CLOCK_TIMEBASE);
    assert_eq!(stamp.domain, ClockDomainId::MONOTONIC);
    assert_eq!(stamp.provenance, TimestampProvenance::Carried);

    // No capture instant → no stamp (never fabricated from `callback`).
    let mut no_capture = info;
    no_capture.timestamp.capture = None;
    assert!(no_capture.capture_stamp().is_none());

    // A foreign-base capture instant is rejected, not relabeled as
    // `DEVICE_CLOCK_TIMEBASE` ticks.
    let mut foreign_base = info;
    foreign_base.timestamp.capture = Some(MediaTimestamp {
        time: MediaTime::new(8_000_000, TimeBase::new(1, 48_000).unwrap()),
        provenance: TimestampProvenance::Carried,
    });
    assert!(foreign_base.capture_stamp().is_none());

    // Negative ticks → `None`, not a wrapped `u64`.
    let mut negative = info;
    negative.timestamp.capture = Some(MediaTimestamp {
        time: MediaTime::new(-1, DEVICE_CLOCK_TIMEBASE),
        provenance: TimestampProvenance::Carried,
    });
    assert!(negative.capture_stamp().is_none());
}

/// A stub output stream carrying a scriptable clock domain — for the
/// same-clock correlation check.
struct StubOutputStream {
    format: AudioFormat,
    domain: ClockDomainId,
}

impl OutputStream for StubOutputStream {
    fn start(&self) -> DeviceResult<()> {
        Ok(())
    }
    fn stop(&self) -> DeviceResult<()> {
        Ok(())
    }
    fn format(&self) -> AudioFormat {
        self.format
    }
    fn buffer_size(&self) -> DeviceResult<BufferSize> {
        Ok(BufferSize::Fixed(256))
    }
    fn now(&self) -> DeviceResult<MediaTime> {
        Ok(MediaTime::new(0, DEVICE_CLOCK_TIMEBASE))
    }
    fn clock_domain(&self) -> ClockDomainId {
        self.domain
    }
}

#[test]
fn same_clock_correlation_via_clock_domain() {
    // Two streams on one domain — a duplex-capable capture pair.
    let in_stream: Box<dyn InputStream> = {
        let (enumerator, _) = fake_input_enumerator(InputScript::default());
        let device = enumerator.default_input_device().unwrap().unwrap();
        device
            .open_input_stream(
                &InputStreamConfig {
                    format: stereo_format(),
                    buffer_size: BufferSize::Default,
                },
                Box::new(|_, _| {}),
                Box::new(|_| {}),
            )
            .unwrap()
    };
    let out_stream = StubOutputStream {
        format: stereo_format(),
        domain: ClockDomainId::MONOTONIC,
    };
    assert!(same_clock_domain(
        in_stream.clock_domain(),
        out_stream.clock_domain()
    ));

    // A different domain is not the same clock.
    let other = StubOutputStream {
        format: stereo_format(),
        domain: ClockDomainId::SAMPLE_CLOCK,
    };
    assert!(!same_clock_domain(
        in_stream.clock_domain(),
        other.clock_domain()
    ));

    // `UNKNOWN` == `UNKNOWN` is not a same-clock claim — the backend
    // could not identify the clock.
    let unknown_a = StubOutputStream {
        format: stereo_format(),
        domain: ClockDomainId::UNKNOWN,
    };
    let unknown_b = StubOutputStream {
        format: stereo_format(),
        domain: ClockDomainId::UNKNOWN,
    };
    assert!(!same_clock_domain(
        unknown_a.clock_domain(),
        unknown_b.clock_domain()
    ));
}

#[test]
fn bounded_handoff_delivers_all_blocks_in_order() {
    let (enumerator, _shared) = fake_input_enumerator(InputScript {
        blocks: vec![
            block(4, 0, 1_000_000_000),
            block(4, 4, 1_001_000_000),
            block(8, 8, 1_002_000_000),
        ],
        ..InputScript::default()
    });
    let device = enumerator.default_input_device().unwrap().unwrap();

    let (handoff_state, mut free_tx, mut filled_rx, dropped, starved, in_flight) = handoff(4, 4);
    for _ in 0..4 {
        free_tx.try_push(CaptureBlock::with_capacity(2, 8)).unwrap();
    }
    let stream = device
        .open_input_stream(
            &InputStreamConfig {
                format: stereo_format(),
                buffer_size: BufferSize::Fixed(480),
            },
            handoff_callback(handoff_state),
            Box::new(|_| {}),
        )
        .unwrap();
    stream.start().unwrap();

    // Non-RT drain: every scripted block arrives in order with its
    // samples and metadata intact; blocks recycle to the free pool.
    let mut expected_pos = 0;
    for (index, expected_frames) in [4_usize, 4, 8].iter().enumerate() {
        let drained = filled_rx.try_pop().unwrap();
        assert_eq!(drained.channels, 2);
        assert_eq!(drained.frames, *expected_frames);
        assert_eq!(drained.frame_position, Some(expected_pos));
        assert_eq!(drained.clock_domain, ClockDomainId::MONOTONIC);
        assert!(drained.capture.is_some());
        // Data integrity: the planar samples themselves, not just the
        // metadata — a mixed-up or offset copy is visible here.
        for ch in 0..drained.channels {
            for f in 0..drained.frames {
                assert_eq!(
                    drained.samples[ch * drained.frames + f],
                    scripted_sample(ch, index * 1_000 + f),
                );
            }
        }
        expected_pos += *expected_frames as u64;
        free_tx.try_push(drained).unwrap(); // recycle
    }
    assert!(filled_rx.try_pop().is_err());
    assert_eq!(dropped.load(Ordering::Relaxed), 0);
    assert_eq!(starved.load(Ordering::Relaxed), 0);
    assert_eq!(in_flight.load(Ordering::Relaxed), 0);
}

#[test]
fn queue_pressure_drops_are_observed_not_silent() {
    let (enumerator, _shared) = fake_input_enumerator(InputScript {
        blocks: vec![
            block(4, 0, 1_000_000_000),
            block(4, 4, 1_001_000_000),
            block(4, 8, 1_002_000_000),
            block(4, 12, 1_003_000_000),
        ],
        ..InputScript::default()
    });
    let device = enumerator.default_input_device().unwrap().unwrap();

    // Deliberate pressure: `filled` holds one block, pool holds two.
    let (handoff_state, mut free_tx, mut filled_rx, dropped, _starved, in_flight) = handoff(2, 1);
    for _ in 0..2 {
        free_tx.try_push(CaptureBlock::with_capacity(2, 8)).unwrap();
    }
    let stream = device
        .open_input_stream(
            &InputStreamConfig {
                format: stereo_format(),
                buffer_size: BufferSize::Fixed(480),
            },
            handoff_callback(handoff_state),
            Box::new(|_| {}),
        )
        .unwrap();
    stream.start().unwrap();

    // The consumer never drained: only the first block was delivered,
    // the second is still held in `working` for retry (backpressure,
    // not loss — `in_flight` keeps the undrained tail observable), and
    // the last two blocks' frames were counted as observable loss —
    // not silently discarded.
    let first = filled_rx.try_pop().unwrap();
    assert_eq!(first.frame_position, Some(0));
    assert!(filled_rx.try_pop().is_err());
    assert_eq!(dropped.load(Ordering::Relaxed), 8);
    assert_eq!(in_flight.load(Ordering::Relaxed), 4);
}

#[test]
fn scripted_errors_reach_the_error_callback() {
    let (enumerator, _shared) = fake_input_enumerator(InputScript {
        errors: vec![
            DeviceError::Xrun,
            DeviceError::ClockDiscontinuity,
            DeviceError::Backend("device_changed"),
            DeviceError::StreamInvalidated,
        ],
        ..InputScript::default()
    });
    let device = enumerator.default_input_device().unwrap().unwrap();
    // The error callback is RT-adjacent (no alloc · no block): the
    // consumer-side pattern is a bounded SPSC sink — `DeviceError` is
    // `Copy`, overflow is counted, never a lock or a `Vec::push`.
    let (mut err_tx, mut err_rx) = ruviform_rt::channel::<DeviceError>(8);
    let overflow = Arc::new(AtomicU64::new(0));
    let overflow_sink = Arc::clone(&overflow);
    let stream = device
        .open_input_stream(
            &InputStreamConfig {
                format: stereo_format(),
                buffer_size: BufferSize::Default,
            },
            Box::new(|_, _| {}),
            Box::new(move |error| {
                if err_tx.try_push(error).is_err() {
                    overflow_sink.fetch_add(1, Ordering::Relaxed);
                }
            }),
        )
        .unwrap();
    stream.start().unwrap();
    let mut got = Vec::new();
    while let Ok(error) = err_rx.try_pop() {
        got.push(error);
    }
    assert_eq!(
        got,
        vec![
            DeviceError::Xrun,
            DeviceError::ClockDiscontinuity,
            DeviceError::Backend("device_changed"),
            DeviceError::StreamInvalidated,
        ],
    );
    assert_eq!(overflow.load(Ordering::Relaxed), 0);
}

#[test]
fn open_failures_use_the_defined_error_surface() {
    // Backend cannot open input at all.
    let (enumerator, _) = fake_input_enumerator(InputScript {
        fail_open: Some(DeviceError::Unsupported),
        ..InputScript::default()
    });
    let device = enumerator.default_input_device().unwrap().unwrap();
    assert!(matches!(
        device.open_input_stream(
            &InputStreamConfig {
                format: stereo_format(),
                buffer_size: BufferSize::Default,
            },
            Box::new(|_, _| {}),
            Box::new(|_| {}),
        ),
        Err(DeviceError::Unsupported),
    ));

    // A non-F32 config is outside the supported space.
    let (enumerator, _) = fake_input_enumerator(InputScript::default());
    let device = enumerator.default_input_device().unwrap().unwrap();
    let mut config = InputStreamConfig {
        format: stereo_format(),
        buffer_size: BufferSize::Default,
    };
    config.format.sample_format = SampleFormat::F64;
    assert!(matches!(
        device.open_input_stream(&config, Box::new(|_, _| {}), Box::new(|_| {})),
        Err(DeviceError::UnsupportedConfig),
    ));

    // The input-only fake reports the output direction honestly:
    // empty supported space, `Unsupported` at open.
    assert_eq!(device.supported_output_formats().unwrap().len(), 0);
    assert!(matches!(
        device.open_output_stream(
            &OutputStreamConfig {
                format: stereo_format(),
                buffer_size: BufferSize::Default,
            },
            Box::new(|_, _| {}),
            Box::new(|_| {}),
        ),
        Err(DeviceError::Unsupported),
    ));
}

#[test]
fn cpal_adapter_reports_input_open_as_unsupported() {
    // The real adapter's honest failure surface: enumeration compiles
    // and `open_input_stream` returns `Unsupported` — never a fake
    // capture. Skips silently on hosts with no input device (headless
    // CI lanes).
    let host = ruviform_audio_device::default_host().unwrap();
    if let Ok(devices) = host.input_devices()
        && let Some(device) = devices.first()
    {
        let result = device.open_input_stream(
            &InputStreamConfig {
                format: stereo_format(),
                buffer_size: BufferSize::Default,
            },
            Box::new(|_, _| {}),
            Box::new(|_| {}),
        );
        assert!(matches!(result, Err(DeviceError::Unsupported)));

        // Format queries compile and map through the same F32 rules —
        // either a usable range list/format or a typed error.
        let _ = device.supported_input_formats();
        let _ = device.default_input_format();
    }
    // `default_input_device` is callable — `Ok(None)` is legal.
    let _ = host.default_input_device();
}

/// RT-lane check: view construction + bounded planar copy + `try_push`
/// delivery — the whole callback→handoff path — allocates nothing.
#[allow(clippy::float_cmp)]
#[test]
fn rt_input_callback_handoff_allocates_nothing() {
    // Setup (allocating, outside the window).
    let (mut free_tx, free_rx) = ruviform_rt::channel::<CaptureBlock>(2);
    let (filled_tx, mut filled_rx) = ruviform_rt::channel::<CaptureBlock>(2);
    free_tx.try_push(CaptureBlock::with_capacity(2, 8)).unwrap();
    let dropped = Arc::new(AtomicU64::new(0));
    let starved = Arc::new(AtomicU64::new(0));
    let handoff = Handoff {
        free: free_rx,
        filled: filled_tx,
        working: None,
        dropped_frames: Arc::clone(&dropped),
        starved: Arc::clone(&starved),
        in_flight: Arc::new(AtomicU64::new(0)),
    };
    let mut cb = handoff_callback(handoff);

    let slab = [0.5_f32; 16]; // 2ch × 8 frames
    let info = InputCallbackInfo {
        timestamp: InputTimestamp {
            callback: MediaTime::new(1_000_000_000, DEVICE_CLOCK_TIMEBASE),
            capture: Some(MediaTimestamp {
                time: MediaTime::new(998_000_000, DEVICE_CLOCK_TIMEBASE),
                provenance: TimestampProvenance::Carried,
            }),
        },
        clock_domain: ClockDomainId::MONOTONIC,
        frame_position: Some(0),
    };
    assert_no_alloc::assert_no_alloc(|| {
        cb(AudioBufferView::try_new(&slab, 2, 8).unwrap(), info);
    });
    let delivered = filled_rx.try_pop().unwrap();
    assert_eq!(delivered.frames, 8);
    assert_eq!(delivered.samples[..8], [0.5; 8]);
    assert_eq!(dropped.load(Ordering::Relaxed), 0);
}

/// RT-lane check: `filled` full + `free` empty is bounded — the
/// callback returns with loss counted, never blocking or allocating.
#[test]
fn rt_input_callback_queue_pressure_stays_bounded() {
    let (mut free_tx, free_rx) = ruviform_rt::channel::<CaptureBlock>(1);
    let (mut filled_tx, _filled_rx) = ruviform_rt::channel::<CaptureBlock>(1);
    // Pre-fill `filled` to capacity and hold one block in `working`:
    // every callback then hits the retry-fail → drop path.
    filled_tx
        .try_push(CaptureBlock::with_capacity(2, 8))
        .unwrap();
    free_tx.try_push(CaptureBlock::with_capacity(2, 8)).unwrap();
    let dropped = Arc::new(AtomicU64::new(0));
    let starved = Arc::new(AtomicU64::new(0));
    let handoff = Handoff {
        free: free_rx,
        filled: filled_tx,
        working: Some(CaptureBlock::with_capacity(2, 8)),
        dropped_frames: Arc::clone(&dropped),
        starved: Arc::clone(&starved),
        in_flight: Arc::new(AtomicU64::new(0)),
    };
    let mut cb = handoff_callback(handoff);
    let slab = [0.5_f32; 16];
    let info = InputCallbackInfo {
        timestamp: InputTimestamp {
            callback: MediaTime::new(0, DEVICE_CLOCK_TIMEBASE),
            capture: None,
        },
        clock_domain: ClockDomainId::UNKNOWN,
        frame_position: None,
    };
    // Two callbacks under pressure inside the window: retry-fail then
    // drop-count, both bounded.
    assert_no_alloc::assert_no_alloc(|| {
        let view = AudioBufferView::try_new(&slab, 2, 8).unwrap();
        cb(view, info);
        cb(view, info);
    });
    assert_eq!(dropped.load(Ordering::Relaxed), 16);
}
