//! Plugin → graph E2E (Issue #113 task 6 —
//! `docs/tasks/plugin-graph-e2e-and-rt-lane.md`, oracle
//! `docs/test-specs/plugin-graph-e2e-tests.md`).
//!
//! The adhesion the spec pins: a `Send` `PluginEndpoint` moves into
//! a `FnMut(&mut AudioBufferViewMut<'_, f32>, &RtContext) + Send`
//! closure, the closure pushes onto `Graph` as `Box<dyn AudioNode>`
//! through the blanket impl, and `Session` drives the plan through a
//! mock device's data callback — the test thread fires the callback
//! manually, so nothing here touches cpal or a real audio device.
//!
//! Route coverage is the real public API both ways: L0 through
//! `register_factory` + `execute(HostCommand::Instantiate)`, L1
//! through `register_out_of_process` spawning the crate-local
//! `l1-test-worker` binary (`CARGO_BIN_EXE_l1-test-worker` — the
//! same env-var closure `l1_proxy_e2e` uses).

use std::ffi::OsString;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use ruviform_audio::{AudioNode, Graph, GraphPlan, PlanSwap, Session};
use ruviform_audio_core::device::{
    BufferSize, DEVICE_CLOCK_TIMEBASE, Device, DeviceEnumerator, DeviceError, DeviceId,
    DeviceResult, InputDataCallback, InputErrorCallback, InputStream, InputStreamConfig,
    OutputCallbackInfo, OutputDataCallback, OutputStream, OutputStreamConfig, OutputTimestamp,
    SupportedInputFormat, SupportedOutputFormat,
};
use ruviform_audio_core::{
    AudioBufferViewMut, AudioFormat, ChannelLayout, SampleFormat, SampleRate,
};
use ruviform_media_core::MediaTime;
use ruviform_plugin_core::{
    BufferView, BufferViewMut, EventList, HostError, LifecycleState, ParamInfo, PluginDescriptor,
    PluginProcessor, ProcessConfig, ProcessContext, ProcessResult, ProcessStatus, TransportInfo,
};
use ruviform_plugin_host::{
    EndpointBounds, ErasedPlugin, HostCommand, HostNotification, HostReply, OutOfProcessSpec,
    Plugin, PluginEndpoint, PluginFactory, PluginHandle, PluginHost, erase, erase_sendable,
};
use ruviform_rt::RtContext;

/// `assert_no_alloc` arms a thread-local forbid flag through the
/// global allocator — required exactly once per test binary (the
/// `rt_endpoint_*` contract tests).
#[cfg(debug_assertions)]
#[global_allocator]
static ALLOC_DISABLER: assert_no_alloc::AllocDisabler = assert_no_alloc::AllocDisabler;

/// How long L1 tests poll for asynchronous `Failed` convergence —
/// crash detection needs at most a driver poll plus scheduling; the
/// generous bound leaves headroom for loaded CI hosts (mirrors
/// `l1_proxy_e2e`).
const NOTIFY_TIMEOUT: Duration = Duration::from_secs(10);

// -------------------------------------------------------------------
// Mock device double — `DeviceEnumerator`/`Device`/`OutputStream`
// with a captured data callback the test thread drives manually.
// -------------------------------------------------------------------

/// The opened-stream half — reports the negotiated format, every
/// control verb is a no-op `Ok`.
struct MockStream {
    format: AudioFormat,
}

impl OutputStream for MockStream {
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
}

/// The device half — `open_output_stream` captures the data
/// callback into the shared slot instead of spawning an RT thread.
struct MockDevice {
    format: AudioFormat,
    callback: Arc<Mutex<Option<OutputDataCallback>>>,
}

impl Device for MockDevice {
    fn name(&self) -> DeviceResult<String> {
        Ok("e2e-mock".to_owned())
    }

    fn id(&self) -> DeviceResult<Option<DeviceId>> {
        Ok(None)
    }

    fn default_output_format(&self) -> DeviceResult<AudioFormat> {
        Ok(self.format)
    }

    fn supported_output_formats(&self) -> DeviceResult<Vec<SupportedOutputFormat>> {
        Ok(Vec::new())
    }

    fn open_output_stream(
        &self,
        config: &OutputStreamConfig,
        data_callback: OutputDataCallback,
        _error_callback: ruviform_audio_core::device::OutputErrorCallback,
    ) -> DeviceResult<Box<dyn OutputStream>> {
        *self.callback.lock().unwrap() = Some(data_callback);
        Ok(Box::new(MockStream {
            format: config.format,
        }))
    }

    fn default_input_format(&self) -> DeviceResult<AudioFormat> {
        Err(DeviceError::Unsupported)
    }

    fn supported_input_formats(&self) -> DeviceResult<Vec<SupportedInputFormat>> {
        Ok(Vec::new())
    }

    fn open_input_stream(
        &self,
        _config: &InputStreamConfig,
        _data_callback: InputDataCallback,
        _error_callback: InputErrorCallback,
    ) -> DeviceResult<Box<dyn InputStream>> {
        Err(DeviceError::Unsupported)
    }
}

/// The enumerator half — hands out `MockDevice`s that all share the
/// test's callback slot.
struct MockEnumerator {
    format: AudioFormat,
    callback: Arc<Mutex<Option<OutputDataCallback>>>,
}

impl MockEnumerator {
    /// An enumerator plus the slot its devices fill with the data
    /// callback `Session::open` hands over.
    fn new() -> (Self, Arc<Mutex<Option<OutputDataCallback>>>) {
        let slot = Arc::new(Mutex::new(None));
        (
            Self {
                format: AudioFormat {
                    sample_rate: SampleRate::new(48_000).unwrap(),
                    layout: ChannelLayout::Stereo,
                    sample_format: SampleFormat::F32,
                },
                callback: Arc::clone(&slot),
            },
            slot,
        )
    }

    fn device(&self) -> Box<dyn Device> {
        Box::new(MockDevice {
            format: self.format,
            callback: Arc::clone(&self.callback),
        })
    }
}

impl DeviceEnumerator for MockEnumerator {
    fn output_devices(&self) -> DeviceResult<Vec<Box<dyn Device>>> {
        Ok(vec![self.device()])
    }

    fn default_output_device(&self) -> DeviceResult<Option<Box<dyn Device>>> {
        Ok(Some(self.device()))
    }

    fn input_devices(&self) -> DeviceResult<Vec<Box<dyn Device>>> {
        Ok(Vec::new())
    }

    fn default_input_device(&self) -> DeviceResult<Option<Box<dyn Device>>> {
        Ok(None)
    }
}

/// The stereo stream config every session below opens with.
fn stream_config() -> OutputStreamConfig {
    OutputStreamConfig {
        format: AudioFormat {
            sample_rate: SampleRate::new(48_000).unwrap(),
            layout: ChannelLayout::Stereo,
            sample_format: SampleFormat::F32,
        },
        buffer_size: BufferSize::Default,
    }
}

/// The callback-info payload — timestamps are inert for these
/// tests.
fn info() -> OutputCallbackInfo {
    OutputCallbackInfo {
        timestamp: OutputTimestamp {
            callback: MediaTime::new(0, DEVICE_CLOCK_TIMEBASE),
            playback: None,
        },
    }
}

/// Takes the captured data callback out of the slot — the test's
/// manual RT-thread stand-in.
fn take_callback(slot: &Arc<Mutex<Option<OutputDataCallback>>>) -> OutputDataCallback {
    slot.lock()
        .unwrap()
        .take()
        .expect("session stored a callback")
}

// -------------------------------------------------------------------
// The Send-capable gain mock — `create_sendable` opts instances into
// `Shared` slots so `prepare_endpoint` can mint an L0 endpoint.
// -------------------------------------------------------------------

const GAIN_DESCRIPTOR: PluginDescriptor = PluginDescriptor {
    id: "org.ruviform.test.graph-e2e-gain",
    name: "Graph E2E Gain",
    vendor: "Ruviform",
    version: "0.1.0",
    features: &[],
};

/// A `Send` gain plugin: `out = in × factor`, with a shared panic
/// injection (process-block convergence tests) and an optional drop
/// probe (teardown-chain observation).
struct Gain {
    factor: f32,
    panic: Arc<AtomicBool>,
    dropped: Option<Arc<AtomicBool>>,
}

impl PluginProcessor for Gain {
    fn prepare(
        &mut self,
        _config: &ProcessConfig,
        _context: &mut ProcessContext,
    ) -> Result<(), ruviform_plugin_core::PrepareError> {
        Ok(())
    }

    fn process<I, O>(
        &mut self,
        inputs: &[I],
        outputs: &mut [O],
        _events: EventList<'_>,
        _transport: Option<TransportInfo>,
        _context: &mut ProcessContext,
    ) -> ProcessResult
    where
        I: BufferView,
        O: BufferViewMut,
    {
        assert!(
            !self.panic.load(Ordering::Relaxed),
            "injected process panic"
        );
        for (input, output) in inputs.iter().zip(outputs.iter_mut()) {
            for channel in 0..output.channels() {
                if let (Some(src), Some(dst)) =
                    (input.channel(channel), output.channel_mut(channel))
                {
                    for (s, d) in src.iter().zip(dst.iter_mut()) {
                        *d = *s * self.factor;
                    }
                }
            }
        }
        ProcessResult::new(ProcessStatus::Continue, ProcessResult::TAIL_NONE)
    }
}

impl Plugin for Gain {
    fn descriptor(&self) -> PluginDescriptor {
        GAIN_DESCRIPTOR
    }

    fn params(&self) -> &[ParamInfo] {
        &[]
    }

    fn save_state(&mut self) -> Result<Vec<u8>, HostError> {
        Ok(self.factor.to_le_bytes().to_vec())
    }

    fn restore_state(&mut self, state: &[u8]) -> Result<(), HostError> {
        let bytes: [u8; 4] = state.try_into().map_err(|_| HostError::PluginFailed)?;
        self.factor = f32::from_le_bytes(bytes);
        Ok(())
    }
}

impl Drop for Gain {
    /// The optional teardown probe — set when the `SendCore` cell
    /// holding this plugin is finally dropped.
    fn drop(&mut self) {
        if let Some(flag) = &self.dropped {
            flag.store(true, Ordering::SeqCst);
        }
    }
}

/// `create_sendable`-capable factory — clones the shared panic flag
/// and drop probe into every instance it makes.
struct GainFactory {
    gain: f32,
    panic: Arc<AtomicBool>,
    dropped: Option<Arc<AtomicBool>>,
}

impl GainFactory {
    fn new(gain: f32) -> Self {
        Self {
            gain,
            panic: Arc::new(AtomicBool::new(false)),
            dropped: None,
        }
    }
}

impl PluginFactory for GainFactory {
    fn descriptor(&self) -> PluginDescriptor {
        GAIN_DESCRIPTOR
    }

    fn create(&self) -> Box<dyn ErasedPlugin> {
        erase(Gain {
            factor: self.gain,
            panic: Arc::clone(&self.panic),
            dropped: self.dropped.clone(),
        })
    }

    fn create_sendable(&self) -> Option<Box<dyn ErasedPlugin + Send>> {
        Some(erase_sendable(Gain {
            factor: self.gain,
            panic: Arc::clone(&self.panic),
            dropped: self.dropped.clone(),
        }))
    }
}

// -------------------------------------------------------------------
// Host fixtures — the real public API paths on both routes.
// -------------------------------------------------------------------

fn config(min_frames: u32, max_frames: u32) -> ProcessConfig {
    ProcessConfig::new(SampleRate::new(48_000).unwrap(), min_frames, max_frames).unwrap()
}

fn bounds(max_frames: u32, channels: u32) -> EndpointBounds {
    EndpointBounds {
        max_frames,
        channels,
    }
}

/// Instantiates the registered `plugin_id` and returns the handle.
fn instantiate(host: &mut PluginHost, plugin_id: &str) -> PluginHandle {
    match host
        .execute(HostCommand::Instantiate {
            plugin_id: plugin_id.to_owned(),
        })
        .unwrap()
    {
        HostReply::Instantiated { handle } => handle,
        _ => panic!("unexpected reply"),
    }
}

/// L0 route: `register_factory` + `Instantiate` driven to
/// `Processing`.
fn l0_processing(factory: Box<dyn PluginFactory>) -> (PluginHost, PluginHandle) {
    let mut host = PluginHost::new();
    host.register_factory(factory);
    let handle = instantiate(&mut host, GAIN_DESCRIPTOR.id);
    host.execute(HostCommand::Initialize { handle }).unwrap();
    host.execute(HostCommand::Activate {
        handle,
        config: config(1, 512),
    })
    .unwrap();
    host.execute(HostCommand::StartProcessing { handle })
        .unwrap();
    (host, handle)
}

/// The descriptor `l1-test-worker` reports at `Scan` — the
/// handshake field-matches it (same value `l1_proxy_e2e` pins).
const WORKER_DESCRIPTOR: PluginDescriptor = PluginDescriptor {
    id: "org.ruviform.test.l1-test-worker",
    name: "L1 Test Worker",
    vendor: "Ruviform",
    version: "0.1.0",
    features: &[],
};

/// L1 route: `register_out_of_process` against the crate-local
/// worker binary + `Instantiate` driven to `Processing`.
/// `lookahead_blocks` rides `OutOfProcessSpec::new`'s default (2).
fn l1_processing() -> (PluginHost, PluginHandle) {
    let mut host = PluginHost::new();
    let spec = OutOfProcessSpec::new(
        WORKER_DESCRIPTOR,
        OsString::from(env!("CARGO_BIN_EXE_l1-test-worker")),
    );
    host.register_out_of_process(spec).unwrap();
    let handle = instantiate(&mut host, WORKER_DESCRIPTOR.id);
    host.execute(HostCommand::Initialize { handle }).unwrap();
    host.execute(HostCommand::Activate {
        handle,
        config: config(1, 512),
    })
    .unwrap();
    host.execute(HostCommand::StartProcessing { handle })
        .unwrap();
    (host, handle)
}

/// Polls `lifecycle_state` until the instance converges `Failed`
/// (the L1 driver's asynchronous detection).
fn wait_for_failed(host: &PluginHost, handle: PluginHandle) {
    let deadline = Instant::now() + NOTIFY_TIMEOUT;
    while host.lifecycle_state(handle) != Some(LifecycleState::Failed) {
        assert!(Instant::now() < deadline, "instance never reached Failed");
        thread::sleep(Duration::from_millis(5));
    }
}

/// Polls `try_recv_notification` until the `InstanceFailed`
/// notification arrives — the `&mut` drain inside it is what pulls
/// the driver's child-exit observation forward (mirrors
/// `l1_proxy_e2e`'s wait helper).
fn wait_instance_failed(host: &mut PluginHost) -> PluginHandle {
    let deadline = Instant::now() + NOTIFY_TIMEOUT;
    loop {
        if let Some(HostNotification::InstanceFailed { handle, .. }) = host.try_recv_notification()
        {
            return handle;
        }
        assert!(
            Instant::now() < deadline,
            "InstanceFailed notification never arrived"
        );
        thread::sleep(Duration::from_millis(5));
    }
}

/// The transform-position node — `out` carries the upstream block
/// in and the plugin result back; the closure records each call's
/// `Result` into `results` for post-hoc assertions — a
/// test-harness recorder, uncontended by construction (the test
/// thread is the only callback driver), not a production RT
/// pattern. `Send`-correct because `PluginEndpoint` is `Send` —
/// `Box<dyn AudioNode>` would reject the coercion otherwise.
fn transform_node(
    mut endpoint: PluginEndpoint,
    results: Arc<Mutex<Vec<Result<ProcessResult, HostError>>>>,
) -> impl FnMut(&mut AudioBufferViewMut<'_, f32>, &RtContext) + Send {
    move |out, ctx| {
        let result = endpoint.process_audio(
            out,
            EventList::empty(u32::try_from(out.frames()).unwrap()),
            None,
            ctx,
        );
        results.lock().unwrap().push(result);
    }
}

/// The source-position node — the adapter contract's `fill(0.0)`
/// convention runs before `process_audio`, so a bypassed or
/// rejected block emits silence instead of stale upstream samples.
fn source_node(
    mut endpoint: PluginEndpoint,
) -> impl FnMut(&mut AudioBufferViewMut<'_, f32>, &RtContext) + Send {
    move |out, ctx| {
        out.data_mut().fill(0.0);
        let _ = endpoint.process_audio(
            out,
            EventList::empty(u32::try_from(out.frames()).unwrap()),
            None,
            ctx,
        );
    }
}

/// An endpoint-bearing node with a drop probe — pins *when* the
/// retired plan's destructor chain runs (never on the RT pull path).
struct EndpointNode {
    endpoint: PluginEndpoint,
    dropped: Arc<AtomicBool>,
}

impl AudioNode for EndpointNode {
    fn pull(&mut self, out: &mut AudioBufferViewMut<'_, f32>, ctx: &RtContext) {
        let _ = self.endpoint.process_audio(
            out,
            EventList::empty(u32::try_from(out.frames()).unwrap()),
            None,
            ctx,
        );
    }
}

impl Drop for EndpointNode {
    fn drop(&mut self) {
        self.dropped.store(true, Ordering::SeqCst);
    }
}

/// Bit-exact sample comparison — gain factors (×2.0, ×0.5) are
/// exactly representable so no epsilon is needed or wanted.
fn all_eq(data: &[f32], value: f32) -> bool {
    data.iter().all(|s| s.to_bits() == value.to_bits())
}

fn sample_eq(actual: f32, expected: f32) -> bool {
    actual.to_bits() == expected.to_bits()
}

/// Whether `sample` is a defined L1 output value: silence or the
/// worker-gained (×0.5) copy of an input that could have completed.
fn is_defined_l1_sample(sample: f32, inputs: &[f32]) -> bool {
    sample_eq(sample, 0.0) || inputs.iter().any(|&v| sample_eq(sample, 0.5 * v))
}

// -------------------------------------------------------------------
// Compile-time Send evidence (spec AC2): the types that cross onto
// the RT callback path are `Send` structurally — no `unsafe impl`.
// The node closure itself is proven `Send` by `Graph::push`'s
// `Box<dyn AudioNode>` bound in every test below.
// -------------------------------------------------------------------
static_assertions::assert_impl_all!(PluginEndpoint: Send);
static_assertions::assert_impl_all!(GraphPlan: Send, Sync);
static_assertions::assert_impl_all!(ruviform_audio::RtPlan: Send);
static_assertions::assert_impl_all!(PlanSwap: Send, Sync);
static_assertions::assert_impl_all!(Session: Send);
static_assertions::assert_impl_all!(OutputDataCallback: Send);

fn expect_send<T: Send>(_: &T) {}

/// The concrete closure type the spec pins — an `FnMut(&mut
/// AudioBufferViewMut<'_, f32>, &RtContext)` capturing a
/// `PluginEndpoint` — is `Send` (compile-checked, not merely
/// coerced).
#[test]
fn endpoint_capturing_closure_is_send() {
    let (mut host, handle) = l0_processing(Box::new(GainFactory::new(2.0)));
    let endpoint = host.prepare_endpoint(handle, bounds(512, 2)).unwrap();
    let node = source_node(endpoint);
    expect_send(&node);
    let mut graph = Graph::new();
    graph.push(Box::new(node));
}

// -------------------------------------------------------------------
// L0 graph E2E — gain correctness through `Session`'s real callback
// wiring plus the endpoint's reported contract values.
// -------------------------------------------------------------------

/// Send-capable gain (×2.0) → `prepare_endpoint` → transform-node
/// closure → `Graph::push` → `Session::open` on the mock device →
/// manually fired callback: `0.5` upstream becomes `1.0`, and the
/// call's `ProcessResult` (`CONTINUE`, tail preserved) is recorded.
#[test]
fn l0_endpoint_node_drives_gain_through_session() {
    let (mut host, handle) = l0_processing(Box::new(GainFactory::new(2.0)));
    let endpoint = host
        .prepare_endpoint(handle, bounds(512, 2))
        .expect("L0 Shared attach");
    // The attach-time contract values the endpoint reports.
    assert_eq!(endpoint.bounds(), bounds(512, 2));
    assert_eq!(endpoint.max_frames(), 512);
    assert_eq!(endpoint.channels(), 2);
    assert_eq!(endpoint.latency_frames(), 0);
    assert_eq!(host.latency_frames(handle), Some(0));

    let results = Arc::new(Mutex::new(Vec::new()));
    let mut graph = Graph::new();
    graph.push(Box::new(transform_node(endpoint, Arc::clone(&results))));

    let (enumerator, slot) = MockEnumerator::new();
    let device = enumerator.default_output_device().unwrap().unwrap();
    let session = Session::open(&*device, stream_config(), GraphPlan::new(graph)).unwrap();
    session.start().unwrap();
    let mut cb = take_callback(&slot);

    // 2ch × 8f upstream block — transform reads it through
    // `in_scratch` and writes the gained result back.
    let mut slab = [0.5_f32; 16];
    for _ in 0..8 {
        slab.fill(0.5);
        cb(
            AudioBufferViewMut::try_new(&mut slab, 2, 8).unwrap(),
            info(),
        );
        assert!(all_eq(&slab, 1.0));
    }
    // Every call answered `Ok(CONTINUE)` — `tail_frames` preserved.
    assert_eq!(
        results.lock().unwrap().as_slice(),
        &[Ok(ProcessResult::CONTINUE); 8]
    );
    drop(session);
}

/// A plugin panic inside the shared cell converges the endpoint to
/// `Failed`: `Ok(CONTINUE)` + silence — never an `Err`, never a
/// hang — and the host-side verb surface reports `Err(Failed)`.
#[test]
fn l0_panic_converges_failed_with_silence() {
    let factory = GainFactory::new(2.0);
    let panic = Arc::clone(&factory.panic);
    let (mut host, handle) = l0_processing(Box::new(factory));
    let endpoint = host.prepare_endpoint(handle, bounds(512, 2)).unwrap();

    let results = Arc::new(Mutex::new(Vec::new()));
    let mut graph = Graph::new();
    graph.push(Box::new(transform_node(endpoint, Arc::clone(&results))));
    let (enumerator, slot) = MockEnumerator::new();
    let device = enumerator.default_output_device().unwrap().unwrap();
    let session = Session::open(&*device, stream_config(), GraphPlan::new(graph)).unwrap();
    let mut cb = take_callback(&slot);

    // Healthy first block — transform passes 7.0 upstream through
    // the staged copy, the plugin doubles it.
    let mut slab = [7.0_f32; 16];
    cb(
        AudioBufferViewMut::try_new(&mut slab, 2, 8).unwrap(),
        info(),
    );
    assert!(all_eq(&slab, 14.0));

    panic.store(true, Ordering::Relaxed);
    slab.fill(7.0);
    // The `catch_unwind` convergence writes silence and answers
    // `Ok(CONTINUE)` — `Failed` is a terminal output row, not an
    // `Err`.
    cb(
        AudioBufferViewMut::try_new(&mut slab, 2, 8).unwrap(),
        info(),
    );
    assert!(all_eq(&slab, 0.0));
    assert_eq!(
        results.lock().unwrap().as_slice(),
        &[Ok(ProcessResult::CONTINUE); 2]
    );
    assert_eq!(host.lifecycle_state(handle), Some(LifecycleState::Failed));
    assert_eq!(
        host.execute(HostCommand::SaveState { handle }),
        Err(HostError::Failed)
    );
    drop(session);
}

/// The whole attached lifecycle on the graph: `StopProcessing` →
/// `Err` bypass (transform keeps upstream), `Deactivate` → same,
/// bound-fitting `Activate` + `StartProcessing` → processing
/// resumes, an over-bound `Activate` is `Err(Prepare(..))`, and a
/// post-`Destroy` endpoint answers `Err(Lifecycle)` unchanged.
#[test]
fn l0_lifecycle_while_attached() {
    let (mut host, handle) = l0_processing(Box::new(GainFactory::new(2.0)));
    let endpoint = host.prepare_endpoint(handle, bounds(512, 2)).unwrap();

    let results = Arc::new(Mutex::new(Vec::new()));
    let mut graph = Graph::new();
    graph.push(Box::new(transform_node(endpoint, Arc::clone(&results))));
    let (enumerator, slot) = MockEnumerator::new();
    let device = enumerator.default_output_device().unwrap().unwrap();
    let session = Session::open(&*device, stream_config(), GraphPlan::new(graph)).unwrap();
    let mut cb = take_callback(&slot);
    let mut slab = [0.5_f32; 16];
    let mut drive = |slab: &mut [f32; 16], fill: f32| {
        slab.fill(fill);
        cb(
            AudioBufferViewMut::try_new(&mut slab[..], 2, 8).unwrap(),
            info(),
        );
    };

    drive(&mut slab, 0.5);
    assert!(all_eq(&slab, 1.0));

    // `StopProcessing` — the leg's `Err(Lifecycle)` leaves `out`
    // unchanged: the transform-position bypass row.
    host.execute(HostCommand::StopProcessing { handle })
        .unwrap();
    drive(&mut slab, 0.5);
    assert!(all_eq(&slab, 0.5));
    // `Deactivate` — same defined bypass.
    host.execute(HostCommand::Deactivate { handle }).unwrap();
    drive(&mut slab, 0.5);
    assert!(all_eq(&slab, 0.5));
    // An over-bound re-`Activate` is the attach-bound gate.
    assert!(matches!(
        host.execute(HostCommand::Activate {
            handle,
            config: config(1, 1024),
        }),
        Err(HostError::Prepare(_))
    ));
    // Bound-fitting re-activation resumes — the endpoint's
    // `SendCore.activated` republishes through the same lock the
    // verb took.
    host.execute(HostCommand::Activate {
        handle,
        config: config(1, 512),
    })
    .unwrap();
    host.execute(HostCommand::StartProcessing { handle })
        .unwrap();
    drive(&mut slab, 0.5);
    assert!(all_eq(&slab, 1.0));

    // `Destroy` from `Activated` (the legal transition) frees the
    // slot; the surviving endpoint keeps the cell's `Arc`s and the
    // stale mirror answers `Err(Lifecycle)` — `out` unchanged.
    host.execute(HostCommand::StopProcessing { handle })
        .unwrap();
    host.execute(HostCommand::Deactivate { handle }).unwrap();
    host.execute(HostCommand::Destroy { handle }).unwrap();
    drive(&mut slab, 0.5);
    assert!(all_eq(&slab, 0.5));
    // The recorded per-call Results pin the doc's output rows —
    // `Processing` blocks answer `Ok`, non-`Processing` blocks
    // `Err(Lifecycle)` and leave `out` to the bypass.
    assert!(matches!(
        results.lock().unwrap().as_slice(),
        [
            Ok(_),
            Err(HostError::Lifecycle(_)),
            Err(HostError::Lifecycle(_)),
            Ok(_),
            Err(HostError::Lifecycle(_)),
        ]
    ));
    drop(session);
}

/// Graph-attached verbs reach the plugin while the endpoint is
/// attached: `SaveState`/`RestoreState`/`StopProcessing` (there is
/// no param verb on the v1 command surface) all answer `Ok` — the
/// endpoint's `try_lock` may bypass during a verb's critical
/// section but the graph never hangs.
#[test]
fn l0_graph_attached_verbs_answer() {
    let (mut host, handle) = l0_processing(Box::new(GainFactory::new(2.0)));
    let endpoint = host.prepare_endpoint(handle, bounds(512, 2)).unwrap();
    let mut graph = Graph::new();
    graph.push(Box::new(source_node(endpoint)));
    let (enumerator, slot) = MockEnumerator::new();
    let device = enumerator.default_output_device().unwrap().unwrap();
    let session = Session::open(&*device, stream_config(), GraphPlan::new(graph)).unwrap();
    let mut cb = take_callback(&slot);
    let mut slab = [0.5_f32; 16];

    let saved = host
        .execute(HostCommand::SaveState { handle })
        .expect("SaveState reaches the attached instance");
    let HostReply::State { bytes } = saved else {
        panic!("SaveState replies State");
    };
    assert_eq!(bytes, 2.0_f32.to_le_bytes());
    host.execute(HostCommand::RestoreState { handle, bytes })
        .unwrap();
    host.execute(HostCommand::StopProcessing { handle })
        .unwrap();
    host.execute(HostCommand::StartProcessing { handle })
        .unwrap();
    // The graph still drives — defined output, no hang.
    cb(
        AudioBufferViewMut::try_new(&mut slab, 2, 8).unwrap(),
        info(),
    );
    drop(session);
}

// -------------------------------------------------------------------
// Bound-clip + adapter conventions — the `Err ⇒ out unchanged` rows.
// -------------------------------------------------------------------

/// Transform position on a bound-clip violation: `channels !=
/// bound.channels` and `frames > bound.max_frames` both yield
/// `Err(Prepare(UnsupportedConfig))` with `out` untouched — the
/// upstream bypass.
#[test]
fn bound_clip_err_leaves_out_unchanged() {
    let (mut host, handle) = l0_processing(Box::new(GainFactory::new(2.0)));
    let endpoint = host.prepare_endpoint(handle, bounds(512, 2)).unwrap();
    let results = Arc::new(Mutex::new(Vec::new()));
    let mut graph = Graph::new();
    graph.push(Box::new(transform_node(endpoint, Arc::clone(&results))));
    let (enumerator, slot) = MockEnumerator::new();
    let device = enumerator.default_output_device().unwrap().unwrap();
    let session = Session::open(&*device, stream_config(), GraphPlan::new(graph)).unwrap();
    let mut cb = take_callback(&slot);

    // Mono `out` against a stereo bound.
    let mut mono = [9.0_f32; 8];
    cb(
        AudioBufferViewMut::try_new(&mut mono, 1, 8).unwrap(),
        info(),
    );
    assert!(all_eq(&mono, 9.0));
    // 1024-frame `out` over the 512 bound.
    let mut oversized = [9.0_f32; 2048];
    cb(
        AudioBufferViewMut::try_new(&mut oversized, 2, 1024).unwrap(),
        info(),
    );
    assert!(all_eq(&oversized, 9.0));
    assert_eq!(
        results.lock().unwrap().as_slice(),
        &[
            Err(HostError::Prepare(
                ruviform_plugin_core::PrepareError::UnsupportedConfig
            )),
            Err(HostError::Prepare(
                ruviform_plugin_core::PrepareError::UnsupportedConfig
            )),
        ]
    );
    drop(session);
}

/// Source position: the closure's `fill(0.0)` convention turns a
/// bound-clip `Err` into silence — never stale upstream contents.
#[test]
fn source_position_err_yields_silence_not_stale() {
    let (mut host, handle) = l0_processing(Box::new(GainFactory::new(2.0)));
    let endpoint = host.prepare_endpoint(handle, bounds(512, 2)).unwrap();
    let mut graph = Graph::new();
    graph.push(Box::new(source_node(endpoint)));
    let (enumerator, slot) = MockEnumerator::new();
    let device = enumerator.default_output_device().unwrap().unwrap();
    let session = Session::open(&*device, stream_config(), GraphPlan::new(graph)).unwrap();
    let mut cb = take_callback(&slot);

    // Channels mismatch: stale 9.0 upstream is zeroed by the
    // convention, then the `Err` leaves it silent.
    let mut mono = [9.0_f32; 8];
    cb(
        AudioBufferViewMut::try_new(&mut mono, 1, 8).unwrap(),
        info(),
    );
    assert!(all_eq(&mono, 0.0));
    drop(session);
}

// -------------------------------------------------------------------
// Plan-swap retirement — the endpoint-bearing plan's destructor runs
// on the non-RT drain, never inside a callback.
// -------------------------------------------------------------------

/// `Session::swap` displaces the endpoint-bearing plan; the node's
/// drop probe stays unset through the RT `active()` pickup and arms
/// only on `Session::drain_retired` — `retired_overflows` stays 0,
/// and the freed L0 epoch is attachable again.
#[test]
fn plan_swap_retires_endpoint_node_on_drain() {
    // The plugin-side probe rides the `SendCore` cell — it fires
    // only once every `Arc` handle (retired endpoint and host
    // slot alike) is gone, so it pins *where* the teardown chain
    // ends: never the RT pull path.
    let plugin_dropped = Arc::new(AtomicBool::new(false));
    let mut factory = GainFactory::new(2.0);
    factory.dropped = Some(Arc::clone(&plugin_dropped));
    let (mut host, handle) = l0_processing(Box::new(factory));
    let endpoint = host.prepare_endpoint(handle, bounds(512, 2)).unwrap();
    let dropped = Arc::new(AtomicBool::new(false));
    let mut graph = Graph::new();
    graph.push(Box::new(EndpointNode {
        endpoint,
        dropped: Arc::clone(&dropped),
    }));
    let (enumerator, slot) = MockEnumerator::new();
    let device = enumerator.default_output_device().unwrap().unwrap();
    let session = Session::open(&*device, stream_config(), GraphPlan::new(graph)).unwrap();
    let mut cb = take_callback(&slot);
    let mut slab = [0.5_f32; 16];
    cb(
        AudioBufferViewMut::try_new(&mut slab, 2, 8).unwrap(),
        info(),
    );
    assert!(all_eq(&slab, 1.0));

    // Swap the endpoint node out — the "node removal" case is the
    // same plan-swap mechanism (the v1 graph has no per-node
    // removal API).
    session.swap(GraphPlan::new(Graph::new()));
    slab.fill(0.5);
    cb(
        AudioBufferViewMut::try_new(&mut slab, 2, 8).unwrap(),
        info(),
    );
    // `active()` displaced + retired the old plan — queued only:
    // no destructor ran inside the callback.
    assert!(!dropped.load(Ordering::SeqCst));
    assert!(all_eq(&slab, 0.5), "the empty plan bypasses");
    // The non-RT drain runs the destructor chain — node, endpoint,
    // epoch CAS.
    assert_eq!(session.drain_retired(), 1);
    assert!(dropped.load(Ordering::SeqCst));
    assert_eq!(session.retired_overflows(), 0);
    // The drain retired only the endpoint's `SendCore` handle —
    // the host slot still shares the cell, so the plugin probe
    // stays unset.
    assert!(!plugin_dropped.load(Ordering::SeqCst));
    // The L0 epoch returned: a fresh attach succeeds.
    let again = host.prepare_endpoint(handle, bounds(512, 2)).unwrap();
    drop(again);
    // Host-slot teardown releases the last `SendCore` handle —
    // the plugin probe fires on the non-RT command path, still
    // never inside a callback.
    host.execute(HostCommand::StopProcessing { handle })
        .unwrap();
    host.execute(HostCommand::Deactivate { handle }).unwrap();
    host.execute(HostCommand::Destroy { handle }).unwrap();
    assert!(plugin_dropped.load(Ordering::SeqCst));
    drop(session);
}

/// Teardown retirement: `Drop for RtPlan` (inside the captured
/// callback) retires the cached plan; the collector's own drop at
/// `Session` teardown runs the queued destructors on this thread —
/// no explicit drain needed.
#[test]
fn session_teardown_collects_retired_plan() {
    let (mut host, handle) = l0_processing(Box::new(GainFactory::new(2.0)));
    let endpoint = host.prepare_endpoint(handle, bounds(512, 2)).unwrap();
    let dropped = Arc::new(AtomicBool::new(false));
    let mut graph = Graph::new();
    graph.push(Box::new(EndpointNode {
        endpoint,
        dropped: Arc::clone(&dropped),
    }));
    let (enumerator, slot) = MockEnumerator::new();
    let device = enumerator.default_output_device().unwrap().unwrap();
    let session = Session::open(&*device, stream_config(), GraphPlan::new(graph)).unwrap();
    let mut cb = take_callback(&slot);
    let mut slab = [0.0_f32; 16];
    cb(
        AudioBufferViewMut::try_new(&mut slab, 2, 8).unwrap(),
        info(),
    );
    // The callback (and its `RtPlan`) drops before the session —
    // `Drop for RtPlan` retires the cached plan, then the
    // collector's teardown drop runs the destructors.
    drop(cb);
    drop(session);
    assert!(dropped.load(Ordering::SeqCst));
}

/// L1 leg return through retirement: the drained plan drops the
/// endpoint, whose `Drop` pushes the checked-out `RtLeg` onto the
/// return channel; the next `&mut` host entry point restores it and
/// a fresh `prepare_endpoint` succeeds.
#[test]
fn l1_leg_returns_through_retired_plan_drain() {
    let (mut host, handle) = l1_processing();
    let endpoint = host.prepare_endpoint(handle, bounds(512, 2)).unwrap();
    let dropped = Arc::new(AtomicBool::new(false));
    let mut graph = Graph::new();
    graph.push(Box::new(EndpointNode {
        endpoint,
        dropped: Arc::clone(&dropped),
    }));
    let (enumerator, slot) = MockEnumerator::new();
    let device = enumerator.default_output_device().unwrap().unwrap();
    let session = Session::open(&*device, stream_config(), GraphPlan::new(graph)).unwrap();
    let mut cb = take_callback(&slot);
    let mut slab = [0.5_f32; 16];
    cb(
        AudioBufferViewMut::try_new(&mut slab, 2, 8).unwrap(),
        info(),
    );
    // While the leg is checked out a second attach is the defined
    // occupancy rejection.
    assert_eq!(
        host.prepare_endpoint(handle, bounds(512, 2)).err(),
        Some(HostError::EndpointUnavailable)
    );

    session.swap(GraphPlan::new(Graph::new()));
    cb(
        AudioBufferViewMut::try_new(&mut slab, 2, 8).unwrap(),
        info(),
    );
    assert!(!dropped.load(Ordering::SeqCst));
    // The drain drops the endpoint → `return_tx` queues the leg →
    // `prepare_endpoint`'s own `&mut` drain restores it.
    assert_eq!(session.drain_retired(), 1);
    assert!(dropped.load(Ordering::SeqCst));
    assert_eq!(session.retired_overflows(), 0);
    let endpoint = host
        .prepare_endpoint(handle, bounds(512, 2))
        .expect("the returned leg re-attaches");
    assert_eq!(endpoint.latency_frames(), 1024);
    drop(session);
    // `Destroy` from `Processing` is an illegal transition — the
    // clean path stops and deactivates first.
    host.execute(HostCommand::StopProcessing { handle })
        .unwrap();
    host.execute(HostCommand::Deactivate { handle }).unwrap();
    host.execute(HostCommand::Destroy { handle }).unwrap();
}

// -------------------------------------------------------------------
// L1 graph E2E — the real child process behind the leg.
// -------------------------------------------------------------------

/// `l1-test-worker` (gain ×0.5) through the full public route:
/// `register_out_of_process` → `Instantiate` → lifecycle →
/// `prepare_endpoint` → transform node → `Session::open` → paced
/// callback blocks. Latency reports `lookahead_blocks × activated
/// max_frames` (2 × 512 = 1024); early blocks pass through silence
/// and every output sample is a defined value — gained input or
/// silence, never torn data.
#[test]
fn l1_endpoint_node_drives_worker_gain_through_session() {
    let (mut host, handle) = l1_processing();
    let endpoint = host.prepare_endpoint(handle, bounds(512, 2)).unwrap();
    assert_eq!(endpoint.bounds(), bounds(512, 2));
    // `lookahead_blocks (2) × activated max_frames (512)`.
    assert_eq!(endpoint.latency_frames(), 1024);
    assert_eq!(host.latency_frames(handle), Some(1024));

    let results = Arc::new(Mutex::new(Vec::new()));
    let mut graph = Graph::new();
    graph.push(Box::new(transform_node(endpoint, Arc::clone(&results))));
    let (enumerator, slot) = MockEnumerator::new();
    let device = enumerator.default_output_device().unwrap().unwrap();
    let session = Session::open(&*device, stream_config(), GraphPlan::new(graph)).unwrap();
    let mut cb = take_callback(&slot);

    let mut inputs: Vec<f32> = Vec::new();
    let mut matched = 0_u32;
    let mut slab = [0.0_f32; 16];
    for call in 0..32_u32 {
        let value = 0.2 + 0.01 * f32::from(u8::try_from(call).unwrap() + 1);
        // A real-time-scale pace between doorbells — the worker
        // stays comfortably ahead of the 2-block lookahead.
        thread::sleep(Duration::from_millis(5));
        slab.fill(value);
        cb(
            AudioBufferViewMut::try_new(&mut slab, 2, 8).unwrap(),
            info(),
        );
        if call >= 2 {
            let expected = 0.5 * inputs[call as usize - 2];
            if all_eq(&slab, expected) {
                matched += 1;
            } else {
                // Underrun/pass-through — still only defined
                // values (silence or an older gained block).
                assert!(
                    slab.iter().all(|&s| is_defined_l1_sample(s, &inputs)),
                    "call {call}: undefined sample {}",
                    slab[0]
                );
            }
        } else {
            // `seq - lookahead` underflow — defined silence.
            assert!(all_eq(&slab, 0.0));
        }
        inputs.push(value);
    }
    // The pipeline must actually deliver — generous headroom for
    // loaded CI hosts (mirrors `l1_proxy_e2e`'s bound).
    assert!(
        matched >= 16,
        "the lookahead pipeline never filled — {matched}"
    );
    assert!(
        results
            .lock()
            .unwrap()
            .iter()
            .all(|r| *r == Ok(ProcessResult::CONTINUE)),
        "every L1 block answers Ok(CONTINUE)"
    );

    // `Destroy` is legal from `Processing` only via the lifecycle —
    // stop first, then the clean teardown.
    host.execute(HostCommand::StopProcessing { handle })
        .unwrap();
    host.execute(HostCommand::Deactivate { handle }).unwrap();
    host.execute(HostCommand::Destroy { handle }).unwrap();
    drop(session);
}

/// `kill_child` converges the instance `Failed`: the
/// `InstanceFailed` notification arrives exactly once, the
/// endpoint's blocks emit defined silence (`Ok(CONTINUE)` — never
/// an `Err`, never a hang), and post-`Failed` commands reject
/// `Err(Failed)` — `Destroy` still tears down cleanly.
#[test]
fn l1_kill_child_converges_failed_once_then_silence() {
    let (mut host, handle) = l1_processing();
    let endpoint = host.prepare_endpoint(handle, bounds(512, 2)).unwrap();
    let results = Arc::new(Mutex::new(Vec::new()));
    let mut graph = Graph::new();
    graph.push(Box::new(transform_node(endpoint, Arc::clone(&results))));
    let (enumerator, slot) = MockEnumerator::new();
    let device = enumerator.default_output_device().unwrap().unwrap();
    let session = Session::open(&*device, stream_config(), GraphPlan::new(graph)).unwrap();
    let mut cb = take_callback(&slot);
    let mut slab = [0.0_f32; 16];
    // A few live blocks first — defined outputs only.
    for _ in 0..4 {
        thread::sleep(Duration::from_millis(5));
        slab.fill(1.0);
        cb(
            AudioBufferViewMut::try_new(&mut slab, 2, 8).unwrap(),
            info(),
        );
        assert!(slab.iter().all(|&s| sample_eq(s, 0.0) || sample_eq(s, 0.5)));
    }

    host.kill_child(handle).unwrap();
    wait_for_failed(&host, handle);
    // `Failed` convergence: every block is `Ok(CONTINUE)` + silence.
    for _ in 0..8 {
        slab.fill(9.0);
        cb(
            AudioBufferViewMut::try_new(&mut slab, 2, 8).unwrap(),
            info(),
        );
        assert!(all_eq(&slab, 0.0), "a Failed instance must output silence");
    }
    // The recorded per-call Results pin the doc claim — `Failed`
    // convergence is a defined output row, delivered as `Ok`,
    // never an `Err`.
    assert!(
        results.lock().unwrap().iter().all(Result::is_ok),
        "every recorded block answered Ok, never Err"
    );
    // The notification arrives exactly once — the `&mut` drain
    // inside `try_recv_notification` pulls the driver's child-exit
    // observation forward.
    assert_eq!(wait_instance_failed(&mut host), handle);
    assert!(host.try_recv_notification().is_none());
    // Post-`Failed` commands (except `Destroy`) are `Err(Failed)`.
    assert_eq!(
        host.execute(HostCommand::SaveState { handle }),
        Err(HostError::Failed)
    );
    host.execute(HostCommand::Destroy { handle }).unwrap();
    drop(session);
}

// -------------------------------------------------------------------
// rt_ lane — `process_audio` under the graph pull path, alloc-free.
// -------------------------------------------------------------------

/// Issue #22 lane: the whole RT block path — `RtPlan::active` →
/// `GraphPlan::drive` → `Graph::pull` → the endpoint-capturing
/// `FnMut` node → `process_audio` — allocates nothing. Fixture,
/// `RtContext`, `PlanSwap` and warm-up sit outside the window.
#[test]
fn rt_endpoint_graph_pull_no_alloc() {
    let (mut host, handle) = l0_processing(Box::new(GainFactory::new(2.0)));
    let endpoint = host.prepare_endpoint(handle, bounds(512, 2)).unwrap();
    let mut graph = Graph::new();
    graph.push(Box::new(source_node(endpoint)));
    let (_swap, mut rt_plan) = PlanSwap::new(GraphPlan::new(graph));
    let ctx = RtContext::mint();
    let mut slab = [0.0_f32; 16];
    // Warm-up outside the window — first `active` read, first pull,
    // first `catch_unwind`.
    rt_plan.active().unwrap().drive(
        &mut AudioBufferViewMut::try_new(&mut slab, 2, 8).unwrap(),
        &ctx,
    );
    assert_no_alloc::assert_no_alloc(|| {
        for _ in 0..64 {
            let plan = rt_plan.active().unwrap();
            plan.drive(
                &mut AudioBufferViewMut::try_new(&mut slab, 2, 8).unwrap(),
                &ctx,
            );
        }
    });
}
