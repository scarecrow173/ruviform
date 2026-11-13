//! Host-side CLAP bridge (`feature = "host"`): loads a real `.clap`
//! shared object in-process via `clack-host` and maps its descriptors,
//! parameters and lifecycle onto the neutral
//! [`PluginFactory`]/[`Plugin`] contracts for the L1 out-of-process
//! runner (Issue #114).
//!
//! ## Lifecycle mapping (neutral → CLAP)
//!
//! | Neutral verb | CLAP call |
//! | --- | --- |
//! | [`Plugin::init`] | `PluginInstance::new` (fused `create_plugin` + `plugin.init`) |
//! | [`PluginProcessor::prepare`] | `PluginInstance::activate` |
//! | [`Plugin::start_processing`] | `start_processing` (`Stopped` → `Started`) |
//! | [`PluginProcessor::process`] | `StartedPluginAudioProcessor::process` |
//! | [`Plugin::stop_processing`] | `stop_processing` (`Started` → `Stopped`) |
//! | `Drop` | best-effort `stop_processing` → `deactivate` → `plugin.destroy` |
//!
//! Teardown is `Drop`-driven: the neutral `Plugin` surface has no
//! `Deactivate`/`Destroy`/`Shutdown` hook (the export Runner owns those
//! lifecycle transitions itself), so the symmetric CLAP teardown runs
//! in `ClapPlugin::drop`.
//!
//! ## `'static` metadata (bounded intern-once)
//!
//! [`PluginDescriptor`] and [`ParamInfo`] fields are `&'static`, but
//! scanned plugin metadata arrives as runtime `String`/`Vec`. The
//! bridge materializes `'static` storage with `String::leak`/`Vec::leak`
//! **only** at `ClapLibrary::load` scan time (descriptors and
//! per-plugin param tables — one throwaway instance per descriptor,
//! once per load). This is the
//! sanctioned exception to plugin-core's no-forged-statics rule: the
//! leak is *bounded* (~5 + N strings per descriptor, 2 per param),
//! *once-per-load*, *process-scoped* (the L1 runner is a short-lived
//! child process reclaimed on exit), and *feature-sealed* inside
//! `host`. `descriptors()`/`descriptor()`/`params()` only ever return
//! these cached values — no per-call growth.
//!
//! ## RT path (`process`)
//!
//! `process` performs no allocation: every buffer the CLAP call needs
//! is pre-sized in `prepare`.
//!
//! - **Input bounce** — `HostInput` yields `&[f32]` per channel, but
//!   `clap_audio_buffer` input channels are `*mut` (a plugin may write
//!   its input in place). Feeding immutable input through
//!   `InputAudioBuffers::from_raw_buffers` as a fake `*mut` is
//!   UB-adjacent and rejected: the bridge copies host input into an
//!   owned scratch (`declared channels × max_frames`, allocated at
//!   `prepare`) and hands the plugin the scratch. An unconnected
//!   input channel delivers silence.
//! - **Output bounce** — `HostOutput::channel_mut` could in principle
//!   alias `OutputAudioBuffers` directly, but a `&mut dyn BufferViewMut`
//!   yields only one `&mut [f32]` per live borrow, so wiring more than
//!   one channel per bus needs raw-pointer assembly (`unsafe`). The
//!   bridge keeps `PluginEntry::load` as its single `unsafe` boundary
//!   and bounces outputs through scratch, copying back after the call;
//!   an unconnected output channel is dropped by definition.
//! - **Events** — `EventKind::ParamValue` maps to `ParamValueEvent`
//!   (normalized → plain via [`ParamInfo::denormalize`]) pushed into a
//!   persistent [`EventBuffer`], bounded at `MAX_EVENTS_PER_BLOCK`;
//!   the plugin's output events go to [`OutputEvents::void`] —
//!   discarded, not mapped (v1), in a sink the plugin cannot force to
//!   allocate on the RT path.
//! - **`steady_time`/`transport`** — always `None`/`None`: the neutral
//!   `TransportInfo` snapshot cannot honestly derive the beat-domain
//!   loop/tempo fields `clap_event_transport` requires (no tempo map
//!   is kept), so v1 declares the explicit free-running boundary
//!   instead of a zero-stuffed partial projection.
//!
//! ## Thread-check policy
//!
//! `BridgeHandlers::declare_extensions` registers **no** host
//! extensions — in particular not `HostThreadCheck`. The runner drives
//! `process` from a single command thread that is not an RT audio
//! thread, so answering `is_audio_thread = true` would be a false
//! capability claim; the fixture plugin resolves `thread-check` at
//! `plugin.init` and falls back to no check when the extension is
//! absent (`plugin.rs` `GainShared::initialize_with_host`).
//!
//! ## Defined boundaries (v1)
//!
//! - Load/scan/instantiate/lifecycle failures converge to
//!   `Err(ClapHostError)`/`Err(HostError)`/`Err(PrepareError)` — never
//!   a panic.
//! - `save_state`/`restore_state`/`open_gui`/`close_gui` keep the
//!   [`Plugin`] defaults (`Err(HostError::Unsupported)`) — the bridge
//!   never calls a standard state or GUI verb.
//! - `create_sendable` keeps its `None` default — `PluginInstance` is
//!   `!Send`/`!Sync` (`PhantomData<*const ()>`), so no `Send` instance
//!   can be honestly produced.
//! - A block whose declared length falls outside the negotiated
//!   `[min_frames, max_frames]` bound — `0 < min` included — or
//!   `process` outside the started state, answers
//!   `ProcessStatus::Error`.
//! - A wired channel whose slice length differs from the declared
//!   block length violates the neutral equal-length contract — a
//!   caller bug answered `ProcessStatus::Error`, never silently
//!   clamped.
//! - `steady_time`/`transport` are always `None`/`None` — the explicit
//!   free-running boundary in the `process` section above.
//! - Plugin-reported enumeration counts (descriptors, params, audio
//!   ports, per-port channels) are clamped to `MAX_SCAN_*` caps before
//!   they can size an allocation or a scan loop.

use std::ffi::{CStr, CString};
use std::fmt;
use std::path::Path;

use clack_common::events::Pckn;
use clack_common::events::event_types::ParamValueEvent;
use clack_common::events::io::{EventBuffer, OutputEvents};
use clack_common::process::ProcessStatus as ClapProcessStatus;
use clack_common::utils::ClapId;
use clack_extensions::audio_ports::{AudioPortInfoBuffer, PluginAudioPorts};
use clack_extensions::params::{ParamInfoBuffer, ParamInfoFlags, PluginParams};
use clack_host::entry::{PluginEntry, PluginEntryError};
use clack_host::host::{HostExtensions, HostHandlers, HostInfo, SharedHandler};
use clack_host::plugin::{
    PluginDescriptor as ClapPluginDescriptor, PluginInstance, PluginMainThreadHandle,
};
use clack_host::process::audio_buffers::{
    AudioPortBuffer, AudioPortBufferType, AudioPorts, InputChannel,
};
use clack_host::process::{PluginAudioConfiguration, PluginAudioProcessor};

use ruviform_plugin_core::wire::WIRE_MAX_FRAMES;
use ruviform_plugin_core::{
    BufferView, BufferViewMut, ErasedPlugin, EventKind, EventList, HostError, ParamFlags, ParamId,
    ParamInfo, Plugin, PluginDescriptor, PluginFactory, PluginProcessor, PrepareError,
    ProcessConfig, ProcessContext, ProcessResult, ProcessStatus, TransportInfo, erase,
};

/// Upper bound of param events forwarded per block — the same scale as
/// the plugin-side adapter's `MAX_EVENTS_PER_BLOCK`. Excess events are
/// dropped by definition rather than growing the persistent
/// [`EventBuffer`] on the RT path.
const MAX_EVENTS_PER_BLOCK: usize = 256;

/// Defensive caps on plugin-reported enumeration counts. `count()` /
/// `channel_count` come from loaded plugin code — untrusted: a garbage
/// value would turn `Vec::with_capacity`/`vec![…]` into an abort, so
/// every scan-side allocation and `get_info` loop clamps first. The
/// magnitudes sit well above any honest plugin (a factory advertising
/// more than this keeps only its leading entries — the same
/// truncation the wire protocol's own `WIRE_MAX_*` ceilings impose
/// downstream).
const MAX_SCAN_DESCRIPTORS: u32 = 1_024;
/// See [`MAX_SCAN_DESCRIPTORS`].
const MAX_SCAN_PARAMS: u32 = 4_096;
/// See [`MAX_SCAN_DESCRIPTORS`].
const MAX_SCAN_PORTS: u32 = 64;
/// See [`MAX_SCAN_DESCRIPTORS`].
const MAX_SCAN_CHANNELS: u32 = 64;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Why loading or driving a CLAP plugin through the bridge failed.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy — new variants may be added without breaking downstream
/// `match` arms.
///
/// **RT contract:** non-RT only — load/scan/instantiate errors.
#[non_exhaustive]
#[derive(Debug)]
pub enum ClapHostError {
    /// `PluginEntry::load` failed — dlopen error, missing or
    /// incompatible `clap_entry`, or entry init failure. Carries the
    /// underlying error erased as `Box<dyn Error>`: diagnostics survive
    /// through `Display`/`source()` while no binding-crate type appears
    /// in the public signature (crate policy).
    Entry(Box<dyn std::error::Error + Send + Sync + 'static>),
    /// The entry does not expose a `clap_plugin_factory` — a
    /// misbehaving or non-plugin binary.
    MissingPluginFactory,
    /// A scanned descriptor lacks its mandatory `id` field — the
    /// plugin cannot be addressed for instantiation.
    MissingPluginId,
    /// Instantiation or a lifecycle call into the plugin failed —
    /// same erased payload as [`Self::Entry`].
    Instance(Box<dyn std::error::Error + Send + Sync + 'static>),
}

impl ClapHostError {
    /// Erases a `clack` entry error into the opaque [`Self::Entry`]
    /// payload — conversion lives on the crate side so the public
    /// surface never names a binding-crate type (no public
    /// `From<PluginEntryError>` impl).
    fn entry(error: PluginEntryError) -> Self {
        Self::Entry(Box::new(error))
    }
}

impl fmt::Display for ClapHostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Entry(error) => write!(f, "CLAP entry load failed: {error}"),
            Self::MissingPluginFactory => f.write_str("entry exposes no clap_plugin_factory"),
            Self::MissingPluginId => f.write_str("scanned descriptor lacks a plugin id"),
            Self::Instance(error) => write!(f, "CLAP instance error: {error}"),
        }
    }
}

impl std::error::Error for ClapHostError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Entry(error) | Self::Instance(error) => Some(&**error),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Bounded intern-once helpers
// ---------------------------------------------------------------------------

/// Materializes a `&'static str` for one scanned string — see the
/// module-level intern-once policy for why this leak is sanctioned.
/// Empty input returns the `'static` literal without leaking.
fn intern_str(text: &str) -> &'static str {
    if text.is_empty() {
        ""
    } else {
        String::leak(text.to_owned())
    }
}

/// `intern_str` over a `&CStr` — lossy UTF-8, bounded once.
fn intern_cstr(text: &CStr) -> &'static str {
    intern_str(&text.to_string_lossy())
}

/// `intern_str` over a `&[u8]` (CLAP metadata byte fields) — lossy
/// UTF-8, bounded once.
fn intern_bytes(bytes: &[u8]) -> &'static str {
    intern_str(&String::from_utf8_lossy(bytes))
}

/// Maps one scanned `clap_plugin_descriptor` to the neutral
/// [`PluginDescriptor`], interning every field once. `features` keep
/// their declared order.
fn intern_descriptor(raw: &ClapPluginDescriptor, id: &CStr) -> PluginDescriptor {
    let features: Vec<&'static str> = raw.features().map(intern_cstr).collect();
    PluginDescriptor {
        id: intern_cstr(id),
        name: intern_cstr(raw.name().unwrap_or(c"")),
        vendor: intern_cstr(raw.vendor().unwrap_or(c"")),
        version: intern_cstr(raw.version().unwrap_or(c"")),
        features: if features.is_empty() {
            &[]
        } else {
            features.leak()
        },
    }
}

// ---------------------------------------------------------------------------
// Host callbacks
// ---------------------------------------------------------------------------

/// The bridge's `[thread-safe]` host handler — acknowledges the
/// plugin's restart/process/callback requests without acting on them;
/// the runner drives everything from its single command thread.
struct BridgeShared;

impl SharedHandler<'_> for BridgeShared {
    fn request_restart(&self) {}
    fn request_process(&self) {}
    fn request_callback(&self) {}
}

/// The bridge's `[audio-thread]` host handler — a `Send` marker; the
/// fixture's `process` path queries nothing host-side beyond the
/// (unregistered) `thread-check` extension.
struct BridgeAudioProcessor;

impl clack_host::host::AudioProcessorHandler<'_> for BridgeAudioProcessor {}

/// Handler-type bundle tying `BridgeShared`/`()`/`BridgeAudioProcessor`
/// together for [`PluginInstance`].
struct BridgeHandlers;

impl HostHandlers for BridgeHandlers {
    type Shared<'a> = BridgeShared;
    type MainThread<'a> = ();
    type AudioProcessor<'a> = BridgeAudioProcessor;

    fn declare_extensions(_builder: &mut HostExtensions<Self>, _shared: &BridgeShared) {
        // No host extensions — notably not `HostThreadCheck`. See the
        // module-level thread-check policy: absence of the extension is
        // the honest answer, and the fixture's `None` fallback keeps
        // behavior unchanged.
    }
}

/// The host identity presented to loaded plugins.
fn bridge_host_info() -> HostInfo {
    // `HostInfo::new` fails only on interior NULs — every argument is a
    // NUL-free literal or `CARGO_PKG_VERSION` (NUL-free by cargo).
    HostInfo::new(
        "ruviform-clap-host",
        "Ruviform",
        "",
        env!("CARGO_PKG_VERSION"),
    )
    .expect("host info fields are NUL-free")
}

// ---------------------------------------------------------------------------
// Library / descriptor scan
// ---------------------------------------------------------------------------

/// A loaded `.clap` shared object: wraps [`PluginEntry`] and carries
/// the load-time descriptor scan (interned once — see module docs).
///
/// The entry handle is `Arc`-backed; factories and instances clone it
/// so the library stays mapped for as long as anything derived from it
/// lives (clack keeps the provider alive per instance).
pub struct ClapLibrary {
    entry: PluginEntry,
    host_info: HostInfo,
    /// Index-aligned with `descriptors` — the instantiation id for each
    /// scanned plugin.
    plugin_ids: Box<[CString]>,
    /// Cached, interned-once descriptors — `descriptors()` returns this
    /// slice verbatim.
    descriptors: Vec<PluginDescriptor>,
    /// Index-aligned with `descriptors` — the interned param table per
    /// scanned plugin, enumerated once at load (see `enumerate_params`
    /// for why discovery needs a throwaway instance).
    params_tables: Vec<&'static [ParamInfo]>,
}

impl ClapLibrary {
    /// Loads a `.clap` shared object and performs the descriptor scan.
    ///
    /// **RT contract:** non-RT only — dlopen, entry init, factory scan.
    ///
    /// Note: on macOS `PluginEntry::load` performs `.clap` *bundle*
    /// resolution, while the fixture/runner convention resolves the
    /// raw `.dylib` — that path is exercised on Windows/Linux and
    /// remains unverified on macOS.
    ///
    /// # Errors
    ///
    /// [`ClapHostError::Entry`] when the file cannot be loaded or its
    /// `clap_entry` is missing/incompatible;
    /// [`ClapHostError::MissingPluginFactory`] when the entry exposes
    /// no plugin factory; [`ClapHostError::MissingPluginId`] when a
    /// scanned descriptor has no `id`.
    #[expect(unsafe_code)]
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ClapHostError> {
        // SAFETY: `PluginEntry::load` maps an external shared object
        // into this process — loading arbitrary plugin code is the
        // caller's contract (the same boundary as the OFX adapter's
        // `load_library`). The resulting entry is only exercised
        // through clack's checked APIs, and `host` keeps this the
        // crate's single unsafe boundary.
        let entry = unsafe { PluginEntry::load(path.as_ref()) }.map_err(ClapHostError::entry)?;
        Self::from_entry(entry)
    }

    /// Descriptor + params scan shared by `load` — queries the
    /// entry's `clap_plugin_factory` once, interns every returned
    /// `clap_plugin_descriptor`, and enumerates each plugin's param
    /// table here (once per load — `factories()` only reads the
    /// cache, so repeated calls neither re-instantiate a throwaway
    /// plugin nor leak a fresh table).
    fn from_entry(entry: PluginEntry) -> Result<Self, ClapHostError> {
        let factory = entry
            .get_plugin_factory()
            .ok_or(ClapHostError::MissingPluginFactory)?;

        let host_info = bridge_host_info();
        let mut plugin_ids = Vec::new();
        let mut descriptors = Vec::new();
        let mut params_tables = Vec::new();
        // `plugin_count` is plugin-reported — `.take` bounds the scan
        // (and the per-descriptor intern/instantiate cost) before it
        // can run away.
        for raw in factory
            .plugin_descriptors()
            .take(usize::try_from(MAX_SCAN_DESCRIPTORS).unwrap_or(usize::MAX))
        {
            let id = raw.id().ok_or(ClapHostError::MissingPluginId)?;
            plugin_ids.push(id.to_owned());
            descriptors.push(intern_descriptor(raw, id));
            params_tables.push(enumerate_params(&entry, id, &host_info));
        }
        Ok(Self {
            entry,
            host_info,
            plugin_ids: plugin_ids.into_boxed_slice(),
            descriptors,
            params_tables,
        })
    }

    /// The descriptors scanned at load time, in declaration order —
    /// the cached, interned slice (identical on every call; no re-scan
    /// or per-call leak growth).
    ///
    /// **RT contract:** non-RT only — borrowed cached slice.
    #[must_use]
    pub fn descriptors(&self) -> &[PluginDescriptor] {
        &self.descriptors
    }

    /// One bridge factory per scanned descriptor. Param tables come
    /// from the load-time enumeration cache (index-aligned with
    /// `descriptors`) so [`Plugin::params`] works at every lifecycle
    /// point including pre-`init`.
    ///
    /// **RT contract:** non-RT only — allocates the factory vec; no
    /// plugin instantiation or metadata scan happens here.
    #[must_use]
    pub fn factories(&self) -> Vec<ClapPluginFactory> {
        self.descriptors
            .iter()
            .zip(self.plugin_ids.iter())
            .zip(self.params_tables.iter())
            .map(|((&descriptor, plugin_id), &params)| ClapPluginFactory {
                descriptor,
                params,
                entry: self.entry.clone(),
                plugin_id: plugin_id.clone(),
                host_info: self.host_info.clone(),
            })
            .collect()
    }
}

/// Enumerates the plugin's `params` extension into an interned neutral
/// [`ParamInfo`] table.
///
/// Params discovery needs a live instance (`clap_plugin_params` is an
/// instance extension), so a throwaway [`PluginInstance`] is created —
/// bounded once per descriptor at `ClapLibrary::load` and dropped
/// immediately. A plugin that cannot instantiate, or that lacks the
/// extension, gets an honest empty table; the real failure surfaces at
/// [`Plugin::init`] instead of being fabricated here.
///
/// `unit` is always `""` — `clap_param_info` has no unit field, so the
/// table carries the honest synthetic value rather than a filler.
fn enumerate_params(
    entry: &PluginEntry,
    plugin_id: &CStr,
    host_info: &HostInfo,
) -> &'static [ParamInfo] {
    let Ok(mut instance) = PluginInstance::<BridgeHandlers>::new(
        |()| BridgeShared,
        |_| (),
        entry,
        plugin_id,
        host_info,
    ) else {
        return &[];
    };
    let handle = instance.plugin_handle();
    let Some(params) = handle.get_extension::<PluginParams>() else {
        return &[];
    };
    // `count` is plugin-reported — clamp before it sizes the
    // allocation or the `get_info` scan loop.
    let count = params.count(&handle).min(MAX_SCAN_PARAMS);
    let mut infos = Vec::with_capacity(usize::try_from(count).unwrap_or(0));
    for index in 0..count {
        let mut buffer = ParamInfoBuffer::new();
        let Some(info) = params.get_info(&handle, index, &mut buffer) else {
            continue;
        };
        infos.push(ParamInfo {
            id: ParamId(info.id.get()),
            name: intern_bytes(info.name),
            unit: "",
            min_value: info.min_value,
            max_value: info.max_value,
            default_value: info.default_value,
            flags: map_param_flags(info.flags),
        });
    }
    infos.leak()
}

/// `clap_param_info.flags` → neutral [`ParamFlags`]. CLAP-only
/// refinements (per-channel/key/note-id/port variants, `IS_PERIODIC`,
/// `IS_ENUM`, `REQUIRES_PROCESS`) have no neutral counterpart and are
/// dropped — the v1 defined boundary.
fn map_param_flags(flags: ParamInfoFlags) -> ParamFlags {
    let mut mapped = ParamFlags::NONE;
    if flags.contains(ParamInfoFlags::IS_AUTOMATABLE) {
        mapped = mapped.union(ParamFlags::AUTOMATABLE);
    }
    if flags.contains(ParamInfoFlags::IS_MODULATABLE) {
        mapped = mapped.union(ParamFlags::MODULATABLE);
    }
    if flags.contains(ParamInfoFlags::IS_STEPPED) {
        mapped = mapped.union(ParamFlags::STEPPED);
    }
    if flags.contains(ParamInfoFlags::IS_HIDDEN) {
        mapped = mapped.union(ParamFlags::HIDDEN);
    }
    if flags.contains(ParamInfoFlags::IS_READONLY) {
        mapped = mapped.union(ParamFlags::READONLY);
    }
    if flags.contains(ParamInfoFlags::IS_BYPASS) {
        mapped = mapped.union(ParamFlags::BYPASS);
    }
    mapped
}

/// The declared channel count of every audio port in one direction —
/// empty for a plugin without the `audio-ports` extension.
fn port_channel_counts(handle: PluginMainThreadHandle<'_>, is_input: bool) -> Vec<u32> {
    let Some(ports) = handle.get_extension::<PluginAudioPorts>() else {
        return Vec::new();
    };
    // Plugin-reported counts are untrusted — clamp the port count and
    // each port's channel count before they size `prepare`'s scratch
    // (`MAX_SCAN_*` bounds both the alloc and this `get` loop).
    let count = ports.count(&handle, is_input).min(MAX_SCAN_PORTS);
    let mut channels = Vec::with_capacity(usize::try_from(count).unwrap_or(0));
    for index in 0..count {
        let mut buffer = AudioPortInfoBuffer::new();
        let channel_count = ports
            .get(&handle, index, is_input, &mut buffer)
            .map_or(0, |info| info.channel_count.min(MAX_SCAN_CHANNELS));
        channels.push(channel_count);
    }
    channels
}

// ---------------------------------------------------------------------------
// Factory
// ---------------------------------------------------------------------------

/// Bridge [`PluginFactory`] for one scanned descriptor — carries the
/// cached descriptor and interned param table from load/factory time.
///
/// **RT contract:** non-RT only — registration and instantiation.
pub struct ClapPluginFactory {
    descriptor: PluginDescriptor,
    params: &'static [ParamInfo],
    /// Shared handle on the loaded library — the instance created from
    /// this factory can never outlive its mapped code.
    entry: PluginEntry,
    plugin_id: CString,
    host_info: HostInfo,
}

impl PluginFactory for ClapPluginFactory {
    fn descriptor(&self) -> PluginDescriptor {
        self.descriptor
    }

    fn create(&self) -> Box<dyn ErasedPlugin> {
        erase(ClapPlugin::dormant(
            self.descriptor,
            self.params,
            self.entry.clone(),
            self.plugin_id.clone(),
            self.host_info.clone(),
        ))
    }

    // `create_sendable` keeps its `None` default — `PluginInstance` is
    // `!Send`, so a `Send` erased plugin cannot be honestly produced.
}

// ---------------------------------------------------------------------------
// Instance
// ---------------------------------------------------------------------------

/// Bridge [`Plugin`] over a dormant [`PluginInstance`] — `create()`
/// only wires identity; CLAP instantiate+`plugin.init` runs in `init`
/// per the lifecycle table in the module docs.
struct ClapPlugin {
    descriptor: PluginDescriptor,
    params: &'static [ParamInfo],
    host_info: HostInfo,
    entry: PluginEntry,
    plugin_id: CString,
    instance: Option<PluginInstance<BridgeHandlers>>,
    /// `Stopped`/`Started` processor produced by `activate` — `None`
    /// until `prepare`.
    processor: Option<PluginAudioProcessor<BridgeHandlers>>,
    /// Negotiated block-size bounds — `process` rejects a block whose
    /// declared length falls outside `[min_frames, max_frames]`, the
    /// must-bound the plugin agreed to at `activate`.
    min_frames: usize,
    max_frames: usize,
    /// Input bounce: `ports → channels → max_frames`, allocated at
    /// `prepare` (see module docs for why inputs cannot alias).
    input_scratch: Vec<Vec<Vec<f32>>>,
    /// Output bounce: same shape — copied back to `HostOutput` after
    /// each block (see module docs for why outputs cannot alias either).
    output_scratch: Vec<Vec<Vec<f32>>>,
    /// Persistent port-descriptor storage for `with_input_buffers` —
    /// Vec-backed, so it must live outside the RT call to avoid
    /// per-block reallocation.
    input_ports: AudioPorts,
    output_ports: AudioPorts,
    /// Persistent input event buffer — capacity-bounded at `prepare`,
    /// cleared per block. Plugin output events go to
    /// [`OutputEvents::void`] instead of a buffer: never read in v1,
    /// and a plugin cannot force a host `Vec` past capacity on the RT
    /// path.
    input_events: EventBuffer,
}

impl ClapPlugin {
    /// A dormant instance: identity wired, no CLAP instance yet (the
    /// fused instantiate+`plugin.init` belongs to `init`).
    fn dormant(
        descriptor: PluginDescriptor,
        params: &'static [ParamInfo],
        entry: PluginEntry,
        plugin_id: CString,
        host_info: HostInfo,
    ) -> Self {
        Self {
            descriptor,
            params,
            host_info,
            entry,
            plugin_id,
            instance: None,
            processor: None,
            min_frames: 0,
            max_frames: 0,
            input_scratch: Vec::new(),
            output_scratch: Vec::new(),
            input_ports: AudioPorts::with_capacity(0, 0),
            output_ports: AudioPorts::with_capacity(0, 0),
            input_events: EventBuffer::new(),
        }
    }
}

impl Drop for ClapPlugin {
    fn drop(&mut self) {
        // Symmetric teardown, best-effort: the neutral surface has no
        // deactivate/destroy hook, so they run here. A still-started
        // processor is stopped first (CLAP requires stop before
        // deactivate), then `deactivate` consumes the stopped handle
        // and runs `plugin.deactivate`; the instance drop at scope end
        // runs `plugin.destroy` while the entry keeps the library
        // mapped.
        if let Some(processor) = self.processor.take() {
            let stopped = processor.into_stopped();
            if let Some(instance) = self.instance.as_mut()
                && instance.is_active()
            {
                instance.deactivate(stopped);
            }
        }
    }
}

/// Validates the neutral equal-length contract against the declared
/// block size ([`EventList::block_frames`]): every wired channel — any
/// `Some` slice within `channels()` — must carry exactly `declared`
/// frames. A `None` channel is the unconnected-pin boundary the bounce
/// maps to silence/drop, not a length violation. A `Some` channel of
/// any other length is a caller bug — `process` answers
/// `ProcessStatus::Error` rather than silently clamping.
fn channels_match_declared<I: BufferView, O: BufferViewMut>(
    inputs: &[I],
    outputs: &[O],
    declared: usize,
) -> bool {
    for bus in inputs {
        for index in 0..bus.channels() {
            if let Some(channel) = bus.channel(index)
                && channel.len() != declared
            {
                return false;
            }
        }
    }
    for bus in outputs {
        for index in 0..bus.channels() {
            if let Some(channel) = bus.channel(index)
                && channel.len() != declared
            {
                return false;
            }
        }
    }
    true
}

/// Input bounce — copy `frames` from every wired host channel into
/// scratch; a channel the host did not wire delivers silence.
fn bounce_input<I: BufferView>(scratch: &mut [Vec<Vec<f32>>], inputs: &[I], frames: usize) {
    for (port, port_scratch) in scratch.iter_mut().enumerate() {
        for (index, channel) in port_scratch.iter_mut().enumerate() {
            let buffer = &mut channel[..frames];
            match inputs.get(port).and_then(|bus| bus.channel(index)) {
                Some(source) if source.len() >= frames => {
                    buffer.copy_from_slice(&source[..frames]);
                }
                _ => buffer.fill(0.0),
            }
        }
    }
}

/// Output bounce — copy `frames` of processed scratch back to
/// host-visible channels; a channel the host did not wire is dropped
/// (unconnected-pin semantics).
fn bounce_output<O: BufferViewMut>(scratch: &[Vec<Vec<f32>>], outputs: &mut [O], frames: usize) {
    for (port, port_scratch) in scratch.iter().enumerate() {
        let Some(bus) = outputs.get_mut(port) else {
            break;
        };
        for (index, channel) in port_scratch.iter().enumerate() {
            if let Some(target) = bus.channel_mut(index) {
                let len = target.len().min(frames);
                target[..len].copy_from_slice(&channel[..len]);
            }
        }
    }
}

/// Neutral → CLAP event translation into the persistent input buffer:
/// `EventKind::ParamValue` (normalized) → `ParamValueEvent` (plain via
/// the cached [`ParamInfo`]'s `denormalize`). Unknown parameter ids,
/// non-param event kinds, out-of-block offsets and events past
/// [`MAX_EVENTS_PER_BLOCK`] are dropped by definition — the buffer
/// never grows on the RT path.
fn push_events(
    buffer: &mut EventBuffer,
    params: &[ParamInfo],
    events: EventList<'_>,
    frames: usize,
) {
    buffer.clear();
    let frame_bound = u32::try_from(frames).unwrap_or(u32::MAX);
    for event in events.iter().take(MAX_EVENTS_PER_BLOCK) {
        let EventKind::ParamValue(param) = event.kind() else {
            continue;
        };
        if event.frame_offset() >= frame_bound {
            continue;
        }
        let (Some(id), Some(info)) = (
            ClapId::from_raw(param.param_id.0),
            params.iter().find(|info| info.id == param.param_id),
        ) else {
            continue;
        };
        buffer.push(&ParamValueEvent::new(
            event.frame_offset(),
            id,
            Pckn::match_all(),
            info.denormalize(param.value),
        ));
    }
}

impl PluginProcessor for ClapPlugin {
    /// `prepare` → `activate`: stores the `StoppedPluginAudioProcessor`
    /// `activate` returns and pre-sizes every buffer the RT call needs
    /// — input/output scratch (`declared channels × max_frames`), the
    /// Vec-backed [`AudioPorts`]/`EventBuffer` fields — so `process`
    /// never allocates.
    ///
    /// **RT contract:** non-RT only — allocates.
    ///
    /// # Errors
    ///
    /// [`PrepareError::UnsupportedConfig`] for frame bounds clack
    /// rejects outright (`min_frames == 0`, `max_frames == 0` —
    /// `PluginAudioConfiguration::validate` would panic) or a
    /// `max_frames` above the wire protocol's [`WIRE_MAX_FRAMES`]
    /// ceiling; [`PrepareError::ActivationFailed`] when the instance
    /// is missing, already activated/processing, or `plugin.activate`
    /// declines.
    fn prepare(
        &mut self,
        config: &ProcessConfig,
        _context: &mut ProcessContext,
    ) -> Result<(), PrepareError> {
        if config.min_frames() == 0
            || config.max_frames() == 0
            || config.max_frames() > WIRE_MAX_FRAMES
        {
            return Err(PrepareError::UnsupportedConfig);
        }
        let Some(instance) = self.instance.as_mut() else {
            // `prepare` before `init` — a lifecycle violation, reported
            // as an activation failure rather than silently ignored.
            return Err(PrepareError::ActivationFailed);
        };
        if self.processor.is_some() || instance.is_active() {
            // `prepare` while activated/processing — a protocol
            // violation rejected *before* the port queries below:
            // `audio-ports` `get`/`count` are main-thread-only
            // extension calls that must never run while the plugin
            // processes.
            return Err(PrepareError::ActivationFailed);
        }

        // Declared topology sizes the scratch — queried while still
        // deactivated (main-thread handle, before activate).
        let in_channels = port_channel_counts(instance.plugin_handle(), true);
        let out_channels = port_channel_counts(instance.plugin_handle(), false);

        let stopped = instance
            .activate(
                |_, ()| BridgeAudioProcessor,
                PluginAudioConfiguration {
                    sample_rate: f64::from(config.sample_rate().get()),
                    min_frames_count: config.min_frames(),
                    max_frames_count: config.max_frames(),
                },
            )
            .map_err(|_| PrepareError::ActivationFailed)?;
        self.processor = Some(PluginAudioProcessor::Stopped(stopped));

        // Both bounds are `u32` and `max` is already ≤
        // `WIRE_MAX_FRAMES`; `usize` conversion cannot fail.
        self.min_frames = usize::try_from(config.min_frames()).unwrap_or(usize::MAX);
        let max_frames = usize::try_from(config.max_frames()).unwrap_or(usize::MAX);
        self.max_frames = max_frames;
        self.input_scratch = in_channels
            .iter()
            .map(|&count| vec![vec![0.0; max_frames]; usize::try_from(count).unwrap_or(0)])
            .collect();
        self.output_scratch = out_channels
            .iter()
            .map(|&count| vec![vec![0.0; max_frames]; usize::try_from(count).unwrap_or(0)])
            .collect();
        let total_in: usize = in_channels
            .iter()
            .map(|&count| usize::try_from(count).unwrap_or(usize::MAX))
            .sum();
        let total_out: usize = out_channels
            .iter()
            .map(|&count| usize::try_from(count).unwrap_or(usize::MAX))
            .sum();
        self.input_ports = AudioPorts::with_capacity(total_in, in_channels.len());
        self.output_ports = AudioPorts::with_capacity(total_out, out_channels.len());
        self.input_events = EventBuffer::with_capacity(MAX_EVENTS_PER_BLOCK);
        Ok(())
    }

    /// `process` → `StartedPluginAudioProcessor::process` — see the
    /// module docs for the input/output bounce design, event mapping
    /// and the `None`/`None` steady-time/transport boundary.
    ///
    /// **RT contract:** RT-suitable — bounded copies into pre-sized
    /// scratch, no allocation, no blocking.
    fn process<I, O>(
        &mut self,
        inputs: &[I],
        outputs: &mut [O],
        events: EventList<'_>,
        _transport: Option<TransportInfo>,
        _context: &mut ProcessContext,
    ) -> ProcessResult
    where
        I: BufferView,
        O: BufferViewMut,
    {
        let Some(PluginAudioProcessor::Started(started)) = self.processor.as_mut() else {
            // `process` outside Processing — a defined rejection, not a
            // fabricated transition.
            return ProcessResult::new(ProcessStatus::Error, ProcessResult::TAIL_NONE);
        };

        let frames = usize::try_from(events.block_frames()).unwrap_or(usize::MAX);
        if !(self.min_frames..=self.max_frames).contains(&frames) {
            // A block outside the negotiated `[min,max]` must-bound
            // must not reach `clap_process` — the plugin only agreed
            // to that interval, and `frames == 0` would silently drop
            // every event offset besides. Defined rejection, like the
            // plugin-side adapter's own oversized-block path.
            return ProcessResult::new(ProcessStatus::Error, ProcessResult::TAIL_NONE);
        }
        if !channels_match_declared(inputs, outputs, frames) {
            // A wired channel whose slice length differs from the
            // declared block length is a caller bug (equal-length
            // contract) — reported honestly, never silently clamped.
            return ProcessResult::new(ProcessStatus::Error, ProcessResult::TAIL_NONE);
        }

        // Bounce the immutable host input into scratch (see the module
        // docs for why inputs cannot alias `*mut` CLAP buffers).
        bounce_input(&mut self.input_scratch, inputs, frames);
        let input_buffers = self
            .input_ports
            .with_input_buffers(
                self.input_scratch
                    .iter_mut()
                    .map(|scratch| AudioPortBuffer {
                        latency: 0,
                        channels: AudioPortBufferType::f32_input_only(scratch.iter_mut().map(
                            |channel| InputChannel {
                                buffer: &mut channel[..frames],
                                is_constant: false,
                            },
                        )),
                    }),
            );
        let mut output_buffers =
            self.output_ports
                .with_output_buffers(self.output_scratch.iter_mut().map(|scratch| {
                    AudioPortBuffer {
                        latency: 0,
                        channels: AudioPortBufferType::f32_output_only(
                            scratch.iter_mut().map(|channel| &mut channel[..frames]),
                        ),
                    }
                }));

        push_events(&mut self.input_events, self.params, events, frames);
        let input_events = self.input_events.as_input();
        // Plugin output events are unmapped in v1 — `void` discards
        // every `try_push` with no backing store a plugin could grow
        // on the RT path.
        let mut output_events = OutputEvents::void();

        let status = started.process(
            &input_buffers,
            &mut output_buffers,
            &input_events,
            &mut output_events,
            // `steady_time`/`transport` — the v1 defined boundary; see
            // the module docs.
            None,
            None,
        );

        // Bounce processed scratch back to the host-visible output.
        bounce_output(&self.output_scratch, outputs, frames);

        match status {
            Ok(ClapProcessStatus::Continue | ClapProcessStatus::ContinueIfNotQuiet) => {
                ProcessResult::CONTINUE
            }
            Ok(ClapProcessStatus::Tail) => {
                ProcessResult::new(ProcessStatus::Continue, ProcessResult::TAIL_INFINITE)
            }
            Ok(ClapProcessStatus::Sleep) => ProcessResult::SLEEP,
            Err(_) => ProcessResult::new(ProcessStatus::Error, ProcessResult::TAIL_NONE),
        }
    }
}

impl Plugin for ClapPlugin {
    fn descriptor(&self) -> PluginDescriptor {
        self.descriptor
    }

    fn params(&self) -> &[ParamInfo] {
        self.params
    }

    /// `init` → `PluginInstance::new` — the fused CLAP
    /// instantiate+`plugin.init`. `create()` never instantiates.
    ///
    /// **RT contract:** non-RT only — instantiates the plugin.
    ///
    /// # Errors
    ///
    /// [`HostError::PluginFailed`] on a second `init` or when the CLAP
    /// instantiate/`plugin.init` path declines.
    fn init(&mut self) -> Result<(), HostError> {
        if self.instance.is_some() {
            return Err(HostError::PluginFailed);
        }
        let instance = PluginInstance::<BridgeHandlers>::new(
            |()| BridgeShared,
            |_| (),
            &self.entry,
            &self.plugin_id,
            &self.host_info,
        )
        .map_err(|_| HostError::PluginFailed)?;
        self.instance = Some(instance);
        Ok(())
    }

    /// `start_processing` → `Stopped` → `Started` processor transition.
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// [`HostError::PluginFailed`] when the instance is not activated
    /// or `plugin.start_processing` declines.
    fn start_processing(&mut self) -> Result<(), HostError> {
        let Some(processor) = self.processor.as_mut() else {
            return Err(HostError::PluginFailed);
        };
        processor
            .start_processing()
            .map(|_| ())
            .map_err(|_| HostError::PluginFailed)
    }

    /// `stop_processing` → `Started` → `Stopped` processor transition.
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// [`HostError::PluginFailed`] — advisory; the instance is already
    /// out of `Processing` or was never started.
    fn stop_processing(&mut self) -> Result<(), HostError> {
        let Some(processor) = self.processor.as_mut() else {
            return Err(HostError::PluginFailed);
        };
        processor
            .stop_processing()
            .map(|_| ())
            .map_err(|_| HostError::PluginFailed)
    }
}

// ---------------------------------------------------------------------------
// Fixture resolution helper (shared with the Task-3 runner / Task-4 E2E)
// ---------------------------------------------------------------------------

/// The workspace-built CLAP fixture's dynamic-library file name:
/// `{DLL_PREFIX}ruviform_plugin_clap{DLL_SUFFIX}` — the same
/// `current_exe`-relative convention the L1 runner and the contract
/// tests resolve by.
#[must_use]
pub fn fixture_lib_name() -> String {
    format!(
        "{}ruviform_plugin_clap{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    )
}

// ---------------------------------------------------------------------------
// Tests — in-process dlopen of the workspace-built fixture cdylib
// ---------------------------------------------------------------------------

#[cfg(all(test, feature = "host"))]
mod tests {
    use super::*;
    use ruviform_audio_core::{PlanarBufferView, PlanarBufferViewMut, SampleRate};
    use ruviform_plugin_core::{Event, HostInput, HostOutput, ParamEvent, ParamValue};
    use std::path::PathBuf;

    const FRAMES: usize = 64;
    const FRAMES_U32: u32 = 64;

    /// `current_exe` sibling `deps/{DLL_PREFIX}ruviform_plugin_clap{DLL_SUFFIX}`
    /// — the test spec's fixture resolution; a missing binary is an
    /// `assert!` failure, never a silent skip.
    fn fixture_path() -> PathBuf {
        let exe = std::env::current_exe().expect("current_exe is available");
        let deps = exe.parent().expect("test binary lives in deps/");
        deps.join(fixture_lib_name())
    }

    fn load_fixture() -> ClapLibrary {
        let path = fixture_path();
        assert!(path.is_file(), "fixture cdylib missing: {}", path.display());
        ClapLibrary::load(&path).expect("fixture dlopen succeeds")
    }

    /// Stereo input pair `0.5 / -0.25` through the fixture and back.
    fn drive_block(
        plugin: &mut dyn ErasedPlugin,
        events: EventList<'_>,
    ) -> (ProcessResult, [Vec<f32>; 2]) {
        let in_left = vec![0.5_f32; FRAMES];
        let in_right = vec![-0.25_f32; FRAMES];
        let in_channels = [&in_left[..], &in_right[..]];
        let in_view = PlanarBufferView::new(&in_channels).unwrap();
        let mut out_left = vec![0.0_f32; FRAMES];
        let mut out_right = vec![0.0_f32; FRAMES];
        let result = {
            let mut out_channels = [&mut out_left[..], &mut out_right[..]];
            let mut out_view = PlanarBufferViewMut::new(&mut out_channels).unwrap();
            let inputs = [HostInput(&in_view)];
            let mut outputs = [HostOutput(&mut out_view)];
            plugin.process_host(
                &inputs,
                &mut outputs,
                events,
                None,
                &mut ProcessContext::new(),
            )
        };
        (result, [out_left, out_right])
    }

    fn prepared_plugin() -> Box<dyn ErasedPlugin> {
        let library = load_fixture();
        let factories = library.factories();
        assert_eq!(factories.len(), 1);
        let mut plugin = factories[0].create();
        plugin.init().expect("init");
        let config = ProcessConfig::new(SampleRate::new(48_000).unwrap(), 1, FRAMES_U32).unwrap();
        plugin
            .prepare(&config, &mut ProcessContext::new())
            .expect("prepare");
        plugin.start_processing().expect("start_processing");
        plugin
    }

    #[test]
    fn descriptors_match_fixture_known_values() {
        let library = load_fixture();
        let descriptors = library.descriptors();
        assert_eq!(descriptors.len(), 1);
        let descriptor = descriptors[0];
        assert_eq!(descriptor.id, "org.ruviform.clap-gain-poc");
        assert_eq!(descriptor.name, "Ruviform Gain PoC");
        assert_eq!(descriptor.vendor, "Ruviform");
        assert_eq!(descriptor.version, env!("CARGO_PKG_VERSION"));
        // Feature order is preserved from the declared list.
        assert_eq!(descriptor.features, ["audio-effect", "stereo"]);
    }

    #[test]
    fn descriptors_are_cached_across_calls() {
        let library = load_fixture();
        let first = library.descriptors();
        let second = library.descriptors();
        // The intern-once cached slice — same pointer, same values, no
        // per-call scan or leak growth.
        assert!(std::ptr::eq(first.as_ptr(), second.as_ptr()));
        assert_eq!(first, second);
    }

    #[test]
    fn load_missing_path_returns_error() {
        let result = ClapLibrary::load("definitely/not/a/plugin.clap");
        assert!(matches!(result, Err(ClapHostError::Entry(_))));
    }

    #[test]
    fn entry_error_keeps_erased_display_and_source() {
        // Erased payload — diagnostics survive via `Display` +
        // `source()` while no binding-crate type appears in the
        // public signature (crate policy: binding-neutral API).
        let Err(error) = ClapLibrary::load("definitely/not/a/plugin.clap") else {
            panic!("missing path must fail");
        };
        assert!(matches!(error, ClapHostError::Entry(_)));
        assert_ne!(error.to_string(), "");
        assert!(std::error::Error::source(&error).is_some());
    }

    #[test]
    fn load_non_clap_binary_returns_error() {
        // The running test binary is a loadable image without
        // `clap_entry` — a defined `Err`, never a panic.
        let exe = std::env::current_exe().expect("current_exe");
        assert!(ClapLibrary::load(&exe).is_err());
    }

    #[test]
    fn factory_reports_cached_descriptor_and_no_sendable() {
        let library = load_fixture();
        let factories = library.factories();
        assert_eq!(factories.len(), 1);
        let factory = &factories[0];
        assert_eq!(factory.descriptor(), library.descriptors()[0]);
        // `PluginInstance` is `!Send` — the opt-in stays `None`.
        assert!(factory.create_sendable().is_none());
    }

    #[test]
    fn params_table_matches_fixture_at_every_lifecycle_point() {
        let library = load_fixture();
        let factories = library.factories();
        let plugin = factories[0].create();
        // Pre-init — the eager factory-time enumeration already caches
        // the table (lazy enumeration could not answer here).
        let params = plugin.params();
        assert_eq!(params.len(), 1);
        let param = params[0];
        assert_eq!(param.id, ParamId(0));
        assert_eq!(param.name, "Gain");
        // `unit` is the honest synthetic value — CLAP param_info has
        // no unit field (not a filler string).
        assert_eq!(param.unit, "");
        assert_eq!(param.min_value, 0.0);
        assert_eq!(param.max_value, 2.0);
        assert_eq!(param.default_value, 1.0);

        // Post-init and activated — still the same cached table.
        let mut plugin = prepared_plugin();
        assert_eq!(plugin.params()[0], param);
        plugin.stop_processing().expect("stop_processing");
        assert_eq!(plugin.params()[0], param);
    }

    #[test]
    fn lifecycle_roundtrip_processes_passthrough_block() {
        let mut plugin = prepared_plugin();
        let (result, [left, right]) = drive_block(&mut *plugin, EventList::empty(FRAMES_U32));
        assert_eq!(result.status, ProcessStatus::Continue);
        // Default gain 1.0 — input carried verbatim.
        assert_eq!(left, vec![0.5_f32; FRAMES]);
        assert_eq!(right, vec![-0.25_f32; FRAMES]);
        plugin.stop_processing().expect("stop_processing");
        // `drop` runs the symmetric teardown — stop (already stopped),
        // deactivate, `plugin.destroy`.
    }

    #[test]
    fn param_event_reaches_plugin_denormalized() {
        let mut plugin = prepared_plugin();
        // Normalized 0.25 over [0.0, 2.0] → plain 0.5 gain.
        let events = [Event::new(
            0,
            EventKind::ParamValue(ParamEvent {
                param_id: ParamId(0),
                value: ParamValue::new(0.25).unwrap(),
            }),
        )];
        let events = EventList::new(&events, FRAMES_U32).unwrap();
        let (result, [left, right]) = drive_block(&mut *plugin, events);
        assert_eq!(result.status, ProcessStatus::Continue);
        assert_eq!(left, vec![0.25_f32; FRAMES]);
        assert_eq!(right, vec![-0.125_f32; FRAMES]);
    }

    #[test]
    fn prepare_while_processing_is_defined_error() {
        let mut plugin = prepared_plugin();
        // A second `prepare` while activated+started must be rejected
        // before the main-thread `audio-ports` queries — CLAP forbids
        // them during processing.
        let config = ProcessConfig::new(SampleRate::new(48_000).unwrap(), 1, FRAMES_U32).unwrap();
        assert_eq!(
            plugin.prepare(&config, &mut ProcessContext::new()),
            Err(PrepareError::ActivationFailed)
        );
    }

    #[test]
    fn prepare_above_wire_max_frames_is_unsupported() {
        let library = load_fixture();
        let factories = library.factories();
        let mut plugin = factories[0].create();
        plugin.init().expect("init");
        // `max_frames` beyond the wire protocol's `WIRE_MAX_FRAMES`
        // ceiling is rejected before it can size scratch.
        let config =
            ProcessConfig::new(SampleRate::new(48_000).unwrap(), 1, WIRE_MAX_FRAMES + 1).unwrap();
        assert_eq!(
            plugin.prepare(&config, &mut ProcessContext::new()),
            Err(PrepareError::UnsupportedConfig)
        );
    }

    #[test]
    fn process_below_min_frames_is_defined_error() {
        let mut plugin = prepared_plugin();
        // `frames == 0` is below the negotiated `min_frames = 1` —
        // rejected before `clap_process` sees a must-bound violation
        // (and before events could be silently dropped).
        let (result, _) = drive_block(&mut *plugin, EventList::empty(0));
        assert_eq!(result.status, ProcessStatus::Error);
    }

    #[test]
    fn process_above_max_frames_is_defined_error() {
        let mut plugin = prepared_plugin();
        // Beyond negotiated `max_frames` — the pre-existing bound,
        // still a defined rejection.
        let (result, _) = drive_block(&mut *plugin, EventList::empty(FRAMES_U32 * 2));
        assert_eq!(result.status, ProcessStatus::Error);
    }

    #[test]
    fn process_with_mismatched_channel_length_is_defined_error() {
        let mut plugin = prepared_plugin();
        // A wired channel shorter than the declared block violates the
        // equal-length contract — an honest `Error`, not a silent
        // clamp. (`PlanarBufferView::new` itself enforces uniform
        // channel lengths; the mismatch is view-vs-`block_frames`.)
        let short = FRAMES / 2;
        let in_left = vec![0.5_f32; short];
        let in_right = vec![-0.25_f32; short];
        let in_channels = [&in_left[..], &in_right[..]];
        let in_view = PlanarBufferView::new(&in_channels).unwrap();
        let mut out_left = vec![0.0_f32; FRAMES];
        let mut out_right = vec![0.0_f32; FRAMES];
        let result = {
            let mut out_channels = [&mut out_left[..], &mut out_right[..]];
            let mut out_view = PlanarBufferViewMut::new(&mut out_channels).unwrap();
            let inputs = [HostInput(&in_view)];
            let mut outputs = [HostOutput(&mut out_view)];
            plugin.process_host(
                &inputs,
                &mut outputs,
                EventList::empty(FRAMES_U32),
                None,
                &mut ProcessContext::new(),
            )
        };
        assert_eq!(result.status, ProcessStatus::Error);
    }

    /// Issue #22 RT-verification lane — the bridge `process` path
    /// (input/output bounce into pre-sized scratch, `with_*_buffers`
    /// within reserved capacity, `push_events`, `OutputEvents::void`)
    /// performs no allocation after `prepare`; `assert_no_alloc`
    /// aborts the test binary on any (de)allocation inside the
    /// window. The `#[global_allocator]` `AllocDisabler` armed in
    /// `plugin.rs`'s test module covers this binary. Warm-up
    /// (first-touch initialization in the loaded plugin / clack
    /// paths) and every buffer fixture stay outside the window.
    #[test]
    fn rt_process_drives_fixture_block_allocation_free() {
        let mut plugin = prepared_plugin();
        let _ = drive_block(&mut *plugin, EventList::empty(FRAMES_U32));

        let in_left = vec![0.5_f32; FRAMES];
        let in_right = vec![-0.25_f32; FRAMES];
        let in_channels = [&in_left[..], &in_right[..]];
        let in_view = PlanarBufferView::new(&in_channels).unwrap();
        let mut out_left = vec![0.0_f32; FRAMES];
        let mut out_right = vec![0.0_f32; FRAMES];
        let mut out_channels = [&mut out_left[..], &mut out_right[..]];
        let mut out_view = PlanarBufferViewMut::new(&mut out_channels).unwrap();
        let inputs = [HostInput(&in_view)];
        let mut outputs = [HostOutput(&mut out_view)];
        let mut context = ProcessContext::new();
        let events = EventList::empty(FRAMES_U32);

        let result = assert_no_alloc::assert_no_alloc(|| {
            plugin.process_host(&inputs, &mut outputs, events, None, &mut context)
        });
        assert_eq!(result.status, ProcessStatus::Continue);
        assert_eq!(out_left, vec![0.5_f32; FRAMES]);
        assert_eq!(out_right, vec![-0.25_f32; FRAMES]);
    }

    #[test]
    fn process_before_start_is_defined_error() {
        let library = load_fixture();
        let factories = library.factories();
        let mut plugin = factories[0].create();
        plugin.init().expect("init");
        let config = ProcessConfig::new(SampleRate::new(48_000).unwrap(), 1, FRAMES_U32).unwrap();
        plugin
            .prepare(&config, &mut ProcessContext::new())
            .expect("prepare");
        // Activated but never started — defined rejection, no panic.
        let (result, _) = drive_block(&mut *plugin, EventList::empty(FRAMES_U32));
        assert_eq!(result.status, ProcessStatus::Error);
    }

    #[test]
    fn unsupported_state_and_gui_verbs_are_defaults() {
        let library = load_fixture();
        let factories = library.factories();
        let mut plugin = factories[0].create();
        assert_eq!(plugin.save_state(), Err(HostError::Unsupported));
        assert_eq!(
            plugin.restore_state(&[1, 2, 3]),
            Err(HostError::Unsupported)
        );
        assert_eq!(
            plugin.open_gui(ruviform_plugin_core::ParentLease::new(0, 0)),
            Err(HostError::Unsupported)
        );
        assert_eq!(plugin.close_gui(), Err(HostError::Unsupported));
    }
}
