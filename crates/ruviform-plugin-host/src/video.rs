//! The video host surface — the NONRT sibling of the audio
//! `HostCommand`/`HostReply`/`PluginHost` vocabulary, reached
//! through [`PluginHost::video`].
//!
//! [`VideoHostCommand`]/[`VideoHostReply`] mirror the audio
//! command/reply parity for the video instance lifecycle —
//! `Scan`, `Instantiate`, the lifecycle verbs, `Render` (the video
//! `process` equivalent — a whole `VideoFrame` per call, never a
//! per-block leg) and the plain-parameter verbs — over the same
//! unified [`PluginHandle`] space the audio commands use.
//!
//! Video is structurally **Dedicated-only**: [`VideoOutOfProcessSpec`]
//! carries no `isolation` and no `lookahead_blocks`, so a bundled
//! video route cannot be expressed — every video instantiation
//! gets its own child process. The frame transport is a host-owned
//! `vshm` shared-memory segment per instance (one `in` + one `out`
//! region behind a fixed header, plane layout in canonical
//! packing), never the audio slot-ring machinery.
//!
//! Cross-kind rules: a `plugin_id` is claimed once for either
//! surface — duplicate registration is
//! [`HostError::DuplicatePluginId`]; `Instantiate` names a
//! video-registered id only — an audio id is
//! [`HostError::UnknownPluginId`]; every other video verb on an
//! audio slot (and vice versa) is [`HostError::Unsupported`]. The
//! audio endpoint/RT surface does not exist for video handles —
//! `PluginHost::process`/`prepare_endpoint`/`attach_endpoint`
//! against one answer `Unsupported`/`EndpointUnavailable` while
//! `lifecycle_state`/`kill_child`/`Destroy`/`try_recv_notification`
//! work normally.

use std::ffi::OsString;
use std::time::Duration;

use ruviform_media_core::MediaTime;
use ruviform_plugin_core::{
    HostError, LifecycleState, ParamId, PlainParamInfoOwned, PlainParamValue, PluginDescriptor,
    PluginHandle, VideoProcessConfig,
};
use ruviform_video_core::VideoFrame;

use crate::{HostNotification, PluginHost};

/// The video command vocabulary — [`VideoPluginHost::execute`]'s
/// input half, the video counterpart of
/// [`HostCommand`](crate::HostCommand).
///
/// Every command is **non-RT only** — the same contract as the
/// audio commands (scan, instantiate, lifecycle and parameter verbs
/// all run synchronously over the child's framed control plane).
#[non_exhaustive]
#[derive(Clone, Debug)]
pub enum VideoHostCommand {
    /// List this surface's registered descriptors →
    /// [`VideoHostReply::Scanned`]. Kind-filtered: only
    /// video-registered descriptors appear — audio registrations
    /// never leak in.
    Scan,
    /// Mint an instance of a registered video plugin on a fresh
    /// dedicated child → [`VideoHostReply::Instantiated`].
    /// `plugin_id` must name a video-registered descriptor (an
    /// audio id is `Err(UnknownPluginId)`); `config` is the
    /// `VideoProcessConfig` the instance's `vshm` segment is sized
    /// from and every `Render` input is validated against.
    Instantiate {
        /// Registered video plugin id.
        plugin_id: String,
        /// Processing configuration — format, dims, frame rate.
        config: VideoProcessConfig,
    },
    /// Destroy the instance and reclaim its runner registry slot →
    /// [`VideoHostReply::Ok`]. Always defined — live, `Failed` and
    /// already-destroyed instances all answer `Ok`.
    Destroy {
        /// Instance handle.
        handle: PluginHandle,
    },
    /// `Created → Initialized` — replay the create-time `config`
    /// into `VideoPluginInstance::initialize` →
    /// [`VideoHostReply::Ok`].
    Initialize {
        /// Instance handle.
        handle: PluginHandle,
    },
    /// `Initialized → Activated` → [`VideoHostReply::Ok`].
    Activate {
        /// Instance handle.
        handle: PluginHandle,
    },
    /// `Activated → Processing` → [`VideoHostReply::Ok`].
    StartProcessing {
        /// Instance handle.
        handle: PluginHandle,
    },
    /// Render one frame — `VideoPluginInstance::render(input, at)`
    /// → [`VideoHostReply::Rendered`]. `input` must match the
    /// instance `config` (`format`/`width`/`height`) and the mirror
    /// must be `Processing`; the frame travels the instance's
    /// `vshm` segment (host packs `in` canonically before the
    /// request, reads `out` only after a validated `Rendered`
    /// reply).
    Render {
        /// Instance handle.
        handle: PluginHandle,
        /// Input frame — `format`/`width`/`height` must equal the
        /// instance config's.
        input: VideoFrame,
        /// Presentation timestamp passed to `render`.
        at: MediaTime,
    },
    /// `Processing → Activated` → [`VideoHostReply::Ok`].
    StopProcessing {
        /// Instance handle.
        handle: PluginHandle,
    },
    /// `Activated → Initialized` → [`VideoHostReply::Ok`].
    Deactivate {
        /// Instance handle.
        handle: PluginHandle,
    },
    /// List the instance's parameter ids →
    /// [`VideoHostReply::ParamIds`].
    GetParams {
        /// Instance handle.
        handle: PluginHandle,
    },
    /// Owned parameter metadata → [`VideoHostReply::ParamInfo`]
    /// (`None` is the plugin-declared unknown-id reply).
    GetParamInfo {
        /// Instance handle.
        handle: PluginHandle,
        /// Parameter to describe.
        id: ParamId,
    },
    /// Current parameter value → [`VideoHostReply::ParamValue`].
    GetParamValue {
        /// Instance handle.
        handle: PluginHandle,
        /// Parameter to read.
        id: ParamId,
    },
    /// Write a parameter → [`VideoHostReply::Ok`]. A non-finite
    /// [`PlainParamValue::Double`] is rejected host-locally with
    /// `Err(HostError::InvalidValue)` — it never reaches the wire.
    SetParamValue {
        /// Instance handle.
        handle: PluginHandle,
        /// Parameter to write.
        id: ParamId,
        /// New parameter value.
        value: PlainParamValue,
    },
}

/// The reply vocabulary of [`VideoHostCommand`] — the video
/// counterpart of [`HostReply`](crate::HostReply).
#[non_exhaustive]
#[derive(Clone, Debug)]
pub enum VideoHostReply {
    /// The command completed with no payload.
    Ok,
    /// `Scan` result: this surface's registered descriptors.
    Scanned(Vec<PluginDescriptor>),
    /// `Instantiate` result: the minted handle.
    Instantiated {
        /// The new instance's unified handle.
        handle: PluginHandle,
    },
    /// `Render` result: the frame the plugin produced, restored
    /// out of the `vshm` `out` region.
    Rendered {
        /// Rendered frame — config dims/format, plugin-set
        /// metadata (`pts`, `duration`, `color`).
        frame: VideoFrame,
    },
    /// `GetParams` result: the instance's parameter ids.
    ParamIds(Vec<ParamId>),
    /// `GetParamInfo` result: owned parameter metadata, or `None`
    /// for the plugin-declared unknown-id reply.
    ParamInfo(Option<PlainParamInfoOwned>),
    /// `GetParamValue` result: the current parameter value.
    ParamValue(PlainParamValue),
}

/// Registration unit for the video out-of-process route — the
/// counterpart of [`OutOfProcessSpec`](crate::OutOfProcessSpec)
/// minus every field that could express a non-dedicated route:
/// there is no `isolation` (video is structurally Dedicated-only —
/// a bundled pool cannot be expressed) and no `lookahead_blocks`
/// (the video transport is request→reply, not the audio slot
/// ring's result horizon).
///
/// Not `#[non_exhaustive]`: callers mutate the pub fields after
/// `new` (the same shape as `OutOfProcessSpec` — a spec is
/// plain data, not a closed vocabulary).
#[derive(Debug)]
pub struct VideoOutOfProcessSpec {
    /// `'static` descriptor the child's `Scan` reply must
    /// field-match.
    pub descriptor: PluginDescriptor,
    /// Runner executable.
    pub program: OsString,
    /// Extra argv prefix for the runner.
    pub args: Vec<OsString>,
    /// Defined timeout for `Hello`, `Scan` and every non-render
    /// command's reply wait — default 10 s.
    pub command_timeout: Duration,
    /// Defined timeout for one `Render`'s request→reply window —
    /// default 60 s (a render legitimately outlasts
    /// `command_timeout`).
    pub render_timeout: Duration,
}

impl VideoOutOfProcessSpec {
    /// A spec with the design defaults (`command_timeout` 10 s,
    /// `render_timeout` 60 s); adjust the fields as needed.
    ///
    /// **RT contract:** non-RT only — allocation-free value move.
    #[must_use]
    pub fn new(descriptor: PluginDescriptor, program: OsString) -> Self {
        Self {
            descriptor,
            program,
            args: Vec::new(),
            command_timeout: Duration::from_secs(10),
            render_timeout: Duration::from_secs(60),
        }
    }
}

/// The borrowed video facade [`PluginHost::video`] returns —
/// `&mut PluginHost` minus the audio-only direct surface: video
/// instances live in the same slot/handle space, so `Scan`,
/// `Instantiate`, lifecycle, `Render` and parameter verbs route
/// through here while `Destroy`, `lifecycle_state`, `kill_child`
/// and `try_recv_notification` delegate back to the shared
/// composite semantics.
///
/// All operations are **non-RT only**.
#[non_exhaustive]
pub struct VideoPluginHost<'a> {
    host: &'a mut PluginHost,
}

impl<'a> VideoPluginHost<'a> {
    /// `pub(crate)` — minted only by [`PluginHost::video`].
    pub(crate) fn new(host: &'a mut PluginHost) -> Self {
        Self { host }
    }

    /// Register a dedicated video route — claims `spec.descriptor.id`
    /// for the video surface (`Err(DuplicatePluginId)` across both
    /// kinds). Every instantiation gets its own fresh child —
    /// there is no pooled video route.
    ///
    /// **RT contract:** non-RT only — map insert.
    pub fn register_out_of_process(
        &mut self,
        spec: VideoOutOfProcessSpec,
    ) -> Result<(), HostError> {
        self.host.video_register_out_of_process(spec)
    }

    /// Dispatch one [`VideoHostCommand`] — the video counterpart of
    /// [`PluginHost::execute`].
    ///
    /// **RT contract:** non-RT only — blocks up to
    /// `command_timeout` per command (`render_timeout` for `Render`).
    pub fn execute(&mut self, command: VideoHostCommand) -> Result<VideoHostReply, HostError> {
        self.host.video_execute(command)
    }

    /// The instance's mirrored lifecycle state — the same answer
    /// [`PluginHost::lifecycle_state`] gives an audio handle.
    ///
    /// **RT contract:** non-RT only by call site — a single atomic
    /// load plus a slot-table read.
    #[must_use]
    pub fn lifecycle_state(&self, handle: PluginHandle) -> Option<LifecycleState> {
        self.host.lifecycle_state(handle)
    }

    /// Kills the instance's child process — the
    /// `#[doc(hidden)] kill_child` contract for driving the crash
    /// paths in tests. The child's `Failed` convergence (and this
    /// instance's one-shot notification) lands asynchronously.
    ///
    /// **RT contract:** non-RT only.
    #[doc(hidden)]
    pub fn kill_child(&mut self, handle: PluginHandle) -> Result<(), HostError> {
        self.host.kill_child(handle)
    }

    /// Pops the next queued failure notification — the same queue
    /// [`PluginHost::try_recv_notification`] drains. One
    /// `InstanceFailed` per converged instance, exactly once.
    ///
    /// **RT contract:** non-RT only — `try_recv`, never blocks.
    #[must_use]
    pub fn try_recv_notification(&mut self) -> Option<HostNotification> {
        self.host.try_recv_notification()
    }
}
