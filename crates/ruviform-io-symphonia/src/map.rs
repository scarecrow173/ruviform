//! Pure backend → boundary value mappings.
//!
//! Every function in this module converts a backend value type into the
//! media-io / domain type model without exposing the backend type in any
//! public signature. Mappings that cannot be represented in the boundary
//! model degrade honestly: unknown codecs → [`CodecId::Other`], streams
//! without a domain-format mapping →
//! [`CodecFormat::Data`](ruviform_media_io::CodecFormat), and unmappable
//! channel sets → `None` (which the caller normalizes into
//! [`ErrorKind::Unsupported`](ruviform_media_io::ErrorKind::Unsupported)).

use ruviform_audio_core::{AudioFormat, ChannelLayout, SampleFormat, SampleRate, SpeakerMask};
use ruviform_media_core::{StreamKind, TimeBase};
use ruviform_media_io::CodecId;
use symphonia::core::audio::{Channels, Position};
use symphonia::core::codecs::CodecParameters;
use symphonia::core::codecs::audio::well_known as audio_id;
use symphonia::core::codecs::audio::{AudioCodecId, AudioCodecParameters};
use symphonia::core::codecs::video::VideoCodecId;
use symphonia::core::codecs::video::well_known as video_id;
use symphonia::core::units::TimeBase as BackendTimeBase;

/// Converts a backend time base (guaranteed non-zero components) into a
/// boundary [`TimeBase`].
///
/// The fallback is unreachable in practice — the backend stores
/// `NonZero` components — but keeps the conversion total without an
/// `unwrap`.
pub(crate) fn to_time_base(base: BackendTimeBase) -> TimeBase {
    TimeBase::new(base.numer.get(), base.denom.get()).unwrap_or(TimeBase::COMMON_90K)
}

/// Maps backend codec parameters to a boundary [`StreamKind`].
///
/// Subtitle tracks and tracks with unidentified parameters land in
/// `Data` — the boundary has no subtitle kind yet.
pub(crate) fn stream_kind(params: Option<&CodecParameters>) -> StreamKind {
    match params {
        Some(CodecParameters::Audio(_)) => StreamKind::Audio,
        Some(CodecParameters::Video(_)) => StreamKind::Video,
        _ => StreamKind::Data,
    }
}

/// Maps a backend track's codec parameters to a boundary [`CodecId`].
///
/// Only the codecs the Ruviform registry names are claimed; every other
/// codec lands on [`CodecId::Other`].
pub(crate) fn codec_id(params: Option<&CodecParameters>) -> CodecId {
    match params {
        Some(CodecParameters::Audio(p)) => codec_id_for_audio(p.codec),
        Some(CodecParameters::Video(p)) => codec_id_for_video(p.codec),
        _ => CodecId::Other,
    }
}

/// Maps backend audio codec IDs to boundary [`CodecId`]s.
///
/// The PCM family covers every `CODEC_ID_PCM_*` wire encoding (integer,
/// float, planar variants, and A-law/µ-law) — the encoding detail is
/// preserved through `codec_private` while the Ruviform registry only
/// names the family. Layer III alone maps to [`CodecId::Mp3`]; MP1/MP2
/// are distinct codecs. Codecs the Ruviform registry does not name yet
/// (ADPCM, WMA, AC-3, …) land on [`CodecId::Other`].
fn codec_id_for_audio(codec: AudioCodecId) -> CodecId {
    /// Every `CODEC_ID_PCM_*` variant — one per wire encoding.
    const PCM_IDS: &[AudioCodecId] = &[
        audio_id::CODEC_ID_PCM_S32LE,
        audio_id::CODEC_ID_PCM_S32LE_PLANAR,
        audio_id::CODEC_ID_PCM_S32BE,
        audio_id::CODEC_ID_PCM_S32BE_PLANAR,
        audio_id::CODEC_ID_PCM_S24LE,
        audio_id::CODEC_ID_PCM_S24LE_PLANAR,
        audio_id::CODEC_ID_PCM_S24BE,
        audio_id::CODEC_ID_PCM_S24BE_PLANAR,
        audio_id::CODEC_ID_PCM_S16LE,
        audio_id::CODEC_ID_PCM_S16LE_PLANAR,
        audio_id::CODEC_ID_PCM_S16BE,
        audio_id::CODEC_ID_PCM_S16BE_PLANAR,
        audio_id::CODEC_ID_PCM_S8,
        audio_id::CODEC_ID_PCM_S8_PLANAR,
        audio_id::CODEC_ID_PCM_U32LE,
        audio_id::CODEC_ID_PCM_U32LE_PLANAR,
        audio_id::CODEC_ID_PCM_U32BE,
        audio_id::CODEC_ID_PCM_U32BE_PLANAR,
        audio_id::CODEC_ID_PCM_U24LE,
        audio_id::CODEC_ID_PCM_U24LE_PLANAR,
        audio_id::CODEC_ID_PCM_U24BE,
        audio_id::CODEC_ID_PCM_U24BE_PLANAR,
        audio_id::CODEC_ID_PCM_U16LE,
        audio_id::CODEC_ID_PCM_U16LE_PLANAR,
        audio_id::CODEC_ID_PCM_U16BE,
        audio_id::CODEC_ID_PCM_U16BE_PLANAR,
        audio_id::CODEC_ID_PCM_U8,
        audio_id::CODEC_ID_PCM_U8_PLANAR,
        audio_id::CODEC_ID_PCM_F32LE,
        audio_id::CODEC_ID_PCM_F32LE_PLANAR,
        audio_id::CODEC_ID_PCM_F32BE,
        audio_id::CODEC_ID_PCM_F32BE_PLANAR,
        audio_id::CODEC_ID_PCM_F64LE,
        audio_id::CODEC_ID_PCM_F64LE_PLANAR,
        audio_id::CODEC_ID_PCM_F64BE,
        audio_id::CODEC_ID_PCM_F64BE_PLANAR,
        audio_id::CODEC_ID_PCM_ALAW,
        audio_id::CODEC_ID_PCM_MULAW,
    ];
    if PCM_IDS.contains(&codec) {
        CodecId::Pcm
    } else if codec == audio_id::CODEC_ID_VORBIS {
        CodecId::Vorbis
    } else if codec == audio_id::CODEC_ID_OPUS {
        CodecId::Opus
    } else if codec == audio_id::CODEC_ID_MP3 {
        CodecId::Mp3
    } else if codec == audio_id::CODEC_ID_AAC {
        CodecId::Aac
    } else if codec == audio_id::CODEC_ID_FLAC {
        CodecId::Flac
    } else if codec == audio_id::CODEC_ID_ALAC {
        CodecId::Alac
    } else {
        CodecId::Other
    }
}

/// Maps backend video codec IDs to boundary [`CodecId`]s.
fn codec_id_for_video(codec: VideoCodecId) -> CodecId {
    if codec == video_id::CODEC_ID_H264 {
        CodecId::H264
    } else if codec == video_id::CODEC_ID_HEVC {
        CodecId::Hevc
    } else if codec == video_id::CODEC_ID_VP9 {
        CodecId::Vp9
    } else if codec == video_id::CODEC_ID_AV1 {
        CodecId::Av1
    } else {
        CodecId::Other
    }
}

/// Builds the boundary [`AudioFormat`] for an audio track, when the
/// backend parameters carry enough information.
///
/// `None` when the sample rate is absent/zero or the channel set cannot
/// be represented in the boundary layout model — the caller then reports
/// `CodecFormat::Data`, honestly signalling "no domain format mapping".
/// `sample_format` is always [`SampleFormat::F32`] because this backend's
/// decode contract is canonical planar f32 regardless of wire encoding.
pub(crate) fn audio_format(params: &AudioCodecParameters) -> Option<AudioFormat> {
    let rate = params.sample_rate?;
    let layout = channel_layout(params.channels.as_ref()?)?.0;
    Some(AudioFormat {
        sample_rate: SampleRate::new(rate).ok()?,
        layout,
        sample_format: SampleFormat::F32,
    })
}

/// Maps a backend channel set to a boundary [`ChannelLayout`].
///
/// `Positioned` masks matching a standard layout name it exactly; other
/// positioned masks and the unpositioned channel sets (`Discrete`,
/// label-based `Custom`, `Ambisonic`) become a [`ChannelLayout::Custom`]
/// whose mask carries named positions on their named bits and every
/// remaining channel on a reserved bit — the bit count always equals the
/// channel count, which [`ChannelLayout::channels`] and downstream
/// `AudioBuffer` construction depend on. Returns `None` for the empty
/// `Channels::None` set or when the set needs more reserved bits than
/// [`SpeakerMask`]'s u32 provides.
///
/// The returned `Vec<usize>` is the channel-order permutation: decoded
/// plane `i` (backend canonical order — position bits LSB→MSB) belongs
/// to boundary channel slot `order[i]` (layout-mask bits LSB→MSB). It is
/// the identity for standard layouts and for every unpositioned set, but
/// a `Positioned` mask mixing unnamed positions below `SIDE_LEFT`
/// (backend bits 6–8) with named side channels (bits 9–10) reorders —
/// the boundary packs unnamed positions onto reserved bits above the
/// named range.
pub(crate) fn channel_layout(channels: &Channels) -> Option<(ChannelLayout, Vec<usize>)> {
    match channels {
        Channels::Positioned(positions) => positioned_layout(*positions),
        Channels::None => None,
        other => custom_layout_for_count(other.count()).map(|layout| {
            let order = (0..layout.channels()).map(|index| index as usize).collect();
            (layout, order)
        }),
    }
}

/// Maps a `Positioned` mask: exact standard layouts first, then a
/// `Custom` mask preserving named positions and channel count.
///
/// The second tuple element is the plane → channel-slot permutation (see
/// [`channel_layout`]); standard layouts yield the identity.
fn positioned_layout(positions: Position) -> Option<(ChannelLayout, Vec<usize>)> {
    /// Backend `Position` bit → boundary `SpeakerMask` bit.
    ///
    /// The backend numbers its first 18 positions like the
    /// `WAVEFORMATEX` mask, while `SpeakerMask` packs only the eight
    /// named positions into the low bits — the mapping is explicit, not
    /// a bit copy.
    const NAMED: [(u64, SpeakerMask); 8] = [
        (1 << 0, SpeakerMask::FRONT_LEFT),
        (1 << 1, SpeakerMask::FRONT_RIGHT),
        (1 << 2, SpeakerMask::FRONT_CENTER),
        // `LFE1` is the boundary's `LOW_FREQUENCY`.
        (1 << 3, SpeakerMask::LOW_FREQUENCY),
        // `REAR_LEFT/RIGHT` are the boundary's `BACK_LEFT/RIGHT`.
        (1 << 4, SpeakerMask::BACK_LEFT),
        (1 << 5, SpeakerMask::BACK_RIGHT),
        (1 << 9, SpeakerMask::SIDE_LEFT),
        (1 << 10, SpeakerMask::SIDE_RIGHT),
    ];

    let bits = positions.bits();
    // Exact standard layouts — the mask must equal the position set,
    // not merely contain it.
    let standard = match bits {
        0x4 => Some(ChannelLayout::Mono),
        0x3 => Some(ChannelLayout::Stereo),
        0x33 => Some(ChannelLayout::Quad),
        0x3f => Some(ChannelLayout::FivePointOne),
        0x63f => Some(ChannelLayout::SevenPointOne),
        _ => None,
    };
    if let Some(standard) = standard {
        let order = (0..standard.channels())
            .map(|index| index as usize)
            .collect();
        return Some((standard, order));
    }

    // Non-standard set: named positions keep their named bits; every
    // unnamed position is assigned a reserved bit so the mask bit count
    // stays equal to the channel count. `mapped` records each backend
    // bit (LSB→MSB = plane order) against the mask bit it landed on, so
    // the boundary's bit-ascending channel slots can be reconstructed.
    let mut mask = SpeakerMask::empty();
    let mut mapped: Vec<u32> = Vec::new();
    let mut reserved = 0x100_u32; // first bit above the named range
    let mut remaining = bits;
    while remaining != 0 {
        let bit = remaining & remaining.wrapping_neg();
        if let Some((_, named)) = NAMED.iter().find(|(position, _)| *position == bit) {
            mask = mask.union(*named);
            mapped.push(named.bits());
        } else {
            if reserved == 0 {
                return None; // more unnamed channels than free mask bits
            }
            mask = mask.union(SpeakerMask::from_bits(reserved));
            mapped.push(reserved);
            reserved <<= 1;
        }
        remaining &= remaining - 1;
    }
    let layout = ChannelLayout::custom(mask).ok()?;
    // Plane i carries backend bit `mapped[i]`; its boundary slot is that
    // bit's rank among the mask's set bits.
    let mask_bits = mask.bits();
    let order = mapped
        .iter()
        .map(|bit| (mask_bits & (bit - 1)).count_ones() as usize)
        .collect();
    Some((layout, order))
}

/// Builds a `Custom` layout holding `count` reserved bits — used for
/// channel sets the backend does not position (`Discrete`, label-based
/// `Custom`, `Ambisonic`). `None` when `count` is zero or exceeds the 24
/// reserved bits above the eight named positions.
fn custom_layout_for_count(count: usize) -> Option<ChannelLayout> {
    /// `SpeakerMask` bits 8..32 are unnamed — 24 reserved slots.
    const RESERVED_BITS: usize = 24;
    if count == 0 || count > RESERVED_BITS {
        return None;
    }
    let mask = SpeakerMask::from_bits(((1u32 << count) - 1) << 8);
    ChannelLayout::custom(mask).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positioned_standard_layouts_map_exactly() {
        let mono = Position::FRONT_CENTER;
        assert_eq!(
            positioned_layout(mono),
            Some((ChannelLayout::Mono, vec![0]))
        );

        let stereo = Position::FRONT_LEFT.union(Position::FRONT_RIGHT);
        assert_eq!(
            positioned_layout(stereo),
            Some((ChannelLayout::Stereo, vec![0, 1]))
        );
    }

    #[test]
    fn nonstandard_positioned_mask_preserves_count() {
        // Front left + rear center: FL is named (0x1), RC unnamed →
        // one reserved bit. Two channels total.
        let mask = Position::FRONT_LEFT.union(Position::REAR_CENTER);
        let layout = positioned_layout(mask);
        assert!(layout.is_some());
        assert_eq!(layout.map(|(layout, _)| layout.channels()), Some(2));
    }

    #[test]
    fn six_point_one_mask_reorders_side_channels() {
        // FL FR FC LFE RC SL SR — the 6.1 set. The backend's canonical
        // plane order is bit-ascending (…, RC=8, SL=9, SR=10), but the
        // boundary mask packs RC onto a reserved bit above the named
        // SIDE_* slots, so planes 4–6 land on boundary slots 6, 4, 5.
        let mask = Position::FRONT_LEFT
            .union(Position::FRONT_RIGHT)
            .union(Position::FRONT_CENTER)
            .union(Position::LFE1)
            .union(Position::REAR_CENTER)
            .union(Position::SIDE_LEFT)
            .union(Position::SIDE_RIGHT);
        let (layout, order) = positioned_layout(mask).expect("mappable mask");
        assert_eq!(layout.channels(), 7);
        assert_eq!(order, vec![0, 1, 2, 3, 6, 4, 5]);
    }

    #[test]
    fn discrete_channels_land_on_reserved_bits() {
        let layout = channel_layout(&Channels::Discrete(2));
        assert!(layout.is_some());
        assert_eq!(
            layout.as_ref().map(|(layout, _)| layout.channels()),
            Some(2)
        );
        // Bits must sit above the named range.
        let bits = layout.map(|(layout, _)| layout.speaker_mask().bits());
        assert_eq!(bits, Some(0x300));
    }

    #[test]
    fn empty_channels_yield_no_layout() {
        assert_eq!(channel_layout(&Channels::None), None);
        assert_eq!(custom_layout_for_count(0), None);
        assert_eq!(custom_layout_for_count(25), None);
    }

    #[test]
    fn pcm_codecs_map_to_pcm_id() {
        assert_eq!(
            codec_id_for_audio(audio_id::CODEC_ID_PCM_S16LE),
            CodecId::Pcm
        );
        assert_eq!(
            codec_id_for_audio(audio_id::CODEC_ID_PCM_F32BE),
            CodecId::Pcm
        );
        assert_eq!(
            codec_id_for_audio(audio_id::CODEC_ID_PCM_ALAW),
            CodecId::Pcm
        );
        assert_eq!(
            codec_id_for_audio(audio_id::CODEC_ID_VORBIS),
            CodecId::Vorbis
        );
        assert_eq!(
            codec_id_for_audio(audio_id::CODEC_ID_ADPCM_MS),
            CodecId::Other
        );
        assert_eq!(codec_id_for_audio(audio_id::CODEC_ID_MP2), CodecId::Other);
    }
}
