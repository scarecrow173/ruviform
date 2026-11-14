//! Test-only deterministic Audio/MIDI test runtime core (Issue #628
//! first slice).
//!
//! This crate provides the deterministic core of Ruviform's shared
//! test runtime: a [`VirtualClock`](clock::VirtualClock) with
//! deterministic drift/jitter/offset, deterministic PCM fixtures
//! ([`PcmFixture`](pcm::PcmFixture)/[`PcmScenario`](pcm::PcmScenario)),
//! a MIDI trace driver that exercises the production
//! [`TransportScheduler`](ruviform_media_core::TransportScheduler) /
//! [`SpanSink`](ruviform_media_core::SpanSink) contract, independent
//! oracles ([`oracle`](oracle)) and Case Schema v1 run metadata
//! ([`RunMetadata`](metadata::RunMetadata)).
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
//! # Scope of this slice
//!
//! Deterministic core only. Scripted port-trait backends, fault
//! injection, production pipeline wiring, CI lane integration and
//! consumer oracles are later slices of Issue #628. Virtual-backend
//! success here is OS-L1 deterministic evidence
//! (`environment_kind = deterministic-backend`), never a claim of real
//! OS API / GUI / plugin ABI success (POLICIES.md 2026-11-15
//! virtual-first terms; `scarecrow173/ruviform#627` contract).
//!
//! # RT contract
//!
//! Clock reads, PCM `fill`, and the MIDI trace `SpanSink` path are
//! allocation-free, non-blocking and I/O-free; `rt_`-prefixed tests
//! prove this under `assert_no_alloc` (collected by `mise run
//! test-rt`). All other APIs are non-RT and may allocate.

pub mod clock;
pub mod metadata;
pub mod midi;
pub mod oracle;
pub mod pcm;
