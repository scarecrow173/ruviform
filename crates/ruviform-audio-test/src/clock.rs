//! Deterministic virtual clock for the test runtime (Issue #628).
//!
//! [`VirtualClock`] is the shared read side: an atomic sample-frame tick
//! (via [`ruviform_rt::MasterClock`]) plus a monotonic nanosecond
//! counter, both published by a single logical writer —
//! [`VirtualClockDriver`]. Tick computation is a pure function of the
//! virtual elapsed frames and an immutable [`VirtualClockConfig`]
//! (offset, linear drift in ppm, deterministic per-period jitter), so
//! identical `(seed-equivalent) config + elapsed` always yields
//! identical ticks on every platform — no libm, no `HashMap`, no
//! floating point in the clock path.
//!
//! **RT contract:** [`VirtualClock::tick`],
//! [`VirtualClock::monotonic_nanos`] and the `*_at` config evaluators
//! are wait-free atomic loads / pure integer math — allocation-free,
//! non-blocking, I/O-free. The publish and driver methods are non-RT
//! (single-writer) though the underlying stores are the same
//! wait-free atomics [`MasterClock`] uses.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use ruviform_media_core::{Error, Result};
use ruviform_rt::MasterClock;

/// Deterministic jitter specification.
///
/// Jitter is constant inside each `period_frames` window and derived
/// from `seed` — it models per-callback clock jitter without any
/// entropy source. `period_frames == 0` disables jitter.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct JitterSpec {
    /// Mix seed for the deterministic per-period jitter value.
    pub seed: u64,
    /// Frames per constant-jitter window; `0` disables jitter.
    pub period_frames: u64,
    /// Maximum absolute jitter deviation in ticks (`0` = no jitter).
    pub amplitude_ticks: u32,
}

impl JitterSpec {
    /// Jitter disabled (`period_frames == 0`, `amplitude_ticks == 0`).
    pub const DISABLED: Self = Self {
        seed: 0,
        period_frames: 0,
        amplitude_ticks: 0,
    };
}

/// Immutable deterministic clock configuration.
///
/// `tick_at(elapsed)` is the single authority mapping virtual elapsed
/// frames to the published sample-frame tick:
///
/// ```text
/// tick = clamp_u64( offset_ticks
///                  + elapsed_frames
///                  + trunc(elapsed_frames * drift_ppm / 1_000_000)
///                  + jitter_ticks(elapsed_frames) )
/// ```
///
/// where `jitter_ticks` is a deterministic `[-amplitude, +amplitude]`
/// value per `period_frames` window (see [`JitterSpec`]). All math is
/// `i128`/`u128` integer — no floating point, no libm.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct VirtualClockConfig {
    /// Sample rate in Hz; one tick is `1/sample_rate` seconds.
    pub sample_rate: u32,
    /// Clock epoch identifier (consumers may map it to
    /// `ClockEpochId::new(epoch)`).
    pub epoch: u64,
    /// Deterministic tick offset; negative values model pre-roll and
    /// are clamped at `0` in the final tick.
    pub offset_ticks: i64,
    /// Deterministic linear drift in parts-per-million; negative
    /// values model a slow clock. Truncated toward zero per tick.
    pub drift_ppm: i32,
    /// Deterministic jitter specification.
    pub jitter: JitterSpec,
}

impl VirtualClockConfig {
    /// The published sample-frame tick for `elapsed_frames` of virtual
    /// time — pure integer math, identical on every platform.
    ///
    /// **RT contract:** RT-suitable — pure, allocation-free.
    #[must_use]
    pub fn tick_at(&self, elapsed_frames: u64) -> u64 {
        let drift = (i128::from(elapsed_frames) * i128::from(self.drift_ppm)) / 1_000_000;
        let base = i128::from(self.offset_ticks)
            + i128::from(elapsed_frames)
            + drift
            + i128::from(self.jitter_ticks_at(elapsed_frames));
        let clamped = base.clamp(0, i128::from(u64::MAX));
        u64::try_from(clamped).unwrap_or(u64::MAX)
    }

    /// The monotonic nanosecond time for `elapsed_frames` —
    /// `elapsed * 1e9 / sample_rate` in `u128` integer math.
    ///
    /// **RT contract:** RT-suitable — pure, allocation-free.
    #[must_use]
    pub fn monotonic_nanos_at(&self, elapsed_frames: u64) -> u64 {
        let nanos = (u128::from(elapsed_frames) * 1_000_000_000u128) / u128::from(self.sample_rate);
        u64::try_from(nanos).unwrap_or(u64::MAX)
    }

    /// Deterministic jitter in `[-amplitude, +amplitude]` ticks for the
    /// period window containing `elapsed_frames`; `0` when disabled.
    fn jitter_ticks_at(&self, elapsed_frames: u64) -> i64 {
        let j = &self.jitter;
        if j.period_frames == 0 || j.amplitude_ticks == 0 {
            return 0;
        }
        let period = elapsed_frames / j.period_frames;
        let h = mix64(j.seed ^ period.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let span = u64::from(j.amplitude_ticks) * 2 + 1;
        i64::try_from(h % span).unwrap_or(0) - i64::from(j.amplitude_ticks)
    }
}

/// splitmix64-style integer mixer — pure `u64` arithmetic, identical
/// on every platform.
fn mix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

/// Shared deterministic clock — the read side handed to RT consumers
/// (e.g. inside `Arc<VirtualClock>`).
///
/// All reads are wait-free atomic loads; all publishes are wait-free
/// atomic stores on the same pattern as [`MasterClock`]. The single
/// logical writer is [`VirtualClockDriver`].
///
/// **RT contract:** RT-suitable — every method is a wait-free atomic or
/// fixed-value access; none allocate or block.
#[derive(Debug)]
pub struct VirtualClock {
    clock: MasterClock,
    monotonic_nanos: AtomicU64,
    config: VirtualClockConfig,
}

impl VirtualClock {
    /// Creates a clock at virtual elapsed `0`; nothing is published
    /// until the driver syncs (or [`VirtualClockDriver::new`] does).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] when `config.sample_rate == 0`.
    pub fn new(config: VirtualClockConfig) -> Result<Self> {
        if config.sample_rate == 0 {
            return Err(Error::InvalidValue("sample_rate"));
        }
        // `sample_rate != 0` was validated above, so this cannot fail.
        let clock =
            MasterClock::new(1, config.sample_rate).map_err(|_| Error::InvalidValue("timebase"))?;
        Ok(Self {
            clock,
            monotonic_nanos: AtomicU64::new(0),
            config,
        })
    }

    /// The last published sample-frame tick.
    ///
    /// **RT contract:** RT-suitable — wait-free atomic load.
    #[must_use]
    pub fn tick(&self) -> u64 {
        self.clock.tick()
    }

    /// The last published monotonic nanosecond time.
    ///
    /// **RT contract:** RT-suitable — wait-free atomic load.
    #[must_use]
    pub fn monotonic_nanos(&self) -> u64 {
        self.monotonic_nanos.load(Ordering::Acquire)
    }

    /// The `(numerator, denominator)` timebase — always `(1,
    /// sample_rate)`.
    ///
    /// **RT contract:** RT-suitable — fixed-value access.
    #[must_use]
    pub fn timebase(&self) -> (u32, u32) {
        self.clock.timebase()
    }

    /// The configured clock epoch.
    ///
    /// **RT contract:** RT-suitable — fixed-value access.
    #[must_use]
    pub fn epoch(&self) -> u64 {
        self.config.epoch
    }

    /// The immutable clock configuration.
    ///
    /// **RT contract:** RT-suitable — read-only borrow.
    #[must_use]
    pub fn config(&self) -> &VirtualClockConfig {
        &self.config
    }

    /// Publishes `tick` — single-writer store, same memory ordering as
    /// [`MasterClock::publish_tick`].
    ///
    /// **RT contract:** RT-suitable — wait-free atomic store, but only
    /// the single logical writer ([`VirtualClockDriver`]) may call it.
    pub fn publish_tick(&self, tick: u64) {
        self.clock.publish_tick(tick);
    }

    /// Publishes `nanos` — single-writer store paired with
    /// [`Self::publish_tick`].
    ///
    /// **RT contract:** RT-suitable — wait-free atomic store, but only
    /// the single logical writer may call it.
    pub fn publish_monotonic_nanos(&self, nanos: u64) {
        self.monotonic_nanos.store(nanos, Ordering::Release);
    }
}

/// Single-writer driver for a [`VirtualClock`]: owns the virtual
/// elapsed-frames state (pause/resume, seek/discontinuity) and
/// publishes the deterministic tick + monotonic time.
///
/// Non-RT — mutating methods may be called from the test driver only,
/// never from an RT callback.
///
/// # Examples
///
/// ```
/// use std::sync::Arc;
/// use ruviform_audio_test::clock::{VirtualClock, VirtualClockConfig, VirtualClockDriver, JitterSpec};
///
/// let clock = Arc::new(VirtualClock::new(VirtualClockConfig {
///     sample_rate: 48_000,
///     epoch: 1,
///     offset_ticks: 0,
///     drift_ppm: 0,
///     jitter: JitterSpec::DISABLED,
/// }).unwrap());
/// let mut driver = VirtualClockDriver::new(Arc::clone(&clock));
/// driver.advance_frames(48_000);
/// assert_eq!(clock.tick(), 48_000);
/// ```
#[derive(Debug)]
pub struct VirtualClockDriver {
    clock: Arc<VirtualClock>,
    elapsed_frames: u64,
    paused: bool,
}

impl VirtualClockDriver {
    /// Creates a driver bound to `clock` and publishes the initial
    /// state (`elapsed = 0`).
    #[must_use]
    pub fn new(clock: Arc<VirtualClock>) -> Self {
        let driver = Self {
            clock,
            elapsed_frames: 0,
            paused: false,
        };
        driver.sync();
        driver
    }

    /// The shared clock.
    #[must_use]
    pub fn clock(&self) -> &Arc<VirtualClock> {
        &self.clock
    }

    /// Virtual elapsed frames since driver creation / last seek.
    #[must_use]
    pub fn elapsed_frames(&self) -> u64 {
        self.elapsed_frames
    }

    /// Whether virtual time is currently frozen.
    #[must_use]
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// Freezes virtual time — [`Self::advance_frames`] becomes a no-op
    /// on elapsed until [`Self::resume`].
    pub fn pause(&mut self) {
        self.paused = true;
    }

    /// Resumes virtual time from the frozen elapsed position.
    pub fn resume(&mut self) {
        self.paused = false;
    }

    /// Advances virtual time by `frames` and republishes — a no-op on
    /// elapsed while paused (still republishes the unchanged state).
    pub fn advance_frames(&mut self, frames: u64) {
        if !self.paused {
            self.elapsed_frames = self.elapsed_frames.saturating_add(frames);
        }
        self.sync();
    }

    /// Sets virtual elapsed directly — models a deterministic time
    /// discontinuity (device restart / seek on the clock domain).
    pub fn seek_frames(&mut self, elapsed_frames: u64) {
        self.elapsed_frames = elapsed_frames;
        self.sync();
    }

    /// Republishes tick + monotonic time for the current elapsed.
    pub fn sync(&self) {
        let config = self.clock.config();
        self.clock.publish_tick(config.tick_at(self.elapsed_frames));
        self.clock
            .publish_monotonic_nanos(config.monotonic_nanos_at(self.elapsed_frames));
    }

    /// The target tick `frames` ahead of the current elapsed — a
    /// deterministic deadline/timeout marker. Consumers compare
    /// `clock.tick() >= deadline` (a paused clock never reaches it).
    #[must_use]
    pub fn deadline_in(&self, frames: u64) -> u64 {
        self.clock
            .config()
            .tick_at(self.elapsed_frames.saturating_add(frames))
    }
}
