//! The L1 video instance facade — [`VideoL1Instance`], the
//! `CreateVideoInstance` counterpart of [`L1Instance`]. It reuses
//! the shared child machinery ([`L1Child`], the `inner` registry,
//! waiter/demux/convergence and the mirror/`rt_state` pair) but
//! carries **no audio data plane**: no `RtLeg`, no staging ring,
//! no slot-ring `shm::Segment`, no endpoint channels. Its whole
//! frame transport is the host-owned [`vshm::Segment`] it repacks
//! into before `VideoRender` and restores out of after `Rendered`.
//!
//! Mirror bookkeeping follows the design's commit-then-advisory
//! video mapping: [`post_command_state`] commits on `Ok` replies,
//! [`err_committed_state`] still commits `VideoStopProcessing`
//! (`Activated`) and `VideoDeactivate` (`Initialized`) on a hook
//! `Err` — the runner transitions before the hook runs — while
//! `Err(Lifecycle)` is always pre-commit and every other `Err`
//! leaves the mirror in the pre-command state.

use std::ffi::OsString;
use std::sync::atomic::{AtomicU8, Ordering, fence};
use std::sync::{Arc, mpsc};
use std::time::Duration;

use ruviform_media_core::MediaTime;
use ruviform_plugin_core::{
    HostError, LifecycleEvent, LifecycleState, PluginDescriptor, VideoProcessConfig, wire,
};
use ruviform_video_core::VideoFrame;

use super::{
    ChildExit, InstKind, L1Child, SpawnSpecView, Terminal, VideoInstanceParts, converge_instance,
    data, err_committed_state, post_command_state, remove_instance, vshm,
};

#[cfg(test)]
use super::{InstPhase, InstShared, Notify};

/// Spawn parameters for one L1 video instance — the `pub(crate)`
/// mirror of the public `crate::video::VideoOutOfProcessSpec`.
/// Dedicated-only by construction: there is no `isolation`, no
/// `lookahead_blocks`, no pool field anywhere in the video surface,
/// so a bundled video route cannot be expressed.
#[derive(Debug)]
pub(crate) struct VideoSpawnSpec {
    /// Registration-supplied `'static` descriptor: the `Scan`
    /// field-match oracle and the `CreateVideoInstance`
    /// `plugin_id`.
    pub(crate) descriptor: PluginDescriptor,
    /// Runner executable.
    pub(crate) program: OsString,
    /// Extra arguments prepended to the runner's argv — the runner's
    /// own flags only.
    pub(crate) args: Vec<OsString>,
    /// Defined timeout for `Hello`, `Scan` and every later
    /// non-render `Reply` wait (default 10 s).
    pub(crate) command_timeout: Duration,
    /// Defined timeout for one `VideoRender` request→reply window
    /// (default 60 s) — a render legitimately outlasts
    /// `command_timeout`, so it gets its own bound.
    pub(crate) render_timeout: Duration,
}

impl VideoSpawnSpec {
    /// A spec with the design defaults (`command_timeout` 10 s,
    /// `render_timeout` 60 s); adjust the fields as needed.
    pub(crate) fn new(descriptor: PluginDescriptor, program: OsString) -> Self {
        Self {
            descriptor,
            program,
            args: Vec::new(),
            command_timeout: Duration::from_secs(10),
            render_timeout: Duration::from_secs(60),
        }
    }

    /// The shared spawn-parameter view [`L1Child::spawn_scanned`]/
    /// [`L1Child::scan_match`] consume — same fields an audio spec
    /// lends, minus `lookahead_blocks` (audio-only).
    pub(crate) fn view(&self) -> SpawnSpecView<'_> {
        SpawnSpecView {
            descriptor: &self.descriptor,
            program: &self.program,
            args: &self.args,
            command_timeout: self.command_timeout,
        }
    }
}

/// One L1 video instance: its registry entry on a shared
/// [`L1Child`] (always the dedicated route — capacity 1, no pool),
/// the per-instance notification queue, the owned [`vshm::Segment`]
/// frame transport, the cached [`VideoProcessConfig`] and the local
/// lifecycle mirror snapshot.
///
/// **RT contract:** every call here is **non-RT only** — spawns,
/// segment writes, thread joins and pipe writes may block and
/// allocate. `execute` waits at most `command_timeout`;
/// `render` waits at most `render_timeout`; `destroy`/`drop` are
/// bounded by the shutdown watchdog's kill guarantee.
///
/// **Send/Sync:** `!Send`/`!Sync` — `segment` holds a
/// `shared_memory::Shmem` (raw pointer), so an instance cannot leave
/// its owning thread; `PluginHost` inherits that (documented on the
/// composite).
pub(crate) struct VideoL1Instance {
    /// `child.shared`'s clone — the registry lock handle
    /// (`shared` for `inner` access, `child` for driver/ownership).
    shared: Arc<super::Shared>,
    /// The child this instance lives on — `Arc::strong_count == 1`
    /// inside `destroy`/`Drop` means the last referrer, which owns
    /// the bounded teardown.
    child: Arc<L1Child>,
    /// The minted wire handle — stamps every command's envelope.
    instance: u32,
    /// This instance's own `ChildExit` notification queue —
    /// `InstShared.notify_tx`'s pair.
    notify_rx: mpsc::Receiver<ChildExit>,
    /// The facade's clone of `InstShared.rt_state` — the lock-free
    /// mirror snapshot `lifecycle_state` reads; survives entry
    /// removal (`destroy` writes `Destroyed` onto it).
    rt_state: Arc<AtomicU8>,
    /// The owned `vshm` segment — the `VideoRender` frame
    /// transport. `!Send` (`shared_memory::Shmem`); kept alive so
    /// the child's attach stays valid, unlinked on drop (RAII).
    segment: vshm::Segment,
    /// The `CreateVideoInstance` `config` — the input-desc shape
    /// check and the `Rendered` output gate read this same
    /// authority the runner stashed.
    config: VideoProcessConfig,
    /// `VideoRender`'s reply-wait bound — longer than
    /// `command_timeout` by design.
    render_timeout: Duration,
}

impl VideoL1Instance {
    /// Spawns a dedicated child and runs the full handshake —
    /// [`L1Child::spawn_scanned`] (`Hello` version gate + `Scan` →
    /// descriptor field-match) then
    /// [`L1Child::create_video_instance`]. Every failure — spawn
    /// error, timeout, version/descriptor mismatch — aborts the
    /// fresh child (kill + reap, no `Shutdown` grace) and returns
    /// the error; `Err(InstanceLimit)` is the one reply that
    /// propagates unflattened.
    ///
    /// **RT contract:** non-RT only — blocks up to
    /// `4 × command_timeout` in the worst case.
    ///
    /// `config` is the `VideoHostCommand::Instantiate` payload — a
    /// per-instance value, deliberately not part of the spec (the
    /// registered route is reusable).
    pub(crate) fn spawn(
        spec: &VideoSpawnSpec,
        config: VideoProcessConfig,
    ) -> Result<Self, HostError> {
        let child = L1Child::spawn_scanned(&spec.view())?;
        let parts = match child.create_video_instance(&spec.view(), config) {
            Ok(parts) => parts,
            Err(error) => {
                // Fresh-child failure — kill + reap, no
                // notification: the instance never went live.
                child.abort();
                return Err(error);
            }
        };
        Ok(Self::from_parts(child, parts, spec.render_timeout))
    }

    /// Assembles a facade around a minted video registry entry —
    /// the `config` is read back out of the entry's
    /// `InstKind::Video` pin so the facade and the registry can
    /// never disagree about which config the runner stashed.
    ///
    /// **RT contract:** non-RT only — plain field moves.
    pub(crate) fn from_parts(
        child: Arc<L1Child>,
        parts: VideoInstanceParts,
        render_timeout: Duration,
    ) -> Self {
        let config = {
            let inner = child.shared.lock();
            match inner.instances.get(&parts.instance).map(|inst| &inst.kind) {
                Some(InstKind::Video { config }) => *config,
                // `create_video_instance` just registered this entry
                // under `InstKind::Video` — anything else is a bug.
                _ => panic!("video entry must be InstKind::Video"),
            }
        };
        Self {
            shared: Arc::clone(&child.shared),
            child,
            instance: parts.instance,
            notify_rx: parts.notify_rx,
            rt_state: parts.rt_state,
            segment: parts.segment,
            config,
            render_timeout,
        }
    }

    /// The instance-scope gate + child-side request — every video
    /// verb is stamped with this instance's minted handle (no
    /// child-scoped verb exists on the video surface: `Scan` is
    /// composite-side only).
    fn request(&self, cmd: wire::Command) -> Result<wire::Reply, HostError> {
        self.child.request(self.instance, cmd)
    }

    /// Sends one video wire command and waits for its reply — the
    /// command/reply parity surface for the `Video*` verb set
    /// (`Render` has its own [`VideoL1Instance::render`] path;
    /// `CreateVideoInstance` is create-time only).
    ///
    /// Defined rejections resolved without touching the wire:
    /// `Failed` instance → `Err(HostError::Failed)`;
    /// `Destroying`/removed entry → `Err(Ipc)`. A reply that never
    /// arrives inside `command_timeout` is `Err(HostError::Ipc)`
    /// plus child-scope convergence — a timeout is unattributable,
    /// so the whole child goes `Failed`, not this instance alone.
    ///
    /// Mirror rule: `post_command_state` commits on `Ok`;
    /// `err_committed_state` commits the
    /// `VideoStopProcessing`/`VideoDeactivate` transition even on a
    /// hook `Err` (the runner's commit-then-advisory ordering —
    /// `Err(Lifecycle)` is always pre-commit and moves nothing);
    /// every other verb's `Err` leaves the mirror in the
    /// pre-command state.
    ///
    /// **RT contract:** non-RT only — waits at most `command_timeout`.
    pub(crate) fn execute(&self, cmd: wire::Command) -> Result<wire::Reply, HostError> {
        {
            let inner = self.shared.lock();
            match inner.terminal {
                Some(Terminal::Failed) => return Err(HostError::Failed),
                Some(Terminal::Clean) => return Err(HostError::Ipc),
                None => {}
            }
            if inner.shutdown_initiated {
                return Err(HostError::Ipc);
            }
            // Instance-scope rejections — a `Failed` entry is
            // `Err(Failed)`; `Destroying`/removed entries are
            // `Err(Ipc)`. `Creating` is unreachable — the entry
            // flips `Live` inside `create_video_instance` before
            // the facade exists.
            match inner.instances.get(&self.instance) {
                Some(inst) if inst.mirror.is_failed() => return Err(HostError::Failed),
                Some(inst) if inst.live() => {}
                _ => return Err(HostError::Ipc),
            }
        }
        let post = post_command_state(&cmd);
        let on_err = err_committed_state(&cmd);
        let result = self.request(cmd);
        // The mirror moves only while the entry is still live (a
        // convergence racing the reply must not be regressed back
        // out of `Failed`). `Err(Lifecycle)` is the pre-commit
        // legality rejection — no commit; other `Err`s commit only
        // the commit-then-advisory verbs' post-state.
        let committed = match &result {
            Ok(_) => post,
            Err(HostError::Lifecycle(_)) => None,
            Err(_) => on_err,
        };
        if let Some(state) = committed {
            let mut inner = self.shared.lock();
            if inner.terminal.is_none()
                && let Some(inst) = inner.instances.get_mut(&self.instance)
                && inst.live()
            {
                inst.set_mirror(state);
            }
        }
        result
    }

    /// `VideoRender` — the video `process` equivalent: validates
    /// `input` against `config`, repacks it canonically into the
    /// `in` region, waits `render_timeout` for `Rendered`, then
    /// validates the returned output descriptor against the same
    /// `config`/canonical-layout/bounds rules before restoring a
    /// fresh `VideoFrame` out of the `out` region.
    ///
    /// Defined rejections resolved without touching the wire, in
    /// order:
    /// 1. **entry gate** — `Failed` instance → `Err(Failed)`;
    ///    `Destroying`/removed → `Err(Ipc)`;
    /// 2. **input shape** — `input`'s `format`/`width`/`height`
    ///    must equal `config`'s → `Err(HostError::InvalidValue)`;
    /// 3. **lifecycle** — the mirror must be `Processing` (the L0
    ///    `transit(Process)` pre-check) → `Err(HostError::Lifecycle)`.
    ///
    /// A `Rendered.output` that fails the semantic gate is a
    /// well-formed-but-lies reply: it is *instance*-attributable,
    /// so this instance converges `Failed` (`converge_instance` —
    /// siblings stay live) and the caller gets `Err(Ipc)`. Output
    /// is committed only after a successful `Rendered` reply and a
    /// validated descriptor — never on an `Err` reply or a
    /// rejected desc.
    ///
    /// **RT contract:** non-RT only — segment writes plus a
    /// `render_timeout` wait.
    pub(crate) fn render(
        &mut self,
        input: &VideoFrame,
        at: MediaTime,
    ) -> Result<VideoFrame, HostError> {
        // (1) entry gate — the same terminal/live answers every
        // non-`Destroy` verb gives.
        {
            let inner = self.shared.lock();
            match inner.terminal {
                Some(Terminal::Failed) => return Err(HostError::Failed),
                Some(Terminal::Clean) => return Err(HostError::Ipc),
                None => {}
            }
            if inner.shutdown_initiated {
                return Err(HostError::Ipc);
            }
            match inner.instances.get(&self.instance) {
                Some(inst) if inst.mirror.is_failed() => return Err(HostError::Failed),
                Some(inst) if inst.live() => {}
                _ => return Err(HostError::Ipc),
            }
        }
        // (2) caller-supplied frame must match the create-time
        // config — a domain violation, `Err(InvalidValue)`, no wire
        // send.
        if input.format() != self.config.format()
            || input.width() != self.config.width()
            || input.height() != self.config.height()
        {
            return Err(HostError::InvalidValue);
        }
        // (3) mirror must be `Processing` — the L0
        // `transit(Process)` pre-check's mapping.
        self.lifecycle_state()
            .transition(LifecycleEvent::Process)
            .map_err(HostError::Lifecycle)?;
        // Repack `input` into the `in` region under canonical
        // packing (`stride == row_bytes`, `offset(p) = Σ len(0..p)`)
        // — a padded-stride caller frame is re-laid out here so the
        // wire desc is always canonical. Serialized by the
        // request→reply window: no render is in flight, so the
        // region write is exclusive.
        let mut planes = Vec::with_capacity(self.config.format().plane_count());
        {
            let region = self.segment.in_region_mut();
            for index in 0..self.config.format().plane_count() {
                let layout = wire::vshm::plane_layout(
                    self.config.format(),
                    self.config.width(),
                    self.config.height(),
                    index,
                )
                .expect("plane index within format.plane_count()");
                let in_plane = input.plane(index).ok_or(HostError::InvalidValue)?;
                let row_bytes =
                    usize::try_from(layout.stride).map_err(|_| HostError::InvalidValue)?;
                let plane_len = usize::try_from(layout.len).map_err(|_| HostError::InvalidValue)?;
                let start = usize::try_from(layout.offset).map_err(|_| HostError::InvalidValue)?;
                let dst = &mut region[start..start + plane_len];
                for y in 0..layout.height {
                    let row = in_plane.row(y).ok_or(HostError::InvalidValue)?;
                    let y = usize::try_from(y).map_err(|_| HostError::InvalidValue)?;
                    dst[y * row_bytes..(y + 1) * row_bytes].copy_from_slice(row);
                }
                planes.push(wire::VideoFramePlane {
                    offset: u32::try_from(layout.offset).map_err(|_| HostError::InvalidValue)?,
                    len: u32::try_from(layout.len).map_err(|_| HostError::InvalidValue)?,
                    stride: u32::try_from(layout.stride).map_err(|_| HostError::InvalidValue)?,
                });
            }
        }
        let desc = wire::VideoFrameDesc {
            format: self.config.format(),
            width: self.config.width(),
            height: self.config.height(),
            planes,
            pts: input.pts(),
            duration: input.duration(),
            color: input.color(),
        };
        // Release edge — the `in`-region writes must be visible to
        // the child before the `VideoRender` request it reads them
        // under (the same Ordering rule the audio in-slot write
        // obeys).
        fence(Ordering::Release);
        let reply = self.child.request_timed(
            self.instance,
            wire::Command::VideoRender { at, input: desc },
            self.render_timeout,
        )?;
        let wire::Reply::Rendered { output } = reply else {
            // A well-formed reply of the wrong kind is a protocol
            // anomaly — `Err(Ipc)`, no convergence (the same answer
            // a reply-kind mismatch gives on the audio path).
            return Err(HostError::Ipc);
        };
        if !validate_rendered_desc(&self.config, &output, self.segment.region_bytes()) {
            // Well-formed but lies — the desc disagrees with the
            // instance's own config or violates canonicality/bounds.
            // Instance-attributable: converge this instance `Failed`
            // (siblings stay live) and answer `Err(Ipc)`.
            converge_instance(&self.shared, self.instance);
            return Err(HostError::Ipc);
        }
        // Acquire edge — pairs the child's `fence(Release)` before
        // its `Rendered` write; the `out`-region read below must
        // observe the rendered bytes.
        fence(Ordering::Acquire);
        let mut frame =
            VideoFrame::new_owned(output.format, output.width, output.height, output.pts)
                .map_err(|_| HostError::Ipc)?;
        frame.set_duration(output.duration);
        frame.set_color(output.color);
        {
            let region = self.segment.out_region();
            for index in 0..output.format.plane_count() {
                let layout =
                    wire::vshm::plane_layout(output.format, output.width, output.height, index)
                        .expect("validated by validate_rendered_desc");
                let start = usize::try_from(layout.offset).map_err(|_| HostError::Ipc)?;
                let len = usize::try_from(layout.len).map_err(|_| HostError::Ipc)?;
                let src = &region[start..start + len];
                let mut plane = frame
                    .plane_mut(index)
                    .expect("new_owned frame planes are mutable");
                let dst = plane.data_mut();
                dst.copy_from_slice(&src[..dst.len()]);
            }
        }
        Ok(frame)
    }

    /// `Destroy`'s control-plane meaning — the same shape as
    /// [`L1Instance::destroy`]: `DestroyInstance` reclaims the
    /// child's registry slot, the entry leaves `Inner.instances`,
    /// the mirror outlives the entry as `Destroyed`, and the last
    /// referrer runs the bounded `Shutdown`+reap teardown. The one
    /// video difference: **no** `DriverCmd::RemoveInstance` — a
    /// video entry never installed driver ends.
    ///
    /// Always succeeds and is idempotent — `Destroy → Ok` for
    /// live, `Failed` and already-destroyed instances alike.
    ///
    /// **RT contract:** non-RT only — bounded by `command_timeout`
    /// of shutdown grace plus the kill guarantee.
    pub(crate) fn destroy(&mut self) {
        let (child_terminal, registered) = {
            let inner = self.shared.lock();
            (
                inner.terminal.is_some(),
                inner.instances.contains_key(&self.instance),
            )
        };
        if registered {
            // Reclaim the registry slot while the child can still
            // answer — a dead/wedged reply path still proceeds to
            // removal below (teardown is unconditional). A
            // terminal child's request no-ops to `Err`.
            if !child_terminal {
                let _ = self.request(wire::Command::DestroyInstance);
            }
            let mut inner = self.shared.lock();
            remove_instance(&mut inner, self.instance);
            drop(inner);
            // No `DriverCmd::RemoveInstance` — nothing was ever
            // installed on the driver for a video entry.
        }
        self.rt_state.store(
            data::mirror_encode(LifecycleState::Destroyed),
            Ordering::Release,
        );
        if Arc::strong_count(&self.child) == 1 {
            self.child.teardown();
        }
    }

    /// Kills the child process — the `#[doc(hidden)] kill_child`
    /// contract's backing operation, used to drive the crash paths
    /// in tests. The driver's `try_wait` (or stdout EOF) performs
    /// the `Failed` convergence asynchronously.
    ///
    /// **RT contract:** non-RT only — the kill goes through the
    /// shared handle; the driver's poll observes the death.
    pub(crate) fn kill(&self) {
        super::kill_child(&self.shared);
    }

    /// The local lifecycle mirror — `Created` through `Processing`
    /// track successful commands, `Failed`/`Destroyed` are terminal
    /// conditions. Reads the facade's lock-free `rt_state`
    /// snapshot, still answering after the entry itself is removed.
    ///
    /// **RT contract:** non-RT only by call site — a single atomic
    /// load, technically non-blocking.
    pub(crate) fn lifecycle_state(&self) -> LifecycleState {
        data::mirror_decode(self.rt_state.load(Ordering::Acquire))
    }

    /// Pops the queued `Failed` notification — `Some` exactly once
    /// per convergence, `None` on clean teardown or before failure.
    ///
    /// **RT contract:** non-RT only — `try_recv`, never blocks.
    pub(crate) fn try_recv_notification(&self) -> Option<ChildExit> {
        self.notify_rx.try_recv().ok()
    }
}

impl Drop for VideoL1Instance {
    /// Dropping without `Destroy` runs the same unconditional
    /// path — `DestroyInstance` slot reclaim (bounded request),
    /// entry removal, then the last-reference teardown (the
    /// dedicated route's count is always 1): `Shutdown`+grace or
    /// the driver's `Disconnected` kill/reap.
    fn drop(&mut self) {
        if !self.lifecycle_state().is_destroyed() {
            self.destroy();
        }
    }
}

/// The `Rendered.output` semantic gate — everything the codec's
/// structural bounds did not promise: `desc`'s `format`/`width`/
/// `height` equal `config`'s, `planes.len() ==
/// format.plane_count()`, and every plane's `(offset, stride, len)`
/// equals [`wire::vshm::plane_layout`]'s canonical answers with
/// `offset + len` inside the `out` region. `false` means the reply
/// is well-formed but wrong — an instance-attributable violation.
fn validate_rendered_desc(
    config: &VideoProcessConfig,
    desc: &wire::VideoFrameDesc,
    region_bytes: usize,
) -> bool {
    if desc.format != config.format()
        || desc.width != config.width()
        || desc.height != config.height()
        || desc.planes.len() != desc.format.plane_count()
    {
        return false;
    }
    desc.planes.iter().enumerate().all(|(index, plane)| {
        let Some(layout) = wire::vshm::plane_layout(desc.format, desc.width, desc.height, index)
        else {
            return false;
        };
        u64::from(plane.offset) == layout.offset
            && u64::from(plane.stride) == layout.stride
            && u64::from(plane.len) == layout.len
            && u64::from(plane.offset) + u64::from(plane.len) <= region_bytes as u64
    })
}

#[cfg(test)]
impl VideoL1Instance {
    /// Child-free fixture for the facade/dispatch tests — a real
    /// `Shared` with a registered `InstKind::Video` entry and a
    /// real `vshm` segment, but no spawned child: `request` fails
    /// `Err(Ipc)` past the local gates (the render/desc/mirror
    /// checks are still exercised — they run before the wire).
    /// `mirror` is the state `mirror`/`rt_state` start at
    /// (`Failed`/`Destroyed` registers `Failed`-phase).
    pub(crate) fn test_fixture(mirror: LifecycleState, config: VideoProcessConfig) -> Self {
        Self::test_fixture_on(&L1Child::test_fixture(0), 0, mirror, config)
    }

    /// [`VideoL1Instance::test_fixture`] on a caller-supplied child —
    /// the `create_harness`-style tests pin a live `DriverCmd`
    /// channel behind the child so a reply can be fed by hand.
    /// The entry registers under wire handle `instance`.
    pub(crate) fn test_fixture_on(
        child: &Arc<L1Child>,
        instance: u32,
        mirror: LifecycleState,
        config: VideoProcessConfig,
    ) -> Self {
        let (notify_tx, notify_rx) = mpsc::channel();
        let rt_state = Arc::new(AtomicU8::new(data::mirror_encode(mirror)));
        let segment = vshm::Segment::create(
            u32::try_from(super::INSTANCE_COUNTER.fetch_add(1, Ordering::Relaxed))
                .unwrap_or(u32::MAX),
            &config,
        )
        .expect("fixture segment");
        child.shared.lock().instances.insert(
            instance,
            InstShared {
                rt_state: Arc::clone(&rt_state),
                mirror,
                notify_tx,
                notify: Notify::Idle,
                kind: InstKind::Video { config },
                phase: if mirror.is_failed() || mirror.is_destroyed() {
                    InstPhase::Failed
                } else {
                    InstPhase::Live
                },
            },
        );
        Self {
            shared: Arc::clone(&child.shared),
            child: Arc::clone(child),
            instance,
            notify_rx,
            rt_state,
            segment,
            config,
            render_timeout: Duration::from_millis(50),
        }
    }
}

#[cfg(test)]
mod tests {
    //! `VideoL1Instance` facade tests — the render rejection
    //! ladder (entry gate → input-shape → mirror), the
    //! commit-then-advisory mirror mapping, the `Rendered.output`
    //! semantic gate and the `vshm` header/in-region byte pins.
    //! The `DriverCmd`-harness tests feed replies through the
    //! waiter registry by hand — no real child is spawned.

    use std::collections::{HashMap, HashSet};
    use std::sync::Mutex;
    use std::sync::atomic::AtomicU64;
    use std::thread;

    use ruviform_media_core::{FrameRate, TimeBase};
    use ruviform_video_core::PixelFormat;

    use super::*;
    use crate::l1::{DriverCmd, Inner, Shared};

    fn config(width: u32, height: u32) -> VideoProcessConfig {
        VideoProcessConfig::new(
            PixelFormat::Yuv420p,
            width,
            height,
            FrameRate::new(30, 1).unwrap(),
        )
        .unwrap()
    }

    fn input_frame(cfg: &VideoProcessConfig) -> VideoFrame {
        VideoFrame::new_owned(
            cfg.format(),
            cfg.width(),
            cfg.height(),
            MediaTime::new(0, TimeBase::COMMON_90K),
        )
        .unwrap()
    }

    /// An `L1Child` whose `DriverCmd` channel the test services —
    /// `create_harness`'s shape in `l1::mod::tests` applied to the
    /// video side (a `Shared` with no entries; the video fixture
    /// registers its own).
    fn driver_harness() -> (Arc<L1Child>, mpsc::Receiver<DriverCmd>) {
        let (driver_tx, driver_rx) = mpsc::channel();
        let child = Arc::new(L1Child {
            shared: Arc::new(Shared {
                inner: Mutex::new(Inner {
                    waiters: HashMap::new(),
                    instances: HashMap::new(),
                    block_in_flight: HashSet::new(),
                    terminal: None,
                    shutdown_initiated: false,
                    seen_eof: false,
                }),
                child: std::sync::OnceLock::new(),
                next_cmd_seq: AtomicU64::new(0),
            }),
            driver_tx: Mutex::new(Some(driver_tx)),
            driver_join: Mutex::new(None),
            reader_join: Mutex::new(None),
            command_timeout: Duration::from_secs(30),
        });
        (child, driver_rx)
    }

    /// Drains the harness channel until the first
    /// `DriverCmd::Command` arrives — the waiter seq to feed a
    /// reply onto.
    fn recv_command_seq(driver_rx: &mpsc::Receiver<DriverCmd>) -> u64 {
        match driver_rx.recv_timeout(Duration::from_secs(30)) {
            Ok(DriverCmd::Command { seq, .. }) => seq,
            Ok(_) => panic!("expected DriverCmd::Command"),
            Err(error) => panic!("driver channel died before Command: {error}"),
        }
    }

    /// Feeds `result` to the waiter `seq` on `shared` — the
    /// reader-thread's `Reply` delivery by hand.
    fn feed_reply(shared: &Arc<Shared>, seq: u64, result: Result<wire::Reply, HostError>) {
        let waiter = shared
            .lock()
            .waiters
            .remove(&seq)
            .expect("waiter registered");
        waiter.tx.send(result).expect("waiter channel open");
    }

    /// The desc `render` would send for `config` — canonical
    /// `in`-region placement under `wire::vshm::plane_layout`.
    fn canonical_desc(cfg: &VideoProcessConfig) -> wire::VideoFrameDesc {
        wire::VideoFrameDesc {
            format: cfg.format(),
            width: cfg.width(),
            height: cfg.height(),
            planes: (0..cfg.format().plane_count())
                .map(|index| {
                    let layout =
                        wire::vshm::plane_layout(cfg.format(), cfg.width(), cfg.height(), index)
                            .unwrap();
                    wire::VideoFramePlane {
                        offset: u32::try_from(layout.offset).unwrap(),
                        len: u32::try_from(layout.len).unwrap(),
                        stride: u32::try_from(layout.stride).unwrap(),
                    }
                })
                .collect(),
            pts: MediaTime::new(0, TimeBase::COMMON_90K),
            duration: None,
            color: ruviform_video_core::ColorInfo::default(),
        }
    }

    // --- `Rendered.output` semantic gate ------------------------

    /// The canonical `Rendered{output}` for the instance config
    /// passes the gate.
    #[test]
    fn validate_rendered_desc_accepts_canonical() {
        let cfg = config(64, 48);
        let segment_len = wire::vshm::frame_bytes(&cfg);
        let desc = canonical_desc(&cfg);
        assert!(validate_rendered_desc(
            &cfg,
            &desc,
            usize::try_from(segment_len).unwrap()
        ));
    }

    /// Every semantic violation the codec bounds don't cover:
    /// dims/format disagreeing with the instance config, a
    /// non-canonical stride or offset, a plane count mismatch and
    /// an `offset + len` past the region end.
    #[test]
    fn validate_rendered_desc_rejects_semantic_violations() {
        let cfg = config(64, 48);
        let region = usize::try_from(wire::vshm::frame_bytes(&cfg)).unwrap();
        let good = canonical_desc(&cfg);

        // Wrong dims / format vs the instance config.
        let mut desc = good.clone();
        desc.width = 32;
        assert!(!validate_rendered_desc(&cfg, &desc, region));
        let mut desc = good.clone();
        desc.height = 24;
        assert!(!validate_rendered_desc(&cfg, &desc, region));
        let mut desc = good.clone();
        desc.format = PixelFormat::Yuv444p;
        assert!(!validate_rendered_desc(&cfg, &desc, region));
        // Re-laid out to the new format's plane count — still
        // rejected: the *format field itself* disagrees.
        desc.planes = (0..desc.format.plane_count())
            .map(|index| {
                let layout =
                    wire::vshm::plane_layout(desc.format, desc.width, desc.height, index).unwrap();
                wire::VideoFramePlane {
                    offset: u32::try_from(layout.offset).unwrap(),
                    len: u32::try_from(layout.len).unwrap(),
                    stride: u32::try_from(layout.stride).unwrap(),
                }
            })
            .collect();
        assert!(!validate_rendered_desc(&cfg, &desc, region));

        // Plane count ≠ format.plane_count().
        let mut desc = good.clone();
        desc.planes.pop();
        assert!(!validate_rendered_desc(&cfg, &desc, region));

        // Non-canonical stride / offset on plane 0.
        let mut desc = good.clone();
        desc.planes[0].stride += 4;
        assert!(!validate_rendered_desc(&cfg, &desc, region));
        let mut desc = good.clone();
        desc.planes[0].offset = 4;
        assert!(!validate_rendered_desc(&cfg, &desc, region));

        // offset + len past the region end.
        let mut desc = good;
        let last = desc.planes.last_mut().unwrap();
        last.offset = u32::try_from(region).unwrap();
        assert!(!validate_rendered_desc(&cfg, &desc, region));
    }

    // --- `render` rejection ladder ------------------------------

    /// A caller frame whose dims disagree with the instance
    /// config is `Err(InvalidValue)` — the wire is never touched.
    #[test]
    fn render_rejects_input_config_mismatch() {
        let cfg = config(64, 48);
        let mut instance = VideoL1Instance::test_fixture(LifecycleState::Processing, cfg);
        let wrong = VideoFrame::new_owned(
            cfg.format(),
            32,
            cfg.height(),
            MediaTime::new(0, TimeBase::COMMON_90K),
        )
        .unwrap();
        assert!(matches!(
            instance.render(&wrong, MediaTime::new(0, TimeBase::COMMON_90K)),
            Err(HostError::InvalidValue)
        ));
        let wrong_format = VideoFrame::new_owned(
            PixelFormat::Rgba8,
            cfg.width(),
            cfg.height(),
            MediaTime::new(0, TimeBase::COMMON_90K),
        )
        .unwrap();
        assert!(matches!(
            instance.render(&wrong_format, MediaTime::new(0, TimeBase::COMMON_90K)),
            Err(HostError::InvalidValue)
        ));
    }

    /// `Render` outside `Processing` is `Err(Lifecycle)` — the L0
    /// `transit(Process)` pre-check, wire untouched.
    #[test]
    fn render_rejects_non_processing_mirror() {
        let cfg = config(64, 48);
        let mut instance = VideoL1Instance::test_fixture(LifecycleState::Activated, cfg);
        let frame = input_frame(&cfg);
        assert!(matches!(
            instance.render(&frame, MediaTime::new(0, TimeBase::COMMON_90K)),
            Err(HostError::Lifecycle(_))
        ));
    }

    /// A `Failed` instance answers `Err(Failed)` before the wire —
    /// every verb but `Destroy` shares the gate.
    #[test]
    fn render_and_execute_reject_failed_mirror() {
        let cfg = config(64, 48);
        let mut instance = VideoL1Instance::test_fixture(LifecycleState::Failed, cfg);
        let frame = input_frame(&cfg);
        assert!(matches!(
            instance.render(&frame, MediaTime::new(0, TimeBase::COMMON_90K)),
            Err(HostError::Failed)
        ));
        assert!(matches!(
            instance.execute(wire::Command::VideoGetParams),
            Err(HostError::Failed)
        ));
    }

    /// `Rendered.output` failing the semantic gate is an
    /// *instance-attributable* protocol violation: the caller gets
    /// `Err(Ipc)` and the instance alone converges `Failed` —
    /// mirror, one-shot notification and all.
    #[test]
    fn render_malformed_output_converges_instance() {
        let cfg = config(64, 48);
        let (child, driver_rx) = driver_harness();
        let mut instance =
            VideoL1Instance::test_fixture_on(&child, 0, LifecycleState::Processing, cfg);
        // The `!Send` facade stays on this thread (it blocks inside
        // `render`'s reply wait); the reply feeder — channel end +
        // `Shared`, both `Send` — serves the wire from its own
        // thread.
        let mut bad = canonical_desc(&cfg);
        bad.planes[0].stride += 4;
        let shared = Arc::clone(&child.shared);
        let feeder = thread::spawn(move || {
            let seq = recv_command_seq(&driver_rx);
            feed_reply(&shared, seq, Ok(wire::Reply::Rendered { output: bad }));
            shared
        });
        let frame = input_frame(&cfg);
        let result = instance.render(&frame, MediaTime::new(0, TimeBase::COMMON_90K));
        assert!(matches!(result, Err(HostError::Ipc)));
        let shared = feeder.join().unwrap();
        let inner = shared.lock();
        let inst = inner.instances.get(&0).expect("entry still registered");
        assert!(inst.mirror.is_failed());
        assert!(matches!(inst.phase, InstPhase::Failed));
        drop(inner);
    }

    /// A well-formed `Rendered{output}` round-trips: the `out`
    /// region is read only after the reply + Acquire fence, and
    /// the returned frame carries the plugin's `desc` metadata.
    #[test]
    fn render_round_trips_valid_output() {
        let cfg = config(64, 48);
        let (child, driver_rx) = driver_harness();
        let mut instance =
            VideoL1Instance::test_fixture_on(&child, 0, LifecycleState::Processing, cfg);
        let out_desc = canonical_desc(&cfg);
        let shared = Arc::clone(&child.shared);
        let feeder = thread::spawn(move || {
            let seq = recv_command_seq(&driver_rx);
            feed_reply(&shared, seq, Ok(wire::Reply::Rendered { output: out_desc }));
        });
        let frame = input_frame(&cfg);
        let frame = instance
            .render(&frame, MediaTime::new(0, TimeBase::COMMON_90K))
            .expect("render ok");
        feeder.join().unwrap();
        assert_eq!(frame.format(), cfg.format());
        assert_eq!(frame.width(), cfg.width());
        assert_eq!(frame.height(), cfg.height());
    }

    // --- mirror mapping -----------------------------------------

    /// `Err` reply mirror invariance — the L0-faithful video rule:
    /// a non-commit-advisory verb's `Err` leaves the mirror in the
    /// pre-command state (the fixture's `Err(Ipc)` send failure
    /// stands in for the reply-path `Err`).
    #[test]
    fn execute_err_reply_leaves_mirror() {
        let cfg = config(64, 48);
        let instance = VideoL1Instance::test_fixture(LifecycleState::Processing, cfg);
        assert!(matches!(
            instance.execute(wire::Command::VideoInitialize),
            Err(HostError::Ipc)
        ));
        assert_eq!(instance.lifecycle_state(), LifecycleState::Processing);
        // Param/render verbs never transition — `Ok` or `Err`.
        assert!(matches!(
            instance.execute(wire::Command::VideoGetParams),
            Err(HostError::Ipc)
        ));
        assert_eq!(instance.lifecycle_state(), LifecycleState::Processing);
    }

    /// The commit-then-advisory verbs: `VideoStopProcessing` commits
    /// `Activated` and `VideoDeactivate` commits `Initialized` even
    /// when the reply is `Err` — the runner transitions before the
    /// hook reports.
    #[test]
    fn execute_err_reply_commits_advisory_verbs() {
        let cfg = config(64, 48);
        let instance = VideoL1Instance::test_fixture(LifecycleState::Processing, cfg);
        assert!(matches!(
            instance.execute(wire::Command::VideoStopProcessing),
            Err(HostError::Ipc)
        ));
        assert_eq!(instance.lifecycle_state(), LifecycleState::Activated);
        assert!(matches!(
            instance.execute(wire::Command::VideoDeactivate),
            Err(HostError::Ipc)
        ));
        assert_eq!(instance.lifecycle_state(), LifecycleState::Initialized);
    }

    /// `Err(Lifecycle)` is always pre-commit — even a
    /// commit-then-advisory verb moves nothing.
    #[test]
    fn execute_lifecycle_err_is_precommit() {
        let cfg = config(64, 48);
        let (child, driver_rx) = driver_harness();
        let instance = VideoL1Instance::test_fixture_on(&child, 0, LifecycleState::Processing, cfg);
        let shared = Arc::clone(&child.shared);
        let feeder = thread::spawn(move || {
            let seq = recv_command_seq(&driver_rx);
            feed_reply(
                &shared,
                seq,
                Err(HostError::Lifecycle(
                    ruviform_plugin_core::LifecycleError::IllegalTransition {
                        from: LifecycleState::Processing,
                        event: LifecycleEvent::StopProcessing,
                    },
                )),
            );
        });
        let result = instance.execute(wire::Command::VideoStopProcessing);
        feeder.join().unwrap();
        assert!(matches!(result, Err(HostError::Lifecycle(_))));
        assert_eq!(instance.lifecycle_state(), LifecycleState::Processing);
    }
}
