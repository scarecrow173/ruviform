//! `ruviform-plugin-export` — the plugin-side (production-side) L1
//! out-of-process runner of the Ruviform media foundation (Issue #76;
//! wire-v2 instance registry per Issue #117; wire-v3 video path per
//! Issue #118).
//!
//! The crate is a thin library with two public entries: [`run`]
//! (audio) and [`run_video`] (video, wire v3). A plugin author's (or
//! adapter's) binary calls the matching one from `main` with the
//! built-in factories, and the child process then serves plugin
//! instances to the parent host over the L1 protocol. On the parent
//! side the binary is registered as
//! `ruviform_plugin_host::OutOfProcessSpec::program` (audio) or the
//! video spec counterpart — the `PluginHost` composite spawns it and
//! registers instances inside it.
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
//! The video runner ([`run_video`]) serves the same framed pipe and
//! registry contract with `CreateVideoInstance` in place of
//! `CreateInstance` — the create payload carries the
//! `VideoProcessConfig` the instance's `vshm` segment (one `in` + one
//! `out` frame region, see the internal `vshm` module) was sized
//! from. The video channel is purely message-driven — `VideoRender`
//! carries its frame descriptor inside the command — so a video
//! child owns no doorbells: an inbound `BlockIn` is direction-legal
//! foreign traffic and is dropped silently.
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
//! The crate contains no `unsafe` outside the scoped
//! `shared_memory` slice accessors in the internal `shm`/`vshm`
//! modules (two each) — see their deviation notes (the design assumed
//! safe accessors; `shared_memory` 0.12.4's are `unsafe fn`).
//!
//! **RT contract:** every item here is **non-RT only** — the runner is
//! the non-RT end of the L1 link; the host's RT leg lives in
//! `ruviform-plugin-host`.
//!
//! Contract source: `docs/designs/plugin-l1-out-of-process.md` (Issue
//! #76 design), `docs/specs/plugin-l1-out-of-process.md`,
//! `docs/designs/issue-117-bundle-child-process.md` (wire v2),
//! `docs/designs/issue-118-video-l1.md` (wire v3 video).

use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::process::ExitCode;
use std::sync::atomic::{Ordering, fence};

use ruviform_media_core::MediaTime;
use ruviform_plugin_core::wire::{self, Command as WireCommand, Message, Reply as WireReply};
use ruviform_plugin_core::{
    BufferView, BufferViewMut, ErasedPlugin, ErasedVideoPlugin, Event, EventList, HostError,
    HostInput, HostOutput, Lifecycle, LifecycleEvent, LifecycleState, ParamId, PlainParamValue,
    PluginFactory, PrepareError, ProcessConfig, ProcessContext, ProcessResult, VideoPluginFactory,
    VideoProcessConfig,
};
use ruviform_video_core::VideoFrame;

mod shm;
mod vshm;

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
    let stdout = io::stdout();
    let mut output = stdout.lock();
    let mut encode_buf = match boot(&mut output) {
        Ok(scratch) => scratch,
        Err(code) => return code,
    };

    let mut runner = Runner {
        factories,
        instances: HashMap::new(),
        events_scratch: Vec::new(),
    };

    let stdin = io::stdin();
    let mut input = stdin.lock();
    serve(&mut runner, &mut input, &mut output, &mut encode_buf)
}

/// The child-process entry point of the **video** runner — the
/// [`run`] counterpart a video plugin binary calls from `main` and
/// returns as the process exit code (wire v3, Issue #118).
///
/// `factories` are the video plugin types built into the binary; the
/// child registers instances of them as `CreateVideoInstance`
/// commands arrive — the create payload carries the
/// `VideoProcessConfig` whose `initialize`-time replay and per-
/// instance `vshm` sizing the registry then owns. The framed pipe,
/// `Hello`/`Scan`/`Shutdown` handshake, `argv` rejection, exit
/// semantics, panic normalization and teardown are [`run`]'s
/// verbatim; the differences are the verb set (video tags) and the
/// absent doorbell data plane — see `RunnerCore`.
///
/// # Examples
///
/// ```no_run
/// // In a video plugin binary's `main`:
/// fn main() -> std::process::ExitCode {
///     ruviform_plugin_export::run_video(vec![])
/// }
/// ```
#[must_use]
pub fn run_video(factories: Vec<Box<dyn VideoPluginFactory>>) -> ExitCode {
    let stdout = io::stdout();
    let mut output = stdout.lock();
    let mut encode_buf = match boot(&mut output) {
        Ok(scratch) => scratch,
        Err(code) => return code,
    };

    let mut runner = VideoRunner {
        factories,
        instances: HashMap::new(),
    };

    let stdin = io::stdin();
    let mut input = stdin.lock();
    serve(&mut runner, &mut input, &mut output, &mut encode_buf)
}

/// The shared boot contract both runners open with — wire v3: no
/// bootstrap arguments (the retired `--ruviform-*` flags moved into
/// the create command payloads, and a `run*()`-based runner accepts
/// no user args either; any argv entry is the defined usage
/// rejection), then the `Hello` write — the first frame a child
/// emits, before any create lands, so a peer sees a live handshake
/// even while the registry is still empty. On success this returns
/// the shared encode scratch [`serve`] reuses; on failure the early
/// process exit code.
fn boot(output: &mut impl Write) -> Result<Vec<u8>, ExitCode> {
    if std::env::args_os().nth(1).is_some() {
        eprintln!("ruviform-plugin-export: unexpected arguments\n{USAGE}");
        return Err(ExitCode::from(EXIT_USAGE));
    }
    let mut encode_buf = Vec::new();
    if wire::write_message(
        output,
        &Message::Hello {
            version: wire::WIRE_VERSION,
            capabilities: 0,
        },
        &mut encode_buf,
    )
    .is_err()
    {
        return Err(ExitCode::FAILURE);
    }
    Ok(encode_buf)
}

/// What a kind-specific runner supplies the shared [`serve`] loop:
/// command dispatch, inbound-doorbell handling and teardown
/// normalization. [`Runner`] (audio) and [`VideoRunner`] implement
/// it, so framing, replies and the protocol-violation exits stay
/// identical across kinds.
trait RunnerCore {
    /// Resolves the envelope `instance` and dispatches `command`.
    fn dispatch(&mut self, instance: u32, command: WireCommand) -> Result<WireReply, HostError>;
    /// Handles one inbound `BlockIn` doorbell — `Some(reply)` to
    /// answer it (`BlockDone` on success, `Reply{seq, Err}` on
    /// rejection) or `None` to drop it: a kind with no doorbell data
    /// plane treats the direction-legal `BlockIn` as unattributable
    /// foreign traffic, not a protocol violation.
    fn block_in(&mut self, instance: u32, seq: u64) -> Option<Message>;
    /// Best-effort lifecycle normalization of every live entry.
    fn normalize_all(&mut self);
}

/// The framed stdin/stdout command loop. `encode_buf` is the shared
/// scratch buffer the caller already used for `Hello` — one
/// allocation reused for every outbound frame. Returns the process
/// exit code: `SUCCESS` on `Shutdown` or a clean EOF at a frame
/// boundary, `FAILURE` on any transport/protocol violation (after
/// best-effort lifecycle normalization in both cases).
fn serve(
    runner: &mut impl RunnerCore,
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
            // runner echoes the seq and instance verbatim —
            // `BlockDone` on success, `Reply{seq, Err}` on rejection
            // — or drops the doorbell (`block_in` → `None`, the
            // video kind, which owns no doorbell data plane): no
            // child-side partition work is needed either way.
            Message::BlockIn { seq, instance } => {
                if let Some(reply) = runner.block_in(instance, seq)
                    && wire::write_message(output, &reply, encode_buf).is_err()
                {
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

impl RunnerCore for Runner {
    fn dispatch(&mut self, instance: u32, command: WireCommand) -> Result<WireReply, HostError> {
        Runner::dispatch(self, instance, command)
    }

    fn block_in(&mut self, instance: u32, seq: u64) -> Option<Message> {
        // The doorbell's release edge on the writer side is paired
        // here before touching shm — a real atomic fence
        // (`std::sync::atomic::fence`), not a compiler fence:
        // ordering across the process boundary rests on the fence
        // plus the pipe-read syscall.
        fence(Ordering::Acquire);
        Some(match self.process_block(instance, seq) {
            Ok(result) => {
                // Publish the out-slot before the doorbell — same
                // real-fence Release edge.
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
        })
    }

    fn normalize_all(&mut self) {
        Runner::normalize_all(self);
    }
}

/// Runs a plugin hook catching unwinds; a panic maps to
/// `Err(HostError::PluginFailed)` — the child process survives. The
/// instance is an explicit parameter so callers can hold disjoint
/// field borrows (e.g. `context`, `segment`). `AssertUnwindSafe`: a
/// panicking hook may leave the plugin in an inconsistent state —
/// that is the documented `PluginFailed` contract, and the runner
/// keeps dispatching afterwards.
fn guarded<R, P: ?Sized>(plugin: &mut P, hook: impl FnOnce(&mut P) -> R) -> Result<R, HostError> {
    catch_unwind(AssertUnwindSafe(|| hook(plugin))).map_err(|_| HostError::PluginFailed)
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
            WireCommand::SaveState => guarded(&mut *self.instance, ErasedPlugin::save_state)
                .and_then(|inner| inner)
                .map(|bytes| WireReply::State { bytes }),
            WireCommand::RestoreState { bytes } => {
                guarded(&mut *self.instance, |plugin| plugin.restore_state(&bytes))
                    .and_then(|inner| inner)
                    .map(|()| WireReply::Ok)
            }
            WireCommand::OpenGui { parent } => {
                guarded(&mut *self.instance, |plugin| plugin.open_gui(parent))
                    .and_then(|inner| inner)
                    .map(|gui| WireReply::Opened { gui })
            }
            WireCommand::CloseGui => guarded(&mut *self.instance, ErasedPlugin::close_gui)
                .and_then(|inner| inner)
                .map(|()| WireReply::Ok),
            // `#[non_exhaustive]` — the wire-v3 video verbs are the
            // cross-kind defined rejection on an audio entry, as is
            // any verb a later WIRE_VERSION adds.
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
        match guarded(&mut *self.instance, ErasedPlugin::init) {
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
        let prepared = guarded(&mut *self.instance, |plugin| {
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
        match guarded(&mut *self.instance, ErasedPlugin::start_processing) {
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
        match guarded(&mut *self.instance, ErasedPlugin::stop_processing) {
            Ok(Ok(())) => Ok(WireReply::Ok),
            Ok(Err(error)) => Err(error),
            Err(panic) => Err(panic),
        }
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
            let _ = guarded(&mut *self.instance, ErasedPlugin::stop_processing);
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

/// The video message-loop state (wire v3, Issue #118): the video
/// factories plus the live video-instance registry — the [`Runner`]
/// counterpart for the `VideoPluginInstance` kind.
struct VideoRunner {
    factories: Vec<Box<dyn VideoPluginFactory>>,
    /// Live video instances by their wire `instance` envelope handle
    /// — same `WIRE_MAX_INSTANCES` capacity contract as the audio
    /// registry, same codec-enforced key range.
    instances: HashMap<u32, VideoEntry>,
}

/// One registered video instance: the plugin object, its lifecycle
/// mirror, the `CreateVideoInstance`-stashed config — replayed into
/// `initialize` on `VideoInitialize`, and the geometry every
/// `VideoRender` descriptor is checked against — and its attached
/// `vshm` segment.
struct VideoEntry {
    instance: Box<dyn ErasedVideoPlugin>,
    lifecycle: Lifecycle,
    config: VideoProcessConfig,
    segment: vshm::Segment,
}

impl RunnerCore for VideoRunner {
    /// `Scan`/registry verbs are runner-level; every other command
    /// resolves the envelope `instance` to its entry — an
    /// unregistered handle is `Err(HostError::InvalidHandle)`,
    /// never a dead child. That `get_mut` fallthrough is also what
    /// turns cross-kind traffic into the defined rejections for
    /// free: the audio `CreateInstance` lands here (unregistered →
    /// `InvalidHandle`, registered → the entry catch-all's
    /// `Unsupported`), exactly as `CreateVideoInstance` does on an
    /// audio runner.
    fn dispatch(&mut self, instance: u32, command: WireCommand) -> Result<WireReply, HostError> {
        match command {
            WireCommand::Scan => self.scan(),
            WireCommand::CreateVideoInstance {
                plugin_id,
                shm_key,
                config,
            } => self.create_video_instance(instance, &plugin_id, &shm_key, &config),
            WireCommand::DestroyInstance => self.destroy_instance(instance),
            instance_command => match self.instances.get_mut(&instance) {
                Some(entry) => entry.dispatch(instance_command),
                None => Err(HostError::InvalidHandle),
            },
        }
    }

    fn block_in(&mut self, _instance: u32, _seq: u64) -> Option<Message> {
        // A video child owns no doorbell data plane: an inbound
        // `BlockIn` is direction-legal but unattributable foreign
        // traffic — dropped silently, never a protocol violation.
        None
    }

    fn normalize_all(&mut self) {
        for entry in self.instances.values_mut() {
            entry.normalize();
        }
    }
}

impl VideoRunner {
    /// `Scan`: enumerate every video factory's descriptor →
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

    /// `CreateVideoInstance`: the audio create's step order —
    /// plugin id, capacity, handle freshness, segment attach (the
    /// vshm two-layer validation inside [`vshm::Segment::attach`]),
    /// then `create()`. Every failure is a defined `Err` that
    /// leaves the registry untouched and keeps the child alive.
    fn create_video_instance(
        &mut self,
        instance: u32,
        plugin_id: &str,
        shm_key: &str,
        config: &VideoProcessConfig,
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
        let segment = vshm::Segment::attach(shm_key, config).map_err(|_| HostError::Ipc)?;
        let plugin = catch_unwind(AssertUnwindSafe(|| self.factories[index].create()))
            .map_err(|_| HostError::PluginFailed)?;
        self.instances.insert(
            instance,
            VideoEntry {
                instance: plugin,
                lifecycle: Lifecycle::new(),
                config: *config,
                segment,
            },
        );
        Ok(WireReply::Ok)
    }

    /// `DestroyInstance`: normalize the entry's lifecycle, detach
    /// its segment (drop) and remove the handle → `Reply::Ok`. An
    /// unregistered handle is `Err(InvalidHandle)`.
    fn destroy_instance(&mut self, instance: u32) -> Result<WireReply, HostError> {
        let Some(mut entry) = self.instances.remove(&instance) else {
            return Err(HostError::InvalidHandle);
        };
        entry.normalize();
        Ok(WireReply::Ok)
    }
}

impl VideoEntry {
    /// Dispatches one instance-scoped video `wire::Command`,
    /// mirroring the audio entry's lifecycle semantics. Plugin hooks
    /// run under the shared panic guard: a panic becomes
    /// `Err(HostError::PluginFailed)` and never kills the child.
    /// Audio verbs on a video entry hit the catch-all — the
    /// cross-kind defined rejection.
    fn dispatch(&mut self, command: WireCommand) -> Result<WireReply, HostError> {
        match command {
            WireCommand::VideoInitialize => self.initialize(),
            WireCommand::VideoActivate => self.activate(),
            WireCommand::VideoStartProcessing => self.start_processing(),
            WireCommand::VideoRender { at, input } => self.render(at, &input),
            WireCommand::VideoStopProcessing => self.stop_processing(),
            WireCommand::VideoDeactivate => self.deactivate(),
            WireCommand::VideoGetParams => self.get_params(),
            WireCommand::VideoGetParamInfo { id } => self.get_param_info(ParamId(id)),
            WireCommand::VideoGetParamValue { id } => self.get_param_value(ParamId(id)),
            WireCommand::VideoSetParamValue { id, value } => {
                self.set_param_value(ParamId(id), value)
            }
            // `#[non_exhaustive]` — the audio verbs are the
            // cross-kind defined rejection on a video entry, as is
            // any verb a later WIRE_VERSION adds.
            _ => Err(HostError::Unsupported),
        }
    }

    /// `Created → Initialized`: validate first (`InitFailed` is only
    /// legal from `Created`), replay the stashed `CreateVideoInstance`
    /// config into the `initialize` hook, then commit.
    fn initialize(&mut self) -> Result<WireReply, HostError> {
        self.lifecycle
            .state()
            .transition(LifecycleEvent::Initialize)
            .map_err(HostError::Lifecycle)?;
        let config = self.config;
        match guarded(&mut *self.instance, |plugin| plugin.initialize(&config)) {
            Ok(Ok(())) => {
                self.lifecycle
                    .transition(LifecycleEvent::Initialize)
                    .map_err(HostError::Lifecycle)?;
                Ok(WireReply::Ok)
            }
            Ok(Err(error)) | Err(error) => {
                let _ = self.lifecycle.transition(LifecycleEvent::InitFailed);
                Err(error)
            }
        }
    }

    /// `Initialized → Activated`: validate, run the hook, then
    /// commit — `ActivateFailed` reports a hook failure, as in the
    /// audio entry.
    fn activate(&mut self) -> Result<WireReply, HostError> {
        self.lifecycle
            .state()
            .transition(LifecycleEvent::Activate)
            .map_err(HostError::Lifecycle)?;
        match guarded(&mut *self.instance, ErasedVideoPlugin::activate) {
            Ok(Ok(())) => {
                self.lifecycle
                    .transition(LifecycleEvent::Activate)
                    .map_err(HostError::Lifecycle)?;
                Ok(WireReply::Ok)
            }
            Ok(Err(error)) | Err(error) => {
                let _ = self.lifecycle.transition(LifecycleEvent::ActivateFailed);
                Err(error)
            }
        }
    }

    /// `Activated → Processing`: commit first; a hook failure rolls
    /// back to `Activated`.
    fn start_processing(&mut self) -> Result<WireReply, HostError> {
        self.lifecycle
            .transition(LifecycleEvent::StartProcessing)
            .map_err(HostError::Lifecycle)?;
        match guarded(&mut *self.instance, ErasedVideoPlugin::start_processing) {
            Ok(Ok(())) => Ok(WireReply::Ok),
            Ok(Err(error)) | Err(error) => {
                let _ = self.lifecycle.transition(LifecycleEvent::StopProcessing);
                Err(error)
            }
        }
    }

    /// `Processing → Activated`: commit first — a hook failure is
    /// advisory (the lifecycle has already left `Processing`).
    fn stop_processing(&mut self) -> Result<WireReply, HostError> {
        self.lifecycle
            .transition(LifecycleEvent::StopProcessing)
            .map_err(HostError::Lifecycle)?;
        match guarded(&mut *self.instance, ErasedVideoPlugin::stop_processing) {
            Ok(Ok(())) => Ok(WireReply::Ok),
            Ok(Err(error)) | Err(error) => Err(error),
        }
    }

    /// `Activated → Initialized`: commit first — the same
    /// commit-then-advisory shape as `stop_processing`: the mirror
    /// stays `Initialized` even when the video `deactivate` hook
    /// reports a failure; the hook's `Err` is the reply payload.
    fn deactivate(&mut self) -> Result<WireReply, HostError> {
        self.lifecycle
            .transition(LifecycleEvent::Deactivate)
            .map_err(HostError::Lifecycle)?;
        match guarded(&mut *self.instance, ErasedVideoPlugin::deactivate) {
            Ok(Ok(())) => Ok(WireReply::Ok),
            Ok(Err(error)) | Err(error) => Err(error),
        }
    }

    /// `VideoRender{at, input}`: `Processing` gate first, then the
    /// descriptor's post-decode semantic check, the `in`-region
    /// frame rebuild, the metadata-seeded output frame and the hook
    /// — success publishes the `out` region and answers `Rendered`
    /// with the runner-derived canonical descriptor; a hook failure
    /// or panic commits nothing to `out`.
    fn render(
        &mut self,
        at: MediaTime,
        desc: &wire::VideoFrameDesc,
    ) -> Result<WireReply, HostError> {
        self.lifecycle
            .transition(LifecycleEvent::Process)
            .map_err(HostError::Lifecycle)?;
        check_input_desc(desc, &self.config, self.segment.in_bytes())?;
        let input_frame = self.input_frame(desc)?;
        // Output seeded with the input descriptor's metadata — a
        // plugin that never touches frame metadata (the OFX
        // pass-through default) still echoes pts/duration/color.
        let mut output_frame = VideoFrame::new_owned(
            self.config.format(),
            self.config.width(),
            self.config.height(),
            desc.pts,
        )
        .map_err(|_| HostError::Ipc)?;
        output_frame.set_duration(desc.duration);
        output_frame.set_color(desc.color);
        guarded(&mut *self.instance, |plugin| {
            plugin.render(&input_frame, &mut output_frame, at)
        })??;
        self.write_output(&output_frame)?;
        Ok(WireReply::Rendered {
            output: output_desc(&output_frame),
        })
    }

    /// Rebuilds the `VideoFrame` `desc` describes out of the `in`
    /// region — a fresh owned frame (`new_owned`, tightly packed)
    /// filled plane-by-plane. `check_input_desc` already pinned
    /// canonical placement and region fit, so every slice bound and
    /// the `data_mut`/`source` length agreement hold.
    fn input_frame(&self, desc: &wire::VideoFrameDesc) -> Result<VideoFrame, HostError> {
        let mut frame = VideoFrame::new_owned(desc.format, desc.width, desc.height, desc.pts)
            .map_err(|_| HostError::Ipc)?;
        frame.set_duration(desc.duration);
        frame.set_color(desc.color);
        // The doorbell-free equivalent of the audio `BlockIn` acquire
        // edge: the host's release side is its pipe write, so a real
        // fence plus the pipe read orders the frame bytes we are
        // about to read.
        fence(Ordering::Acquire);
        let region = self.segment.in_region();
        for (index, declared) in desc.planes.iter().enumerate() {
            let start = declared.offset as usize;
            let end = start + declared.len as usize;
            let source = &region[start..end];
            let mut plane = frame.plane_mut(index).ok_or(HostError::Ipc)?;
            let target = plane.data_mut();
            if target.len() != source.len() {
                return Err(HostError::Ipc);
            }
            target.copy_from_slice(source);
        }
        Ok(frame)
    }

    /// Writes `frame` into the `out` region at canonical placement —
    /// the mirror of [`Self::input_frame`]. The segment pinned
    /// `out_bytes == frame_bytes(config)` at attach and `frame` is
    /// config-shaped, so every plane lands inside the region; the
    /// trailing release fence publishes the bytes before the
    /// `Rendered` reply can reach the pipe.
    fn write_output(&mut self, frame: &VideoFrame) -> Result<(), HostError> {
        let region = self.segment.out_region_mut();
        for index in 0..frame.plane_count() {
            let layout =
                wire::vshm::plane_layout(frame.format(), frame.width(), frame.height(), index)
                    .ok_or(HostError::Ipc)?;
            let start = usize::try_from(layout.offset).map_err(|_| HostError::Ipc)?;
            let end = usize::try_from(layout.offset + layout.len).map_err(|_| HostError::Ipc)?;
            let source = frame.plane(index).ok_or(HostError::Ipc)?.data();
            let target = region.get_mut(start..end).ok_or(HostError::Ipc)?;
            if target.len() != source.len() {
                return Err(HostError::Ipc);
            }
            target.copy_from_slice(source);
        }
        // Publish the out region before the `Rendered` reply — the
        // same real-fence Release edge as the audio doorbell.
        fence(Ordering::Release);
        Ok(())
    }

    /// `VideoGetParams`: the plugin's id list — encode obligation
    /// (c) already bounds it inside the erased surface →
    /// `Reply::Params`.
    fn get_params(&mut self) -> Result<WireReply, HostError> {
        let ids = guarded(&mut *self.instance, |plugin| {
            plugin
                .parameters()
                .map(|ids| ids.iter().map(|id| id.0).collect::<Vec<u32>>())
        })??;
        Ok(WireReply::Params { ids })
    }

    /// `VideoGetParamInfo`: the `parameters()` membership check
    /// first — an unknown id is the plugin-declared
    /// `ParamInfo{info: None}` without ever invoking `parameter()` —
    /// then the `Option` contract passes through verbatim.
    fn get_param_info(&mut self, id: ParamId) -> Result<WireReply, HostError> {
        if !self.has_param(id)? {
            return Ok(WireReply::ParamInfo { info: None });
        }
        let info = guarded(&mut *self.instance, |plugin| plugin.parameter(id))??;
        Ok(WireReply::ParamInfo { info })
    }

    /// `VideoGetParamValue`: unknown id → `Err(InvalidValue)` (the
    /// value-domain defined rejection — not folded into
    /// `PluginFailed`), then `parameter_value` under the guard →
    /// `Reply::ParamValue`.
    fn get_param_value(&mut self, id: ParamId) -> Result<WireReply, HostError> {
        if !self.has_param(id)? {
            return Err(HostError::InvalidValue);
        }
        let value = guarded(&mut *self.instance, |plugin| plugin.parameter_value(id))??;
        Ok(WireReply::ParamValue { value })
    }

    /// `VideoSetParamValue`: unknown id → `Err(InvalidValue)`, then
    /// `set_parameter` under the guard → `Reply::Ok`. The wire
    /// decode already rejected a non-finite `Double`, so no
    /// finiteness re-check lives here.
    fn set_param_value(
        &mut self,
        id: ParamId,
        value: PlainParamValue,
    ) -> Result<WireReply, HostError> {
        if !self.has_param(id)? {
            return Err(HostError::InvalidValue);
        }
        guarded(&mut *self.instance, |plugin| {
            plugin.set_parameter(id, value)
        })??;
        Ok(WireReply::Ok)
    }

    /// The `id ∈ parameters()` membership check every parameter verb
    /// shares — plugin code, so under the same panic guard; the
    /// erased surface's obligation (c) `Err(PluginFailed)`
    /// propagates.
    fn has_param(&mut self, id: ParamId) -> Result<bool, HostError> {
        guarded(&mut *self.instance, |plugin| {
            plugin.parameters().map(|ids| ids.contains(&id))
        })?
    }

    /// Best-effort lifecycle normalization before exit (`Shutdown` /
    /// stdin EOF / fatal error) — the audio entry's
    /// `Processing → Activated → Initialized → Destroyed` walk,
    /// firing the video `stop_processing`/`deactivate`/`destroy`
    /// hooks as it goes. Hook errors and panics are ignored —
    /// teardown never fails.
    fn normalize(&mut self) {
        if self.lifecycle.state() == LifecycleState::Processing {
            let _ = self.lifecycle.transition(LifecycleEvent::StopProcessing);
            let _ = guarded(&mut *self.instance, ErasedVideoPlugin::stop_processing);
        }
        if self.lifecycle.state() == LifecycleState::Activated {
            let _ = self.lifecycle.transition(LifecycleEvent::Deactivate);
            let _ = guarded(&mut *self.instance, ErasedVideoPlugin::deactivate);
        }
        if matches!(
            self.lifecycle.state(),
            LifecycleState::Created | LifecycleState::Initialized | LifecycleState::Failed
        ) {
            let _ = self.lifecycle.transition(LifecycleEvent::Destroy);
            let _ = guarded(&mut *self.instance, ErasedVideoPlugin::destroy);
        }
    }
}

/// `VideoRender.input`'s post-decode semantic check — the checks the
/// codec bounds layer cannot express: every declared plane must sit
/// in canonical placement (`stride == row_bytes`, `offset(p) ==
/// Σlen(0..p)`, `len` the canonical plane size) inside the `in`
/// region, and the descriptor's dims/format must equal the instance
/// config's. A non-canonical or overflowing descriptor is
/// `Err(HostError::Ipc)` — a host contract violation in the same
/// class as a duplicate handle — while a config dims/format mismatch
/// is `Err(HostError::Prepare(PrepareError::UnsupportedConfig))`,
/// the audio `Activate` bound rejection's mapping. Both are `Err`
/// replies that leave the child alive.
fn check_input_desc(
    desc: &wire::VideoFrameDesc,
    config: &VideoProcessConfig,
    region_bytes: usize,
) -> Result<(), HostError> {
    for (index, declared) in desc.planes.iter().enumerate() {
        let layout = wire::vshm::plane_layout(desc.format, desc.width, desc.height, index)
            .ok_or(HostError::Ipc)?;
        if u64::from(declared.stride) != layout.stride
            || u64::from(declared.offset) != layout.offset
            || u64::from(declared.len) != layout.len
            || u64::from(declared.offset) + u64::from(declared.len) > region_bytes as u64
        {
            return Err(HostError::Ipc);
        }
    }
    if desc.format != config.format()
        || desc.width != config.width()
        || desc.height != config.height()
    {
        return Err(HostError::Prepare(PrepareError::UnsupportedConfig));
    }
    Ok(())
}

/// The canonical descriptor of the frame just written to the `out`
/// region — runner-derived metadata, never the plugin's own report:
/// the frame's dims/format, canonical plane placement, and its final
/// pts/duration/color (input-seeded, possibly hook-adjusted).
fn output_desc(frame: &VideoFrame) -> wire::VideoFrameDesc {
    let planes = (0..frame.plane_count())
        .map(|index| {
            let layout =
                wire::vshm::plane_layout(frame.format(), frame.width(), frame.height(), index)
                    .expect("canonical layout exists for every frame plane");
            wire::VideoFramePlane {
                offset: u32::try_from(layout.offset).expect("canonical offset fits u32"),
                len: u32::try_from(layout.len).expect("canonical len fits u32"),
                stride: u32::try_from(layout.stride).expect("canonical stride fits u32"),
            }
        })
        .collect();
    wire::VideoFrameDesc {
        format: frame.format(),
        width: frame.width(),
        height: frame.height(),
        planes,
        pts: frame.pts(),
        duration: frame.duration(),
        color: frame.color(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruviform_media_core::{FrameRate, MediaDuration, TimeBase};
    use ruviform_video_core::{ColorInfo, ColorPrimaries, PixelFormat, TransferCharacteristics};

    /// A small RGBA8 config — 64×64 × 4 B = 16 KiB per frame.
    fn config() -> VideoProcessConfig {
        VideoProcessConfig::new(PixelFormat::Rgba8, 64, 64, FrameRate::new(30, 1).unwrap())
            .expect("valid config")
    }

    /// The canonical descriptor a conforming host sends — same
    /// construction the `Rendered` answer derives.
    fn canonical_desc(config: &VideoProcessConfig) -> wire::VideoFrameDesc {
        let planes = (0..config.format().plane_count())
            .map(|index| {
                let layout = wire::vshm::plane_layout(
                    config.format(),
                    config.width(),
                    config.height(),
                    index,
                )
                .expect("canonical layout exists");
                wire::VideoFramePlane {
                    offset: u32::try_from(layout.offset).expect("offset fits u32"),
                    len: u32::try_from(layout.len).expect("len fits u32"),
                    stride: u32::try_from(layout.stride).expect("stride fits u32"),
                }
            })
            .collect();
        wire::VideoFrameDesc {
            format: config.format(),
            width: config.width(),
            height: config.height(),
            planes,
            pts: MediaTime::new(90_000, TimeBase::COMMON_90K),
            duration: Some(MediaDuration::new(3_000, TimeBase::COMMON_90K)),
            color: ColorInfo::default(),
        }
    }

    #[test]
    fn check_input_desc_accepts_the_canonical_descriptor() {
        let config = config();
        let desc = canonical_desc(&config);
        let region =
            usize::try_from(wire::vshm::frame_bytes(&config)).expect("frame bytes fit usize");
        assert_eq!(check_input_desc(&desc, &config, region), Ok(()));
    }

    #[test]
    fn check_input_desc_rejects_non_canonical_placement_as_ipc() {
        let config = config();
        let region =
            usize::try_from(wire::vshm::frame_bytes(&config)).expect("frame bytes fit usize");
        for declared in [
            // stride ≠ row_bytes (padding is non-canonical).
            wire::VideoFramePlane {
                offset: 0,
                len: 16_384,
                stride: 300,
            },
            // offset ≠ Σlen(0..p) — plane 0 must start the region.
            wire::VideoFramePlane {
                offset: 4,
                len: 16_384,
                stride: 256,
            },
            // len ≠ the canonical plane size.
            wire::VideoFramePlane {
                offset: 0,
                len: 100,
                stride: 256,
            },
        ] {
            let mut desc = canonical_desc(&config);
            desc.planes[0] = declared;
            assert_eq!(
                check_input_desc(&desc, &config, region),
                Err(HostError::Ipc),
                "{declared:?} must be the Ipc rejection"
            );
        }
    }

    #[test]
    fn check_input_desc_rejects_region_overflow_as_ipc() {
        let config = config();
        let region =
            usize::try_from(wire::vshm::frame_bytes(&config)).expect("frame bytes fit usize");
        // Canonical-looking stride/offset but `offset + len` past the
        // region end — a `len` that overflows the mapped capacity.
        let mut desc = canonical_desc(&config);
        desc.planes[0].offset = u32::try_from(region).expect("region fits u32") - 4;
        desc.planes[0].len = 16_384;
        assert_eq!(
            check_input_desc(&desc, &config, region),
            Err(HostError::Ipc)
        );
    }

    #[test]
    fn check_input_desc_rejects_config_mismatch_as_unsupported_config() {
        let config = config();
        let region =
            usize::try_from(wire::vshm::frame_bytes(&config)).expect("frame bytes fit usize");
        // A canonical *under its own dims* descriptor whose dims do
        // not match the instance config — each small enough to still
        // fit the 16 KiB region, so the config check is what fires.
        for (width, height, format) in
            [(32, 64, PixelFormat::Rgba8), (64, 64, PixelFormat::Yuv420p)]
        {
            let mut desc = canonical_desc(&config);
            desc.width = width;
            desc.height = height;
            desc.format = format;
            desc.planes = (0..format.plane_count())
                .map(|index| {
                    let layout = wire::vshm::plane_layout(format, width, height, index)
                        .expect("canonical layout exists");
                    wire::VideoFramePlane {
                        offset: u32::try_from(layout.offset).expect("offset fits u32"),
                        len: u32::try_from(layout.len).expect("len fits u32"),
                        stride: u32::try_from(layout.stride).expect("stride fits u32"),
                    }
                })
                .collect();
            assert_eq!(
                check_input_desc(&desc, &config, region),
                Err(HostError::Prepare(PrepareError::UnsupportedConfig)),
                "{width}×{height} {format:?} must be the config-mismatch rejection"
            );
        }
    }

    #[test]
    fn output_desc_reports_canonical_placement_and_frame_metadata() {
        let config = config();
        let mut frame = VideoFrame::new_owned(
            config.format(),
            config.width(),
            config.height(),
            MediaTime::ZERO,
        )
        .expect("frame");
        let color = ColorInfo {
            primaries: ColorPrimaries::Bt709,
            transfer: TransferCharacteristics::Bt709,
            ..ColorInfo::default()
        };
        frame.set_pts(MediaTime::new(45_000, TimeBase::COMMON_90K));
        frame.set_duration(Some(MediaDuration::new(3_000, TimeBase::COMMON_90K)));
        frame.set_color(color);
        let desc = output_desc(&frame);
        assert_eq!(desc.format, config.format());
        assert_eq!(desc.width, config.width());
        assert_eq!(desc.height, config.height());
        assert_eq!(desc.planes.len(), config.format().plane_count());
        let layout = wire::vshm::plane_layout(config.format(), config.width(), config.height(), 0)
            .expect("canonical layout");
        assert_eq!(
            u64::from(desc.planes[0].offset),
            layout.offset,
            "canonical placement"
        );
        assert_eq!(u64::from(desc.planes[0].len), layout.len);
        assert_eq!(u64::from(desc.planes[0].stride), layout.stride);
        assert_eq!(desc.pts, MediaTime::new(45_000, TimeBase::COMMON_90K));
        assert_eq!(
            desc.duration,
            Some(MediaDuration::new(3_000, TimeBase::COMMON_90K))
        );
        assert_eq!(desc.color, color);
    }
}
