//! Thread-priority promotion — OS-provided real-time scheduling
//! (Windows MMCSS "Pro Audio", macOS time-constraint policy, Linux
//! `SCHED_FIFO`) behind a `non-RT only` self-promotion API.
//!
//! `audio_thread_priority` is the wrapped implementation detail — no
//! `audio_thread_priority` type, error or feature name appears in the
//! public API.

use core::fmt;

/// Parameters of a priority-promotion request.
///
/// The wrapped OS service derives the scheduling budget from the audio
/// buffer's block length and sample rate.
///
/// **RT contract:** RT-suitable — a plain value type; constructing or
/// copying it never allocates.
///
/// # Examples
///
/// ```
/// let params = ruviform_exec::ThreadPriorityParams::audio(256, 48_000);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct ThreadPriorityParams {
    buffer_frames: u32,
    sample_rate: u32,
}

impl ThreadPriorityParams {
    /// Builds promotion parameters for an audio-path thread:
    /// `buffer_frames` frames per callback at `sample_rate` Hz.
    ///
    /// **RT contract:** RT-suitable — pure value construction (no
    /// allocation). Its *use* is restricted to the non-RT setup
    /// interval: the values are consumed by
    /// [`promote_current_thread_to_realtime`], which is `non-RT only`.
    ///
    /// # Examples
    ///
    /// ```
    /// let params = ruviform_exec::ThreadPriorityParams::audio(512, 44_100);
    /// ```
    #[must_use]
    pub fn audio(buffer_frames: u32, sample_rate: u32) -> Self {
        Self {
            buffer_frames,
            sample_rate,
        }
    }
}

/// Promotes the **calling thread** to real-time scheduling
/// (self-promotion). On success returns the [`ThreadPriorityGuard`]
/// demotion token.
///
/// **RT contract:** non-RT only — performs a blocking OS syscall.
/// The call model is self-promotion by the target thread itself:
///
/// - exec-managed threads: call at spawn-time setup.
/// - device-callback threads (future audio-device): call in the
///   preamble of the first callback — **before the RT contract
///   interval begins** (before `RtContext` is minted and before the
///   first `process` call).
///
/// Calling this from inside an active RT contract interval is a
/// contract violation.
///
/// Failure is reported as `Err(PriorityError)` and never panics —
/// promotion failure is not fatal and the caller may continue
/// unpromoted (best-effort). In this `default-features = false`
/// configuration, promotion on an unprivileged stock desktop Linux
/// (no rtkit / `CAP_SYS_NICE` / `RLIMIT_RTPRIO`) typically fails and
/// returns `Err` — that is the spec'd, documented failure behavior.
/// On platforms without a real-time backend the call may report
/// no-op success (following the wrapped implementation's semantics).
///
/// # Examples
///
/// ```
/// let params = ruviform_exec::ThreadPriorityParams::audio(256, 48_000);
/// match ruviform_exec::promote_current_thread_to_realtime(params) {
///     Ok(guard) => {
///         // This thread now runs at real-time priority; demotion runs
///         // on this same thread when leaving the real-time path.
///         let _demoted = ruviform_exec::demote_current_thread_from_realtime(guard);
///     }
///     Err(_error) => {
///         // Expected on unprivileged desktop Linux — continue
///         // without promotion.
///     }
/// }
/// ```
pub fn promote_current_thread_to_realtime(
    params: ThreadPriorityParams,
) -> Result<ThreadPriorityGuard, PriorityError> {
    audio_thread_priority::promote_current_thread_to_real_time(
        params.buffer_frames,
        params.sample_rate,
    )
    .map(|handle| ThreadPriorityGuard {
        handle: Some(handle),
    })
    .map_err(|_error| PriorityError::PromotionFailed("os real-time promotion request failed"))
}

/// Reverts a previous promotion, consuming the [`ThreadPriorityGuard`]
/// so a double demote is prevented by the type.
///
/// Like [`promote_current_thread_to_realtime`], demotion targets the
/// calling thread — call it from the promoted thread itself.
///
/// **RT contract:** non-RT only — performs an OS syscall.
///
/// # Examples
///
/// ```
/// let params = ruviform_exec::ThreadPriorityParams::audio(256, 48_000);
/// if let Ok(guard) = ruviform_exec::promote_current_thread_to_realtime(params) {
///     // `demote` consumes the guard — a second demote cannot compile.
///     let _result = ruviform_exec::demote_current_thread_from_realtime(guard);
/// }
/// ```
pub fn demote_current_thread_from_realtime(
    mut guard: ThreadPriorityGuard,
) -> Result<(), PriorityError> {
    match guard.handle.take() {
        Some(handle) => audio_thread_priority::demote_current_thread_from_real_time(handle)
            .map_err(|_error| {
                PriorityError::DemotionFailed("os real-time demotion request failed")
            }),
        // Unreachable: `handle` is `Some` from construction until this
        // `take` — the consumed guard's `Drop` then runs as a no-op.
        None => Ok(()),
    }
}

/// Token holding a successful promotion (opaque — the internal OS
/// handle is private). Meant to stay on the promoted thread: demotion
/// targets the calling thread.
///
/// `Drop` attempts best-effort demotion and discards any failure —
/// call [`demote_current_thread_from_realtime`] when failure detection
/// is needed. Because `Drop` performs an OS syscall, a guard must not
/// be created or dropped inside an RT contract interval.
///
/// **RT contract:** non-RT only.
///
/// # Examples
///
/// ```
/// let params = ruviform_exec::ThreadPriorityParams::audio(256, 48_000);
/// if let Ok(guard) = ruviform_exec::promote_current_thread_to_realtime(params) {
///     // Dropping the guard attempts best-effort demotion; for
///     // detected demotion use `demote_current_thread_from_realtime`.
///     drop(guard);
/// }
/// ```
pub struct ThreadPriorityGuard {
    handle: Option<audio_thread_priority::RtPriorityHandle>,
}

impl fmt::Debug for ThreadPriorityGuard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ThreadPriorityGuard").finish()
    }
}

impl Drop for ThreadPriorityGuard {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            // Best-effort demotion: the failure is intentionally
            // discarded — detection requires
            // `demote_current_thread_from_realtime`.
            let _dropped = audio_thread_priority::demote_current_thread_from_real_time(handle);
        }
    }
}

/// Failure classification of the thread-priority mechanism
/// ([`promote_current_thread_to_realtime`] /
/// [`demote_current_thread_from_realtime`]). Module-scoped by design —
/// this crate has a single failure family, so no crate-wide `Error` /
/// `Result` exists.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy — new variants may be added without breaking downstream
/// `match` arms.
///
/// **RT contract:** RT-suitable — `Copy`, allocation-free.
///
/// # Examples
///
/// ```
/// use ruviform_exec::PriorityError;
///
/// let error = PriorityError::PromotionFailed("context");
/// assert_ne!(error.to_string(), "");
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PriorityError {
    /// The OS rejected or could not satisfy the promotion request
    /// (insufficient privileges, unsupported scheduling policy, …).
    /// The payload is a static context description.
    PromotionFailed(&'static str),
    /// Reverting to the previous priority failed. The payload is a
    /// static context description.
    DemotionFailed(&'static str),
}

impl fmt::Display for PriorityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PromotionFailed(context) => {
                write!(f, "thread-priority promotion failed: {context}")
            }
            Self::DemotionFailed(context) => {
                write!(f, "thread-priority demotion failed: {context}")
            }
        }
    }
}

impl core::error::Error for PriorityError {}

#[cfg(test)]
mod tests {
    use super::*;

    /// Promotion smoke test: a spawned thread calls `promote` and the
    /// contract is that a `Result` comes back without a panic. `Ok` /
    /// `Err` are both legal — `Err` is the spec'd outcome on
    /// unprivileged desktop Linux — so the outcome itself is not
    /// asserted. On success the guard is demoted on the same (promoted)
    /// thread, mirroring the self-promotion call model. Only the
    /// `Send`-safe `Result<(), PriorityError>` crosses the join.
    /// (`promotion_*` naming — no `rt_` substring, which is reserved
    /// for the RT-contract test lane.)
    #[test]
    fn promotion_returns_result() {
        let outcome = std::thread::spawn(|| {
            let params = ThreadPriorityParams::audio(256, 48_000);
            match promote_current_thread_to_realtime(params) {
                Ok(guard) => demote_current_thread_from_realtime(guard),
                Err(error) => Err(error),
            }
        })
        .join()
        .expect("promotion thread panicked");

        match outcome {
            // `Err` is the spec'd outcome on unprivileged desktop
            // Linux; `Ok` requires platform support + privileges. Both
            // are legal — the contract is that a `Result` comes back
            // without a panic, which reaching this point proves.
            Ok(()) | Err(_) => {}
        }
    }

    /// `Display` covers every error variant.
    #[test]
    fn display_covers_every_variant() {
        assert_ne!(PriorityError::PromotionFailed("context").to_string(), "");
        assert_ne!(PriorityError::DemotionFailed("context").to_string(), "");
    }
}
