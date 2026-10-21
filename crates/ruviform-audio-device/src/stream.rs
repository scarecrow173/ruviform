//! [`CpalOutputStream`] — wraps the backend stream privately and
//! implements the port's [`OutputStream`] — plus the callback plumbing
//! (`DataCallbackState`) that turns each backend data callback into a
//! planar view call into the engine.
//!
//! Callback plumbing contract (design "コールバック配管"):
//!
//! - The planar scratch (`Vec<f32>`) is allocated **once at open** —
//!   `frames_capacity * channels`, where `frames_capacity` is
//!   `min(effective_max, FRAMES_CAPACITY_HARD_CAP)`. The callback path
//!   never grows it.
//! - Each callback fires [`convert::write_interleaved`]: per-chunk
//!   zero-fill → user callback → `copy_to_interleaved` — all bounded
//!   and allocation-free.
//! - The data and error closures own their state outright — no
//!   `Arc`/`Mutex` shared between them (a lock on the RT thread would
//!   violate the RT contract).
//! - Timestamps come from `StreamInstant::as_nanos()` (primary path)
//!   with an epoch-delta fallback when a `u128` nanosecond value
//!   overflows `i64`.
//!
//! [`OutputStream`] methods are control-path (non-RT) operations.

use cpal::traits::StreamTrait;
use cpal::{OutputCallbackInfo as CpalOutputCallbackInfo, OutputStreamTimestamp, StreamInstant};
use ruviform_audio_core::AudioFormat;
use ruviform_audio_core::device::{
    BufferSize, DEVICE_CLOCK_TIMEBASE, DeviceError, DeviceResult, OutputCallbackInfo,
    OutputDataCallback, OutputStream, OutputTimestamp,
};
use ruviform_media_core::MediaTime;

use crate::convert::write_interleaved;
use crate::error::{ErrorCtx, map_error};

/// Hard cap on the scratch's per-pass frame capacity: a
/// backend-reported `SupportedBufferSize::Range` maximum is adopted
/// only up to this bound — an unbounded report (hundreds of MB to GB)
/// would abort the process mid-open. Overruns are absorbed by chunking.
pub(crate) const FRAMES_CAPACITY_HARD_CAP: usize = 16_384;

/// An opened output stream — behind `Box<dyn OutputStream>` on the
/// port surface. Dropping stops delivery (backend-defined).
pub(crate) struct CpalOutputStream {
    inner: cpal::Stream,
    format: AudioFormat,
    /// The stream-clock instant captured at open — the epoch anchor for
    /// the saturating timestamp fallback.
    epoch: StreamInstant,
}

impl CpalOutputStream {
    pub(crate) fn new(inner: cpal::Stream, format: AudioFormat, epoch: StreamInstant) -> Self {
        Self {
            inner,
            format,
            epoch,
        }
    }
}

impl OutputStream for CpalOutputStream {
    fn start(&self) -> DeviceResult<()> {
        self.inner
            .play()
            .map_err(|e| map_error(&e, ErrorCtx::Control))
    }

    fn stop(&self) -> DeviceResult<()> {
        self.inner
            .pause()
            .map_err(|e| map_error(&e, ErrorCtx::Control))
    }

    fn format(&self) -> AudioFormat {
        self.format
    }

    fn buffer_size(&self) -> DeviceResult<BufferSize> {
        self.inner
            .buffer_size()
            .map(|frames| BufferSize::Fixed(usize::try_from(frames).unwrap_or(usize::MAX)))
            .map_err(|e| map_error(&e, ErrorCtx::Control))
    }

    fn now(&self) -> DeviceResult<MediaTime> {
        instant_to_media_time(self.inner.now(), self.epoch)
            .ok_or(DeviceError::Backend("timestamp_overflow"))
    }
}

/// The state moved into — and solely owned by — the backend's data
/// callback closure. Nothing here allocates after construction.
pub(crate) struct DataCallbackState {
    data_callback: OutputDataCallback,
    /// Channel-major planar scratch, `frames_capacity * channels` —
    /// pre-allocated at open, never grown on the callback path.
    scratch: Vec<f32>,
    channels: usize,
    frames_capacity: usize,
    /// First observed callback instant — the epoch anchor for the
    /// saturating timestamp fallback, recorded lazily because the
    /// stream clock is only observable once callbacks (or `now()`)
    /// run.
    epoch: Option<StreamInstant>,
}

impl DataCallbackState {
    pub(crate) fn new(
        data_callback: OutputDataCallback,
        scratch: Vec<f32>,
        channels: usize,
        frames_capacity: usize,
    ) -> Self {
        Self {
            data_callback,
            scratch,
            channels,
            frames_capacity,
            epoch: None,
        }
    }

    /// One backend output data callback — RT-contract section.
    ///
    /// `data` is the backend's interleaved `f32` destination. The
    /// callback instant seeds the epoch anchor on first fire.
    pub(crate) fn handle(&mut self, data: &mut [f32], info: &CpalOutputCallbackInfo) {
        let timestamp = info.timestamp();
        let epoch = *self.epoch.get_or_insert(timestamp.callback);
        let info = OutputCallbackInfo {
            timestamp: map_output_timestamp(&timestamp, epoch),
        };
        write_interleaved(
            &mut self.scratch,
            self.frames_capacity,
            self.channels,
            data,
            info,
            &mut self.data_callback,
        );
    }
}

/// Converts one backend [`StreamInstant`] to a [`MediaTime`] at
/// [`DEVICE_CLOCK_TIMEBASE`] (nanoseconds).
///
/// Primary path: `instant.as_nanos()` (`u128`) into `i64`. Fallback
/// (design "飽和・写像不能時の予備経路"): a device clock whose raw
/// epoch sits too far back to fit `i64` nanoseconds still yields
/// meaningful *relative* instants — take `instant - epoch` instead.
/// `None` only when both paths overflow.
///
/// **RT contract:** RT-suitable — pure, allocation-free.
fn instant_to_media_time(instant: StreamInstant, epoch: StreamInstant) -> Option<MediaTime> {
    if let Ok(nanos) = i64::try_from(instant.as_nanos()) {
        return Some(MediaTime::new(nanos, DEVICE_CLOCK_TIMEBASE));
    }
    let delta = instant.saturating_duration_since(epoch).as_nanos();
    i64::try_from(delta)
        .ok()
        .map(|nanos| MediaTime::new(nanos, DEVICE_CLOCK_TIMEBASE))
}

/// Maps a backend output timestamp onto the port's
/// [`OutputTimestamp`].
///
/// `playback` carries the backend-reported (hardware-latency-included)
/// delivery instant — `None` only on conversion overflow, not
/// "unsupported": cpal always reports it.
///
/// **RT contract:** RT-suitable — pure, allocation-free.
fn map_output_timestamp(
    timestamp: &OutputStreamTimestamp,
    epoch: StreamInstant,
) -> OutputTimestamp {
    OutputTimestamp {
        // `callback` is non-optional on the port: the epoch-delta
        // fallback cannot overflow in practice (same clock, bounded
        // uptime); a doubly-saturated value collapses to the clock
        // origin rather than panicking on the RT path.
        callback: instant_to_media_time(timestamp.callback, epoch)
            .unwrap_or_else(|| MediaTime::new(0, DEVICE_CLOCK_TIMEBASE)),
        playback: instant_to_media_time(timestamp.playback, epoch),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(debug_assertions)]
    #[global_allocator]
    static ALLOC: assert_no_alloc::AllocDisabler = assert_no_alloc::AllocDisabler;

    fn instant(secs: u64, nanos: u32) -> StreamInstant {
        StreamInstant::new(secs, nanos)
    }

    #[test]
    fn instant_to_media_time_primary_path() {
        // as_nanos fits i64 — the instant maps verbatim.
        let media = instant_to_media_time(instant(5, 250), StreamInstant::ZERO).unwrap();
        assert_eq!(media.value(), 5 * 1_000_000_000 + 250);
        assert_eq!(media.base(), DEVICE_CLOCK_TIMEBASE);
    }

    #[test]
    fn instant_to_media_time_falls_back_to_epoch_delta() {
        // ~584 years of nanoseconds exceed i64 — force the fallback by
        // simulating a clock whose raw instant overflows but whose
        // delta from the stream epoch fits.
        let epoch = instant(u64::MAX - 1, 0);
        let later = instant(u64::MAX - 1, 1_000_000_000 - 1);
        let media = instant_to_media_time(later, epoch).unwrap();
        assert_eq!(media.value(), 999_999_999);
    }

    #[test]
    fn instant_to_media_time_none_when_both_overflow() {
        // The raw instant overflows i64 AND sits far past the epoch —
        // the delta overflows too. `None` is the honest "cannot
        // represent" (surfaced as `Err` by `OutputStream::now`).
        let overflowing = instant(u64::MAX, 999_999_999);
        assert_eq!(
            instant_to_media_time(overflowing, StreamInstant::ZERO),
            None,
        );
    }

    #[test]
    fn map_output_timestamp_carries_callback_and_playback() {
        let ts = OutputStreamTimestamp {
            callback: instant(10, 0),
            playback: instant(10, 5_000_000),
        };
        let mapped = map_output_timestamp(&ts, StreamInstant::ZERO);
        assert_eq!(mapped.callback.value(), 10_000_000_000);
        assert_eq!(mapped.playback.unwrap().value(), 10_005_000_000);
    }

    /// RT-lane check: the whole callback path — view construction +
    /// `copy_to_interleaved` pack + a representative user callback +
    /// timestamp mapping — must not allocate.
    ///
    /// `assert_no_alloc` arms a thread-local forbid counter around the
    /// closure; any (de)allocation inside aborts the test binary —
    /// which *is* the failure signal. Selected by `mise run test-rt`
    /// via the `rt_` name prefix.
    ///
    /// Scope note: only the adapter's own code is measurable — cpal/OS
    /// internals inside the real backend thread cannot be wrapped;
    /// observing those is the experiment log's `#[global_allocator]`
    /// counting approach in `play_sine`.
    // Exact comparison: the callback wrote exactly `0.5`.
    #[allow(clippy::float_cmp)]
    #[test]
    fn rt_output_callback_path_allocates_nothing() {
        // Setup (allocating, outside the assertion).
        let mut scratch = vec![0.0_f32; 8]; // capacity 4 × 2ch
        let mut data = vec![0.0_f32; 6]; // 3 frames × 2ch
        let mut cb: OutputDataCallback = Box::new(
            |mut out: ruviform_audio_core::AudioBufferViewMut<'_, f32>,
             _info: OutputCallbackInfo| {
                for channel in 0..out.channels() {
                    out.channel_mut(channel).unwrap().fill(0.5);
                }
            },
        );
        let ts = OutputStreamTimestamp {
            callback: instant(42, 0),
            playback: instant(42, 1_000_000),
        };

        assert_no_alloc::assert_no_alloc(|| {
            let mapped = map_output_timestamp(&ts, StreamInstant::ZERO);
            write_interleaved(
                &mut scratch,
                4,
                2,
                &mut data,
                OutputCallbackInfo { timestamp: mapped },
                &mut cb,
            );
        });
        assert!(data.iter().all(|s| *s == 0.5));
    }
}
