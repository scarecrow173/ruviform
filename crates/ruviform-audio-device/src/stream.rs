//! [`CpalOutputStream`]/[`CpalInputStream`] — wrap the backend streams
//! privately and implement the port's [`OutputStream`]/[`InputStream`]
//! — plus the callback plumbing (`DataCallbackState`/
//! `InputCallbackState`) that turns each backend data callback into a
//! planar view call into the engine.
//!
//! Callback plumbing contract (design "コールバック配管"):
//!
//! - The planar scratch (`Vec<f32>`) is allocated **once at open** —
//!   `frames_capacity * channels`, where `frames_capacity` is
//!   `min(effective_max, FRAMES_CAPACITY_HARD_CAP)`. The callback path
//!   never grows it.
//! - Output fires [`convert::write_interleaved`] (planar →
//!   interleaved); input fires [`convert::read_interleaved`]
//!   (interleaved → planar read view) — all bounded and
//!   allocation-free.
//! - The data and error closures own their state outright — no
//!   `Arc`/`Mutex` shared between them (a lock on the RT thread would
//!   violate the RT contract).
//! - Timestamps come from `StreamInstant::as_nanos()` (primary path)
//!   with an epoch-delta fallback when a `u128` nanosecond value
//!   overflows `i64`.
//! - `clock_domain` is the host profile resolved at enumeration
//!   (see [`crate::device::clock_profile`]): hosts whose cpal
//!   `StreamInstant` clock is a shared monotonic clock report
//!   [`ClockDomainId::MONOTONIC`]; hosts with per-device clocks
//!   (ALSA) or unverifiable ones report [`ClockDomainId::UNKNOWN`],
//!   which `same_clock_domain` correctly treats as *not* same-clock.
//!   `frame_position` is `None` on the cpal path — cpal exposes no
//!   frame position — while the Windows WASAPI-native input path
//!   ([`crate::wasapi_input`]) supplies the per-packet
//!   `IAudioCaptureClient::GetBuffer` device position. Either way it
//!   is a backend-reported value or honest `None`, never synthesized.
//!   `input_latency`/`output_latency` report `Ok(None)` on the cpal
//!   path — cpal exposes no portable latency query.
//!
//! [`OutputStream`] methods are control-path (non-RT) operations.

use cpal::traits::StreamTrait;
use cpal::{
    InputCallbackInfo as CpalInputCallbackInfo, InputStreamTimestamp,
    OutputCallbackInfo as CpalOutputCallbackInfo, OutputStreamTimestamp, StreamInstant,
};
use ruviform_audio_core::AudioFormat;
use ruviform_audio_core::device::{
    BufferSize, DEVICE_CLOCK_TIMEBASE, DeviceError, DeviceResult, InputCallbackInfo,
    InputDataCallback, InputStream, InputTimestamp, OutputCallbackInfo, OutputDataCallback,
    OutputStream, OutputTimestamp, StreamLatency,
};
use ruviform_media_core::{ClockDomainId, MediaTime, MediaTimestamp, TimestampProvenance};

use crate::convert::{read_interleaved, write_interleaved};
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
    /// The clock domain this stream's timestamps belong to — the host
    /// profile resolved at enumeration; `same_clock_domain` decides
    /// duplex eligibility from it.
    clock_domain: ClockDomainId,
}

impl CpalOutputStream {
    pub(crate) fn new(
        inner: cpal::Stream,
        format: AudioFormat,
        epoch: StreamInstant,
        clock_domain: ClockDomainId,
    ) -> Self {
        Self {
            inner,
            format,
            epoch,
            clock_domain,
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

    fn clock_domain(&self) -> ClockDomainId {
        self.clock_domain
    }
}

/// An opened input stream — behind `Box<dyn InputStream>` on the port
/// surface. Dropping stops delivery (backend-defined).
pub(crate) struct CpalInputStream {
    inner: cpal::Stream,
    format: AudioFormat,
    /// The stream-clock instant captured at open — the epoch anchor for
    /// the saturating timestamp fallback.
    epoch: StreamInstant,
    /// The clock domain this stream's timestamps belong to — the host
    /// profile resolved at enumeration; `same_clock_domain` decides
    /// duplex eligibility from it.
    clock_domain: ClockDomainId,
}

impl CpalInputStream {
    pub(crate) fn new(
        inner: cpal::Stream,
        format: AudioFormat,
        epoch: StreamInstant,
        clock_domain: ClockDomainId,
    ) -> Self {
        Self {
            inner,
            format,
            epoch,
            clock_domain,
        }
    }
}

impl InputStream for CpalInputStream {
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

    fn clock_domain(&self) -> ClockDomainId {
        self.clock_domain
    }

    /// cpal exposes no portable input-latency query — `Ok(None)` is the
    /// honest "unsupported" (never `Ok(0)`). The effective input
    /// latency is observable per callback as
    /// `timestamp.callback - timestamp.capture` when `capture` is
    /// `Some`.
    fn input_latency(&self) -> DeviceResult<Option<StreamLatency>> {
        Ok(None)
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
    /// Negotiated sample rate — needed to synthesize per-chunk instant
    /// offsets when a callback delivers more frames than the scratch
    /// capacity (`write_interleaved` chunk path).
    sample_rate: u32,
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
        sample_rate: u32,
    ) -> Self {
        Self {
            data_callback,
            scratch,
            channels,
            frames_capacity,
            sample_rate,
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
            self.sample_rate,
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
pub(crate) fn instant_to_media_time(
    instant: StreamInstant,
    epoch: StreamInstant,
) -> Option<MediaTime> {
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

/// The input-direction counterpart of [`DataCallbackState`] — moved
/// into and solely owned by the backend's input data callback closure.
/// Nothing here allocates after construction.
pub(crate) struct InputCallbackState {
    data_callback: InputDataCallback,
    /// Channel-major planar scratch, `frames_capacity * channels` —
    /// pre-allocated at open, never grown on the callback path.
    scratch: Vec<f32>,
    channels: usize,
    frames_capacity: usize,
    /// Negotiated sample rate — needed to synthesize per-chunk instant
    /// offsets when a callback delivers more frames than the scratch
    /// capacity (`read_interleaved` chunk path).
    sample_rate: u32,
    /// First observed callback instant — the epoch anchor for the
    /// saturating timestamp fallback, recorded lazily because the
    /// stream clock is only observable once callbacks (or `now()`)
    /// run.
    epoch: Option<StreamInstant>,
    /// The clock domain reported on every [`InputCallbackInfo`] — the
    /// host profile resolved at enumeration.
    clock_domain: ClockDomainId,
    /// The provenance label applied to backend capture timestamps —
    /// per host (WASAPI reports a carried `qpc_position` fact; other
    /// audited hosts derive capture as callback-minus-latency).
    capture_provenance: TimestampProvenance,
}

impl InputCallbackState {
    pub(crate) fn new(
        data_callback: InputDataCallback,
        scratch: Vec<f32>,
        channels: usize,
        frames_capacity: usize,
        sample_rate: u32,
        clock_domain: ClockDomainId,
        capture_provenance: TimestampProvenance,
    ) -> Self {
        Self {
            data_callback,
            scratch,
            channels,
            frames_capacity,
            sample_rate,
            epoch: None,
            clock_domain,
            capture_provenance,
        }
    }

    /// One backend input data callback — RT-contract section.
    ///
    /// `data` is the backend's interleaved `f32` capture. The callback
    /// instant seeds the epoch anchor on first fire. `frame_position`
    /// is the backend-reported capture-clock position of the packet's
    /// first frame — `None` on the cpal path (cpal exposes no frame
    /// position), `Some` on the WASAPI-native path (the per-packet
    /// `IAudioCaptureClient::GetBuffer` device position). It is never
    /// synthesized from callback counts, which would hide a real
    /// discontinuity.
    pub(crate) fn handle(
        &mut self,
        data: &[f32],
        info: &CpalInputCallbackInfo,
        frame_position: Option<u64>,
    ) {
        let timestamp = info.timestamp();
        let epoch = *self.epoch.get_or_insert(timestamp.callback);
        let info = InputCallbackInfo {
            timestamp: map_input_timestamp(&timestamp, epoch, self.capture_provenance),
            clock_domain: self.clock_domain,
            frame_position,
        };
        read_interleaved(
            &mut self.scratch,
            self.frames_capacity,
            self.channels,
            data,
            info,
            self.sample_rate,
            &mut self.data_callback,
        );
    }
}

/// Maps a backend input timestamp onto the port's [`InputTimestamp`].
///
/// `callback` is non-optional on the port: the epoch-delta fallback
/// cannot overflow in practice (same clock, bounded uptime); a
/// doubly-saturated value collapses to the clock origin rather than
/// panicking on the RT path. `capture` is `Some` only when the
/// backend instant converts — `None` is the honest "cannot represent",
/// never a substituted `callback` value (which would shift placement
/// by the buffering latency).
///
/// **RT contract:** RT-suitable — pure, allocation-free.
fn map_input_timestamp(
    timestamp: &InputStreamTimestamp,
    epoch: StreamInstant,
    capture_provenance: TimestampProvenance,
) -> InputTimestamp {
    InputTimestamp {
        callback: instant_to_media_time(timestamp.callback, epoch)
            .unwrap_or_else(|| MediaTime::new(0, DEVICE_CLOCK_TIMEBASE)),
        // A `StreamInstant::ZERO` capture is a backend "not reported"
        // sentinel in practice (no real capture predates the stream
        // clock origin) — surface it as `None` rather than a
        // clock-origin timestamp consumers could misplace.
        capture: if timestamp.capture == StreamInstant::ZERO {
            None
        } else {
            instant_to_media_time(timestamp.capture, epoch).map(|time| MediaTimestamp {
                time,
                provenance: capture_provenance,
            })
        },
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
                48_000,
                &mut cb,
            );
        });
        assert!(data.iter().all(|s| *s == 0.5));
    }

    fn input_instant(secs: u64, nanos: u32) -> StreamInstant {
        StreamInstant::new(secs, nanos)
    }

    fn cpal_input_info(callback: StreamInstant, capture: StreamInstant) -> CpalInputCallbackInfo {
        CpalInputCallbackInfo::new(InputStreamTimestamp { callback, capture })
    }

    #[test]
    fn map_input_timestamp_carries_callback_and_capture() {
        let ts = InputStreamTimestamp {
            callback: input_instant(10, 0),
            capture: input_instant(9, 500_000_000),
        };
        let mapped = map_input_timestamp(&ts, StreamInstant::ZERO, TimestampProvenance::Derived);
        assert_eq!(mapped.callback.value(), 10_000_000_000);
        let capture = mapped.capture.unwrap();
        assert_eq!(capture.time.value(), 9_500_000_000);
        assert_eq!(capture.time.base(), DEVICE_CLOCK_TIMEBASE);
        assert_eq!(capture.provenance, TimestampProvenance::Derived);
    }

    #[test]
    fn map_input_timestamp_zero_capture_is_unreported() {
        // `StreamInstant::ZERO` is the backend "not reported" sentinel —
        // surfaced as `None`, never a clock-origin timestamp.
        let ts = InputStreamTimestamp {
            callback: input_instant(10, 0),
            capture: StreamInstant::ZERO,
        };
        let mapped = map_input_timestamp(&ts, StreamInstant::ZERO, TimestampProvenance::Carried);
        assert_eq!(mapped.callback.value(), 10_000_000_000);
        assert!(mapped.capture.is_none());
    }

    #[test]
    fn map_input_timestamp_capture_overflow_is_none() {
        // An unrepresentable capture instant yields `None` — the
        // honest "cannot represent", never a substituted callback.
        let ts = InputStreamTimestamp {
            callback: input_instant(10, 0),
            capture: input_instant(u64::MAX, 999_999_999),
        };
        let mapped = map_input_timestamp(&ts, StreamInstant::ZERO, TimestampProvenance::Derived);
        assert_eq!(mapped.callback.value(), 10_000_000_000);
        assert!(mapped.capture.is_none());
    }

    /// RT-lane check: the whole input callback path — interleaved→
    /// planar unpack + view construction + a representative user
    /// callback + timestamp/domain mapping — must not allocate.
    ///
    /// `assert_no_alloc` arms a thread-local forbid counter around the
    /// closure; any (de)allocation inside aborts the test binary —
    /// which *is* the failure signal. Selected by `mise run test-rt`
    /// via the `rt_` name prefix.
    // Exact comparisons: the input carried exactly `1.0`/`2.0`.
    #[allow(clippy::float_cmp)]
    #[test]
    fn rt_input_callback_path_allocates_nothing() {
        // Setup (allocating, outside the assertion).
        let mut state = InputCallbackState::new(
            Box::new(
                |input: ruviform_audio_core::AudioBufferView<'_, f32>, info: InputCallbackInfo| {
                    assert_eq!(input.channels(), 2);
                    assert_eq!(info.clock_domain, ClockDomainId::MONOTONIC);
                    assert_eq!(info.frame_position, None);
                    assert!(info.timestamp.capture.is_some());
                    // Read-only access — values verified after.
                    let _ = input.channel(0).unwrap()[0];
                },
            ),
            vec![0.0_f32; 8], // capacity 4 × 2ch
            2,
            4,
            48_000,
            ClockDomainId::MONOTONIC,
            TimestampProvenance::Derived,
        );
        // 2 frames interleaved: (1.0, 2.0), (1.0, 2.0).
        let data = [1.0_f32, 2.0, 1.0, 2.0];
        let info = cpal_input_info(input_instant(42, 0), input_instant(41, 0));

        assert_no_alloc::assert_no_alloc(|| {
            state.handle(&data, &info, None);
        });
    }

    #[test]
    fn input_callback_state_delivers_planar_view_and_info() {
        // Non-RT companion to the rt_ test: verify the observable
        // callback payload end-to-end through `handle`.
        let collected = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let captured = std::sync::Arc::clone(&collected);
        let mut state = InputCallbackState::new(
            Box::new(
                move |input: ruviform_audio_core::AudioBufferView<'_, f32>,
                      info: InputCallbackInfo| {
                    captured.lock().unwrap().push((
                        input.data().to_vec(),
                        input.channels(),
                        input.frames(),
                        info,
                    ));
                },
            ),
            vec![0.0_f32; 8],
            2,
            4,
            48_000,
            ClockDomainId::UNKNOWN,
            TimestampProvenance::Derived,
        );
        // 3 frames interleaved + tail sample.
        let data = [1.0_f32, -1.0, 2.0, -2.0, 3.0, -3.0, 9.0];
        let info = cpal_input_info(input_instant(10, 0), input_instant(9, 0));
        state.handle(&data, &info, None);

        let collected = collected.lock().unwrap();
        assert_eq!(collected.len(), 1);
        let (slab, channels, frames, info) = &collected[0];
        assert_eq!(*channels, 2);
        assert_eq!(*frames, 3);
        assert_eq!(slab, &[1.0, 2.0, 3.0, -1.0, -2.0, -3.0]);
        // Epoch anchored at the first callback instant: callback=10s,
        // capture=9s map verbatim.
        assert_eq!(info.timestamp.callback.value(), 10_000_000_000);
        assert_eq!(info.timestamp.capture.unwrap().time.value(), 9_000_000_000);
        assert_eq!(info.clock_domain, ClockDomainId::UNKNOWN);
        assert_eq!(info.frame_position, None);
    }
}
