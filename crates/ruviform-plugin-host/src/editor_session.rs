//! Driver-neutral plugin editor session (`PluginEditorSession`).
//!
//! This module is the session half of the native-dock contract (Issue
//! #389): it models the lifecycle of one plugin's native editor as a
//! typed state machine plus the typed host port ([`EditorSessionOps`])
//! an L0/L1 adapter implements. It carries an opaque
//! [`ParentSlotLease`](ruviform_plugin_core::ParentSlotLease) — never an
//! OS handle, never a `ruviform-ui` slot type — so plugin-side and
//! UI-side code never depend on each other. The GUI driver resolves the
//! lease back into a real parent inside its OS adapter; that adapter is
//! the only place a raw OS handle is resolved — the redeem seam mints
//! the generation-bound [`ParentLease`] the plugin's `open_gui`
//! consumes.
//!
//! Current-reality note (audit `docs/discovery/
//! issue-388-389-native-dock-current-state.md`): L1 GUI is still
//! `Unsupported` — [`L1EditorOps`] expresses that as a defined refusal on
//! every op and never degrades to L0. L0 dispatch lives through
//! [`InProcessEditorOps`], which drives the existing
//! [`crate::InProcessHost`] `OpenGui`/`CloseGui` command path; a plugin
//! whose `open_gui` is still the default `Unsupported` surfaces as
//! [`EditorOpError::Unsupported`], never a fake success.
//!
//! **RT contract:** non-RT only — every type and port here is
//! UI-thread only. No call allocates on or blocks an RT path.

use std::collections::HashMap;
use std::fmt;
use std::time::{Duration, Instant};

use ruviform_plugin_core::{
    HostError, ParentLease, ParentSlotLease, PluginEditorSessionId, PluginHandle,
};

use crate::command::{HostCommand, HostReply};
use crate::host::InProcessHost;

/// How a plugin editor is attached to the host presentation.
///
/// `Docked` is the real-Dock target (inside the GPUI main window's dock
/// layout); `Floating` is an explicitly separate, host-owned floating
/// window — a defined fallback that is *not* counted as a Dock PASS.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy.
///
/// **RT contract:** non-RT only — UI-thread only.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum AttachMode {
    /// Attached inside the host dock layout (the real-Dock target).
    Docked,
    /// Attached in a host-owned floating window (explicit fallback).
    Floating,
}

/// A plugin editor's logical size (scale-independent units).
///
/// **RT contract:** non-RT only — UI-thread only.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct EditorSize {
    /// Width in logical units.
    pub width: f64,
    /// Height in logical units.
    pub height: f64,
}

/// Whether a capability is advertised as supported or explicitly
/// unsupported.
///
/// `Unsupported` is a defined refusal — an operation requested against
/// it must return an error, never fake a success that could be
/// miscounted as a real Dock PASS. `Default` is `Unsupported` so a
/// capability set starts honest and opts in explicitly.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum Support {
    /// The adapter can perform this operation.
    Supported,
    /// Explicitly unsupported — a defined refusal, not a silent no-op.
    #[default]
    Unsupported,
}

impl Support {
    /// `true` when [`Support::Supported`].
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn supported(self) -> bool {
        matches!(self, Self::Supported)
    }
}

/// Capability flags a session adapter (L0/L1) reports.
///
/// An operation against a [`Support::Unsupported`] flag returns
/// [`EditorOpError::Unsupported`] — an explicit refusal, never a fake
/// success.
///
/// **RT contract:** non-RT only — UI-thread only.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct EditorCapabilities {
    /// Can attach the editor docked inside the host layout.
    pub dock: Support,
    /// Can attach the editor as a floating host-owned window.
    pub float: Support,
    /// Can negotiate a resize with the plugin.
    pub resize: Support,
    /// Can hide/show the editor (tab switching).
    pub hide_show: Support,
    /// Can re-parent the editor to another slot/window.
    pub reparent: Support,
    /// Supports IME inside the editor.
    pub ime: Support,
    /// Supports the plugin's own popup/menu windows.
    pub popup: Support,
    /// Can close/destroy the editor.
    pub close: Support,
}

/// Why a session fell into [`EditorSessionState::Failed`] or emitted a
/// terminal [`EditorSessionEvent::Terminated`].
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy.
///
/// **RT contract:** non-RT only — UI-thread only.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum SessionFailCause {
    /// The adapter/plugin explicitly does not support the operation.
    Unsupported,
    /// The parent lease was rejected (stale, revoked, or unknown).
    LeaseInvalid,
    /// A request deadline elapsed.
    Timeout,
    /// The host/instance/process is gone (crash, destroy, child exit).
    HostLost,
    /// The graphics device/driver was lost (device removed, driver
    /// reset, compositor failure) — the editor's pixels may be gone
    /// while the DSP instance survives.
    GraphicsLost,
    /// The parent window/slot vanished under the session.
    WindowLost,
    /// A plugin hook declined.
    PluginFailed,
    /// An (L1) transport/IPC failure.
    Transport,
    /// Internal invariant failure.
    Internal,
}

/// Lifecycle state of one [`PluginEditorSession`].
///
/// `{request}` carries the in-flight request id so a completion can be
/// matched and stale callbacks rejected.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy.
///
/// **RT contract:** non-RT only — UI-thread only.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum EditorSessionState {
    /// No editor — the initial and terminal state.
    Closed,
    /// An open/attach request is in flight.
    Creating {
        /// The in-flight request id.
        request: u64,
    },
    /// The editor is attached and visible.
    Attached {
        /// The active attach mode.
        mode: AttachMode,
    },
    /// The editor is attached but hidden (tab inactive).
    Hidden {
        /// The active attach mode.
        mode: AttachMode,
    },
    /// A detach (pre re-parent/move) request is in flight.
    Detaching {
        /// The in-flight request id.
        request: u64,
    },
    /// A close request is in flight.
    Closing {
        /// The in-flight request id.
        request: u64,
    },
    /// The session failed — recoverable via `recover` or closable.
    Failed {
        /// Why it failed.
        cause: SessionFailCause,
    },
    /// A recovery (re-open after failure) is in flight.
    Recovering {
        /// The in-flight request id.
        request: u64,
    },
}

/// Events emitted by [`PluginEditorSession`].
///
/// `Terminated` is the single terminal event — it fires exactly once
/// when a live session converges to `Closed` on instance destroy, host
/// crash, window loss or child exit. `Closed` is the normal end of a
/// graceful close.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy.
///
/// **RT contract:** non-RT only — UI-thread only.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum EditorSessionEvent {
    /// The editor attached and is visible.
    Attached {
        /// The attach mode.
        mode: AttachMode,
    },
    /// The editor was hidden.
    Hidden,
    /// The editor was shown.
    Shown {
        /// The attach mode.
        mode: AttachMode,
    },
    /// The editor detached (pre re-parent/move).
    Detached,
    /// The editor re-attached under a new parent.
    Reattached {
        /// The attach mode.
        mode: AttachMode,
    },
    /// The session recovered from `Failed` back to `Attached`.
    Recovered {
        /// The attach mode.
        mode: AttachMode,
    },
    /// The editor attached via an explicit degrade to `mode` — the
    /// requested mode was refused and the session fell back (e.g. a
    /// floating window). A degrade is *not* a requested-mode attach
    /// and never counts as a Native Dock PASS; it is emitted instead
    /// of `Attached`/`Recovered`/`Reattached`.
    Degraded {
        /// The mode the session degraded to.
        mode: AttachMode,
    },
    /// The session failed.
    Failed {
        /// Why it failed.
        cause: SessionFailCause,
    },
    /// The editor closed gracefully — normal terminal event.
    Closed,
    /// The session terminated abnormally (destroy/crash/loss) — fires
    /// exactly once, then the session is `Closed`.
    Terminated {
        /// Why it terminated.
        cause: SessionFailCause,
    },
}

/// Errors returned by the [`EditorSessionOps`] adapter layer.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy.
///
/// **RT contract:** non-RT only — UI-thread only.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum EditorOpError {
    /// The adapter/plugin does not support this operation — the
    /// defined refusal for the current L1 path and for unimplemented
    /// `open_gui`.
    Unsupported,
    /// The parent lease was rejected (stale/revoked/unknown).
    LeaseRejected,
    /// The operation deadline elapsed.
    Timeout,
    /// The host/instance is gone.
    HostLost,
    /// The graphics device/driver was lost.
    GraphicsLost,
    /// A plugin hook declined.
    Plugin,
    /// The session/editor is already closed.
    Closed,
    /// The operation is illegal for the current adapter state.
    InvalidState,
    /// Transport/IPC failure.
    Io,
    /// A caller-supplied value fell outside its defined domain.
    InvalidValue,
    /// An internal invariant or unmapped host error.
    Internal,
}

/// Errors returned by [`PluginEditorSession`]/[`PluginEditorSessions`].
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy.
///
/// **RT contract:** non-RT only — UI-thread only.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum EditorSessionError {
    /// The transition is illegal from the current state (double-open,
    /// double-close, op on a closed session).
    InvalidState,
    /// A completion's request id does not match the in-flight request —
    /// a stale async callback is ignored, never converges wrongly.
    StaleRequest,
    /// The instance already has a live (non-`Closed`) session.
    SessionBusy,
    /// The session id names no live session.
    UnknownSession,
    /// A caller-supplied value fell outside its defined domain (e.g. a
    /// non-finite or negative size).
    InvalidValue,
    /// The adapter layer declined the operation; the session maps it
    /// into its state/event vocabulary.
    Ops(EditorOpError),
}

impl fmt::Display for EditorOpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            Self::Unsupported => "operation unsupported by adapter",
            Self::LeaseRejected => "parent lease rejected",
            Self::Timeout => "operation deadline elapsed",
            Self::HostLost => "host or instance is gone",
            Self::GraphicsLost => "graphics device or driver lost",
            Self::Plugin => "plugin hook declined",
            Self::Closed => "editor session is closed",
            Self::InvalidState => "operation illegal for adapter state",
            Self::Io => "transport or IPC failure",
            Self::InvalidValue => "caller-supplied value outside its domain",
            Self::Internal => "internal invariant or unmapped host error",
        };
        f.write_str(msg)
    }
}

impl std::error::Error for EditorOpError {}

impl fmt::Display for EditorSessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidState => f.write_str("transition illegal from current session state"),
            Self::StaleRequest => f.write_str("stale request id on completion"),
            Self::SessionBusy => f.write_str("instance already has a live editor session"),
            Self::UnknownSession => f.write_str("unknown editor session id"),
            Self::InvalidValue => f.write_str("caller-supplied value outside its domain"),
            Self::Ops(error) => write!(f, "editor adapter declined: {error}"),
        }
    }
}

impl std::error::Error for EditorSessionError {}

/// Monotonic clock port for editor-session request deadlines.
///
/// Values are only meaningful as differences — the epoch is the
/// implementation's own. This is a UI-thread (non-RT) port: deadline
/// evaluation happens when the driver calls
/// [`PluginEditorSession::tick`] or [`PluginEditorSessions::tick_all`],
/// never from an RT path.
///
/// **RT contract:** non-RT only — UI-thread only.
pub trait SessionClock {
    /// Current monotonic time in the clock's own domain.
    fn now(&self) -> Duration;
}

/// Default request deadline for in-flight editor-session requests.
///
/// Armed on every session unless
/// [`PluginEditorSession::set_deadline_policy`] replaces or clears it.
/// Enforcement still requires the driver to call
/// [`PluginEditorSession::tick`] / [`PluginEditorSessions::tick_all`] —
/// a session whose driver never ticks leaves requests unmonitored
/// (the pre-hardening behavior).
///
/// **RT contract:** non-RT only — UI-thread only.
pub const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// `SessionClock` over `std::time::Instant` — monotonic elapsed time
/// since construction.
///
/// **RT contract:** non-RT only — UI-thread only.
#[derive(Debug)]
pub struct MonotonicSessionClock {
    start: Instant,
}

impl MonotonicSessionClock {
    /// Creates a clock reading `Duration::ZERO` at construction.
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    #[must_use]
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
        }
    }
}

impl Default for MonotonicSessionClock {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionClock for MonotonicSessionClock {
    fn now(&self) -> Duration {
        self.start.elapsed()
    }
}

/// Bounded-timeout policy for in-flight editor-session requests.
///
/// Installed via [`PluginEditorSession::set_deadline_policy`]; while
/// installed, every request minted by `begin_*`/`complete_detach`
/// records an absolute deadline (`clock.now() + timeout`) that
/// [`PluginEditorSession::tick`] evaluates. A `timeout` of
/// [`Duration::ZERO`] expires the request on the next tick — a valid
/// fail-fast policy.
///
/// **RT contract:** non-RT only — UI-thread only.
pub struct SessionDeadlinePolicy {
    /// Maximum time an in-flight request may remain incomplete.
    pub timeout: Duration,
    /// Monotonic clock the deadline is evaluated against.
    pub clock: Box<dyn SessionClock>,
}

impl fmt::Debug for SessionDeadlinePolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SessionDeadlinePolicy")
            .field("timeout", &self.timeout)
            .finish_non_exhaustive()
    }
}

/// How an open request concluded — a clean attach or an explicit
/// degrade to a fallback mode.
///
/// **RT contract:** non-RT only — UI-thread only.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum OpenOutcome {
    /// Attached in the requested mode — a clean attach.
    Attached(EditorCapabilities),
    /// The requested mode was refused ([`EditorOpError::Unsupported`])
    /// and the session explicitly degraded to `mode`. This is *not* a
    /// requested-mode attach and never counts as a Native Dock PASS —
    /// the session emits [`EditorSessionEvent::Degraded`] instead of
    /// `Attached`/`Recovered`.
    Degraded {
        /// The mode the session degraded to.
        mode: AttachMode,
        /// Capabilities reported by the fallback attach.
        capabilities: EditorCapabilities,
    },
}

/// Maps an adapter [`EditorOpError`] to the session-level
/// [`SessionFailCause`].
fn fail_cause(error: EditorOpError) -> SessionFailCause {
    match error {
        EditorOpError::Unsupported => SessionFailCause::Unsupported,
        EditorOpError::LeaseRejected => SessionFailCause::LeaseInvalid,
        EditorOpError::Timeout => SessionFailCause::Timeout,
        EditorOpError::HostLost => SessionFailCause::HostLost,
        EditorOpError::GraphicsLost => SessionFailCause::GraphicsLost,
        EditorOpError::Plugin => SessionFailCause::PluginFailed,
        EditorOpError::Io => SessionFailCause::Transport,
        EditorOpError::Closed
        | EditorOpError::InvalidState
        | EditorOpError::InvalidValue
        | EditorOpError::Internal => SessionFailCause::Internal,
    }
}

/// The typed host port an L0/L1 adapter implements for editor sessions.
///
/// Each op takes the session id (for adapter bookkeeping) and returns a
/// [`Result`] — `Err` maps into the session's failure vocabulary. The
/// port never names an OS handle; the `lease` is the opaque
/// [`ParentSlotLease`] the adapter redeems internally.
///
/// **RT contract:** non-RT only — UI-thread only.
pub trait EditorSessionOps {
    /// Reports what this adapter can do for `session`.
    ///
    /// # Errors
    /// Returns [`EditorOpError`] on a bad session or backend failure.
    fn capabilities(
        &self,
        session: PluginEditorSessionId,
    ) -> Result<EditorCapabilities, EditorOpError>;

    /// Opens the editor under `lease` at `size` in `mode`, returning
    /// the resulting capabilities.
    ///
    /// # Errors
    /// Returns [`EditorOpError`] — e.g. `LeaseRejected`, `Unsupported`
    /// (unimplemented `open_gui` or the L1 path), `Timeout`, `HostLost`.
    fn open(
        &mut self,
        session: PluginEditorSessionId,
        lease: &ParentSlotLease,
        size: EditorSize,
        mode: AttachMode,
    ) -> Result<EditorCapabilities, EditorOpError>;

    /// Closes the editor.
    ///
    /// # Errors
    /// Returns [`EditorOpError`] on failure.
    fn close(&mut self, session: PluginEditorSessionId) -> Result<(), EditorOpError>;

    /// Requests a resize to `size`.
    ///
    /// # Errors
    /// Returns [`EditorOpError`] — a plugin that refuses a size returns
    /// `Unsupported`/`Plugin` (a defined rejection, not a crash).
    fn set_size(
        &mut self,
        session: PluginEditorSessionId,
        size: EditorSize,
    ) -> Result<(), EditorOpError>;

    /// Shows (`visible = true`) or hides the editor.
    ///
    /// # Errors
    /// Returns [`EditorOpError`] on failure.
    fn set_visibility(
        &mut self,
        session: PluginEditorSessionId,
        visible: bool,
    ) -> Result<(), EditorOpError>;

    /// Re-parents the editor under a new `lease` (dock move / float /
    /// re-dock).
    ///
    /// # Errors
    /// Returns [`EditorOpError`] — `LeaseRejected` if the new lease is
    /// invalid, `Unsupported` if the adapter cannot re-parent.
    fn reattach(
        &mut self,
        session: PluginEditorSessionId,
        lease: &ParentSlotLease,
    ) -> Result<(), EditorOpError>;
}

/// Which terminal event an in-flight `Creating`/`Recovering` attach
/// emits on success — the session's pending-attach marker. The
/// variants are mutually exclusive by construction: a `Creating`
/// reached via `complete_detach` re-attaches, while
/// `open_or_degrade`/`begin_degrade` mark an explicit degrade.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
enum PendingAttach {
    /// Plain attach — `Attached`/`Recovered`.
    #[default]
    None,
    /// `Creating` reached via `complete_detach` — `Reattached`.
    Reattach,
    /// Explicit degrade — `Degraded`, never a requested-mode attach.
    Degrade,
}

/// One plugin editor session: a pure state machine over
/// [`EditorSessionOps`].
///
/// Owns its [`PluginEditorSessionId`], the instance
/// [`PluginHandle`] it belongs to, the in-flight request counter and a
/// drained event queue. Transitions happen through the synchronous
/// convenience methods (`open`/`close`/`set_size`/`set_visibility`/
/// `reattach`/`recover`), which mint a request, call the ops and
/// complete atomically, and through the `begin_*`/`complete_*` pair plus
/// the `notify_*` pushes an asynchronous driver uses — every completion
/// is matched by request id so a stale callback is rejected, never
/// converges to the wrong state.
///
/// **Terminal-event-once:** `notify_terminated` emits
/// [`EditorSessionEvent::Terminated`] exactly once and lands the session
/// in `Closed`; repeated notifies and late callbacks are ignored.
///
/// **RT contract:** non-RT only — UI-thread only.
#[derive(Debug)]
pub struct PluginEditorSession {
    id: PluginEditorSessionId,
    instance: PluginHandle,
    state: EditorSessionState,
    /// Last minted request id (monotonic per session).
    request: u64,
    /// The mode an in-flight `Creating`/`Recovering` should land in.
    pending_mode: AttachMode,
    /// The terminal event an in-flight `Creating`/`Recovering` attach
    /// emits on success — [`PendingAttach::Reattach`] for a
    /// detach-for-reattach `Creating`, [`PendingAttach::Degrade`] for
    /// an explicit degrade (`open_or_degrade`/`begin_degrade`).
    pending_attach: PendingAttach,
    /// Whether an adapter-side editor may still exist: set by a
    /// successful attach, cleared on conclusion. A refused fresh open
    /// never produced an editor, so its `Failed` session closes without
    /// an adapter `close` — otherwise the session pins its instance.
    editor_live: bool,
    /// Whether the session reached `Closed` by an actual lifecycle
    /// path (graceful close or termination) — distinct from a session
    /// that was merely *born* `Closed` and has not opened yet. Only a
    /// concluded session frees its instance binding.
    concluded: bool,
    /// Wall-clock deadline policy for in-flight requests — `None`
    /// leaves requests unbounded (the pre-hardening behavior).
    deadline_policy: Option<SessionDeadlinePolicy>,
    /// Absolute deadline of the current in-flight request in the
    /// policy clock's domain — meaningful only while
    /// `in_flight()`; re-armed by every `next_request`.
    deadline_at: Option<Duration>,
    events: Vec<EditorSessionEvent>,
}

impl PluginEditorSession {
    /// Creates a `Closed` session bound to `id`/`instance`.
    ///
    /// `#[doc(hidden)]` public because minting is a registry-internal
    /// operation — sessions are created through
    /// [`PluginEditorSessions::begin_session`].
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    #[doc(hidden)]
    #[must_use]
    pub fn new(id: PluginEditorSessionId, instance: PluginHandle) -> Self {
        Self {
            id,
            instance,
            state: EditorSessionState::Closed,
            request: 0,
            pending_mode: AttachMode::Docked,
            pending_attach: PendingAttach::None,
            editor_live: false,
            concluded: false,
            deadline_policy: Some(SessionDeadlinePolicy {
                timeout: DEFAULT_REQUEST_TIMEOUT,
                clock: Box::new(MonotonicSessionClock::new()),
            }),
            deadline_at: None,
            events: Vec::new(),
        }
    }

    /// The session id.
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    #[must_use]
    pub fn id(&self) -> PluginEditorSessionId {
        self.id
    }

    /// The instance this session belongs to.
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    #[must_use]
    pub fn instance(&self) -> PluginHandle {
        self.instance
    }

    /// The current state.
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    #[must_use]
    pub fn state(&self) -> EditorSessionState {
        self.state
    }

    /// Whether the session converged to `Closed` through a real
    /// lifecycle path (graceful close or termination) — as opposed to
    /// still being a never-opened session. A concluded session frees
    /// its instance binding; a born-`Closed` one does not.
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    #[must_use]
    pub fn concluded(&self) -> bool {
        self.concluded
    }

    /// Drains pending events.
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    pub fn drain_events(&mut self) -> Vec<EditorSessionEvent> {
        std::mem::take(&mut self.events)
    }

    fn next_request(&mut self) -> u64 {
        self.request = self.request.wrapping_add(1);
        // Every caller enters an in-flight state right after — arm
        // unconditionally (the pre-request state may not be
        // in-flight yet, e.g. `Closed → Creating`).
        self.deadline_at = self
            .deadline_policy
            .as_ref()
            .map(|policy| policy.clock.now().saturating_add(policy.timeout));
        self.request
    }

    /// Whether a request is in flight — the states a deadline covers.
    fn in_flight(&self) -> bool {
        matches!(
            self.state,
            EditorSessionState::Creating { .. }
                | EditorSessionState::Detaching { .. }
                | EditorSessionState::Closing { .. }
                | EditorSessionState::Recovering { .. }
        )
    }

    /// (Re)arms `deadline_at` for the current in-flight request from
    /// the installed policy — `None` clears it when no policy is
    /// installed or no request is in flight.
    fn arm_deadline(&mut self) {
        self.deadline_at = if self.in_flight() {
            self.deadline_policy
                .as_ref()
                .map(|policy| policy.clock.now().saturating_add(policy.timeout))
        } else {
            None
        };
    }

    fn push(&mut self, event: EditorSessionEvent) {
        self.events.push(event);
    }

    fn mark_failed(&mut self, cause: SessionFailCause) {
        self.pending_attach = PendingAttach::None;
        self.state = EditorSessionState::Failed { cause };
        self.push(EditorSessionEvent::Failed { cause });
    }

    // ---- request-initiating (low-level async API) ----

    /// `Closed → Creating{request}`; returns the minted request id.
    ///
    /// # Errors
    /// [`EditorSessionError::InvalidState`] unless `Closed` (double-open
    /// / re-open of a live session is refused).
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    pub fn begin_open(&mut self, mode: AttachMode) -> Result<u64, EditorSessionError> {
        // A `concluded` session is `Closed` but dead — its lifecycle
        // already converged; reopening it would let the registry retire
        // a *live* session (two editors per instance).
        if self.concluded || !matches!(self.state, EditorSessionState::Closed) {
            return Err(EditorSessionError::InvalidState);
        }
        self.pending_mode = mode;
        let request = self.next_request();
        self.state = EditorSessionState::Creating { request };
        Ok(request)
    }

    /// `{Attached|Hidden} → Detaching{request}`; returns the request id.
    ///
    /// # Errors
    /// [`EditorSessionError::InvalidState`] unless attached/hidden.
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    pub fn begin_detach(&mut self) -> Result<u64, EditorSessionError> {
        match self.state {
            EditorSessionState::Attached { .. } | EditorSessionState::Hidden { .. } => {}
            _ => return Err(EditorSessionError::InvalidState),
        }
        let request = self.next_request();
        self.state = EditorSessionState::Detaching { request };
        Ok(request)
    }

    /// `{Attached|Hidden|Detaching|Failed} → Closing{request}`.
    ///
    /// # Errors
    /// [`EditorSessionError::InvalidState`] on `Closed`/`Closing`/
    /// `Creating`/`Recovering` (double-close / close of a never-opened
    /// or mid-create session is refused at this level — a pending
    /// create/recover is abandoned either by completing it with an
    /// `Err` or by `notify_terminated`).
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    pub fn begin_close(&mut self) -> Result<u64, EditorSessionError> {
        match self.state {
            EditorSessionState::Attached { .. }
            | EditorSessionState::Hidden { .. }
            | EditorSessionState::Detaching { .. }
            | EditorSessionState::Failed { .. } => {}
            _ => return Err(EditorSessionError::InvalidState),
        }
        let request = self.next_request();
        self.state = EditorSessionState::Closing { request };
        Ok(request)
    }

    /// `Failed → Recovering{request}`; returns the request id.
    ///
    /// # Errors
    /// [`EditorSessionError::InvalidState`] unless `Failed`.
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    pub fn begin_recover(&mut self, mode: AttachMode) -> Result<u64, EditorSessionError> {
        if !matches!(self.state, EditorSessionState::Failed { .. }) {
            return Err(EditorSessionError::InvalidState);
        }
        self.pending_mode = mode;
        let request = self.next_request();
        self.state = EditorSessionState::Recovering { request };
        Ok(request)
    }

    /// `Failed → Recovering{request}` marked as an explicit degrade
    /// to `mode`; returns the request id.
    ///
    /// Same transition as [`Self::begin_recover`], but a successful
    /// [`Self::complete_attach`] lands `Attached{mode}` emitting
    /// [`EditorSessionEvent::Degraded`] instead of `Recovered` — the
    /// async counterpart of [`Self::open_or_degrade`] for drivers
    /// that drive `ops.open` themselves. Use when the preferred mode
    /// was refused and the session falls back (e.g. Docked →
    /// Floating); a degrade is never a Native Dock PASS.
    ///
    /// # Errors
    /// [`EditorSessionError::InvalidState`] unless `Failed`.
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    pub fn begin_degrade(&mut self, mode: AttachMode) -> Result<u64, EditorSessionError> {
        let request = self.begin_recover(mode)?;
        self.pending_attach = PendingAttach::Degrade;
        Ok(request)
    }

    /// Completes an in-flight `Creating`/`Recovering` attach request.
    ///
    /// `Ok` lands the session `Attached{pending_mode}` (emitting
    /// `Attached` or `Recovered`); `Err` lands it `Failed`. The `request`
    /// must match the in-flight id.
    ///
    /// # Errors
    /// [`EditorSessionError::StaleRequest`] on a mismatched/missing
    /// in-flight request — the callback is ignored, state unchanged.
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    pub fn complete_attach(
        &mut self,
        request: u64,
        result: Result<(), EditorOpError>,
    ) -> Result<(), EditorSessionError> {
        let mode = self.pending_mode;
        match (self.state, result) {
            (
                EditorSessionState::Creating { request: r }
                | EditorSessionState::Recovering { request: r },
                _,
            ) if r != request => Err(EditorSessionError::StaleRequest),
            (EditorSessionState::Creating { .. }, Ok(())) => {
                self.state = EditorSessionState::Attached { mode };
                self.editor_live = true;
                match self.pending_attach {
                    // An explicit degrade emits `Degraded` — never a
                    // requested-mode `Attached`.
                    PendingAttach::Degrade => {
                        self.pending_attach = PendingAttach::None;
                        self.push(EditorSessionEvent::Degraded { mode });
                    }
                    // A `Creating` reached through `complete_detach`
                    // is a re-attach — same event vocabulary as the
                    // sync path.
                    PendingAttach::Reattach => {
                        self.pending_attach = PendingAttach::None;
                        self.push(EditorSessionEvent::Reattached { mode });
                    }
                    PendingAttach::None => {
                        self.push(EditorSessionEvent::Attached { mode });
                    }
                }
                Ok(())
            }
            (EditorSessionState::Recovering { .. }, Ok(())) => {
                self.state = EditorSessionState::Attached { mode };
                self.editor_live = true;
                let degraded = self.pending_attach == PendingAttach::Degrade;
                self.pending_attach = PendingAttach::None;
                if degraded {
                    self.push(EditorSessionEvent::Degraded { mode });
                } else {
                    self.push(EditorSessionEvent::Recovered { mode });
                }
                Ok(())
            }
            (
                EditorSessionState::Creating { .. } | EditorSessionState::Recovering { .. },
                Err(error),
            ) => {
                self.mark_failed(fail_cause(error));
                Err(EditorSessionError::Ops(error))
            }
            _ => Err(EditorSessionError::StaleRequest),
        }
    }

    /// Completes an in-flight `Closing` request → `Closed`.
    ///
    /// # Errors
    /// [`EditorSessionError::StaleRequest`] on a mismatched request.
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    pub fn complete_close(&mut self, request: u64) -> Result<(), EditorSessionError> {
        match self.state {
            EditorSessionState::Closing { request: r } if r == request => {
                self.state = EditorSessionState::Closed;
                self.editor_live = false;
                self.concluded = true;
                self.push(EditorSessionEvent::Closed);
                Ok(())
            }
            _ => Err(EditorSessionError::StaleRequest),
        }
    }

    /// Completes an in-flight `Detaching` request.
    ///
    /// `Ok` re-enters `Creating` with a fresh request (awaiting a
    /// re-attach `complete_attach`); `Err` lands `Failed`. Emits
    /// `Detached` on success.
    ///
    /// # Errors
    /// [`EditorSessionError::StaleRequest`] on a mismatched request.
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    pub fn complete_detach(
        &mut self,
        request: u64,
        result: Result<(), EditorOpError>,
    ) -> Result<u64, EditorSessionError> {
        match (self.state, result) {
            (EditorSessionState::Detaching { request: r }, _) if r != request => {
                Err(EditorSessionError::StaleRequest)
            }
            (EditorSessionState::Detaching { .. }, Ok(())) => {
                self.push(EditorSessionEvent::Detached);
                self.pending_attach = PendingAttach::Reattach;
                let request = self.next_request();
                self.state = EditorSessionState::Creating { request };
                Ok(request)
            }
            (EditorSessionState::Detaching { .. }, Err(error)) => {
                self.mark_failed(fail_cause(error));
                Err(EditorSessionError::Ops(error))
            }
            _ => Err(EditorSessionError::StaleRequest),
        }
    }

    // ---- driver/lifecycle notifications ----

    /// Converges a live session to `Closed`, emitting the single
    /// terminal [`EditorSessionEvent::Terminated`].
    ///
    /// Used on instance destroy, host crash, child exit and window/slot
    /// loss. Fires `Terminated` exactly once — a second notify or any
    /// late callback on an already-`Closed` session is ignored.
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    pub fn notify_terminated(&mut self, cause: SessionFailCause) {
        if matches!(self.state, EditorSessionState::Closed) {
            // Nothing live to terminate — a never-opened or already
            // converged session is poisoned so it can never be opened.
            self.concluded = true;
            return;
        }
        self.concluded = true;
        self.editor_live = false;
        self.state = EditorSessionState::Closed;
        self.push(EditorSessionEvent::Terminated { cause });
    }

    // ---- deadline monitoring (non-RT, driver-driven) ----

    /// Installs (or replaces) the wall-clock deadline policy for
    /// in-flight requests; `None` disarms — requests then stay in
    /// flight until their driver completes them.
    ///
    /// While a policy is installed, every request minted by
    /// `begin_*`/`complete_detach` records an absolute deadline in
    /// the policy clock's domain; [`Self::tick`] expires overdue
    /// requests as [`EditorOpError::Timeout`] → `Failed{Timeout}`.
    /// Re-installing while a request is in flight re-arms its
    /// deadline against the new clock/timeout.
    ///
    /// # Errors — none (infallible).
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    pub fn set_deadline_policy(&mut self, policy: Option<SessionDeadlinePolicy>) {
        self.deadline_policy = policy;
        self.arm_deadline();
    }

    /// The armed deadline of the current in-flight request, in the
    /// policy clock's domain — `None` when no policy is installed or
    /// no request is in flight. A driver uses this to schedule its
    /// next [`Self::tick`].
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    #[must_use]
    pub fn request_deadline(&self) -> Option<Duration> {
        if self.in_flight() {
            self.deadline_at
        } else {
            None
        }
    }

    /// Evaluates the deadline policy once: an in-flight request past
    /// its deadline completes as [`EditorOpError::Timeout`] — the
    /// session lands `Failed{Timeout}` and emits `Failed{Timeout}`.
    /// Returns `true` when a request timed out.
    ///
    /// Call periodically from the GUI timer; a request that is never
    /// ticked stays in flight (monitoring is the driver's job). A
    /// late `complete_*` after a timeout is `StaleRequest`, so
    /// terminal events never duplicate.
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    pub fn tick(&mut self) -> bool {
        let expired = self.deadline_policy.as_ref().is_some_and(|policy| {
            self.in_flight()
                && self
                    .deadline_at
                    .is_some_and(|deadline| policy.clock.now() >= deadline)
        });
        if !expired {
            return false;
        }
        match self.state {
            EditorSessionState::Creating { request }
            | EditorSessionState::Recovering { request } => {
                // `complete_attach` maps `Timeout` → `Failed{Timeout}`.
                let _ = self.complete_attach(request, Err(EditorOpError::Timeout));
            }
            EditorSessionState::Detaching { request } => {
                // `complete_detach` maps `Timeout` → `Failed{Timeout}`.
                let _ = self.complete_detach(request, Err(EditorOpError::Timeout));
            }
            EditorSessionState::Closing { .. } => {
                // `complete_close` carries no error channel — a
                // timed-out close fails the session instead;
                // `editor_live` stays set because the adapter may
                // still have closed it, so a later `close` can
                // converge via the adapter path.
                self.mark_failed(SessionFailCause::Timeout);
            }
            _ => {}
        }
        true
    }

    // ---- synchronous convenience (ops-driven) ----

    /// Full open: `begin_open` + `ops.open` + `complete_attach`.
    ///
    /// On success the session is `Attached{mode}`; on adapter refusal it
    /// is `Failed` and the [`EditorOpError`] is returned.
    ///
    /// # Errors
    /// [`EditorSessionError::InvalidState`] unless `Closed`;
    /// [`EditorSessionError::Ops`] on an adapter refusal.
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    pub fn open<O: EditorSessionOps>(
        &mut self,
        ops: &mut O,
        lease: &ParentSlotLease,
        size: EditorSize,
        mode: AttachMode,
    ) -> Result<EditorCapabilities, EditorSessionError> {
        if !size.width.is_finite()
            || !size.height.is_finite()
            || size.width < 0.0
            || size.height < 0.0
        {
            return Err(EditorSessionError::InvalidValue);
        }
        let request = self.begin_open(mode)?;
        match ops.open(self.id, lease, size, mode) {
            Ok(capabilities) => {
                self.complete_attach(request, Ok(()))?;
                Ok(capabilities)
            }
            Err(error) => {
                let _ = self.complete_attach(request, Err(error));
                Err(EditorSessionError::Ops(error))
            }
        }
    }

    /// Full close: `begin_close` + `ops.close` + `complete_close`.
    ///
    /// On adapter refusal the session lands `Failed` and the error is
    /// returned.
    ///
    /// A `Failed` session whose open was refused has no adapter editor,
    /// so it concludes without an adapter `close` call.
    ///
    /// # Errors
    /// [`EditorSessionError::InvalidState`] on double-close / never-open;
    /// [`EditorSessionError::Ops`] on an adapter refusal.
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    pub fn close<O: EditorSessionOps>(&mut self, ops: &mut O) -> Result<(), EditorSessionError> {
        let request = self.begin_close()?;
        if !self.editor_live {
            return self.complete_close(request);
        }
        match ops.close(self.id) {
            Ok(()) => self.complete_close(request),
            Err(error) => {
                self.mark_failed(fail_cause(error));
                Err(EditorSessionError::Ops(error))
            }
        }
    }

    /// Resizes the editor. A plugin rejection is non-fatal: the session
    /// stays `Attached`/`Hidden` and the error is returned.
    ///
    /// # Errors
    /// [`EditorSessionError::InvalidState`] unless attached/hidden;
    /// [`EditorSessionError::Ops`] on a refusal (e.g. plugin rejects the
    /// size) — the session is left unchanged.
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    pub fn set_size<O: EditorSessionOps>(
        &mut self,
        ops: &mut O,
        size: EditorSize,
    ) -> Result<(), EditorSessionError> {
        if !size.width.is_finite()
            || !size.height.is_finite()
            || size.width < 0.0
            || size.height < 0.0
        {
            return Err(EditorSessionError::InvalidValue);
        }
        if !matches!(
            self.state,
            EditorSessionState::Attached { .. } | EditorSessionState::Hidden { .. }
        ) {
            return Err(EditorSessionError::InvalidState);
        }
        ops.set_size(self.id, size).map_err(EditorSessionError::Ops)
    }

    /// Shows/hides the editor (`Attached ↔ Hidden`).
    ///
    /// # Errors
    /// [`EditorSessionError::InvalidState`] unless attached/hidden;
    /// [`EditorSessionError::Ops`] on a refusal (session unchanged).
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    pub fn set_visibility<O: EditorSessionOps>(
        &mut self,
        ops: &mut O,
        visible: bool,
    ) -> Result<(), EditorSessionError> {
        let (EditorSessionState::Attached { mode } | EditorSessionState::Hidden { mode }) =
            self.state
        else {
            return Err(EditorSessionError::InvalidState);
        };
        ops.set_visibility(self.id, visible)
            .map_err(EditorSessionError::Ops)?;
        // A no-op transition emits no event — event consumers count
        // real state changes only.
        let unchanged = matches!(self.state, EditorSessionState::Attached { .. }) == visible;
        if unchanged {
            return Ok(());
        }
        if visible {
            self.state = EditorSessionState::Attached { mode };
            self.push(EditorSessionEvent::Shown { mode });
        } else {
            self.state = EditorSessionState::Hidden { mode };
            self.push(EditorSessionEvent::Hidden);
        }
        Ok(())
    }

    /// Re-parents the editor under a new `lease` (dock move / float /
    /// re-dock).
    ///
    /// From `Attached`/`Hidden` it detaches then re-attaches; from an
    /// already-`Detaching` state it completes into the re-attach. On
    /// adapter refusal the session lands `Failed`.
    ///
    /// # Errors
    /// [`EditorSessionError::InvalidState`] unless attached/hidden/
    /// detaching; [`EditorSessionError::Ops`] on a refusal (e.g.
    /// `LeaseRejected` on a stale new lease).
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    pub fn reattach<O: EditorSessionOps>(
        &mut self,
        ops: &mut O,
        lease: &ParentSlotLease,
    ) -> Result<(), EditorSessionError> {
        // Sync and async paths emit the same `[Detached, Reattached]`
        // vocabulary — `complete_detach` marks the re-attach so the
        // landing `complete_attach` reports `Reattached`.
        let (mode, completing) = match self.state {
            EditorSessionState::Attached { mode } | EditorSessionState::Hidden { mode } => {
                let request = self.begin_detach()?;
                let completing = self.complete_detach(request, Ok(()))?;
                (mode, Some(completing))
            }
            EditorSessionState::Detaching { .. } => {
                // The caller's in-flight detach completes inside the
                // atomic `ops.reattach` — mirror its events.
                self.push(EditorSessionEvent::Detached);
                self.pending_attach = PendingAttach::Reattach;
                (self.pending_mode, None)
            }
            _ => return Err(EditorSessionError::InvalidState),
        };
        match ops.reattach(self.id, lease) {
            Ok(()) => {
                if let Some(request) = completing {
                    self.complete_attach(request, Ok(()))?;
                } else {
                    self.pending_attach = PendingAttach::None;
                    self.state = EditorSessionState::Attached { mode };
                    self.push(EditorSessionEvent::Reattached { mode });
                }
                Ok(())
            }
            Err(error) => {
                self.mark_failed(fail_cause(error));
                Err(EditorSessionError::Ops(error))
            }
        }
    }

    /// Recovers a `Failed` session: `begin_recover` + `ops.open` +
    /// `complete_attach`.
    ///
    /// # Errors
    /// [`EditorSessionError::InvalidState`] unless `Failed`;
    /// [`EditorSessionError::Ops`] on a refusal (stays `Failed`).
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    pub fn recover<O: EditorSessionOps>(
        &mut self,
        ops: &mut O,
        lease: &ParentSlotLease,
        size: EditorSize,
        mode: AttachMode,
    ) -> Result<EditorCapabilities, EditorSessionError> {
        if !size.width.is_finite()
            || !size.height.is_finite()
            || size.width < 0.0
            || size.height < 0.0
        {
            return Err(EditorSessionError::InvalidValue);
        }
        let request = self.begin_recover(mode)?;
        match ops.open(self.id, lease, size, mode) {
            Ok(capabilities) => {
                self.complete_attach(request, Ok(()))?;
                Ok(capabilities)
            }
            Err(error) => {
                let _ = self.complete_attach(request, Err(error));
                Err(EditorSessionError::Ops(error))
            }
        }
    }

    /// Open with an explicit degrade fallback: tries `preferred`,
    /// and only when the adapter refuses it with
    /// [`EditorOpError::Unsupported`] retries **once** with
    /// `fallback` under the same request id.
    ///
    /// A fallback attach lands `Attached{fallback}` and emits
    /// [`EditorSessionEvent::Degraded`] — *not* `Attached` — and the
    /// result is [`OpenOutcome::Degraded`], so a degrade is never
    /// mistaken for a requested-mode (Native Dock) PASS. Any other
    /// adapter error fails without retry; a failed fallback lands
    /// `Failed` and its error is returned.
    ///
    /// `preferred == fallback` degenerates to a plain open (no retry
    /// is possible). Note the contract assumption shared with the
    /// refused-open discipline: an `Unsupported` open produced no
    /// adapter editor, so the retry cannot orphan one.
    ///
    /// # Errors
    /// [`EditorSessionError::InvalidState`] unless `Closed`;
    /// [`EditorSessionError::InvalidValue`] on a non-finite/negative
    /// size; [`EditorSessionError::Ops`] on adapter refusal.
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    pub fn open_or_degrade<O: EditorSessionOps>(
        &mut self,
        ops: &mut O,
        lease: &ParentSlotLease,
        size: EditorSize,
        preferred: AttachMode,
        fallback: AttachMode,
    ) -> Result<OpenOutcome, EditorSessionError> {
        if preferred == fallback {
            return self
                .open(ops, lease, size, preferred)
                .map(OpenOutcome::Attached);
        }
        if !size.width.is_finite()
            || !size.height.is_finite()
            || size.width < 0.0
            || size.height < 0.0
        {
            return Err(EditorSessionError::InvalidValue);
        }
        let request = self.begin_open(preferred)?;
        match ops.open(self.id, lease, size, preferred) {
            Ok(capabilities) => {
                self.complete_attach(request, Ok(()))?;
                Ok(OpenOutcome::Attached(capabilities))
            }
            Err(EditorOpError::Unsupported) => {
                // The preferred mode was refused — retry once under
                // the fallback, marked so the landing emits
                // `Degraded`, never `Attached`.
                self.pending_mode = fallback;
                self.pending_attach = PendingAttach::Degrade;
                match ops.open(self.id, lease, size, fallback) {
                    Ok(capabilities) => {
                        self.complete_attach(request, Ok(()))?;
                        Ok(OpenOutcome::Degraded {
                            mode: fallback,
                            capabilities,
                        })
                    }
                    Err(error) => {
                        let _ = self.complete_attach(request, Err(error));
                        Err(EditorSessionError::Ops(error))
                    }
                }
            }
            Err(error) => {
                let _ = self.complete_attach(request, Err(error));
                Err(EditorSessionError::Ops(error))
            }
        }
    }
}

/// Registry owning [`PluginEditorSession`]s and their ids.
///
/// Mints [`PluginEditorSessionId`]s from a `{slot,generation}` table
/// (indices recycle with a bumped generation on [`Self::end_session`]),
/// enforces at most one live (non-`Closed`) session per plugin instance,
/// and converges a destroyed instance's live sessions on
/// [`Self::notify_terminated`].
///
/// **RT contract:** non-RT only — UI-thread only.
#[derive(Debug, Default)]
pub struct PluginEditorSessions {
    /// Per-slot generation; index = session slot.
    generations: Vec<u32>,
    /// Live/recent sessions by id.
    sessions: HashMap<PluginEditorSessionId, PluginEditorSession>,
    /// instance → live session id.
    by_instance: HashMap<PluginHandle, PluginEditorSessionId>,
}

impl PluginEditorSessions {
    /// Creates an empty registry.
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Begins a session for `instance`, returning its id.
    ///
    /// Reuses a freed slot index (bumping its generation) when
    /// available. A prior `Closed` session on the same instance is
    /// retired automatically before minting.
    ///
    /// # Errors
    /// [`EditorSessionError::SessionBusy`] if the instance already has a
    /// live (non-`Closed`) session — one editor per instance.
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    pub fn begin_session(
        &mut self,
        instance: PluginHandle,
    ) -> Result<PluginEditorSessionId, EditorSessionError> {
        if let Some(&old) = self.by_instance.get(&instance) {
            let old_live = self.sessions.get(&old).is_some_and(|s| !s.concluded());
            if old_live {
                return Err(EditorSessionError::SessionBusy);
            }
            // The previous session concluded (graceful close or
            // termination) — retire its slot before minting a fresh
            // session for this instance. Propagating the `Closed`-only
            // guard keeps a future invariant break loud instead of
            // silently leaking the old session.
            self.end_session(old)?;
        }
        // Find a free slot index: one whose generation entry exists but
        // has no session occupying it.
        let mut index = None;
        for i in 0..self.generations.len() {
            let occupied = self
                .sessions
                .keys()
                .any(|id| u32::try_from(i).is_ok_and(|index| id.slot() == index));
            if !occupied {
                index = u32::try_from(i).ok();
                break;
            }
        }
        let index = if let Some(i) = index {
            i
        } else {
            self.generations.push(0);
            u32::try_from(self.generations.len() - 1)
                .map_err(|_| EditorSessionError::InvalidValue)?
        };
        let id = PluginEditorSessionId::new(index, self.generations[index as usize]);
        let session = PluginEditorSession::new(id, instance);
        self.sessions.insert(id, session);
        self.by_instance.insert(instance, id);
        Ok(id)
    }

    /// The session for `id`, if live/present.
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    #[must_use]
    pub fn session(&self, id: PluginEditorSessionId) -> Option<&PluginEditorSession> {
        self.sessions.get(&id)
    }

    /// Mutable access to the session for `id`.
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    pub fn session_mut(&mut self, id: PluginEditorSessionId) -> Option<&mut PluginEditorSession> {
        self.sessions.get_mut(&id)
    }

    /// The live session id for `instance`, if any.
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    #[must_use]
    pub fn session_for_instance(&self, instance: PluginHandle) -> Option<PluginEditorSessionId> {
        self.by_instance.get(&instance).copied()
    }

    /// Ends `id`: removes the session and frees its slot index (bumping
    /// the generation so stale ids are rejected).
    ///
    /// # Errors
    /// [`EditorSessionError::UnknownSession`] if `id` names no session.
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    pub fn end_session(&mut self, id: PluginEditorSessionId) -> Result<(), EditorSessionError> {
        let session = self
            .sessions
            .get(&id)
            .ok_or(EditorSessionError::UnknownSession)?;
        // Only a `Closed` session may be retired — removing a live one
        // would orphan the plugin-side editor (no `CloseGui` is ever
        // dispatched here) and free the instance binding for a second
        // live session. Close or converge it first.
        if !matches!(session.state(), EditorSessionState::Closed) {
            return Err(EditorSessionError::InvalidState);
        }
        self.sessions.remove(&id);
        self.by_instance.retain(|_, v| *v != id);
        self.generations[id.slot() as usize] = self.generations[id.slot() as usize].wrapping_add(1);
        Ok(())
    }

    /// Converges `instance`'s live session on a terminal event —
    /// instance destroy, host crash, child exit or window loss. The
    /// session emits `Terminated` once and lands `Closed`; its instance
    /// binding is freed so a new session may begin. The session object
    /// stays queryable until [`Self::end_session`].
    ///
    /// DSP independence: this only touches the session ledger — the
    /// plugin instance's RT/processing lifetime is unaffected.
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    pub fn notify_terminated(&mut self, instance: PluginHandle, cause: SessionFailCause) {
        if let Some(&id) = self.by_instance.get(&instance) {
            if let Some(session) = self.sessions.get_mut(&id) {
                session.notify_terminated(cause);
            }
            self.by_instance.remove(&instance);
        }
    }

    /// Evaluates every session's deadline policy once — sessions
    /// whose in-flight request passed its deadline converge to
    /// `Failed{Timeout}`. Returns the number of requests that timed
    /// out. Drivers call this from their periodic GUI timer.
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    pub fn tick_all(&mut self) -> usize {
        self.sessions
            .values_mut()
            .map(PluginEditorSession::tick)
            .filter(|timed_out| *timed_out)
            .count()
    }
}

/// L0 editor-session ops: drives the existing [`InProcessHost`]
/// `OpenGui`/`CloseGui` command path for one instance.
///
/// Every `OpenGui` carries a [`ParentLease`] minted by the
/// injected [`LeaseRedeemer`] — the only path from a session
/// [`ParentSlotLease`] to a native parent. There is no pass-through: a
/// lease payload is never reinterpreted as a raw token. A plugin whose
/// `open_gui` is the default `Unsupported` surfaces
/// [`EditorOpError::Unsupported`].
///
/// **RT contract:** non-RT only — UI-thread only.
pub struct InProcessEditorOps<'a> {
    host: &'a mut InProcessHost,
    instance: PluginHandle,
    redeemer: Box<LeaseRedeemer<'a>>,
}

/// Redeems an opaque [`ParentSlotLease`] into the [`ParentLease`] a
/// plugin's `open_gui` consumes.
///
/// This is the composition-side seam: the lease payload is the
/// issuer's private encoding, and only the issuer-side adapter (e.g.
/// a GUI driver's surface-slot implementation) can validate it and
/// map it to a real OS parent handle. The host's L0 command surface
/// speaks [`ParentLease`]; this callback is where the
/// redemption — including staleness checks — happens. A failed
/// redemption surfaces as the returned [`EditorOpError`] (typically
/// [`EditorOpError::LeaseRejected`]).
///
/// **RT contract:** non-RT only — UI-thread only.
pub type LeaseRedeemer<'a> = dyn FnMut(&ParentSlotLease) -> Result<ParentLease, EditorOpError> + 'a;

impl<'a> InProcessEditorOps<'a> {
    /// Wraps `host` to drive editor ops for `instance`, redeeming leases
    /// through `redeemer` — the adapter validates the lease against its
    /// own ledger before yielding the raw parent token, so a stale or
    /// forged lease never reaches the plugin.
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    #[must_use]
    pub fn with_redeemer(
        host: &'a mut InProcessHost,
        instance: PluginHandle,
        redeemer: Box<LeaseRedeemer<'a>>,
    ) -> Self {
        Self {
            host,
            instance,
            redeemer,
        }
    }
}

/// Maps a [`HostError`] to [`EditorOpError`].
fn map_host_error(error: HostError) -> EditorOpError {
    match error {
        HostError::Unsupported => EditorOpError::Unsupported,
        HostError::InvalidHandle | HostError::Failed => EditorOpError::HostLost,
        HostError::PluginFailed => EditorOpError::Plugin,
        HostError::Ipc => EditorOpError::Io,
        HostError::InvalidValue => EditorOpError::InvalidValue,
        HostError::EndpointUnavailable => EditorOpError::InvalidState,
        // `HostError` is `#[non_exhaustive]` — unmapped variants (spawn
        // failures, lifecycle/prepare, registry limits, ...) are not a
        // GUI concept and map conservatively.
        _ => EditorOpError::Internal,
    }
}

impl EditorSessionOps for InProcessEditorOps<'_> {
    fn capabilities(
        &self,
        _session: PluginEditorSessionId,
    ) -> Result<EditorCapabilities, EditorOpError> {
        // The L0 command surface has no GUI capability query, so nothing
        // is advertised before an open proves what the plugin supports.
        Ok(EditorCapabilities::default())
    }

    fn open(
        &mut self,
        _session: PluginEditorSessionId,
        lease: &ParentSlotLease,
        _size: EditorSize,
        mode: AttachMode,
    ) -> Result<EditorCapabilities, EditorOpError> {
        // The L0 command surface carries a plain (docked) open only —
        // a non-docked mode would silently land as a docked attach, a
        // fake success. Refuse it explicitly.
        match mode {
            AttachMode::Docked => {}
            _ => return Err(EditorOpError::Unsupported),
        }
        let parent = (self.redeemer)(lease)?;
        match self.host.execute(HostCommand::OpenGui {
            handle: self.instance,
            parent,
        }) {
            // The L0 surface carries open/close only — an accepted docked
            // open proves dock and close; nothing else is claimed.
            Ok(HostReply::Opened { .. }) => Ok(EditorCapabilities {
                dock: Support::Supported,
                close: Support::Supported,
                ..EditorCapabilities::default()
            }),
            // Any other successful reply shape is an invariant violation
            // in the host mapping — surfaced as a plugin-level failure.
            Ok(_) => Err(EditorOpError::Plugin),
            Err(error) => Err(map_host_error(error)),
        }
    }

    fn close(&mut self, _session: PluginEditorSessionId) -> Result<(), EditorOpError> {
        match self
            .host
            .execute(HostCommand::CloseGui {
                handle: self.instance,
            })
            .map_err(map_host_error)?
        {
            HostReply::Ok => Ok(()),
            _ => Err(EditorOpError::Plugin),
        }
    }

    fn set_size(
        &mut self,
        _session: PluginEditorSessionId,
        _size: EditorSize,
    ) -> Result<(), EditorOpError> {
        // No `HostCommand` resize verb exists yet — honest refusal.
        Err(EditorOpError::Unsupported)
    }

    fn set_visibility(
        &mut self,
        _session: PluginEditorSessionId,
        _visible: bool,
    ) -> Result<(), EditorOpError> {
        Err(EditorOpError::Unsupported)
    }

    fn reattach(
        &mut self,
        _session: PluginEditorSessionId,
        _lease: &ParentSlotLease,
    ) -> Result<(), EditorOpError> {
        Err(EditorOpError::Unsupported)
    }
}

/// L1 editor-session ops: the defined `Unsupported` refusal on every op.
///
/// Mirrors the current `L1Instance::execute` short-circuit — L1 GUI is
/// genuinely unimplemented (no wire verbs reach the child, the OS
/// embedding mechanisms are unresolved), so every op returns
/// [`EditorOpError::Unsupported`]. It never silently degrades to L0.
///
/// **RT contract:** non-RT only — UI-thread only.
#[derive(Debug, Default)]
pub struct L1EditorOps;

impl L1EditorOps {
    /// Creates the refusal-only L1 ops adapter.
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl EditorSessionOps for L1EditorOps {
    fn capabilities(
        &self,
        _session: PluginEditorSessionId,
    ) -> Result<EditorCapabilities, EditorOpError> {
        Err(EditorOpError::Unsupported)
    }

    fn open(
        &mut self,
        _session: PluginEditorSessionId,
        _lease: &ParentSlotLease,
        _size: EditorSize,
        _mode: AttachMode,
    ) -> Result<EditorCapabilities, EditorOpError> {
        Err(EditorOpError::Unsupported)
    }

    fn close(&mut self, _session: PluginEditorSessionId) -> Result<(), EditorOpError> {
        Err(EditorOpError::Unsupported)
    }

    fn set_size(
        &mut self,
        _session: PluginEditorSessionId,
        _size: EditorSize,
    ) -> Result<(), EditorOpError> {
        Err(EditorOpError::Unsupported)
    }

    fn set_visibility(
        &mut self,
        _session: PluginEditorSessionId,
        _visible: bool,
    ) -> Result<(), EditorOpError> {
        Err(EditorOpError::Unsupported)
    }

    fn reattach(
        &mut self,
        _session: PluginEditorSessionId,
        _lease: &ParentSlotLease,
    ) -> Result<(), EditorOpError> {
        Err(EditorOpError::Unsupported)
    }
}
