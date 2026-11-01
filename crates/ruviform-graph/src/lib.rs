//! # ruviform-graph
//!
//! Typed multi-bus **Graph IR** and its **non-RT** validation layer
//! (Issue #184 / Migration M1).
//!
//! ## Layer and permitted dependencies
//!
//! Structural layer sitting between the domain foundations
//! (`ruviform-audio-core`, `ruviform-media-core`) and engine
//! consumers (`ruviform-audio` and future compilers). Its only
//! normal dependency is `ruviform-audio-core` (for
//! [`ruviform_audio_core::AudioFormat`]); it never depends on
//! `ruviform-rt`, engine, plugin, or adapter crates — consumers
//! depend on it, not the reverse.
//!
//! ## RT contract
//!
//! **Every API in this crate is non-RT.** Building ([`GraphBuilder`]),
//! validating ([`Validator`]), mapping ([`StableHandleMap`]), and
//! revisioned publication ([`GraphProject`]) allocate and must run
//! outside the real-time callback. No RT publication/swap mechanism
//! is provided here — that is the M2+ consumer's responsibility.
//!
//! ## Model summary
//!
//! - [`GraphBuilder`] issues stable [`NodeId`]/[`PortId`]/
//!   [`ConnectionId`] from ID-free [`NodeDef`]/[`PortDef`]/[`ConnDef`]
//!   and produces a private-field [`GraphSpec`].
//! - Port attributes are orthogonal: [`Direction`],
//!   [`PortRole`], [`PortFormat`] (whose `domain()` is the single
//!   source for [`SignalDomain`]), [`Multiplicity`],
//!   [`PortCapability`], [`Availability`], [`Activation`].
//! - [`Validator`] rejects invalid direction/domain/format, implicit
//!   layout conversion (only explicit [`ChannelMap`] is accepted),
//!   missing node/port/owner mismatch, capacity and required-
//!   connection violations, and **every** cycle — zero-delay SCCs as
//!   [`ValidationError::ZeroDelayCycle`], declared-delay cycles as
//!   [`ValidationError::DelayedCycleUnsupported`] — so a
//!   [`ValidatedGraph`] is always a DAG.
//! - [`GraphProject::prepare`] issues monotonic [`GraphRevision`]s,
//!   distinguishes requested (last attempt) from published (last
//!   success) state, and preserves the previous published snapshot
//!   on validation failure.

mod graph;
mod ids;
mod mapping;
mod port;
mod project;
mod validate;

pub use graph::{
    ConnDef, Connection, EdgeDelay, GraphBuilder, GraphSpec, NodeDef, NodeKind, NodeSpec, PortRef,
    PortSpec,
};
pub use ids::{
    ConnectionId, GraphRevision, NodeId, PortId, RuntimeConnectionIndex, RuntimeNodeIndex,
    RuntimePortIndex,
};
pub use mapping::StableHandleMap;
pub use port::{
    Activation, Availability, ChannelMap, Direction, Multiplicity, PortCapability, PortDef,
    PortFormat, PortRole, SignalDomain,
};
pub use project::{GraphProject, PrepareError, PublishedGraph};
pub use validate::{ValidatedGraph, ValidationError, ValidationErrors, Validator};
