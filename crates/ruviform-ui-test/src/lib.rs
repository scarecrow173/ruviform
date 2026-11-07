//! Test-only GUI Host lifecycle / control / fixture / clock /
//! observation foundation (Issue #312, RF-GT-03).
//!
//! This crate is a **test-layer consumer** of [`ruviform_ui`] — no
//! production crate depends on it, and production release artifacts
//! never link it. It provides:
//!
//! - `rvf.uitest.v1`: a versioned, closed-set control wire riding the
//!   `ruviform_ui::ipc` Intent/Event envelope ([`proto`]).
//! - [`agent::TestAgent`]: the in-app server — typed handshake
//!   (ready/failed), run/session ids, capabilities, backend/window
//!   metadata, typed shutdown/crash/cleanup. Fail-closed: hello
//!   first, run-secret on TCP, capability gating, runtime disable.
//! - [`io::TestIo`]: the port an app/host backend implements —
//!   observation is read-only, clock/fault ops are capability
//!   allowlists, nothing here bypasses product authorization.
//! - [`fake::FakeHost`]: a headless `WindowHandle`-backed backend so
//!   the full contract is CI-verifiable without a real OS window;
//!   [`headless::HeadlessApp`] composes `UiRuntime` + `FakeHost` into
//!   a ready dummy consumer.
//! - [`driver::TestDriver`]: the test-side client — spawns the app
//!   (stdio default / loopback TCP opt-in), performs the handshake,
//!   calls typed ops, and owns cleanup (no orphan process/window/port).
//!
//! Deliberately out of scope (RF-GT-04 / #313): OS-level
//! click/drag/focus injection and DOM manipulation — the honest
//! capability flags report `input_injection`/`dom_access` = false.
//!
//! **RT contract:** non-RT only.

#![warn(missing_docs)]

pub mod agent;
pub mod driver;
pub mod driver_boundary;
pub mod driver_lab;
pub mod fake;
pub mod fake_driver;
pub mod fixture;
pub mod headless;
pub mod io;
pub mod journal;
pub mod lifecycle;
pub mod proto;
pub mod transport;

pub use agent::{AgentConfig, TestAgent, serve, serve_from_env, serve_stdio, serve_tcp};
pub use driver::{DriverError, TestDriver};
pub use fake::{FakeHost, FakeWindow};
pub use fake_driver::{FakeDriver, MAX_EVENT_BACKLOG};
pub use fixture::{FixtureController, TestApp};
pub use headless::HeadlessIo;
pub use io::TestIo;
pub use journal::{DEFAULT_JOURNAL_CAP, Journal, JournalHandle, deliver_routed};
pub use proto::{
    AppMeta, BackendMeta, Capabilities, CleanupReport, ClockDomain, DeniedCapabilities, Direction,
    FaultKind, FixtureReceipt, FixtureSpec, Notice, PROTOCOL_VERSION, ReadyState, Receipt,
    ReceiptPage, Screenshot, ServeOutcome, ShutdownMode, Welcome, WindowMeta, ops, test_codes,
};
pub use transport::{ENV_ADDR, ENV_CONTROL, ENV_DISABLE, ENV_SECRET, gen_token};
