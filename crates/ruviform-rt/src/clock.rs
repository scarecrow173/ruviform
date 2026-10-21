//! The [`MasterClock`] shared clock — a single atomic tick counter with
//! a construction-fixed rational timebase.

use core::sync::atomic::{AtomicU64, Ordering};

use crate::error::{Error, Result};

/// Shared clock providing the master time reference for A/V
/// synchronization.
///
/// Share it as `Arc<MasterClock>` (`Send + Sync`). A single logical
/// writer — the active driver, RT or offline — updates the tick via
/// [`publish_tick`](Self::publish_tick); readers are wait-free. The
/// single-writer rule is a contract-level convention rather than a
/// type-level guarantee: concurrent stores cannot corrupt the atomic
/// value, but monotonicity of the clock relies on the single-writer
/// convention.
///
/// The timebase is fixed at construction, so no tearing/seqlock guard is
/// needed. One tick equals `numerator / denominator` seconds; converting
/// ticks to seconds or to other timebases is the responsibility of upper
/// layers (`ruviform-rt` is bottom-layer and does not depend on
/// `ruviform-media-core`'s time types).
///
/// **RT contract:** RT-suitable — every method is a wait-free atomic or
/// fixed-value access; none allocate or block.
///
/// # Examples
///
/// ```
/// use ruviform_rt::MasterClock;
///
/// let clock = MasterClock::new(1, 48_000).unwrap();
/// clock.publish_tick(48_000);
/// assert_eq!(clock.tick(), 48_000);
/// assert_eq!(clock.timebase(), (1, 48_000));
/// ```
#[derive(Debug)]
pub struct MasterClock {
    /// Monotonic tick counter — the only mutable state.
    tick: AtomicU64,
    /// Rational timebase `(numerator, denominator)`; immutable after
    /// construction.
    timebase: (u32, u32),
}

impl MasterClock {
    /// Creates a clock at tick `0` whose tick unit is
    /// `timebase_num / timebase_den` seconds.
    ///
    /// Returns `Err(Error::InvalidValue("timebase"))` when either
    /// component is `0`.
    ///
    /// **RT contract:** RT-suitable — pure value construction (no
    /// allocation). Reserving the sharing `Arc` is the caller's
    /// responsibility — typically a non-RT setup step.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_rt::{Error, MasterClock};
    ///
    /// assert!(MasterClock::new(1, 48_000).is_ok());
    /// assert_eq!(
    ///     MasterClock::new(0, 48_000).unwrap_err(),
    ///     Error::InvalidValue("timebase")
    /// );
    /// ```
    pub fn new(timebase_num: u32, timebase_den: u32) -> Result<Self> {
        if timebase_num == 0 || timebase_den == 0 {
            return Err(Error::InvalidValue("timebase"));
        }
        Ok(Self {
            tick: AtomicU64::new(0),
            timebase: (timebase_num, timebase_den),
        })
    }

    /// Publishes the current tick. Called only by the single logical
    /// writer (the active driver).
    ///
    /// **RT contract:** RT-suitable — wait-free atomic store
    /// (`Ordering::Release`, pairing with the `Ordering::Acquire` load in
    /// [`tick`](Self::tick) so data published alongside the tick stays
    /// visible to readers).
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_rt::MasterClock;
    ///
    /// let clock = MasterClock::new(1, 1_000).unwrap();
    /// clock.publish_tick(500);
    /// assert_eq!(clock.tick(), 500);
    /// ```
    pub fn publish_tick(&self, tick: u64) {
        self.tick.store(tick, Ordering::Release);
    }

    /// Reads the last published tick.
    ///
    /// **RT contract:** RT-suitable — wait-free atomic load
    /// (`Ordering::Acquire`, paired with the `Ordering::Release` store in
    /// [`publish_tick`](Self::publish_tick)).
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_rt::MasterClock;
    ///
    /// let clock = MasterClock::new(1, 1_000).unwrap();
    /// clock.publish_tick(42);
    /// assert_eq!(clock.tick(), 42);
    /// ```
    #[must_use]
    pub fn tick(&self) -> u64 {
        self.tick.load(Ordering::Acquire)
    }

    /// Returns the construction-fixed `(numerator, denominator)`
    /// timebase.
    ///
    /// **RT contract:** RT-suitable — load of a fixed value.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_rt::MasterClock;
    ///
    /// let clock = MasterClock::new(1, 44_100).unwrap();
    /// assert_eq!(clock.timebase(), (1, 44_100));
    /// ```
    #[must_use]
    pub fn timebase(&self) -> (u32, u32) {
        self.timebase
    }
}
