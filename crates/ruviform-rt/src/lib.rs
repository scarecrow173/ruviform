//! `ruviform-rt` — real-time contract definitions and RT-safe primitives
//! for the Ruviform media foundation.
//!
//! Owns the [`Processor`] / [`RtProcessor`] traits (the `prepare` /
//! `process` separated processing contract), the [`RtContext`]
//! capability token, the [`MasterClock`] shared clock, the crate-wide
//! [`Error`] / [`Result`] types, and the RT-safe primitives — the
//! [`channel`] SPSC ring, the [`retire_channel`] deferred-drop
//! retire channel, and the [`slot`] immutable snapshot exchange.
//! Holds no scheduling or I/O.
//!
//! ## Allowed dependency layers
//!
//! None. External synchronization crates (e.g. `rtrb`) are
//! implementation details; their types must not be exposed in the public
//! API.
//!
//! ## RT contract
//!
//! This crate is the contract definition site. Every public API is
//! either RT-suitable (no-alloc, non-blocking, bounded) or explicitly
//! labelled non-RT-only via a `**RT contract:**` line on the item,
//! mirroring the RT ADR rule that the guaranteed scope is defined per
//! API.
//!
//! Contract source: `docs/designs/ruviform-architecture-design-baseline.md`
//! (crate table A.2, "RT／非 RT の分離方式", "RT 契約"),
//! `docs/adr/realtime-contract-and-verification-policy.md`, and
//! `docs/adr/rt-foundation-primitives-implementation.md`. The core
//! contract types, the SPSC channel, the retire channel, and the
//! snapshot exchange are implemented — the full `ruviform-rt` API
//! surface of Issue #21.

// The public names below are the fixed API shape of the contract crate:
// they deliberately echo their module (`clock::MasterClock`,
// `context::RtContext`, `error::Error`, `processor::Processor`) while
// being re-exported flat at the crate root, mirroring the media-core
// pattern. Every public enum carries enum-level `#[non_exhaustive]` per
// `docs/adr/public-enum-non-exhaustive-policy.md`.
#![allow(clippy::module_name_repetitions)]

#[cfg(test)]
mod alloc_counter;
mod clock;
mod context;
mod error;
mod processor;
mod retire;
mod snapshot;
mod spsc;

pub use clock::MasterClock;
pub use context::RtContext;
pub use error::{Error, Result};
pub use processor::{Processor, RtProcessor};
pub use retire::{DropCollector, Retirer, retire_channel};
pub use snapshot::{SnapshotGuard, SnapshotReader, SnapshotWriter, slot};
pub use spsc::{Consumer, Producer, TryPopError, TryPushError, channel};

/// Returns the crate version string.
///
/// **RT contract:** RT-suitable — pure, allocation-free.
///
/// # Examples
///
/// ```
/// assert_ne!(ruviform_rt::version(), "");
/// ```
#[must_use]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Compile-time bound check: `T` must be `Send`. (`expect_*` naming —
    /// helper names must not contain the `rt_` substring reserved for the
    /// RT-contract test lane.)
    fn expect_send<T: Send>() {}
    /// Compile-time bound check: `T` must be `Send + Sync`.
    fn expect_send_sync<T: Send + Sync>() {}

    /// Test `RtProcessor` that records that `process` ran.
    struct Probe;

    impl RtProcessor for Probe {
        type Io = bool;

        fn process(&mut self, io: &mut Self::Io, _ctx: &RtContext) {
            *io = true;
        }
    }

    /// Test `Processor` that bakes its `Setup` value into the produced
    /// `Rt` object.
    struct ProbeStage {
        prepared_with: Option<u32>,
    }

    struct PreparedProbe {
        gain: u32,
    }

    impl RtProcessor for PreparedProbe {
        type Io = u32;

        fn process(&mut self, io: &mut Self::Io, _ctx: &RtContext) {
            *io = self.gain;
        }
    }

    impl Processor for ProbeStage {
        type Setup = u32;
        type Rt = PreparedProbe;
        type Error = core::convert::Infallible;

        fn prepare(&mut self, setup: Self::Setup) -> core::result::Result<Self::Rt, Self::Error> {
            self.prepared_with = Some(setup);
            Ok(PreparedProbe { gain: setup })
        }
    }

    #[test]
    fn version_is_not_empty() {
        assert_ne!(version(), "");
    }

    /// RT contract: a minted `RtContext` is a usable `process` capability
    /// argument.
    #[test]
    fn rt_mint_token_usable() {
        let ctx = RtContext::mint();
        let mut probe = Probe;
        let mut io = false;
        probe.process(&mut io, &ctx);
        assert!(io);
    }

    /// RT contract: the `prepare` (non-RT) → `process` (RT) lifecycle is
    /// expressed by the two-trait split, and `process` requires
    /// `&RtContext`.
    #[test]
    fn rt_prepare_then_process() {
        let ctx = RtContext::mint();
        let mut stage = ProbeStage {
            prepared_with: None,
        };

        let mut probe = stage.prepare(7).unwrap();
        assert_eq!(stage.prepared_with, Some(7));

        let mut io = 0_u32;
        probe.process(&mut io, &ctx);
        assert_eq!(io, 7);
    }

    /// RT contract: the single writer's `publish_tick` is observable via
    /// `tick`, including across a thread boundary (`Send + Sync` shared
    /// clock, `Release`/`Acquire` pairing).
    #[test]
    fn rt_clock_tick_visibility() {
        let clock = std::sync::Arc::new(MasterClock::new(1, 48_000).unwrap());
        clock.publish_tick(42);

        let reader = std::sync::Arc::clone(&clock);
        let observed = std::thread::spawn(move || reader.tick()).join().unwrap();
        assert_eq!(observed, 42);

        clock.publish_tick(u64::MAX);
        assert_eq!(clock.tick(), u64::MAX);
    }

    /// RT contract: the timebase is construction-fixed — reads keep
    /// returning the build value regardless of tick updates — and zero
    /// components are rejected at construction.
    #[test]
    fn rt_clock_timebase_fixed() {
        let clock = MasterClock::new(1, 44_100).unwrap();
        assert_eq!(clock.timebase(), (1, 44_100));
        clock.publish_tick(9_999);
        assert_eq!(clock.timebase(), (1, 44_100));

        for (num, den) in [(0, 48_000), (1, 0), (0, 0)] {
            assert_eq!(
                MasterClock::new(num, den).unwrap_err(),
                Error::InvalidValue("timebase")
            );
        }
    }

    /// RT contract: thread-boundary bounds are API contract — the shared
    /// clock is `Send + Sync` (Arc-shared between writer and readers) and
    /// the capability token is `Send + Sync` so a driver can carry the
    /// proof to the contract thread.
    #[test]
    fn rt_endpoint_thread_bounds() {
        expect_send::<MasterClock>();
        expect_send_sync::<MasterClock>();
        expect_send::<RtContext>();
        expect_send_sync::<RtContext>();
    }
}
