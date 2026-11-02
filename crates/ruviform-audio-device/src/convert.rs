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
    BufferSize, DEVICE_CLOCK_TIMEBASE, OutputCallbackInfo, OutputDataCallback, OutputStreamConfig,
    SupportedBufferSize, SupportedInputFormat, SupportedOutputFormat,
};
use ruviform_audio_core::{
    AudioBufferViewMut, AudioFormat, ChannelLayout, SampleFormat, SampleRate,
};
use ruviform_media_core::MediaDuration;

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

/// A reported range is a negotiation candidate only when its sample-rate
/// bounds are structurally valid (`min <= max`, nonzero ceiling).
/// Malformed backend reports are skipped, never trusted — `u32::clamp`
/// on inverted bounds would panic, and a zero ceiling admits no rate.
fn valid_sample_rate_bounds(range: &SupportedOutputFormat) -> bool {
    range.min_sample_rate <= range.max_sample_rate && range.max_sample_rate != 0
}

/// The same structural-validity rule for the buffer-size axis: a
/// port-level `Range` with `min > max` or a zero ceiling is a
/// contradictory bound, never a candidate. `Unknown` (and any future
/// variant) imposes no bound, hence nothing to violate — admission is
/// decided in the `negotiate` arms, not here.
fn valid_buffer_bounds(range: &SupportedOutputFormat) -> bool {
    match range.buffer_size {
        SupportedBufferSize::Range { min, max } => min <= max && max != 0,
        _ => true,
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
/// default rate into the picked range. Ranges with malformed rate
/// bounds are skipped before any selection.
///
/// `supported` is expected to be F32-filtered already
/// ([`crate::device`]).
pub(crate) fn synthesize_default_format(
    supported: &[SupportedOutputFormat],
    default_channels: u32,
    default_rate: u32,
) -> Option<AudioFormat> {
    pick_synthesized_format(
        supported.iter().map(range_fields),
        default_channels,
        default_rate,
    )
}

/// The input-direction mirror of [`synthesize_default_format`] — same
/// selection rules over [`SupportedInputFormat`].
pub(crate) fn synthesize_default_input_format(
    supported: &[SupportedInputFormat],
    default_channels: u32,
    default_rate: u32,
) -> Option<AudioFormat> {
    pick_synthesized_format(
        supported.iter().map(range_fields),
        default_channels,
        default_rate,
    )
}

/// The `(channels, min_rate, max_rate)` triple the shared selection
/// core consumes — one shape for both direction-specific range types.
fn range_fields<T: RangeFields>(range: &T) -> (u32, u32, u32) {
    range.range_fields()
}

/// The direction-specific supported-range structs share identical
/// rate/channel fields; this trait lets the selection core consume
/// either without a direction-neutral public type.
trait RangeFields {
    fn range_fields(&self) -> (u32, u32, u32);
}

impl RangeFields for SupportedOutputFormat {
    fn range_fields(&self) -> (u32, u32, u32) {
        (self.channels, self.min_sample_rate, self.max_sample_rate)
    }
}

impl RangeFields for SupportedInputFormat {
    fn range_fields(&self) -> (u32, u32, u32) {
        (self.channels, self.min_sample_rate, self.max_sample_rate)
    }
}

/// The selection core of `synthesize_default_*_format`: prefers a range
/// matching the backend default's channel count, then one containing
/// 48 kHz, then one containing the backend default rate; the chosen
/// rate clamps the default rate into the picked range. Ranges with
/// malformed rate bounds are skipped before any selection.
fn pick_synthesized_format(
    ranges: impl Iterator<Item = (u32, u32, u32)>,
    default_channels: u32,
    default_rate: u32,
) -> Option<AudioFormat> {
    ranges
        .filter(|(_, min, max)| min <= max && *max != 0)
        .filter(|(channels, _, _)| layout_for_channels(*channels).is_some())
        .max_by_key(|(channels, min, max)| {
            (
                *channels == default_channels,
                *min <= 48_000 && 48_000 <= *max,
                *min <= default_rate && default_rate <= *max,
            )
        })
        .and_then(|(channels, min, max)| f32_format(default_rate.clamp(min, max), channels))
}

/// Maps a cpal [`cpal::SupportedBufferSize`] onto the port type:
///
/// - `Unknown` → [`SupportedBufferSize::Unknown`] (unreported bound);
/// - `Range { min: 0, max: 0 }` → `Unknown` — not a contradiction but
///   upstream's "bound not published" encoding: cpal's `PipeWire` backend
///   emits verbatim `min_quantum`/`max_quantum` fields that default to
///   `0` until the server clock metadata arrives, and cpal's own build
///   path treats `max_quantum == 0` as "no bound" (it skips `Fixed`
///   validation entirely). Folding it to `Unknown` mirrors upstream
///   semantics 1:1;
/// - `Range { min > max }` → `None` — a self-contradictory report: the
///   caller ([`map_supported_range`]) drops the whole candidate rather
///   than letting it degrade to `Unknown` and gain `negotiate(Fixed)`
///   admission it never earned.
pub(crate) fn map_supported_buffer_size(
    size: &cpal::SupportedBufferSize,
) -> Option<SupportedBufferSize> {
    match *size {
        cpal::SupportedBufferSize::Range { min, max } if min > max => None,
        cpal::SupportedBufferSize::Range { min: 0, max: 0 }
        | cpal::SupportedBufferSize::Unknown => Some(SupportedBufferSize::Unknown),
        cpal::SupportedBufferSize::Range { min, max } => Some(SupportedBufferSize::Range {
            min: usize::try_from(min).unwrap_or(usize::MAX),
            max: usize::try_from(max).unwrap_or(usize::MAX),
        }),
    }
}

/// Maps one cpal supported-format range onto the port type — or drops
/// it (`None`) when the report is unusable at the untrusted-data
/// boundary:
///
/// - non-F32 sample format (integer-only ranges are not reported);
/// - zero channel count (a report that can never be driven);
/// - malformed sample-rate bounds (`min > max` or a zero ceiling);
/// - a self-contradictory buffer bound (`min > max`) — the whole
///   candidate is discarded rather than folding the bound to `Unknown`:
///   a contradictory bound report is not the same as an unreported
///   bound, and must not gain `negotiate(Fixed)` admission.
///
/// A genuine "unreported" bound — cpal `Unknown`, or `PipeWire`'s
/// unpublished-quanta `Range { min: 0, max: 0 }` — maps through as
/// [`SupportedBufferSize::Unknown`].
///
/// Shared by [`map_supported_range`] (output) and
/// [`map_supported_input_range`]: the untrusted-report validation is
/// direction-independent, only the port type differs.
fn validated_range_fields(
    range: &cpal::SupportedStreamConfigRange,
) -> Option<(u32, u32, u32, SupportedBufferSize)> {
    if map_sample_format(range.sample_format()) != Some(SampleFormat::F32) {
        return None;
    }
    if range.channels() == 0
        || range.min_sample_rate() > range.max_sample_rate()
        || range.max_sample_rate() == 0
    {
        return None;
    }
    Some((
        u32::from(range.channels()),
        range.min_sample_rate(),
        range.max_sample_rate(),
        map_supported_buffer_size(range.buffer_size())?,
    ))
}

pub(crate) fn map_supported_range(
    range: &cpal::SupportedStreamConfigRange,
) -> Option<SupportedOutputFormat> {
    let (channels, min_sample_rate, max_sample_rate, buffer_size) = validated_range_fields(range)?;
    Some(SupportedOutputFormat {
        channels,
        min_sample_rate,
        max_sample_rate,
        buffer_size,
        sample_format: SampleFormat::F32,
    })
}

/// The input-direction mirror of [`map_supported_range`] — same
/// untrusted-report validation, producing [`SupportedInputFormat`].
pub(crate) fn map_supported_input_range(
    range: &cpal::SupportedStreamConfigRange,
) -> Option<SupportedInputFormat> {
    let (channels, min_sample_rate, max_sample_rate, buffer_size) = validated_range_fields(range)?;
    Some(SupportedInputFormat {
        channels,
        min_sample_rate,
        max_sample_rate,
        buffer_size,
        sample_format: SampleFormat::F32,
    })
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
/// Every range matching format, channel count and sample rate is a
/// candidate — malformed rate or buffer bounds are skipped first — and
/// the buffer-size constraint is evaluated across *all* candidates, not
/// only the first match: a [`BufferSize::Fixed`] request is admitted
/// when any candidate's reported bound covers it (`Unknown` imposes no
/// bound).
///
/// Returns `Some` only when: `desired` requests `F32`; at least one
/// candidate exists; and a requested `Fixed` size fits `u32`, is
/// nonzero, and is admitted by some candidate. Anything else is `None`
/// → the caller reports
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
    let mut candidates = supported.iter().filter(|range| {
        valid_sample_rate_bounds(range)
            && valid_buffer_bounds(range)
            && range.sample_format == SampleFormat::F32
            && range.channels == channels
            && range.min_sample_rate <= rate
            && rate <= range.max_sample_rate
    });

    let (buffer_size, effective_max_frames) = match desired.buffer_size {
        BufferSize::Default => {
            // The scratch sizing estimate covers the loosest admissible
            // bound across every candidate — a larger estimate only
            // means a larger pre-sized slab.
            let effective_max = candidates
                .map(|range| match range.buffer_size {
                    SupportedBufferSize::Range { max, .. } => max,
                    _ => DEFAULT_FRAMES_FALLBACK,
                })
                .max()?;
            (cpal::BufferSize::Default, effective_max)
        }
        BufferSize::Fixed(frames) => {
            let frames_u32 = u32::try_from(frames).ok()?;
            if frames_u32 == 0 {
                return None;
            }
            let admitted = candidates.any(|range| match range.buffer_size {
                SupportedBufferSize::Range { min, max } => min <= frames && frames <= max,
                SupportedBufferSize::Unknown => true,
                // A future variant is conservative — it does not admit.
                _ => false,
            });
            if !admitted {
                return None;
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
    sample_rate: u32,
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
        // Per-chunk timestamp synthesis: chunk starting at `frame` plays
        // `frame / sample_rate` seconds after the buffer head, so both
        // instants shift by that offset. The synthesized value is an
        // estimate from buffer position — not a backend-measured instant.
        // On `checked_add` overflow the original instant is kept rather
        // than losing the timestamp.
        let chunk_info = if frame == 0 || sample_rate == 0 {
            info
        } else {
            chunk_timestamp_info(info, frame, sample_rate)
        };
        match AudioBufferViewMut::try_new(&mut scratch[..len], channels, chunk) {
            Ok(view) => {
                data_callback(view, chunk_info);
                // `view` was consumed by the callback; re-borrow the
                // slab for packing.
                pack_chunk(&mut scratch[..len], channels, chunk, dst);
            }
            Err(_) => dst.fill(0.0),
        }
        frame += chunk;
    }
}

/// Synthesizes the per-chunk [`OutputCallbackInfo`]: both instants shift
/// by `frame_offset / sample_rate` seconds expressed as
/// [`DEVICE_CLOCK_TIMEBASE`] (nanosecond) ticks.
///
/// `u128` intermediates keep `frame_offset * 1e9` from overflowing even
/// for implausible frame counts; the result saturates through
/// `i64::try_from` — and if `checked_add` still overflows, the original
/// instant is kept rather than losing the timestamp entirely.
///
/// **RT contract:** RT-suitable — integer math only, no allocation.
fn chunk_timestamp_info(
    info: OutputCallbackInfo,
    frame_offset: usize,
    sample_rate: u32,
) -> OutputCallbackInfo {
    let offset_ns =
        u128::try_from(frame_offset).unwrap_or(u128::MAX) * 1_000_000_000 / u128::from(sample_rate);
    let Ok(offset) = i64::try_from(offset_ns) else {
        return info;
    };
    let delta = MediaDuration::new(offset, DEVICE_CLOCK_TIMEBASE);
    OutputCallbackInfo {
        timestamp: ruviform_audio_core::device::OutputTimestamp {
            callback: info
                .timestamp
                .callback
                .checked_add(delta)
                .unwrap_or(info.timestamp.callback),
            playback: info
                .timestamp
                .playback
                .map(|p| p.checked_add(delta).unwrap_or(p)),
        },
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
    fn map_supported_buffer_size_drops_only_contradictory_bounds() {
        assert_eq!(
            map_supported_buffer_size(&cpal::SupportedBufferSize::Range { min: 32, max: 1024 }),
            Some(SupportedBufferSize::Range { min: 32, max: 1024 }),
        );
        // A self-contradictory bound is `None` — the caller drops the
        // whole candidate rather than degrading it to `Unknown`.
        for size in [
            cpal::SupportedBufferSize::Range { min: 1024, max: 32 },
            cpal::SupportedBufferSize::Range { min: 32, max: 0 },
        ] {
            assert_eq!(map_supported_buffer_size(&size), None, "{size:?}");
        }
        // `Range { min: 0, max: 0 }` is *not* contradictory — it is
        // upstream's "bound not published" encoding (cpal's PipeWire
        // backend emits zero quanta until server clock metadata
        // arrives, and treats `max == 0` as no bound at open time), so
        // it maps to `Unknown` like a genuine unreported bound.
        for size in [
            cpal::SupportedBufferSize::Range { min: 0, max: 0 },
            cpal::SupportedBufferSize::Unknown,
        ] {
            assert_eq!(
                map_supported_buffer_size(&size),
                Some(SupportedBufferSize::Unknown),
                "{size:?}"
            );
        }
    }

    // -- map_supported_range (map → formats → negotiate chain) ------------

    fn cpal_range(
        channels: u16,
        min: u32,
        max: u32,
        buffer: cpal::SupportedBufferSize,
    ) -> cpal::SupportedStreamConfigRange {
        cpal::SupportedStreamConfigRange::new(channels, min, max, buffer, cpal::SampleFormat::F32)
    }

    #[test]
    fn map_supported_range_maps_valid_reports_verbatim() {
        // A well-formed report passes through with every field intact.
        assert_eq!(
            map_supported_range(&cpal_range(
                2,
                44_100,
                48_000,
                cpal::SupportedBufferSize::Range { min: 32, max: 1024 },
            )),
            Some(SupportedOutputFormat {
                channels: 2,
                min_sample_rate: 44_100,
                max_sample_rate: 48_000,
                buffer_size: SupportedBufferSize::Range { min: 32, max: 1024 },
                sample_format: SampleFormat::F32,
            }),
        );
    }

    #[test]
    fn boundary_drops_malformed_reports_instead_of_folding_to_unknown() {
        // The real chain: a *contradictory* buffer bound is not an
        // *unreported* bound — dropping the whole candidate prevents a
        // corrupt report from gaining `negotiate(Fixed)` admission as
        // `Unknown`.
        for buffer in [
            cpal::SupportedBufferSize::Range { min: 1024, max: 32 },
            cpal::SupportedBufferSize::Range { min: 32, max: 0 },
        ] {
            let range = cpal_range(2, 44_100, 48_000, buffer);
            assert_eq!(map_supported_range(&range), None, "{buffer:?}");
        }
        // Malformed sample-rate bounds, zero-channel reports, and
        // non-F32 encodings drop too.
        for range in [
            cpal_range(2, 96_000, 44_100, cpal::SupportedBufferSize::Unknown),
            cpal_range(2, 0, 0, cpal::SupportedBufferSize::Unknown),
            cpal_range(0, 44_100, 48_000, cpal::SupportedBufferSize::Unknown),
            cpal::SupportedStreamConfigRange::new(
                2,
                44_100,
                48_000,
                cpal::SupportedBufferSize::Unknown,
                cpal::SampleFormat::I16,
            ),
        ] {
            assert_eq!(map_supported_range(&range), None, "{range:?}");
        }
    }

    #[test]
    fn chain_genuine_unknown_admits_fixed_but_malformed_cannot() {
        // `map_supported_range → supported_output_formats →
        // negotiate(Fixed)` on synthesized backend reports: only a
        // bound that is genuinely *unreported* — cpal `Unknown` or
        // PipeWire's unpublished-quanta `Range { 0, 0 }` — survives as
        // a constraint-free candidate. A contradictory bound cannot.
        let malformed = cpal_range(
            2,
            44_100,
            48_000,
            cpal::SupportedBufferSize::Range { min: 1024, max: 32 },
        );
        let genuine_unknown = cpal_range(2, 44_100, 48_000, cpal::SupportedBufferSize::Unknown);
        let unpublished = cpal_range(
            2,
            44_100,
            48_000,
            cpal::SupportedBufferSize::Range { min: 0, max: 0 },
        );
        let surviving: Vec<_> = [malformed, genuine_unknown, unpublished]
            .iter()
            .filter_map(map_supported_range)
            .collect();
        assert_eq!(
            surviving.iter().map(|f| f.buffer_size).collect::<Vec<_>>(),
            [SupportedBufferSize::Unknown; 2],
        );
        for mapped in &surviving {
            let config = negotiate(
                &desired(48_000, ChannelLayout::Stereo, BufferSize::Fixed(480)),
                std::slice::from_ref(mapped),
            )
            .unwrap();
            assert_eq!(config.stream.buffer_size, cpal::BufferSize::Fixed(480));
        }
        // …while a space containing only the contradictory report
        // (dropped at the boundary) admits nothing.
        let dropped_only: Vec<_> = [cpal_range(
            2,
            44_100,
            48_000,
            cpal::SupportedBufferSize::Range { min: 1024, max: 32 },
        )]
        .iter()
        .filter_map(map_supported_range)
        .collect();
        assert_eq!(dropped_only, Vec::<SupportedOutputFormat>::new());
        assert_eq!(
            negotiate(
                &desired(48_000, ChannelLayout::Stereo, BufferSize::Fixed(480)),
                &dropped_only,
            ),
            None,
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

    #[test]
    fn negotiate_fixed_considers_every_matching_range() {
        // The first matching range rejects the request; a later one
        // admits it — evaluating only the first match would falsely
        // report `UnsupportedConfig`.
        let supported = [
            f32_range(
                2,
                44_100,
                48_000,
                SupportedBufferSize::Range { min: 32, max: 256 },
            ),
            f32_range(
                2,
                44_100,
                48_000,
                SupportedBufferSize::Range {
                    min: 256,
                    max: 1024,
                },
            ),
        ];
        let config = negotiate(
            &desired(48_000, ChannelLayout::Stereo, BufferSize::Fixed(512)),
            &supported,
        )
        .unwrap();
        assert_eq!(config.stream.buffer_size, cpal::BufferSize::Fixed(512));
        assert_eq!(config.effective_max_frames, 512);
        // And a size no range admits is still rejected.
        assert_eq!(
            negotiate(
                &desired(48_000, ChannelLayout::Stereo, BufferSize::Fixed(2048)),
                &supported,
            ),
            None,
        );
    }

    #[test]
    fn negotiate_fixed_accepts_via_unknown_bound_among_ranges() {
        // An `Unknown` buffer report imposes no bound — it admits any
        // nonzero `Fixed` even when a sibling range is restrictive.
        let supported = [
            f32_range(
                2,
                44_100,
                48_000,
                SupportedBufferSize::Range { min: 32, max: 256 },
            ),
            f32_range(2, 44_100, 48_000, SupportedBufferSize::Unknown),
        ];
        assert!(
            negotiate(
                &desired(48_000, ChannelLayout::Stereo, BufferSize::Fixed(512)),
                &supported,
            )
            .is_some(),
        );
    }

    #[test]
    fn negotiate_default_sizes_scratch_from_the_loosest_match() {
        let supported = [
            f32_range(
                2,
                44_100,
                48_000,
                SupportedBufferSize::Range { min: 32, max: 256 },
            ),
            f32_range(
                2,
                44_100,
                48_000,
                SupportedBufferSize::Range { min: 64, max: 2048 },
            ),
        ];
        let config = negotiate(
            &desired(48_000, ChannelLayout::Stereo, BufferSize::Default),
            &supported,
        )
        .unwrap();
        assert_eq!(config.effective_max_frames, 2048);
    }

    #[test]
    fn negotiate_skips_malformed_ranges() {
        // Inverted rate bounds can never be a candidate (and must not
        // panic while being skipped).
        let inverted = [f32_range(2, 96_000, 44_100, SupportedBufferSize::Unknown)];
        assert_eq!(
            negotiate(
                &desired(48_000, ChannelLayout::Stereo, BufferSize::Default),
                &inverted,
            ),
            None,
        );
        // A contradictory or zero-ceiling buffer bound is skipped too —
        // symmetric with the rate axis, so a malformed port-level range
        // is never a candidate under either `Default` or `Fixed`.
        // (The cpal-level `Range { 0, 0 }` "unpublished" encoding maps
        // to `Unknown` at the boundary before this layer ever sees it.)
        for size in [
            SupportedBufferSize::Range { min: 0, max: 0 },
            SupportedBufferSize::Range { min: 1024, max: 32 },
        ] {
            let malformed = [f32_range(2, 44_100, 48_000, size)];
            assert_eq!(
                negotiate(
                    &desired(48_000, ChannelLayout::Stereo, BufferSize::Default),
                    &malformed,
                ),
                None,
                "{size:?}",
            );
            assert_eq!(
                negotiate(
                    &desired(48_000, ChannelLayout::Stereo, BufferSize::Fixed(480)),
                    &malformed,
                ),
                None,
                "{size:?}",
            );
        }
    }

    #[test]
    fn synthesize_default_skips_malformed_rate_bounds() {
        // Inverted or zero-ceiling bounds are skipped, never clamped.
        let supported = [
            f32_range(2, 96_000, 44_100, SupportedBufferSize::Unknown),
            f32_range(2, 0, 0, SupportedBufferSize::Unknown),
            f32_range(2, 44_100, 48_000, SupportedBufferSize::Unknown),
        ];
        assert_eq!(
            synthesize_default_format(&supported, 2, 96_000),
            f32_format(48_000, 2),
        );
        // With only malformed ranges there is no candidate — and no
        // panic from `clamp` on inverted bounds.
        let malformed_only = [f32_range(2, 96_000, 44_100, SupportedBufferSize::Unknown)];
        assert_eq!(synthesize_default_format(&malformed_only, 2, 48_000), None);
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
            48_000,
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
        write_interleaved(&mut scratch, 4, 2, &mut data, test_info(), 48_000, &mut cb);
        // Per chunk, channel 1 carries the chunk's frame count:
        // chunks of 4, 4, then 2.
        let ch1: Vec<f32> = data.as_chunks::<2>().0.iter().map(|f| f[1]).collect();
        assert_eq!(
            ch1,
            [4.0; 8].into_iter().chain([2.0; 2]).collect::<Vec<_>>()
        );
        // Channel 0 packed the callback output for every frame.
        assert!(data.as_chunks::<2>().0.iter().all(|f| f[0] == 0.25));
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
        write_interleaved(&mut scratch, 8, 2, &mut data, test_info(), 48_000, &mut cb);
        // Every delivered sample is a known value (0.25 callback
        // output or 0.0 fill) — never NaN scratch or stale 9.0.
        assert!(data.iter().all(|s| *s == 0.25 || *s == 0.0), "{data:?}");
    }

    /// Collects each `OutputCallbackInfo` a callback receives into a
    /// shared vec — `OutputDataCallback` requires `'static`.
    fn info_collecting_callback() -> (
        OutputDataCallback,
        std::sync::Arc<std::sync::Mutex<Vec<OutputCallbackInfo>>>,
    ) {
        let infos = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let captured = std::sync::Arc::clone(&infos);
        (
            Box::new(
                move |out: AudioBufferViewMut<'_, f32>, info: OutputCallbackInfo| {
                    let _ = out;
                    captured.lock().unwrap().push(info);
                },
            ),
            infos,
        )
    }

    #[test]
    fn write_interleaved_synthesizes_per_chunk_timestamps() {
        // 10 frames at capacity 4 → chunks starting at frames 0, 4, 8.
        // At 48 kHz each chunk's instants shift by `f_k * 1e9 / 48_000`
        // nanoseconds: 0, 83_333, 166_666 (integer division, sub-tick
        // fraction truncated).
        let (mut cb, infos) = info_collecting_callback();
        let mut scratch = vec![0.0_f32; 8];
        let mut data = vec![0.0_f32; 20];
        let base = OutputCallbackInfo {
            timestamp: OutputTimestamp {
                callback: MediaTime::new(1_000, DEVICE_CLOCK_TIMEBASE),
                playback: Some(MediaTime::new(2_000, DEVICE_CLOCK_TIMEBASE)),
            },
        };
        write_interleaved(&mut scratch, 4, 2, &mut data, base, 48_000, &mut cb);

        let infos = infos.lock().unwrap();
        assert_eq!(infos.len(), 3);
        let expected = [0_i64, 83_333, 166_666];
        for (info, offset) in infos.iter().zip(expected) {
            assert_eq!(info.timestamp.callback.value(), 1_000 + offset);
            assert_eq!(
                info.timestamp.playback.map(MediaTime::value),
                Some(2_000 + offset)
            );
        }
    }

    #[test]
    fn write_interleaved_chunk_timestamp_none_playback_stays_none() {
        let (mut cb, infos) = info_collecting_callback();
        let mut scratch = vec![0.0_f32; 8];
        let mut data = vec![0.0_f32; 20];
        write_interleaved(&mut scratch, 4, 2, &mut data, test_info(), 48_000, &mut cb);
        let infos = infos.lock().unwrap();
        assert!(infos.iter().all(|i| i.timestamp.playback.is_none()));
    }

    #[test]
    fn write_interleaved_chunk_timestamp_add_overflow_keeps_original() {
        // Base instant near `i64::MAX` → `checked_add` fails → the
        // original instant is delivered instead of dropping it.
        let (mut cb, infos) = info_collecting_callback();
        let mut scratch = vec![0.0_f32; 8];
        let mut data = vec![0.0_f32; 20];
        let near_max = OutputCallbackInfo {
            timestamp: OutputTimestamp {
                callback: MediaTime::new(i64::MAX - 10, DEVICE_CLOCK_TIMEBASE),
                playback: Some(MediaTime::new(i64::MAX - 10, DEVICE_CLOCK_TIMEBASE)),
            },
        };
        write_interleaved(&mut scratch, 4, 2, &mut data, near_max, 48_000, &mut cb);
        let infos = infos.lock().unwrap();
        assert_eq!(infos.len(), 3);
        assert_eq!(infos[0].timestamp.callback.value(), i64::MAX - 10);
        // Overflowing chunks keep the original instant rather than None.
        assert_eq!(infos[1].timestamp.callback.value(), i64::MAX - 10);
        assert_eq!(
            infos[1].timestamp.playback.map(MediaTime::value),
            Some(i64::MAX - 10)
        );
    }

    #[test]
    fn write_interleaved_single_chunk_keeps_info_unchanged() {
        let (mut cb, infos) = info_collecting_callback();
        let mut scratch = vec![0.0_f32; 8];
        let mut data = vec![0.0_f32; 8]; // 4 frames ≤ capacity
        write_interleaved(&mut scratch, 4, 2, &mut data, test_info(), 48_000, &mut cb);
        let infos = infos.lock().unwrap();
        assert_eq!(infos.len(), 1);
        assert_eq!(infos[0].timestamp.callback.value(), 0);
        assert!(infos[0].timestamp.playback.is_none());
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
