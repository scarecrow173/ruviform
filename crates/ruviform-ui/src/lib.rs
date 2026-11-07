//! `ruviform-ui` — WebView-independent GUI host foundation for
//! Ruviform (Issue #156).
//!
//! This crate owns the stable, backend-agnostic surface every Ruviform
//! GUI shares:
//!
//! - [`event`]: normalized host events ([`HostEvent`]) and the
//!   host-control port ([`HostHandle`]) — no backend types.
//! - [`ipc`]: the `rvf.ipc.v1` wire schema — [`Intent`]/[`Event`]
//!   envelopes, [`ErrorBody`], stable error codes, size caps.
//! - [`service`] + [`runtime`]: the Intent → [`Service`] → [`JobSpec`]
//!   execution model (progress / cancellation / errors) driven by
//!   [`UiRuntime`]. Jobs reuse `ruviform-exec`'s
//!   `CancellationToken`/`Progress`/`OfflineDriver` contract so the
//!   same non-UI computation is callable from GUI and CLI alike.
//! - [`assets`] + [`data`]: delivery ports — [`AssetSource`] for
//!   `rvf://app` frontend assets and [`DataRegistry`]/[`DataSource`]
//!   for bounded binary pulls (`rvf://data`).
//! - [`bridge`]: the `window.__rvf` JS contract and push helper.
//! - [`render`]: the DOM / `Canvas2D` / `WebGL2` / native-surface
//!   split — type-level port only, implementations live in adapters.
//! - [`window`]: the multi-window contract (Issue #181) —
//!   [`WindowId`], [`WindowEvent`], [`RoutedEvent`], [`CloseDecision`],
//!   [`WindowConfig`], [`MonitorInfo`] and the [`WindowHandle`] port,
//!   so one native event loop can drive many windows/webviews behind
//!   the same backend-free boundary.
//! - [`driver`]: the driver-neutral window/host contract
//!   (Issue #386) — [`WindowSpec`], [`WindowLifecycle`],
//!   [`DriverEvent`], [`WindowCapabilities`]/[`Capability`] and the
//!   [`WindowDriver`] port, the migration target that carries no
//!   JS/`url`/`devtools` assumptions.
//! - [`view`]: the driver-neutral Component/View spec contract and
//!   the [`view::ViewHost`] headless projection engine (Issue #387).
//! - [`behavior`]: the headless value/interaction/command-binding
//!   contract — value mapping, the `Begin → Preview* → Commit|Cancel`
//!   gesture machine, and per-control command binding state
//!   (Issue #387).
//! - [`detect`]: backend detection report types.
//! - [`demo`]: a reference [`Service`] proving the shared GUI/CLI
//!   boundary end to end.
//!
//! Backend implementations (windowing, `WebView` engine, scheme
//! handling, GPU surfaces) live in adapter crates — currently
//! `ruviform-ui-webview` (`wry` + `tao`). This crate must never depend
//! on `wry`/`tao`/`wgpu` or expose their types.
//!
//! ## Dependency layers
//!
//! Extension-layer crate per the architecture baseline (A.2):
//! depends only on Primitives (`ruviform-exec`, `ruviform-rt`) — never
//! on adapters, engines, or application code.
//!
//! ## RT constraints
//!
//! **Non-RT only.** Every type in this crate is UI-thread or
//! job-worker-thread only; nothing here may be called from an RT
//! contract section, and RT paths must never block on UI events, IPC
//! or data pulls.
//!
//! Contract sources: `docs/designs/issue-156-gui-host-foundation.md`,
//! `docs/discovery/issue-156-gui-host-foundation-inventory.md`,
//! `docs/adr/ui-architecture-webview-hybrid.md`.

// Public names deliberately echo their module (data::DataSource,
// event::HostEvent) — the `ruviform-exec` precedent.
#![allow(clippy::module_name_repetitions)]

pub mod assets;
pub mod behavior;
pub mod bridge;
pub mod data;
pub mod demo;
pub mod detect;
pub mod dock;
pub mod driver;
pub mod error;
pub mod event;
pub mod ipc;
pub mod render;
pub mod runtime;
pub mod service;
pub mod view;
pub mod window;

pub use assets::{Asset, AssetSource, MapAssetSource, mime_for, resolve_path};
pub use bridge::{BRIDGE_JS, dispatch_script};
pub use data::{
    DEFAULT_MAX_CHUNK, DataChunk, DataError, DataRegistry, DataRegistryLimits, DataRequest,
    DataSource, VecDataSource,
};
pub use demo::DemoService;
pub use detect::{BackendReport, BackendState};
pub use dock::{
    Axis, DockError, DockEvent, DockNode, DockPath, DockTarget, DockTree, DockWorkspace, DropZone,
    LAYOUT_SCHEMA_VERSION, LayoutSnapshot, MonitorHint, MoveReport, MoveToken, RestoredLayout,
    ViewInstanceId, ViewLocation, WindowCloseDisposition, WindowGeometry, WindowPlacement,
    WindowSnapshot,
};
pub use driver::{
    Capability, DriverEvent, LogicalPosition, LogicalSize, WindowCapabilities, WindowDriver,
    WindowLifecycle, WindowSpec,
};
pub use error::UiError;
pub use event::{HostEvent, HostHandle};
pub use ipc::{
    ErrorBody, Event, Intent, JobState, LifecycleState, MAX_INTENT_BYTES, SCHEMA_VERSION, codes,
    parse_intent, valid_op,
};
pub use render::{NativeSurfaceHost, RenderTier, SurfaceId, SurfaceRegion};
pub use runtime::UiRuntime;
pub use service::{CallOutcome, JobContext, JobSpec, OpInfo, Service, ServiceContext};
pub use window::{
    CloseDecision, MonitorInfo, RoutedEvent, WindowConfig, WindowEvent, WindowHandle, WindowId,
};

/// Returns the crate version string.
///
/// **RT contract:** non-RT only.
#[must_use]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
