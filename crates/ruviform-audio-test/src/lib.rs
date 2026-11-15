//! Test-only deterministic Audio/MIDI test runtime (Issue #628).
//!
//! This crate provides the deterministic core of Ruviform's shared
//! test runtime: a [`VirtualClock`](clock::VirtualClock) with
//! deterministic drift/jitter/offset, deterministic PCM fixtures
//! ([`PcmFixture`](pcm::PcmFixture)/[`PcmScenario`](pcm::PcmScenario)),
//! a MIDI trace driver that exercises the production
//! [`TransportScheduler`](ruviform_media_core::TransportScheduler) /
//! [`SpanSink`](ruviform_media_core::SpanSink) contract, independent
//! oracles ([`oracle`](oracle)), Case Schema v1 run metadata
//! ([`RunMetadata`](metadata::RunMetadata)), and a scripted
//! port-trait backend ([`backend`](backend)) implementing the
//! production `DeviceEnumerator`/`Device`/`InputStream`/
//! `OutputStream` contracts with manual-pump deterministic delivery.
//!
//! # Test-only boundary
//!
//! **This crate is test-only and must never enter product
//! distribution.** It is a workspace member for local/CI verification
//! and downstream consumer test harnesses (AudioLens/Synvelis); no
//! production crate may depend on it — the dependency direction is
//! consumer-of-production-contracts only (`ruviform-audio-core`,
//! `ruviform-media-core`, `ruviform-rt`). `publish = false` is inherited
//! from the workspace.
//!
//! # Scope
//!
//! Deterministic core + scripted device-boundary backend. Production
//! pipeline wiring, consumer oracles, CI lane integration and real-OS
//! evidence are later slices of Issue #628. Virtual-backend success
//! here is OS-L1 deterministic evidence
//! (`environment_kind = deterministic-backend`), never a claim of real
//! OS API / GUI / plugin ABI success (POLICIES.md 2026-11-15
//! virtual-first terms; `scarecrow173/ruviform#627` contract).
//!
//! # RT contract
//!
//! Clock reads, PCM `fill`, the MIDI trace `SpanSink` path, and the
//! scripted backend's callback *firing* section (view construction,
//! timestamp build, callback invocation — measured by `rt_`-prefixed
//! `assert_no_alloc` tests collected by `mise run test-rt`) are
//! allocation-free, non-blocking and I/O-free. The scripted backend's
//! pump itself is a non-RT test-driver path (manual, caller-thread,
//! may allocate for capture). All other APIs are non-RT and may
//! allocate.

pub mod backend;
pub mod clock;
pub mod metadata;
pub mod midi;
pub mod oracle;
pub mod pcm;

pub use metadata::{
    CaseRequirement, CaseResult, EnvironmentKind, RunMetadata, RunParameters, VerificationLevel,
};
