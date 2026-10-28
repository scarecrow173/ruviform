//! `ruviform-plugin-export` — the plugin-side (production-side) L1
//! out-of-process runner of the Ruviform media foundation (Issue #76;
//! wire-v2 instance registry per Issue #117).
//!
//! The crate is a thin library whose single public entry is [`run`]: a
//! plugin author's (or adapter's) binary calls it from `main` with the
//! built-in factories, and the child process then serves plugin
//! instances to the parent host over the L1 protocol. On the parent
//! side the binary is registered as
//! `ruviform_plugin_host::OutOfProcessSpec::program` — the
//! `PluginHost` composite spawns it and registers instances inside it.
//!
//! ## Process model
//!
//! One child process hosts up to [`wire::WIRE_MAX_INSTANCES`] plugin
//! instances in a runner-side registry keyed by the wire-v2 `u32
//! instance` envelope field. The host mints a handle with
//! [`wire::Command::CreateInstance`] (whose payload carries the plugin
//! id and the shm attachment — the v1 argv bootstrap moved onto the
//! wire), reclaims it with [`wire::Command::DestroyInstance`], and
//! finishes with the instance-less `Shutdown`. The control plane is
//! the stdio pipe: framed [`wire::Message`]s — `Hello` (child →
//! parent, first frame), `Command`/`Reply`, the shm doorbells
//! `BlockIn`/`BlockDone` and `Shutdown`. The data plane is one named
//! shared-memory segment per registered instance, created by the
//! parent and attached by the child (see the internal `shm` module).
//! Audio blocks, events and transport travel through fixed-layout
//! shm slots; `BlockIn { seq, instance }` is the doorbell that hands
//! in-slot `seq % in_depth` of that instance's segment to the child.
//!
//! ## `argv` contract
//!
//! ```text
//! <runner>
//! ```
//!
//! The runner accepts **no arguments**. Wire v2 retired the
//! `--ruviform-plugin-id` / `--ruviform-shm-key` bootstrap flags —
//! their values now arrive per instance in `CreateInstance`. Any argv
//! entry, including a `--ruviform-*` flag, is a defined usage
//! rejection: a stderr diagnostic plus exit 2 without a `Hello` (the
//! parent observes spawn → EOF and converges to `HostError::Ipc`
//! instead of hanging).
//!
//! ## Exit semantics
//!
//! - `0` — `Shutdown` received or clean stdin EOF at a frame boundary:
//!   every registered instance's lifecycle is normalized
//!   (`Processing → stop → Activated → Deactivate → Initialized →
//!   Destroy → Destroyed`, best-effort) and the process exits.
//! - `1` — transport/protocol failure: stdin I/O error, a truncated
//!   frame, a frame over [`wire::WIRE_MAX_PAYLOAD`], a decode failure
//!   or an inbound message kind a child must never receive
//!   (`Hello`/`Reply`/`BlockDone` travel child → parent only).
//! - `2` — `argv` usage error (no `Hello` is sent).
//!
//! ## Dispatch semantics
//!
//! Every inbound `Command` resolves its envelope `instance` first:
//! `Scan` is child-scoped ([`wire::WIRE_INSTANCE_NONE`]),
//! `CreateInstance`/`DestroyInstance` manage the registry, and every
//! other verb resolves the handle — an unregistered one is a defined
//! `Err(HostError::InvalidHandle)` reply, never a dead child.
//! `CreateInstance` fails defined too:
//! [`HostError::UnknownPluginId`] (id not among the factories),
//! [`HostError::InstanceLimit`] (registry full),
//! [`HostError::Ipc`] (duplicate handle or shm attach/header
//! mismatch) or [`HostError::PluginFailed`] (factory panic) — and
//! leaves no half-registered entry.
//!
//! Instance commands then map onto the `ErasedPlugin` surface with the
//! same lifecycle semantics as the L0 `InProcessHost` (`Initialize` /
//! `Activate` report `InitFailed`/`ActivateFailed`, `StartProcessing`
//! failure rolls back to `Activated`, `StopProcessing` errors are
//! advisory). Illegal transitions surface as
//! `Err(HostError::Lifecycle(_))` inside the wire `Reply` — never as a
//! panic. A plugin panic (hook or `process_host`) is caught and
//! returned as `Err(HostError::PluginFailed)`; the child stays alive
//! and keeps serving commands.
//!
//! ## `unsafe`
//!
//! The crate contains no `unsafe` outside the two scoped
//! `shared_memory` slice accessors in the internal `shm` module — see
//! that module's deviation note (the design assumed safe accessors;
//! `shared_memory` 0.12.4's are `unsafe fn`).
//!
//! **RT contract:** every item here is **non-RT only** — the runner is
//! the non-RT end of the L1 link; the host's RT leg lives in
//! `ruviform-plugin-host`.
//!
//! Contract source: `docs/designs/plugin-l1-out-of-process.md` (Issue
//! #76 design), `docs/specs/plugin-l1-out-of-process.md`,
//! `docs/designs/issue-117-bundle-child-process.md` (wire v2).

use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::process::ExitCode;
use std::sync::atomic::{Ordering, fence};

use ruviform_plugin_core::wire::{self, Command as WireCommand, Message, Reply as WireReply};
use ruviform_plugin_core::{
    BufferView, BufferViewMut, ErasedPlugin, Event, EventList, HostError, HostInput, HostOutput,
    Lifecycle, LifecycleEvent, LifecycleState, PluginFactory, PrepareError, ProcessConfig,
    ProcessContext, ProcessResult,
};

mod shm;

const USAGE: &str = "usage: <runner>";
const EXIT_USAGE: u8 = 2;

/// The child-process entry point — a plugin binary calls this from
/// `main` and returns the result as the process exit code.
///
/// `factories` are the plugin types built into the binary; the child
/// registers instances of them in its registry as `CreateInstance`
/// commands arrive (the envelope `instance` handle is host-minted).
/// See the crate docs for the `argv` contract, exit semantics and the
/// handshake (`Hello` → `Scan` → `CreateInstance`).
///
/// The function drives a single-threaded message loop on stdin/stdout
/// and returns only on `Shutdown`, stdin EOF or a defined-rejection
/// failure — it never hangs on malformed input.
///
/// # Examples
///
/// ```no_run
/// // In a plugin binary's `main`:
/// fn main() -> std::process::ExitCode {
///     ruviform_plugin_export::run(vec![])
/// }
/// ```
#[must_use]
pub fn run(factories: Vec<Box<dyn PluginFactory>>) -> ExitCode {
    // Wire v2: no bootstrap arguments — the retired `--ruviform-*`
    // flags moved into the `CreateInstance` payload, and a
    // `run()`-based runner accepts no user args either. Any argv
    // entry is the defined usage rejection.
    if std::env::args_os().nth(1).is_some() {
        eprintln!("ruviform-plugin-export: unexpected arguments\n{USAGE}");
        return ExitCode::from(EXIT_USAGE);
    }

    let stdout = io::stdout();
    let mut output = stdout.lock();
    let mut encode_buf = Vec::new();
    // `Hello` is the first frame the child emits — before any
    // `CreateInstance` lands, so a peer sees a live handshake even
    // while the registry is still empty.
    if wire::write_message(
        &mut output,
        &Message::Hello {
            version: wire::WIRE_VERSION,
            capabilities: 0,
        },
        &mut encode_buf,
    )
    .is_err()
    {
        return ExitCode::FAILURE;
    }

    let mut runner = Runner {
        factories,
        instances: HashMap::new(),
        events_scratch: Vec::new(),
    };

    let stdin = io::stdin();
    let mut input = stdin.lock();
    serve(&mut runner, &mut input, &mut output, &mut encode_buf)
}

/// The framed stdin/stdout command loop. `encode_buf` is the shared
/// scratch buffer `run` already used for `Hello` — one allocation
/// reused for every outbound frame. Returns the process exit code:
/// `SUCCESS` on `Shutdown` or a clean EOF at a frame boundary,
/// `FAILURE` on any transport/protocol violation (after best-effort
/// lifecycle normalization in both cases).
fn serve(
    runner: &mut Runner,
    input: &mut impl Read,
    output: &mut impl Write,
    encode_buf: &mut Vec<u8>,
) -> ExitCode {
    let mut frame_buf = Vec::new();
    loop {
        let message = match wire::read_frame(input, &mut frame_buf) {
            Ok(true) => match wire::decode(&frame_buf) {
                Ok(message) => message,
                Err(error) => {
                    eprintln!("ruviform-plugin-export: undecodable frame: {error}");
                    runner.normalize_all();
                    return ExitCode::FAILURE;
                }
            },
            Ok(false) => {
                // Clean EOF at a frame boundary — the parent is gone.
                runner.normalize_all();
                return ExitCode::SUCCESS;
            }
            Err(error) => {
                eprintln!("ruviform-plugin-export: {error}");
                runner.normalize_all();
                return ExitCode::FAILURE;
            }
        };
        match message {
            Message::Command { seq, instance, cmd } => {
                let result = runner.dispatch(instance, cmd);
                if wire::write_message(
                    output,
                    &Message::Reply {
                        seq,
                        instance,
                        result,
                    },
                    encode_buf,
                )
                .is_err()
                {
                    eprintln!("ruviform-plugin-export: control-plane write failed");
                    runner.normalize_all();
                    return ExitCode::FAILURE;
                }
            }
            // The doorbell `seq` space is disjoint from the command
            // `seq` space by construction: the host sets
            // `BLOCK_SEQ_FLAG` (`1 << 63`, `l1::data` in
            // `ruviform-plugin-host`) on every `BlockIn` seq and
            // demultiplexes the child's replies on that bit. The
            // child echoes the seq and instance verbatim —
            // `BlockDone` on success, `Reply{seq, Err}` on rejection
            // — so no child-side partition work is needed.
            Message::BlockIn { seq, instance } => {
                // The doorbell's release edge on the writer side is
                // paired here before touching shm — a real atomic
                // fence (`std::sync::atomic::fence`), not a compiler
                // fence: ordering across the process boundary rests
                // on the fence plus the pipe-read syscall.
                fence(Ordering::Acquire);
                let reply = match runner.process_block(instance, seq) {
                    Ok(result) => {
                        // Publish the out-slot before the doorbell —
                        // same real-fence Release edge.
                        fence(Ordering::Release);
                        Message::BlockDone {
                            seq,
                            instance,
                            result,
                        }
                    }
                    Err(error) => Message::Reply {
                        seq,
                        instance,
                        result: Err(error),
                    },
                };
                if wire::write_message(output, &reply, encode_buf).is_err() {
                    eprintln!("ruviform-plugin-export: control-plane write failed");
                    runner.normalize_all();
                    return ExitCode::FAILURE;
                }
            }
            Message::Shutdown => {
                runner.normalize_all();
                return ExitCode::SUCCESS;
            }
            // `Hello`/`Reply`/`BlockDone` are child → parent messages:
            // receiving one inbound is a protocol violation. The
            // diagnostic is a bounded kind label — a hostile payload
            // (e.g. a max-size `Reply::State`) is never dumped whole.
            _ => {
                eprintln!(
                    "ruviform-plugin-export: protocol violation: inbound {}",
                    inbound_label(&message)
                );
                runner.normalize_all();
                return ExitCode::FAILURE;
            }
        }
    }
}

/// A bounded label for a protocol-violating inbound message — the
/// kind name plus its correlation seq, never the payload.
fn inbound_label(message: &Message) -> String {
    match message {
        Message::Hello { .. } => "Hello".to_owned(),
        Message::Reply { seq, .. } => format!("Reply seq={seq}"),
        Message::BlockDone { seq, .. } => format!("BlockDone seq={seq}"),
        _ => "unassigned kind".to_owned(),
    }
}

/// The message-loop state: factories (for `Scan` and `CreateInstance`)
/// and the live-instance registry.
struct Runner {
    factories: Vec<Box<dyn PluginFactory>>,
    /// Live instances by their wire `instance` envelope handle —
    /// at most [`wire::WIRE_MAX_INSTANCES`] entries. The key range is
    /// codec-enforced (a `Command` cannot decode with an out-of-range
    /// instance), so the map never sees an illegal key.
    instances: HashMap<u32, InstanceEntry>,
    /// Shared `Event` decode scratch — `process_block` serves one
    /// block at a time, so a single buffer covers every instance.
    events_scratch: Vec<Event>,
}

/// One registered instance: the plugin object, its lifecycle mirror,
/// its own attached shm segment and reusable block staging.
struct InstanceEntry {
    instance: Box<dyn ErasedPlugin>,
    lifecycle: Lifecycle,
    context: ProcessContext,
    /// `max_frames` of the last successful `Activate` — bounds the
    /// accepted block length (`None` while not activated).
    activated_max_frames: Option<u32>,
    segment: shm::Segment,
    /// Planar `f32` staging — `channels * frames`, channel-major.
    staging_in: Vec<f32>,
    staging_out: Vec<f32>,
}

impl Runner {
    /// Resolves the envelope `instance` and dispatches `command`:
    /// `Scan`/registry verbs are runner-level, everything else goes to
    /// the entry the handle names — an unregistered handle is
    /// `Err(HostError::InvalidHandle)`, never a dead child.
    fn dispatch(&mut self, instance: u32, command: WireCommand) -> Result<WireReply, HostError> {
        match command {
            WireCommand::Scan => self.scan(),
            WireCommand::CreateInstance {
                plugin_id,
                shm_key,
                in_depth,
                out_depth,
            } => self.create_instance(instance, &plugin_id, &shm_key, in_depth, out_depth),
            WireCommand::DestroyInstance => self.destroy_instance(instance),
            instance_command => match self.instances.get_mut(&instance) {
                Some(entry) => entry.dispatch(instance_command),
                None => Err(HostError::InvalidHandle),
            },
        }
    }

    /// `Scan`: enumerate every factory's descriptor →
    /// `Reply::Scanned`. `factory.descriptor()` is plugin code —
    /// guarded exactly like `create()`/the hooks: a panic is
    /// `Err(HostError::PluginFailed)` and the child survives.
    fn scan(&mut self) -> Result<WireReply, HostError> {
        catch_unwind(AssertUnwindSafe(|| {
            self.factories
                .iter()
                .map(|factory| wire::Descriptor::from(factory.descriptor()))
                .collect()
        }))
        .map(WireReply::Scanned)
        .map_err(|_| HostError::PluginFailed)
    }

    /// `CreateInstance`: validate the plugin id, capacity and handle
    /// freshness, attach the segment, run `create()` — every failure
    /// is a defined `Err` and leaves the registry untouched.
    fn create_instance(
        &mut self,
        instance: u32,
        plugin_id: &str,
        shm_key: &str,
        in_depth: u32,
        out_depth: u32,
    ) -> Result<WireReply, HostError> {
        // `factory.descriptor()` is plugin code — same guard class as
        // `create()` and the hooks.
        let index = catch_unwind(AssertUnwindSafe(|| {
            self.factories
                .iter()
                .position(|factory| factory.descriptor().id == plugin_id)
        }))
        .map_err(|_| HostError::PluginFailed)?
        .ok_or(HostError::UnknownPluginId)?;
        // Capacity precedes the duplicate check: a full registry
        // already holds every legal handle value, so checking the
        // handle first would make `InstanceLimit` unreachable.
        if self.instances.len() >= usize::try_from(wire::WIRE_MAX_INSTANCES).unwrap_or(usize::MAX) {
            return Err(HostError::InstanceLimit);
        }
        if self.instances.contains_key(&instance) {
            // Minting a live handle twice is a host-side contract
            // violation — defined `Err(Ipc)`, not a crash.
            return Err(HostError::Ipc);
        }
        let segment =
            shm::Segment::attach(shm_key, in_depth, out_depth).map_err(|_| HostError::Ipc)?;
        let plugin = catch_unwind(AssertUnwindSafe(|| self.factories[index].create()))
            .map_err(|_| HostError::PluginFailed)?;
        self.instances.insert(
            instance,
            InstanceEntry {
                instance: plugin,
                lifecycle: Lifecycle::new(),
                context: ProcessContext::new(),
                activated_max_frames: None,
                segment,
                staging_in: Vec::new(),
                staging_out: Vec::new(),
            },
        );
        Ok(WireReply::Ok)
    }

    /// `DestroyInstance`: normalize the entry's lifecycle, detach its
    /// segment (drop) and remove the handle → `Reply::Ok`. An
    /// unregistered handle is `Err(InvalidHandle)`.
    fn destroy_instance(&mut self, instance: u32) -> Result<WireReply, HostError> {
        let Some(mut entry) = self.instances.remove(&instance) else {
            return Err(HostError::InvalidHandle);
        };
        entry.normalize();
        Ok(WireReply::Ok)
    }

    /// `BlockIn`: resolve the instance and run its block. An
    /// unregistered handle is a defined `Err(InvalidHandle)` — the
    /// flagged `Reply` the host's block-in-flight map answers with.
    fn process_block(&mut self, instance: u32, seq: u64) -> Result<ProcessResult, HostError> {
        let events_scratch = &mut self.events_scratch;
        match self.instances.get_mut(&instance) {
            Some(entry) => entry.process_block(seq, events_scratch),
            None => Err(HostError::InvalidHandle),
        }
    }

    /// Best-effort lifecycle normalization of every live instance
    /// before exit (`Shutdown` / stdin EOF / fatal error) — teardown
    /// never fails, so sibling instances are normalized even when one
    /// plugin's hooks panic.
    fn normalize_all(&mut self) {
        for entry in self.instances.values_mut() {
            entry.normalize();
        }
    }
}

impl InstanceEntry {
    /// Dispatches one instance-scoped `wire::Command`, mirroring
    /// the L0 `InProcessHost` semantics. Plugin hooks run under
    /// `catch_unwind`: a panic becomes `Err(HostError::PluginFailed)`
    /// and never kills the child.
    fn dispatch(&mut self, command: WireCommand) -> Result<WireReply, HostError> {
        match command {
            WireCommand::Initialize => self.initialize(),
            WireCommand::Activate { config } => self.activate(&config),
            WireCommand::StartProcessing => self.start_processing(),
            WireCommand::StopProcessing => self.stop_processing(),
            WireCommand::Deactivate => {
                self.lifecycle
                    .transition(LifecycleEvent::Deactivate)
                    .map_err(HostError::Lifecycle)?;
                self.activated_max_frames = None;
                Ok(WireReply::Ok)
            }
            WireCommand::SaveState => {
                Self::guarded(&mut *self.instance, |plugin| plugin.save_state())
                    .and_then(|inner| inner)
                    .map(|bytes| WireReply::State { bytes })
            }
            WireCommand::RestoreState { bytes } => {
                Self::guarded(&mut *self.instance, |plugin| plugin.restore_state(&bytes))
                    .and_then(|inner| inner)
                    .map(|()| WireReply::Ok)
            }
            WireCommand::OpenGui { parent } => {
                Self::guarded(&mut *self.instance, |plugin| plugin.open_gui(parent))
                    .and_then(|inner| inner)
                    .map(|gui| WireReply::Opened { gui })
            }
            WireCommand::CloseGui => {
                Self::guarded(&mut *self.instance, |plugin| plugin.close_gui())
                    .and_then(|inner| inner)
                    .map(|()| WireReply::Ok)
            }
            // `#[non_exhaustive]` — a verb added after WIRE_VERSION 2.
            _ => Err(HostError::Unsupported),
        }
    }

    /// `Created → Initialized`: validate first (`InitFailed` is only
    /// legal from `Created`), run the hook, then commit.
    fn initialize(&mut self) -> Result<WireReply, HostError> {
        self.lifecycle
            .state()
            .transition(LifecycleEvent::Initialize)
            .map_err(HostError::Lifecycle)?;
        match Self::guarded(&mut *self.instance, |plugin| plugin.init()) {
            Ok(Ok(())) => {
                self.lifecycle
                    .transition(LifecycleEvent::Initialize)
                    .map_err(HostError::Lifecycle)?;
                Ok(WireReply::Ok)
            }
            Ok(Err(error)) => {
                let _ = self.lifecycle.transition(LifecycleEvent::InitFailed);
                Err(error)
            }
            Err(panic) => {
                let _ = self.lifecycle.transition(LifecycleEvent::InitFailed);
                Err(panic)
            }
        }
    }

    /// `Initialized → Activated`: validate, bound-check the config
    /// against the wire ceiling, run `prepare`, then commit.
    fn activate(&mut self, config: &ProcessConfig) -> Result<WireReply, HostError> {
        self.lifecycle
            .state()
            .transition(LifecycleEvent::Activate)
            .map_err(HostError::Lifecycle)?;
        if config.max_frames() > wire::WIRE_MAX_FRAMES {
            let _ = self.lifecycle.transition(LifecycleEvent::ActivateFailed);
            return Err(HostError::Prepare(PrepareError::UnsupportedConfig));
        }
        let context = &mut self.context;
        let prepared = Self::guarded(&mut *self.instance, |plugin| {
            plugin.prepare(config, context)
        });
        match prepared {
            Ok(Ok(())) => {
                self.lifecycle
                    .transition(LifecycleEvent::Activate)
                    .map_err(HostError::Lifecycle)?;
                self.activated_max_frames = Some(config.max_frames());
                Ok(WireReply::Ok)
            }
            Ok(Err(error)) => {
                let _ = self.lifecycle.transition(LifecycleEvent::ActivateFailed);
                Err(HostError::Prepare(error))
            }
            Err(panic) => {
                let _ = self.lifecycle.transition(LifecycleEvent::ActivateFailed);
                Err(panic)
            }
        }
    }

    /// `Activated → Processing`: commit first; a hook failure rolls
    /// back to `Activated`.
    fn start_processing(&mut self) -> Result<WireReply, HostError> {
        self.lifecycle
            .transition(LifecycleEvent::StartProcessing)
            .map_err(HostError::Lifecycle)?;
        match Self::guarded(&mut *self.instance, |plugin| plugin.start_processing()) {
            Ok(Ok(())) => Ok(WireReply::Ok),
            Ok(Err(error)) => {
                let _ = self.lifecycle.transition(LifecycleEvent::StopProcessing);
                Err(error)
            }
            Err(panic) => {
                let _ = self.lifecycle.transition(LifecycleEvent::StopProcessing);
                Err(panic)
            }
        }
    }

    /// `Processing → Activated`: commit first — a hook failure is
    /// advisory (the lifecycle has already left `Processing`).
    fn stop_processing(&mut self) -> Result<WireReply, HostError> {
        self.lifecycle
            .transition(LifecycleEvent::StopProcessing)
            .map_err(HostError::Lifecycle)?;
        match Self::guarded(&mut *self.instance, |plugin| plugin.stop_processing()) {
            Ok(Ok(())) => Ok(WireReply::Ok),
            Ok(Err(error)) => Err(error),
            Err(panic) => Err(panic),
        }
    }

    /// Runs a plugin hook catching unwinds; a panic maps to
    /// `Err(HostError::PluginFailed)` — the child process survives.
    /// The instance is an explicit parameter so callers can hold
    /// disjoint `Runner` field borrows (e.g. `context`).
    /// `AssertUnwindSafe`: a panicking hook may leave the plugin in an
    /// inconsistent state — that is the documented `PluginFailed`
    /// contract, and the runner keeps dispatching afterwards.
    fn guarded<R>(
        instance: &mut dyn ErasedPlugin,
        hook: impl FnOnce(&mut dyn ErasedPlugin) -> R,
    ) -> Result<R, HostError> {
        catch_unwind(AssertUnwindSafe(|| hook(instance))).map_err(|_| HostError::PluginFailed)
    }

    /// `BlockIn { seq, instance }`: the `process` equivalent.
    /// `Lifecycle` must accept `Process` (i.e. be in `Processing`);
    /// the in-slot is parsed and validated, events/transport decoded,
    /// samples staged, `process_host` run under the panic guard and
    /// the out-slot written. Every rejection maps to the
    /// `Err(HostError)` the caller wraps in a wire `Reply`.
    /// `events_scratch` is the shared decode buffer the parent
    /// `Runner` owns (one block is in flight at a time).
    fn process_block(
        &mut self,
        seq: u64,
        events_scratch: &mut Vec<Event>,
    ) -> Result<ProcessResult, HostError> {
        self.lifecycle
            .transition(LifecycleEvent::Process)
            .map_err(HostError::Lifecycle)?;

        // Immutable segment borrow scope: parse and validate the
        // in-slot, decode transport/events and stage the samples.
        let (frames, channels, events, transport) = {
            let slot = self.segment.in_slot(seq).map_err(|_| HostError::Ipc)?;
            if self
                .activated_max_frames
                .is_some_and(|max| slot.frames > max)
            {
                return Err(HostError::Ipc);
            }
            let transport = if slot.flags & wire::shm::FLAG_TRANSPORT != 0 {
                Some(
                    wire::decode_transport(&mut wire::Reader::new(slot.transport))
                        .map_err(|_| HostError::Ipc)?,
                )
            } else {
                None
            };
            events_scratch.clear();
            let mut reader = wire::Reader::new(slot.events);
            for _ in 0..slot.event_count {
                events_scratch.push(wire::decode_event(&mut reader).map_err(|_| HostError::Ipc)?);
            }
            let frames = slot.frames as usize;
            let channels = slot.channels as usize;
            self.staging_in.resize(channels * frames, 0.0);
            for channel in 0..channels {
                let base = channel * wire::shm::SAMPLE_STRIDE;
                let source = &slot.samples[base..base + frames * 4];
                let target = &mut self.staging_in[channel * frames..(channel + 1) * frames];
                for (sample, bytes) in target.iter_mut().zip(source.as_chunks::<4>().0) {
                    *sample = f32::from_le_bytes(*bytes);
                }
            }
            let events = EventList::new(events_scratch, slot.frames).map_err(|_| HostError::Ipc)?;
            (slot.frames, slot.channels, events, transport)
        };

        let frames_usize = frames as usize;
        let channels_usize = channels as usize;
        self.staging_out.resize(channels_usize * frames_usize, 0.0);
        // `resize` only zeroes *grown* elements — re-zero the whole
        // buffer so a plugin under-writing its out-view cannot leak
        // the previous block's samples into the out-slot.
        self.staging_out.fill(0.0);
        let in_view = SlotInView {
            channels: channels_usize,
            frames: frames_usize,
            data: &self.staging_in,
        };
        let mut out_view = SlotOutView {
            channels: channels_usize,
            frames: frames_usize,
            data: &mut self.staging_out,
        };
        let inputs = [HostInput(&in_view)];
        let mut outputs = [HostOutput(&mut out_view)];

        // Disjoint field borrows: `plugin`/`context`/`in`/`out`/events
        // scratch never alias, so the unwind guard can hold them all.
        let plugin = &mut *self.instance;
        let context = &mut self.context;
        let Ok(result) = catch_unwind(AssertUnwindSafe(|| {
            plugin.process_host(&inputs, &mut outputs, events, transport, context)
        })) else {
            return Err(HostError::PluginFailed);
        };
        self.segment
            .write_out_slot(seq, frames, channels, result, &self.staging_out);
        Ok(result)
    }

    /// Best-effort lifecycle normalization before exit (`Shutdown` /
    /// stdin EOF / fatal error): `Processing → Activated → Initialized
    /// → Destroyed`, as far as the state machine allows. Hook errors
    /// and panics are ignored — teardown never fails.
    fn normalize(&mut self) {
        if self.lifecycle.state() == LifecycleState::Processing {
            let _ = self.lifecycle.transition(LifecycleEvent::StopProcessing);
            let _ = Self::guarded(&mut *self.instance, |plugin| plugin.stop_processing());
        }
        if self.lifecycle.state() == LifecycleState::Activated {
            let _ = self.lifecycle.transition(LifecycleEvent::Deactivate);
            self.activated_max_frames = None;
        }
        if matches!(
            self.lifecycle.state(),
            LifecycleState::Created | LifecycleState::Initialized | LifecycleState::Failed
        ) {
            let _ = self.lifecycle.transition(LifecycleEvent::Destroy);
        }
    }
}

/// Borrowed `BufferView` over the channel-major input staging.
struct SlotInView<'a> {
    channels: usize,
    frames: usize,
    data: &'a [f32],
}

impl BufferView for SlotInView<'_> {
    fn channels(&self) -> usize {
        self.channels
    }
    fn frames(&self) -> usize {
        self.frames
    }
    fn channel(&self, index: usize) -> Option<&[f32]> {
        (index < self.channels).then(|| &self.data[index * self.frames..(index + 1) * self.frames])
    }
}

/// Borrowed `BufferViewMut` over the channel-major output staging.
struct SlotOutView<'a> {
    channels: usize,
    frames: usize,
    data: &'a mut [f32],
}

impl BufferView for SlotOutView<'_> {
    fn channels(&self) -> usize {
        self.channels
    }
    fn frames(&self) -> usize {
        self.frames
    }
    fn channel(&self, index: usize) -> Option<&[f32]> {
        (index < self.channels).then(|| &self.data[index * self.frames..(index + 1) * self.frames])
    }
}

impl BufferViewMut for SlotOutView<'_> {
    fn channel_mut(&mut self, index: usize) -> Option<&mut [f32]> {
        if index < self.channels {
            Some(&mut self.data[index * self.frames..(index + 1) * self.frames])
        } else {
            None
        }
    }
}
