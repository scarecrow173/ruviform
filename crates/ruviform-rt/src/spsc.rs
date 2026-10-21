//! The SPSC channel — a bounded, non-blocking single-producer /
//! single-consumer queue for RT ↔ non-RT handoff.
//!
//! Both directions share the same primitive: RT → non-RT carries
//! meters, events and retired objects; non-RT → RT carries parameters
//! and control commands. `rtrb` is the wrapped implementation detail
//! — no `rtrb` type, error or feature name appears in the public API.

use core::fmt;

/// Builds an SPSC channel with the given fixed `capacity`.
///
/// The capacity is fixed at construction — the channel never grows
/// (a pre-allocated handoff structure). `capacity` is the maximum
/// number of elements the ring can hold.
///
/// `T: Send` is the API contract for thread-crossing payloads: each
/// endpoint is meant to be `Send`-ed to its own thread.
///
/// **RT contract:** non-RT only — construction allocates the ring.
///
/// # Examples
///
/// ```
/// let (mut producer, mut consumer) = ruviform_rt::channel::<u32>(4);
///
/// producer.try_push(1).unwrap();
/// producer.try_push(2).unwrap();
/// assert_eq!(consumer.try_pop(), Ok(1));
/// assert_eq!(consumer.try_pop(), Ok(2));
/// ```
#[must_use]
pub fn channel<T: Send>(capacity: usize) -> (Producer<T>, Consumer<T>) {
    debug_assert!(capacity > 0, "channel capacity must be non-zero");
    let (producer, consumer) = rtrb::RingBuffer::new(capacity);
    (Producer { inner: producer }, Consumer { inner: consumer })
}

/// Sending endpoint of an SPSC channel.
///
/// `Send` when `T: Send` — the endpoint is meant to cross to its own
/// (producing) thread. `!Sync`: single-threaded ownership, inherited
/// from the wrapped endpoint. Created together with its [`Consumer`]
/// counterpart by [`channel`].
///
/// **RT contract:** every method is RT-suitable; construction via
/// [`channel`] is non-RT only.
///
/// # Examples
///
/// ```
/// let (mut producer, _consumer) = ruviform_rt::channel::<u32>(1);
///
/// producer.try_push(10).unwrap();
/// assert!(producer.is_full());
/// ```
#[derive(Debug)]
pub struct Producer<T> {
    inner: rtrb::Producer<T>,
}

impl<T> Producer<T> {
    /// Pushes `value` into the channel without blocking.
    ///
    /// On a full channel the value is handed back as
    /// `Err(TryPushError::Full(value))` — the drop / coalesce / keep
    /// policy decision is the caller's (there is deliberately no
    /// ring-level overflow-policy enum; full-channel policy is
    /// expressed per API). [`Consumer::pop_latest`] covers the
    /// latest-wins case on the receiving side.
    ///
    /// **RT contract:** RT-suitable — wait-free, no allocation,
    /// bounded.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_rt::TryPushError;
    ///
    /// let (mut producer, _consumer) = ruviform_rt::channel::<u32>(1);
    ///
    /// producer.try_push(1).unwrap();
    /// assert_eq!(producer.try_push(2), Err(TryPushError::Full(2)));
    /// ```
    pub fn try_push(&mut self, value: T) -> Result<(), TryPushError<T>> {
        match self.inner.push(value) {
            Ok(()) => Ok(()),
            Err(rtrb::PushError::Full(value)) => Err(TryPushError::Full(value)),
        }
    }

    /// Returns `true` if the channel currently has no free slot.
    ///
    /// Diagnostic accessor: the value is an observation-time
    /// approximation (the consumer may concurrently free slots) —
    /// its accuracy is not part of the contract.
    ///
    /// **RT contract:** RT-suitable — non-blocking atomic read.
    ///
    /// # Examples
    ///
    /// ```
    /// let (mut producer, _consumer) = ruviform_rt::channel::<u32>(1);
    ///
    /// assert!(!producer.is_full());
    /// producer.try_push(1).unwrap();
    /// assert!(producer.is_full());
    /// ```
    #[must_use]
    pub fn is_full(&self) -> bool {
        self.inner.is_full()
    }

    /// Returns the capacity fixed at construction.
    ///
    /// The value itself does not change after construction, but this
    /// is a diagnostic accessor like [`is_full`](Self::is_full) —
    /// its reported value is not part of the contract.
    ///
    /// **RT contract:** RT-suitable — read of a fixed value.
    ///
    /// # Examples
    ///
    /// ```
    /// let (producer, _consumer) = ruviform_rt::channel::<u32>(8);
    ///
    /// assert_eq!(producer.capacity(), 8);
    /// ```
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.inner.buffer().capacity()
    }
}

/// Receiving endpoint of an SPSC channel.
///
/// `Send` when `T: Send` — the endpoint is meant to cross to its own
/// (consuming) thread. `!Sync`: single-threaded ownership, inherited
/// from the wrapped endpoint. Created together with its [`Producer`]
/// counterpart by [`channel`].
///
/// **RT contract:** every method is RT-suitable; construction via
/// [`channel`] is non-RT only.
///
/// # Examples
///
/// ```
/// let (mut producer, mut consumer) = ruviform_rt::channel::<u32>(2);
///
/// producer.try_push(42).unwrap();
/// assert_eq!(consumer.try_pop(), Ok(42));
/// ```
#[derive(Debug)]
pub struct Consumer<T> {
    inner: rtrb::Consumer<T>,
}

impl<T> Consumer<T> {
    /// Pops the oldest pending element without blocking.
    ///
    /// Returns `Err(TryPopError::Empty)` when the channel is empty.
    ///
    /// **RT contract:** RT-suitable — wait-free, no allocation,
    /// bounded.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_rt::TryPopError;
    ///
    /// let (mut producer, mut consumer) = ruviform_rt::channel::<u32>(2);
    ///
    /// assert_eq!(consumer.try_pop(), Err(TryPopError::Empty));
    /// producer.try_push(7).unwrap();
    /// assert_eq!(consumer.try_pop(), Ok(7));
    /// ```
    pub fn try_pop(&mut self) -> Result<T, TryPopError> {
        match self.inner.pop() {
            Ok(value) => Ok(value),
            Err(rtrb::PopError::Empty) => Err(TryPopError::Empty),
        }
    }

    /// Drains pending elements and returns only the newest one —
    /// latest-wins coalescing for parameter application.
    ///
    /// Returns `None` when the channel is empty. The drain loop pops
    /// at most `capacity` elements, so the work stays bounded even
    /// while a producer pushes concurrently. Elements replaced by a
    /// newer value are dropped on the calling thread — see the RT
    /// contract note.
    ///
    /// **RT contract:** RT-suitable *only when* `T`'s destructor is
    /// RT-safe — replaced elements are dropped inline. For payloads
    /// whose `Drop` may deallocate, lock, or run unbounded code,
    /// consume with [`try_pop`](Self::try_pop) and route the value to
    /// the retire channel instead.
    ///
    /// # Examples
    ///
    /// ```
    /// let (mut producer, mut consumer) = ruviform_rt::channel::<f32>(4);
    ///
    /// for gain in [0.1_f32, 0.5, 0.9] {
    ///     producer.try_push(gain).unwrap();
    /// }
    /// // Only the newest value survives — the rest are coalesced.
    /// assert_eq!(consumer.pop_latest(), Some(0.9));
    /// assert!(consumer.is_empty());
    /// ```
    #[must_use]
    pub fn pop_latest(&mut self) -> Option<T> {
        let mut latest = None;
        for _ in 0..self.capacity() {
            match self.inner.pop() {
                Ok(value) => latest = Some(value),
                Err(rtrb::PopError::Empty) => break,
            }
        }
        latest
    }

    /// Returns `true` if the channel currently holds no elements.
    ///
    /// Diagnostic accessor: the value is an observation-time
    /// approximation (the producer may concurrently push) — its
    /// accuracy is not part of the contract.
    ///
    /// **RT contract:** RT-suitable — non-blocking atomic read.
    ///
    /// # Examples
    ///
    /// ```
    /// let (mut producer, mut consumer) = ruviform_rt::channel::<u32>(1);
    ///
    /// assert!(consumer.is_empty());
    /// producer.try_push(1).unwrap();
    /// assert!(!consumer.is_empty());
    /// ```
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns the capacity fixed at construction.
    ///
    /// The value itself does not change after construction, but this
    /// is a diagnostic accessor like [`is_empty`](Self::is_empty) —
    /// its reported value is not part of the contract.
    ///
    /// **RT contract:** RT-suitable — read of a fixed value.
    ///
    /// # Examples
    ///
    /// ```
    /// let (_producer, consumer) = ruviform_rt::channel::<u32>(8);
    ///
    /// assert_eq!(consumer.capacity(), 8);
    /// ```
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.inner.buffer().capacity()
    }
}

/// Failure of [`Producer::try_push`] — hands the rejected value back
/// so the caller can apply its own full-channel policy.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy — new variants may be added without breaking downstream
/// `match` arms.
///
/// **RT contract:** RT-suitable — a plain value type; producing or
/// matching it never allocates.
///
/// # Examples
///
/// ```
/// use ruviform_rt::TryPushError;
///
/// let (mut producer, _consumer) = ruviform_rt::channel::<u32>(1);
/// producer.try_push(1).unwrap();
///
/// let error = producer.try_push(2).unwrap_err();
/// assert_eq!(error, TryPushError::Full(2));
/// assert_eq!(error.into_inner(), 2);
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TryPushError<T> {
    /// The channel was full; the rejected value is handed back.
    Full(T),
}

impl<T> TryPushError<T> {
    /// Extracts the rejected value handed back by
    /// [`Producer::try_push`].
    ///
    /// **RT contract:** RT-suitable — move only.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_rt::TryPushError;
    ///
    /// assert_eq!(TryPushError::Full(7_u32).into_inner(), 7);
    /// ```
    #[must_use]
    pub fn into_inner(self) -> T {
        match self {
            Self::Full(value) => value,
        }
    }
}

impl<T> fmt::Display for TryPushError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // The handed-back value is deliberately not shown.
            Self::Full(_) => f.write_str("channel is full"),
        }
    }
}

impl<T: fmt::Debug> core::error::Error for TryPushError<T> {}

/// Failure of [`Consumer::try_pop`].
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy — new variants may be added without breaking downstream
/// `match` arms.
///
/// **RT contract:** RT-suitable — `Copy` (for `Copy` payloads),
/// allocation-free.
///
/// # Examples
///
/// ```
/// use ruviform_rt::TryPopError;
///
/// let (_producer, mut consumer) = ruviform_rt::channel::<u32>(1);
/// assert_eq!(consumer.try_pop(), Err(TryPopError::Empty));
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TryPopError {
    /// The channel was empty.
    Empty,
}

impl fmt::Display for TryPopError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("channel is empty"),
        }
    }
}

impl core::error::Error for TryPopError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alloc_counter;

    /// Compile-time bound check: `T` must be `Send`. (`expect_*`
    /// naming — helper names must not contain the `rt_` substring
    /// reserved for the RT-contract test lane.)
    fn expect_send<T: Send>() {}

    /// RT contract: `try_push` → `try_pop` preserves FIFO order and
    /// the values themselves.
    #[test]
    fn rt_push_pop_fifo() {
        let (mut producer, mut consumer) = channel::<u32>(4);

        for value in 0..4 {
            producer.try_push(value).unwrap();
        }
        for expected in 0..4 {
            assert_eq!(consumer.try_pop(), Ok(expected));
        }
        assert_eq!(consumer.try_pop(), Err(TryPopError::Empty));
    }

    /// RT contract: a full channel hands the value back as
    /// `TryPushError::Full` — `into_inner` recovers it and the queued
    /// elements keep their FIFO order.
    #[test]
    fn rt_full_try_push_returns_value() {
        let (mut producer, mut consumer) = channel::<u32>(2);
        producer.try_push(1).unwrap();
        producer.try_push(2).unwrap();
        assert!(producer.is_full());

        let error = producer.try_push(3).unwrap_err();
        assert_eq!(error, TryPushError::Full(3));
        assert_eq!(error.into_inner(), 3);

        assert_eq!(consumer.try_pop(), Ok(1));
        assert_eq!(consumer.try_pop(), Ok(2));
        assert_eq!(consumer.try_pop(), Err(TryPopError::Empty));
    }

    /// RT contract: `pop_latest` drains pending elements and returns
    /// only the newest (latest-wins coalescing); `None` when empty.
    #[test]
    fn rt_pop_latest_returns_newest() {
        let (mut producer, mut consumer) = channel::<u32>(4);
        assert_eq!(consumer.pop_latest(), None);

        for value in 0..4 {
            producer.try_push(value).unwrap();
        }
        assert_eq!(consumer.pop_latest(), Some(3));
        // The older elements were drained, not just skipped over.
        assert!(consumer.is_empty());
        assert_eq!(consumer.try_pop(), Err(TryPopError::Empty));
    }

    /// RT contract: `try_push` / `try_pop` / `pop_latest` perform no
    /// allocation or deallocation — measured inside a counting-
    /// allocator window after warm-up.
    #[test]
    fn rt_try_push_pop_no_alloc() {
        let (mut producer, mut consumer) = channel::<u32>(4);

        // Warm the endpoints outside the measured window.
        producer.try_push(0).unwrap();
        assert_eq!(consumer.try_pop(), Ok(0));
        assert_eq!(consumer.pop_latest(), None);

        alloc_counter::enable();
        let pushed = [producer.try_push(1).is_ok(), producer.try_push(2).is_ok()];
        let popped = consumer.try_pop();
        let latest = consumer.pop_latest();
        // Refill to capacity and exercise the full-channel path.
        let refilled = [
            producer.try_push(3).is_ok(),
            producer.try_push(4).is_ok(),
            producer.try_push(5).is_ok(),
            producer.try_push(6).is_ok(),
        ];
        let overflow = producer.try_push(7).unwrap_err();
        alloc_counter::disable();

        assert_eq!(alloc_counter::count(), 0);
        assert!(pushed.iter().all(|ok| *ok));
        assert_eq!(popped, Ok(1));
        assert_eq!(latest, Some(2));
        assert!(refilled.iter().all(|ok| *ok));
        assert_eq!(overflow, TryPushError::Full(7));
        assert_eq!(overflow.into_inner(), 7);
    }

    /// Thread-boundary bounds are API contract: each endpoint is
    /// `Send` (moves to its own thread) when `T: Send`. The `!Sync`
    /// single-threaded ownership is inherited from the wrapped
    /// endpoint — stable Rust cannot negate a `Sync` bound in a test,
    /// so it is asserted by signature review.
    #[test]
    fn rt_endpoint_send_bounds() {
        expect_send::<Producer<u8>>();
        expect_send::<Consumer<u8>>();
    }

    /// Control for the `rt_*_no_alloc` measurements: the counting
    /// allocator must record allocations while enabled — otherwise a
    /// zero count would be vacuous.
    #[test]
    fn counting_alloc_records_when_enabled() {
        alloc_counter::enable();
        drop(String::from("probe"));
        alloc_counter::disable();
        assert!(alloc_counter::count() > 0);
    }

    /// `Display` covers every error variant (without showing the
    /// handed-back value); `into_inner` recovers it.
    #[test]
    fn display_covers_every_variant() {
        let full = TryPushError::Full(1_u32);
        assert_ne!(full.to_string(), "");
        assert_eq!(full.into_inner(), 1);
        assert_ne!(TryPopError::Empty.to_string(), "");
    }
}
