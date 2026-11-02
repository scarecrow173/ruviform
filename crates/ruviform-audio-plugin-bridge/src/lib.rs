//! `ruviform-audio-plugin-bridge` — the Issue #196 M4 adapter gluing a
//! hosted plugin instance (non-RT `PluginHost` control / RT
//! `PluginEndpoint` process leg) into the typed multi-bus audio graph.
//!
//! The bridge owns the three seams the layered crates deliberately do
//! not:
//!
//! - **bus → port** — a declared [`BusDescriptor`] table maps onto typed
//!   [`PortDef`]s ([`bus_to_port_def`]) and is cross-checked against the
//!   host's bus report when one is available ([`validate_buses`]).
//! - **logical span → physical call** — [`PluginNode`] embeds a
//!   `TransportScheduler<EventKind>` and turns every `deliverable` span
//!   into a legal `BlockContract` partition of `process` calls with
//!   process-start-relative `EventList`s; undeliverable spans become
//!   silence plus an auditable report.
//! - **lifecycle ownership** — non-RT verbs stay on `PluginHost`; the
//!   node owns the single RT leg; transport control flows over a
//!   bounded SPSC channel via [`PluginNodeControl`]; `Arc` plans are
//!   kept alive by [`PlanVault`] so no plan destructor ever runs on
//!   the RT side.
//!
//! Layer: consumer-side (`利用側層`) — nothing depends on this crate.
//! `ruviform-plugin-host → ruviform-audio` stays a dev-dependency
//! (TA-005).

#![forbid(unsafe_code)]

mod attach;
mod bus;
mod cap;
mod control;
mod dispatch;
mod factory;
mod input;
mod node;
mod status;
mod target;
mod vault;

pub use attach::{AttachConfig, AttachError, Attached, attach};
pub use bus::{BusError, bus_to_port_def, role_to_port_role, validate_buses};
pub use cap::{AdapterCapability, Backend, CapabilityLevel, capability_for};
pub use control::{NodeControl, NodeReport, PluginNodeControl, QueueFull};
pub use dispatch::{DispatchStage, DropReason, ParamDispatchLedger, ParamValueObservation};
pub use factory::PluginNodeFactory;
pub use input::{
    BlockInputStats, INPUT_STAGE_CAPACITY, InputConfig, InputStage, InputStageCounters, InputTx,
    PumpStats,
};
pub use node::{MAX_BUSES, MAX_CHANNELS, PluginNode, WindowedIn, WindowedOut, buses_from_specs};
pub use status::{NodeStatus, NodeStatusSnapshot};
pub use target::{BindingGeneration, ParamBinding, ParamTargetTable, TargetId, TargetResolution};
pub use vault::PlanVault;
