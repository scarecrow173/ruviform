//! `rvf.uitest.v1` — the typed test-control contract (Issue #312 / RF-GT-03).
//!
//! Wire frames reuse the `rvf.ipc.v1` envelope (`Intent` driver→agent,
//! `Event` agent→driver) over length-prefixed JSON (see
//! [`crate::transport`]). This module owns the `test.*` op names, the
//! protocol's serde types, and the handshake/capability/shutdown
//! contract. The op set is closed: no arbitrary eval, shell, or file
//! access is representable on this channel.
//!
//! **RT contract:** non-RT only.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Protocol version of the `rvf.uitest.v1` control channel.
pub const PROTOCOL_VERSION: u32 = 1;

/// `test.*` op names accepted by [`crate::agent::TestAgent`].
pub mod ops {
    /// First op of a session — ready/failed handshake exchanging run
    /// and session ids, revisions, capabilities, and metadata.
    pub const HELLO: &str = "test.hello";
    /// Re-query the declared capability set and live metadata.
    pub const CAPABILITIES: &str = "test.capabilities";
    /// Prepare a named, app-registered fixture.
    pub const FIXTURE_PREPARE: &str = "test.fixture.prepare";
    /// Advance a declared [`ClockDomain`](crate::ClockDomain).
    pub const CLOCK_ADVANCE: &str = "test.clock.advance";
    /// Arm an allowlisted fault.
    pub const FAULT_SET: &str = "test.fault.set";
    /// Disarm an allowlisted fault.
    pub const FAULT_CLEAR: &str = "test.fault.clear";
    /// Host-level close request for a window (`WindowHandle::request_close`).
    pub const WINDOW_REQUEST_CLOSE: &str = "test.window.request_close";
    /// Read-only observation: window metadata snapshot.
    pub const OBSERVE_WINDOWS: &str = "test.observe.windows";
    /// Read-only observation: paged receipt journal.
    pub const OBSERVE_RECEIPTS: &str = "test.observe.receipts";
    /// Read-only observation: screenshot (capability-gated).
    pub const OBSERVE_SCREENSHOT: &str = "test.observe.screenshot";
    /// Drain asynchronous notices (window created/destroyed, closing).
    pub const NOTICES: &str = "test.notices";
    /// End the session: cleanup report then stream close.
    pub const SHUTDOWN: &str = "test.shutdown";
}

/// Error codes the test protocol adds to `ruviform_ui::ipc::codes`.
pub mod test_codes {
    /// The op arrived before `test.hello` (or with a bad secret).
    pub const UNAUTHENTICATED: &str = "unauthenticated";
    /// The agent is disabled (`RVF_TEST_DISABLE` / `set_enabled`).
    pub const DISABLED: &str = "disabled";
}

/// Application identity reported in [`Welcome::app`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppMeta {
    /// Application name (for example `dummy_app`, `synvelis`).
    pub name: String,
    /// Application version string.
    pub version: String,
    /// Source revision when the app knows one (git sha); `None` when
    /// unavailable — never fabricated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
}

/// Backend identity reported in [`Welcome::backend`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendMeta {
    /// Backend name (`headless-fake`, `wry+tao/webview2`, …).
    pub name: String,
    /// Backend version/detail when the adapter reports one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// Declared capability set carried in [`Welcome::capabilities`] and
/// enforced per op — an absent capability answers `unavailable`.
///
/// `input_injection` and `dom_access` appear on the wire as explicit
/// `false` via [`DeniedCapabilities`]: actual input injection and DOM
/// manipulation belong to the OS/Browser driver (RF-GT-04), never to
/// this port.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capabilities {
    /// Named fixture preparation is supported.
    pub fixture: bool,
    /// Clock domains the host can advance (for example `["host_tick"]`).
    pub clock_domains: Vec<String>,
    /// Allowlisted fault kinds (for example `["window_crash","ipc_disconnect"]`).
    pub faults: Vec<String>,
    /// Observation surfaces (for example `["windows","receipts","notices"]`).
    pub observation: Vec<String>,
    /// Real screenshot capture is implemented on this backend.
    pub screenshot: bool,
    /// Capabilities this port structurally refuses (wire-flattened).
    #[serde(flatten)]
    pub denied: DeniedCapabilities,
}

/// Capabilities permanently out of scope for this control port,
/// declared on the wire as explicit `false` so a driver can tell a
/// "permanently refused" port from an "absent backend feature".
/// Always constructed all-false via [`Default`]; never granted.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeniedCapabilities {
    /// Always `false` — input injection is RF-GT-04's port, not this one.
    pub input_injection: bool,
    /// Always `false` — DOM access is the browser driver's port.
    pub dom_access: bool,
}

/// One window's metadata snapshot (read-only observation).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowMeta {
    /// Host-issued [`ruviform_ui::WindowId`] value.
    pub id: u64,
    /// Window title at creation, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Initial URL, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Logical width at creation.
    pub width: u32,
    /// Logical height at creation.
    pub height: u32,
    /// Whether the window's view has reported `Ready`.
    pub ready: bool,
    /// Whether a `CloseRequested` is pending an answer.
    pub close_pending: bool,
    /// Whether the `ipc_disconnect` fault is armed for this window.
    pub ipc_down: bool,
}

/// `test.hello` reply payload — the `Welcome` half of the
/// ready/failed handshake.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Welcome {
    /// Protocol version ([`PROTOCOL_VERSION`]).
    pub v: u32,
    /// Agent-issued session id — changes on every run/restart.
    pub session_id: String,
    /// Driver-issued run id echoed back.
    pub run_id: String,
    /// Application identity.
    pub app: AppMeta,
    /// `ruviform-ui` crate version serving this app.
    pub ruviform_revision: String,
    /// Backend identity.
    pub backend: BackendMeta,
    /// Declared capability set (enforced per op).
    pub capabilities: Capabilities,
    /// Windows alive at handshake time.
    pub windows: Vec<WindowMeta>,
    /// Target OS (`std::env::consts::OS`).
    pub os: String,
    /// Target arch (`std::env::consts::ARCH`).
    pub arch: String,
}

/// Agent-side readiness, asked at `test.hello` time.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum ReadyState {
    /// The app finished initialization and may serve.
    Ready,
    /// The app failed to reach a servable state — the handshake
    /// answers the `Failed` half of the ready/failed contract.
    Failed {
        /// Why the app is not ready.
        reason: String,
    },
}

/// `test.shutdown` mode.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShutdownMode {
    /// Destroy windows, let the app run its cleanup hook, end serve.
    Graceful,
    /// End serve immediately without window teardown.
    Now,
}

/// Cleanup accounting returned by `test.shutdown` and carried in
/// [`ServeOutcome`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupReport {
    /// Windows open when shutdown was requested.
    pub windows_open: usize,
    /// Windows destroyed during cleanup.
    pub windows_closed: usize,
    /// `true` when the host loop was asked to exit.
    pub exited: bool,
    /// App-reported cleanup detail (job counts, service state —
    /// opaque to the control port).
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub app: Value,
}

/// Why [`crate::agent::serve`] returned.
#[non_exhaustive]
#[derive(Debug)]
pub enum ServeOutcome {
    /// `test.shutdown` ended the session cleanly.
    Shutdown {
        /// The cleanup report already sent to the driver.
        report: CleanupReport,
    },
    /// The driver's end of the channel closed — graceful cleanup ran.
    DriverGone {
        /// Cleanup report produced on disconnect.
        report: CleanupReport,
    },
    /// Serving was refused (`RVF_TEST_DISABLE` or `set_enabled(false)`).
    Disabled {
        /// Why serving was refused.
        reason: String,
    },
    /// The transport or agent failed before/while serving.
    Failed {
        /// What failed.
        reason: String,
    },
}

/// Clock domains the control port may advance.
///
/// Only `HostTick` exists: it advances the host's periodic job-drain
/// wakeup by emitting `WindowEvent::Tick`. It never accelerates OS
/// timers, the wall clock, GPU/vsync, or webview-internal timers —
/// and `capabilities.clock_domains` must never claim otherwise.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClockDomain {
    /// The host's `WindowEvent::Tick` job-drain wakeup.
    HostTick,
}

impl ClockDomain {
    /// Wire name used in `clock_domains` and `test.clock.advance`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::HostTick => "host_tick",
        }
    }

    /// Parses a wire name back into a domain.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "host_tick" => Some(Self::HostTick),
            _ => None,
        }
    }
}

/// Allowlisted fault kinds for `test.fault.set`/`test.fault.clear`.
///
/// `WindowCrash` simulates host-level window loss (`destroy_window` —
/// the `Destroyed` terminal contract is identical to backend loss).
/// `IpcDisconnect` suppresses inbound `HostEvent::Message` delivery
/// for a window at the host boundary — it models a dead
/// frontend→host channel and is recorded as a `suppressed` receipt.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FaultKind {
    /// Host-level window loss; emits `Destroyed`.
    WindowCrash,
    /// Inbound IPC suppression for a window until `fault.clear`.
    IpcDisconnect,
}

impl FaultKind {
    /// Wire name used in `faults` capabilities and `test.fault.set`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::WindowCrash => "window_crash",
            Self::IpcDisconnect => "ipc_disconnect",
        }
    }

    /// Parses a wire name back into a fault kind.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "window_crash" => Some(Self::WindowCrash),
            "ipc_disconnect" => Some(Self::IpcDisconnect),
            _ => None,
        }
    }
}

/// `test.fixture.prepare` payload — a named, app-registered fixture.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixtureSpec {
    /// Registered fixture name (`single`, `dual`, `keep`, …).
    pub name: String,
    /// Deterministic seed for fixture-controlled randomness.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
    /// Fixture-specific arguments.
    #[serde(default)]
    pub params: Value,
}

/// What a fixture reports back after `prepare`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixtureReceipt {
    /// The applied fixture name.
    pub name: String,
    /// Fixture-reported detail (created window ids, seeded sources —
    /// opaque to the control port).
    #[serde(default)]
    pub applied: Value,
}

/// Receipt direction: `in` = host-side event into the app,
/// `out` = app→host (`WindowHandle`) traffic.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    /// Host→app (`WindowEvent` delivered to `TestApp::on_event`).
    In,
    /// App→host (`WindowHandle` call observed by `JournalHandle`).
    Out,
}

/// One journal entry — a receipt for host-boundary traffic.
///
/// Receipts carry kind/window/direction/byte-size only: bulk payloads
/// (message bodies, eval scripts) are never copied into the journal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Receipt {
    /// Monotonic journal sequence.
    pub seq: u64,
    /// Traffic direction.
    pub direction: Direction,
    /// Owning window when the traffic is window-scoped.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window: Option<u64>,
    /// Entry kind: a `HostEvent`/`WindowEvent` variant name for `in`
    /// receipts (`ready`, `message`, `tick`, `destroyed`, …), a
    /// `WindowHandle` op for `out` (`eval`, `respond_close`, …), or
    /// `suppressed` for fault-dropped inbound messages.
    pub kind: String,
    /// Payload byte size (0 when the traffic carries no body).
    pub bytes: u32,
}

/// A paged journal read (`test.observe.receipts` reply).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptPage {
    /// Entries with `seq > since`, oldest first.
    pub entries: Vec<Receipt>,
    /// Cursor for the next read (`seq` of the last returned entry,
    /// or the journal high-water mark).
    pub next_since: u64,
    /// `true` when `since` pointed below the journal's retained floor
    /// — older receipts were dropped by the bounded ring.
    pub truncated: bool,
}

/// A captured screenshot (`test.observe.screenshot` reply).
///
/// Backends that cannot capture declare `screenshot:false` and the op
/// answers `unavailable` — a screenshot is never fabricated.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Screenshot {
    /// Image format tag (for example `png`, `rgba-raw`).
    pub format: String,
    /// Base64-encoded payload.
    pub data_base64: String,
    /// Captured width.
    pub width: u32,
    /// Captured height.
    pub height: u32,
}

/// An asynchronous agent→driver notice (`test.notices` / `Event::Notify`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notice {
    /// Monotonic notice sequence.
    pub seq: u64,
    /// Notice kind (`window.created`, `window.destroyed`,
    /// `window.close_requested`, `app.closing`, `ipc.suppressed`).
    pub kind: String,
    /// The window the notice is about, when window-scoped.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window: Option<u64>,
    /// Notice detail payload.
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub detail: Value,
}
