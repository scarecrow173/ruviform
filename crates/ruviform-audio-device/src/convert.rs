//! Pure conversion/negotiation functions plus the interleaved output
//! write path — all free of backend I/O so every piece is testable with
//! synthesized inputs.
//!
//! Boundary-mapping rules (design "マッピング設計（adapter 内部責務）"):
//!
//! - Sample format: only [`SampleFormat::F32`] is supported in the
//!   `PoC`.
//!   `F64` maps to [`SampleFormat::F64`] so non-F32 space can be
//!   described, but `negotiate` accepts F32 requests only — integer
//!   conversion paths are deliberately unimplemented.
//! - Channels → layout: cpal reports a channel *count* with no speaker
//!   positions, so only the well-known counts map to a named
//!   [`ChannelLayout`]; unknown counts yield `None` (a truthful
//!   `ChannelLayout::Custom` mask cannot be built without position
//!   information).
//! - Interleave write: `n = data.len() / channels` frames are exposed
//!   as an [`AudioBufferViewMut`] over a pre-allocated planar scratch;
//!   any dst remainder (`data.len() % channels`) and any region the
//!   callback or copy leaves unwritten is zero-filled — no undefined
//!   data is ever delivered.

use ruviform_audio_core::device::{
    BufferSize, OutputCallbackInfo, OutputDataCallback, OutputStreamConfig, SupportedBufferSize,
    SupportedOutputFormat,
};
use ruviform_audio_core::{
    AudioBufferViewMut, AudioFormat, ChannelLayout, SampleFormat, SampleRate,
};

/// Maps a cpal sample format onto the port's [`SampleFormat`].
///
/// `F32 → F32`, `F64 → F64`, everything else (integers, DSD, future
/// variants — cpal [`cpal::SampleFormat`] is `#[non_exhaustive]`) →
/// `None`.
pub(crate) fn map_sample_format(format: cpal::SampleFormat) -> Option<SampleFormat> {
    match format {
        cpal::SampleFormat::F32 => Some(SampleFormat::F32),
        cpal::SampleFormat::F64 => Some(SampleFormat::F64),
        _ => None,
    }
}

/// Maps a raw channel count to a named [`ChannelLayout`].
///
/// cpal reports no speaker positions, so only the conventional counts
/// map: 1→Mono, 2→Stereo, 4→Quad, 6→5.1, 8→7.1. Any other count (3ch,
/// 12ch, …) yields `None` — the adapter does not guess a speaker mask.
pub(crate) fn layout_for_channels(channels: u32) -> Option<ChannelLayout> {
    match channels {
        1 => Some(ChannelLayout::Mono),
        2 => Some(ChannelLayout::Stereo),
        4 => Some(ChannelLayout::Quad),
        6 => Some(ChannelLayout::FivePointOne),
        8 => Some(ChannelLayout::SevenPointOne),
        _ => None,
    }
}

/// Synthesizes an [`AudioFormat`] from a raw rate + channel count —
/// always `F32` (the only encoding this adapter opens). `None` when the
/// count maps to no named layout or the rate is invalid.
pub(crate) fn f32_format(sample_rate: u32, channels: u32) -> Option<AudioFormat> {
    let layout = layout_for_channels(channels)?;
    let sample_rate = SampleRate::new(sample_rate).ok()?;
    Some(AudioFormat {
        sample_rate,
        layout,
        sample_format: SampleFormat::F32,
    })
}

/// Picks an openable F32 format from the supported space when the
/// backend default does not itself yield one (non-F32 encoding or
/// unmappable channel count). Prefers a range matching the backend
/// default's channel count, then one containing 48 kHz, then one
/// containing the backend default rate; the chosen rate clamps the
/// default rate into the picked range.
///
/// `supported` is expected to be F32-filtered already
/// ([`crate::device`]).
pub(crate) fn synthesize_default_format(
    supported: &[SupportedOutputFormat],
    default_channels: u32,
    default_rate: u32,
) -> Option<AudioFormat> {
    supported
        .iter()
        .filter(|range| layout_for_channels(range.channels).is_some())
        .max_by_key(|range| {
            (
                range.channels == default_channels,
                range.min_sample_rate <= 48_000 && 48_000 <= range.max_sample_rate,
                range.min_sample_rate <= default_rate && default_rate <= range.max_sample_rate,
            )
        })
        .and_then(|range| {
            let rate = default_rate.clamp(range.min_sample_rate, range.max_sample_rate);
            f32_format(rate, range.channels)
        })
}

/// Maps a cpal [`cpal::SupportedBufferSize`] onto the port type,
/// validating the reported range: a backend-reported `min > max` folds
/// into [`SupportedBufferSize::Unknown`] rather than emitting a
/// violating invariant.
pub(crate) fn map_supported_buffer_size(size: &cpal::SupportedBufferSize) -> SupportedBufferSize {
    match *size {
        cpal::SupportedBufferSize::Range { min, max } if min <= max => SupportedBufferSize::Range {
            min: usize::try_from(min).unwrap_or(usize::MAX),
            max: usize::try_from(max).unwrap_or(usize::MAX),
        },
        cpal::SupportedBufferSize::Range { .. } | cpal::SupportedBufferSize::Unknown => {
            SupportedBufferSize::Unknown
        }
    }
}

/// The result of [`negotiate`]: the concrete cpal stream parameters
/// plus the frame estimate the scratch slab is sized from.
#[derive(Debug, PartialEq)]
pub(crate) struct NegotiatedConfig {
    /// Concrete cpal stream parameters for `build_output_stream`.
    pub(crate) stream: cpal::StreamConfig,
    /// Effective maximum frames per callback used for scratch sizing:
    /// the requested `Fixed` value, or the matched supported range's
    /// `max`, or [`DEFAULT_FRAMES_FALLBACK`] when the backend reports no
    /// bound. `n > frames_capacity` callbacks are still absorbed by
    /// chunking — this is a sizing estimate, not a guarantee.
    pub(crate) effective_max_frames: usize,
}

/// Scratch-size fallback when the backend reports no buffer bound
/// (`SupportedBufferSize::Unknown` under `BufferSize::Default`) —
/// 8192 frames per the design's "フォールバック定数" guidance.
pub(crate) const DEFAULT_FRAMES_FALLBACK: usize = 8192;

/// Negotiates `desired` against the supported space.
///
/// Returns `Some` only when: `desired` requests `F32`; some supported
/// range covers the requested channel count and sample rate; and a
/// requested [`BufferSize::Fixed`] fits `u32` and the matched range's
/// reported bounds (a `Range` report — `Unknown` imposes no bound).
/// Anything else is `None` → the caller reports
/// [`ruviform_audio_core::device::DeviceError::UnsupportedConfig`].
pub(crate) fn negotiate(
    desired: &OutputStreamConfig,
    supported: &[SupportedOutputFormat],
) -> Option<NegotiatedConfig> {
    if desired.format.sample_format != SampleFormat::F32 {
        return None;
    }
    let channels = desired.format.layout.channels();
    let rate = desired.format.sample_rate.get();
    let range = supported.iter().find(|range| {
        range.sample_format == SampleFormat::F32
            && range.channels == channels
            && range.min_sample_rate <= rate
            && rate <= range.max_sample_rate
    })?;

    let (buffer_size, effective_max_frames) = match desired.buffer_size {
        BufferSize::Default => (
            cpal::BufferSize::Default,
            match range.buffer_size {
                SupportedBufferSize::Range { max, .. } => max,
                _ => DEFAULT_FRAMES_FALLBACK,
            },
        ),
        BufferSize::Fixed(frames) => {
            let frames_u32 = u32::try_from(frames).ok()?;
            if frames_u32 == 0 {
                return None;
            }
            if let SupportedBufferSize::Range { min, max } = range.buffer_size {
                if frames < min || frames > max {
                    return None;
                }
            }
            (cpal::BufferSize::Fixed(frames_u32), frames)
        }
        // `BufferSize` is `#[non_exhaustive]` — a future variant is
        // "unsupported" expressed as `None`, not a guess.
        _ => return None,
    };

    Some(NegotiatedConfig {
        stream: cpal::StreamConfig {
            channels: u16::try_from(channels).ok()?,
            sample_rate: rate,
            buffer_size,
        },
        effective_max_frames,
    })
}

/// Drives one output delivery: frames `n = data.len() / channels` from
/// the planar `scratch` into interleaved `data`, invoking
/// `data_callback` once per `frames_capacity`-bounded chunk.
///
/// Per-callback steps (design "コールバック配管"):
///
/// 1. The dst remainder `data.len() % channels` is zero-filled first —
///    partial frames are never exposed to the view.
/// 2. Each chunk zero-fills the exposed scratch region (the safe
///    default against unwritten data), builds a fresh
///    [`AudioBufferViewMut`] over it, and hands it to `data_callback`.
/// 3. After the callback returns (the view is consumed), a second view
///    over the same slab packs into the matching dst sub-slice via
///    `copy_to_interleaved`. Any failure (view construction or copy)
///    zero-fills that chunk's dst — no undefined data is delivered.
///
/// The view is re-created per chunk from a re-borrowed scratch: a view
/// moved into the callback is never reused.
///
/// **RT contract:** bounded, allocation-free — every loop iteration is
/// fixed-work; the scratch is pre-sized at open.
pub(crate) fn write_interleaved(
    scratch: &mut [f32],
    frames_capacity: usize,
    channels: usize,
    data: &mut [f32],
    info: OutputCallbackInfo,
    data_callback: &mut OutputDataCallback,
) {
    // `channels`/`frames_capacity` come from a validated config and the
    // scratch is sized `frames_capacity * channels` at open — but the RT
    // path must never panic on an inconsistent shape, so the chunk is
    // bounded by what the scratch can actually hold.
    if channels == 0 {
        data.fill(0.0);
        return;
    }
    let max_chunk = frames_capacity.min(scratch.len() / channels);
    if max_chunk == 0 {
        data.fill(0.0);
        return;
    }

    let n = data.len() / channels;
    // Zero-fill the dst remainder first: samples past `n * channels`
    // are not part of any whole frame.
    let covered = n * channels;
    data[covered..].fill(0.0);

    let mut frame = 0;
    while frame < n {
        let chunk = (n - frame).min(max_chunk);
        let len = chunk * channels;
        scratch[..len].fill(0.0);
        let dst = &mut data[frame * channels..(frame + chunk) * channels];
        match AudioBufferViewMut::try_new(&mut scratch[..len], channels, chunk) {
            Ok(view) => {
                data_callback(view, info);
                // `view` was consumed by the callback; re-borrow the
                // slab for packing.
                pack_chunk(&mut scratch[..len], channels, chunk, dst);
            }
            Err(_) => dst.fill(0.0),
        }
        frame += chunk;
    }
}

/// Packs `frames` planar frames out of `scratch` (channel-major,
/// `channels * frames` long) into interleaved `dst`.
///
/// The view is rebuilt over the slab (the callback's view was consumed
/// by value) and `copy_to_interleaved` performs the pack. Any failure —
/// view construction or the copy itself — zero-fills `dst` so no
/// undefined data is delivered (per the design's "未定義データを送出
/// しない" rule).
///
/// **RT contract:** bounded, allocation-free.
fn pack_chunk(scratch: &mut [f32], channels: usize, frames: usize, dst: &mut [f32]) {
    match AudioBufferViewMut::try_new(scratch, channels, frames) {
        Ok(view) => {
            if view.copy_to_interleaved(dst).is_err() {
                dst.fill(0.0);
            }
        }
        Err(_) => dst.fill(0.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruviform_audio_core::device::{DEVICE_CLOCK_TIMEBASE, OutputTimestamp};
    use ruviform_media_core::MediaTime;

    fn f32_range(
        channels: u32,
        min: u32,
        max: u32,
        buffer: SupportedBufferSize,
    ) -> SupportedOutputFormat {
        SupportedOutputFormat {
            channels,
            min_sample_rate: min,
            max_sample_rate: max,
            buffer_size: buffer,
            sample_format: SampleFormat::F32,
        }
    }

    fn desired(rate: u32, layout: ChannelLayout, buffer_size: BufferSize) -> OutputStreamConfig {
        OutputStreamConfig {
            format: AudioFormat {
                sample_rate: SampleRate::new(rate).unwrap(),
                layout,
                sample_format: SampleFormat::F32,
            },
            buffer_size,
        }
    }

    fn stereo_silence_callback() -> OutputDataCallback {
        Box::new(
            |mut out: AudioBufferViewMut<'_, f32>, _info: OutputCallbackInfo| {
                for channel in 0..out.channels() {
                    out.channel_mut(channel).unwrap().fill(0.25);
                }
            },
        )
    }

    fn test_info() -> OutputCallbackInfo {
        OutputCallbackInfo {
            timestamp: OutputTimestamp {
                callback: MediaTime::new(0, DEVICE_CLOCK_TIMEBASE),
                playback: None,
            },
        }
    }

    // -- map_sample_format -------------------------------------------------

    #[test]
    fn map_sample_format_maps_floats_only() {
        assert_eq!(
            map_sample_format(cpal::SampleFormat::F32),
            Some(SampleFormat::F32),
        );
        assert_eq!(
            map_sample_format(cpal::SampleFormat::F64),
            Some(SampleFormat::F64),
        );
    }

    #[test]
    fn map_sample_format_rejects_integers_and_dsd() {
        for format in [
            cpal::SampleFormat::I8,
            cpal::SampleFormat::I16,
            cpal::SampleFormat::I24,
            cpal::SampleFormat::I32,
            cpal::SampleFormat::I64,
            cpal::SampleFormat::U8,
            cpal::SampleFormat::U16,
            cpal::SampleFormat::U24,
            cpal::SampleFormat::U32,
            cpal::SampleFormat::U64,
            cpal::SampleFormat::DsdU8,
            cpal::SampleFormat::DsdU16,
            cpal::SampleFormat::DsdU32,
        ] {
            assert_eq!(map_sample_format(format), None, "{format:?}");
        }
    }

    // -- layout_for_channels ------------------------------------------------

    #[test]
    fn layout_for_channels_maps_known_counts() {
        let cases = [
            (1, ChannelLayout::Mono),
            (2, ChannelLayout::Stereo),
            (4, ChannelLayout::Quad),
            (6, ChannelLayout::FivePointOne),
            (8, ChannelLayout::SevenPointOne),
        ];
        for (count, layout) in cases {
            assert_eq!(layout_for_channels(count), Some(layout));
        }
    }

    #[test]
    fn layout_for_channels_rejects_unknown_counts() {
        for count in [0, 3, 5, 7, 9, 12, 64] {
            assert_eq!(layout_for_channels(count), None, "{count}");
        }
    }

    // -- synthesize_default_format -------------------------------------------

    #[test]
    fn synthesize_default_prefers_same_channels_and_48k() {
        let supported = [
            f32_range(2, 44_100, 44_100, SupportedBufferSize::Unknown),
            f32_range(8, 44_100, 96_000, SupportedBufferSize::Unknown),
        ];
        // Same channels wins, rate clamps into the range.
        assert_eq!(
            synthesize_default_format(&supported, 2, 96_000),
            f32_format(44_100, 2),
        );
        // 48 kHz contained → exact rate.
        assert_eq!(
            synthesize_default_format(&supported, 8, 96_000),
            f32_format(96_000, 8),
        );
    }

    #[test]
    fn synthesize_default_returns_none_without_mappable_channels() {
        let supported = [SupportedOutputFormat {
            channels: 12,
            min_sample_rate: 44_100,
            max_sample_rate: 48_000,
            buffer_size: SupportedBufferSize::Unknown,
            sample_format: SampleFormat::F32,
        }];
        assert_eq!(synthesize_default_format(&supported, 12, 48_000), None);
        assert_eq!(synthesize_default_format(&[], 2, 48_000), None);
    }

    // -- map_supported_buffer_size -------------------------------------------

    #[test]
    fn map_supported_buffer_size_validates_range() {
        assert_eq!(
            map_supported_buffer_size(&cpal::SupportedBufferSize::Range { min: 32, max: 1024 }),
            SupportedBufferSize::Range { min: 32, max: 1024 },
        );
        // Violating backend report folds into `Unknown`.
        assert_eq!(
            map_supported_buffer_size(&cpal::SupportedBufferSize::Range { min: 1024, max: 32 }),
            SupportedBufferSize::Unknown,
        );
        assert_eq!(
            map_supported_buffer_size(&cpal::SupportedBufferSize::Unknown),
            SupportedBufferSize::Unknown,
        );
    }

    // -- negotiate -----------------------------------------------------------

    #[test]
    fn negotiate_matches_rate_channels_and_default_buffer() {
        let supported = [f32_range(
            2,
            44_100,
            48_000,
            SupportedBufferSize::Range { min: 32, max: 1024 },
        )];
        let config = negotiate(
            &desired(48_000, ChannelLayout::Stereo, BufferSize::Default),
            &supported,
        )
        .unwrap();
        assert_eq!(config.stream.channels, 2);
        assert_eq!(config.stream.sample_rate, 48_000);
        assert_eq!(config.stream.buffer_size, cpal::BufferSize::Default);
        assert_eq!(config.effective_max_frames, 1024);
    }

    #[test]
    fn negotiate_default_falls_back_when_no_bound_reported() {
        let supported = [f32_range(2, 44_100, 48_000, SupportedBufferSize::Unknown)];
        let config = negotiate(
            &desired(48_000, ChannelLayout::Stereo, BufferSize::Default),
            &supported,
        )
        .unwrap();
        assert_eq!(config.effective_max_frames, DEFAULT_FRAMES_FALLBACK);
    }

    #[test]
    fn negotiate_fixed_must_fit_reported_range() {
        let supported = [f32_range(
            2,
            44_100,
            48_000,
            SupportedBufferSize::Range { min: 32, max: 1024 },
        )];
        assert!(
            negotiate(
                &desired(48_000, ChannelLayout::Stereo, BufferSize::Fixed(480)),
                &supported,
            )
            .is_some(),
        );
        for bad in [0, 16, 2048] {
            assert_eq!(
                negotiate(
                    &desired(48_000, ChannelLayout::Stereo, BufferSize::Fixed(bad)),
                    &supported,
                ),
                None,
                "Fixed({bad}) must be rejected",
            );
        }
        assert_eq!(
            negotiate(
                &desired(48_000, ChannelLayout::Stereo, BufferSize::Fixed(usize::MAX),),
                &supported,
            ),
            None,
            "u32 overflow must be rejected",
        );
    }

    #[test]
    fn negotiate_rejects_unsupported_requests() {
        let supported = [f32_range(2, 44_100, 48_000, SupportedBufferSize::Unknown)];
        // Non-F32 encoding.
        let mut f64 = desired(48_000, ChannelLayout::Stereo, BufferSize::Default);
        f64.format.sample_format = SampleFormat::F64;
        assert_eq!(negotiate(&f64, &supported), None);
        // Rate outside every range.
        assert_eq!(
            negotiate(
                &desired(96_000, ChannelLayout::Stereo, BufferSize::Default),
                &supported,
            ),
            None,
        );
        // Channel count outside every range.
        assert_eq!(
            negotiate(
                &desired(48_000, ChannelLayout::Quad, BufferSize::Default),
                &supported,
            ),
            None,
        );
    }

    // -- write_interleaved -----------------------------------------------------

    /// Builds the state the stream owns at open: a scratch sized
    /// `frames_capacity * channels`.
    fn run_write(
        frames_capacity: usize,
        channels: usize,
        data: &mut [f32],
        cb: &mut OutputDataCallback,
    ) {
        let mut scratch = vec![f32::NAN; frames_capacity * channels];
        write_interleaved(
            &mut scratch,
            frames_capacity,
            channels,
            data,
            test_info(),
            cb,
        );
    }

    #[test]
    fn write_interleaved_packs_planar_into_frame_major() {
        // Channel-major scratch → frame-major dst. The callback writes
        // a per-channel constant: ch0 = 0.25, ch1 = -0.5.
        let mut cb: OutputDataCallback = Box::new(
            |mut out: AudioBufferViewMut<'_, f32>, _info: OutputCallbackInfo| {
                for (value, channel) in [(0.25_f32, 0_usize), (-0.5, 1)] {
                    out.channel_mut(channel).unwrap().fill(value);
                }
            },
        );
        let mut data = vec![1.0_f32; 6]; // 3 frames × 2ch
        run_write(4, 2, &mut data, &mut cb);
        assert_eq!(data, [0.25, -0.5, 0.25, -0.5, 0.25, -0.5]);
    }

    #[test]
    fn write_interleaved_zero_fills_unwritten_and_tail() {
        // Callback writes nothing → everything must be silence, never
        // the NaN scratch or the stale dst.
        let mut cb: OutputDataCallback =
            Box::new(|_out: AudioBufferViewMut<'_, f32>, _info: OutputCallbackInfo| {});
        let mut data = vec![9.0_f32; 7]; // 3 frames + 1-sample tail
        run_write(4, 2, &mut data, &mut cb);
        assert_eq!(data, [0.0; 7]);
    }

    // Exact comparisons: the callback wrote exactly `0.25`/`4.0`/`2.0`.
    #[allow(clippy::float_cmp)]
    #[test]
    fn write_interleaved_chunks_when_frames_exceed_capacity() {
        // n = 10 frames, capacity 4 → 4 + 4 + 2 chunks; each chunk's
        // view is rebuilt from a re-borrowed scratch. The callback
        // stamps its observed `frames()` into channel 1 so chunk
        // boundaries stay observable in the packed output.
        let mut cb: OutputDataCallback = Box::new(
            |mut out: AudioBufferViewMut<'_, f32>, _info: OutputCallbackInfo| {
                let frames = f32::from(u8::try_from(out.frames()).unwrap_or(u8::MAX));
                out.channel_mut(0).unwrap().fill(0.25);
                out.channel_mut(1).unwrap().fill(frames);
            },
        );
        let mut data = vec![0.0_f32; 20]; // 10 frames × 2ch
        let mut scratch = vec![f32::NAN; 8]; // capacity 4 × 2ch
        write_interleaved(&mut scratch, 4, 2, &mut data, test_info(), &mut cb);
        // Per chunk, channel 1 carries the chunk's frame count:
        // chunks of 4, 4, then 2.
        let ch1: Vec<f32> = data.chunks_exact(2).map(|f| f[1]).collect();
        assert_eq!(
            ch1,
            [4.0; 8].into_iter().chain([2.0; 2]).collect::<Vec<_>>()
        );
        // Channel 0 packed the callback output for every frame.
        assert!(data.chunks_exact(2).all(|f| f[0] == 0.25));
    }

    // Exact comparisons: the callback wrote exactly `0.25`.
    #[allow(clippy::float_cmp)]
    #[test]
    fn write_interleaved_bounds_chunks_by_actual_scratch() {
        // A scratch shorter than `frames_capacity` must not panic —
        // the chunk bound falls back to what the slab can hold.
        let mut scratch = vec![f32::NAN; 4]; // only 2 frames × 2ch
        let mut cb = stereo_silence_callback();
        let mut data = vec![9.0_f32; 6];
        write_interleaved(&mut scratch, 8, 2, &mut data, test_info(), &mut cb);
        // Every delivered sample is a known value (0.25 callback
        // output or 0.0 fill) — never NaN scratch or stale 9.0.
        assert!(data.iter().all(|s| *s == 0.25 || *s == 0.0), "{data:?}");
    }

    #[test]
    fn pack_chunk_failure_zero_fills_dst() {
        // `copy_to_interleaved` fails when `dst.len() != channels *
        // frames` — unreachable through `write_interleaved` (which
        // slices dst exactly), exercised here directly.
        let mut slab = vec![0.5_f32; 6]; // 3 frames × 2ch
        let mut short_dst = vec![9.0_f32; 4]; // wrong length
        pack_chunk(&mut slab, 2, 3, &mut short_dst);
        assert_eq!(short_dst, [0.0; 4]);

        // A slab that cannot form the requested view also yields
        // silence, not stale dst.
        let mut short_slab = vec![f32::NAN; 4];
        let mut dst = vec![9.0_f32; 6];
        pack_chunk(&mut short_slab, 2, 3, &mut dst);
        assert_eq!(dst, [0.0; 6]);
    }
}
