//! `ruviform-audio-core` — audio data types for the Ruviform media
//! foundation: the sealed [`Sample`] trait (`f32`/`f64`), the audio time
//! contract ([`SampleRate`], [`SampleIndex`]), channel layouts and
//! speaker masks ([`ChannelLayout`], [`SpeakerMask`]), stream-level
//! format description ([`AudioFormat`], [`SampleFormat`]) and the
//! channel-major contiguous planar [`AudioBuffer`] with its borrowed
//! views ([`AudioBufferView`], [`AudioBufferViewMut`], [`BufferSpec`]).
//!
//! Planar `f32` is the canonical processing format; interleaved sample
//! order exists only at device/file boundaries — through
//! [`AudioBuffer::from_interleaved`], [`AudioBuffer::copy_to_interleaved`]
//! and [`AudioBuffer::copy_from_interleaved`].
//!
//! Holds no processing, I/O, or device implementations. `ruviform-audio`
//! (engine) and `ruviform-audio-device` (adapter) connect through the
//! device boundary defined in a later issue, structurally eliminating a
//! direct engine ↔ adapter dependency.
//!
//! ## Allowed dependency layers
//!
//! Primitives (`ruviform-math`, `ruviform-rt`) and `ruviform-media-core`.
//! Actually used: `ruviform-media-core` ([`TimeBase`], [`MediaTime`],
//! [`Rounding`], [`Error`]/[`Result`]). `ruviform-audio-core` must not
//! depend on `ruviform-video-core` (same-tier dependency is prohibited).
//!
//! ## RT contract
//!
//! Buffer views and accessors are allocation-free, non-blocking borrows —
//! RT-suitable. Allocating constructors ([`AudioBuffer::new`],
//! [`AudioBuffer::from_interleaved`]) are labelled `**RT contract:**
//! non-RT only`. A buffer never grows after construction: no
//! `push`/`reserve`-style API exists. Per-API guarantees are documented
//! as `**RT contract:**` lines on each item, mirroring the RT ADR rule
//! that the guaranteed scope is defined per API.
//!
//! Contract source: `docs/designs/ruviform-architecture-design-baseline.md`
//! (crate table A.2, "音声データモデル") and
//! `docs/adr/realtime-contract-and-verification-policy.md`.

// The public names and shapes below are fixed API decisions of the data
// model: they deliberately echo their module (`buffer::AudioBuffer`,
// `layout::ChannelLayout`, …) while being re-exported at the crate root,
// and as a `publish = false` workspace-internal value-type crate this API
// relies on exhaustive matching and literal construction rather than
// `#[non_exhaustive]` forward-compat guards (`SampleFormat` excepted — it
// is explicitly extensible).
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::exhaustive_enums)]
#![allow(clippy::exhaustive_structs)]

mod buffer;
mod format;
mod layout;
mod sample;
mod time;

pub use buffer::{AudioBuffer, AudioBufferView, AudioBufferViewMut, BufferSpec};
pub use format::{AudioFormat, SampleFormat};
pub use layout::{ChannelLayout, SpeakerMask};
pub use sample::Sample;
pub use time::{SampleIndex, SampleRate};

/// Returns the crate version string.
///
/// **RT contract:** RT-suitable — pure, allocation-free.
///
/// # Examples
///
/// ```
/// assert_ne!(ruviform_audio_core::version(), "");
/// ```
#[must_use]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruviform_media_core::Rounding;

    fn stereo_spec(frames: usize) -> BufferSpec {
        BufferSpec {
            layout: ChannelLayout::Stereo,
            frames,
            sample_rate: SampleRate::new(48_000).unwrap(),
        }
    }

    #[test]
    fn version_is_not_empty() {
        assert_ne!(version(), "");
    }

    /// RT contract: view channel access is an allocation-free borrow;
    /// writes through the mutable view land in the channel-major slab.
    #[test]
    fn rt_view_channel_access() {
        let mut buffer = AudioBuffer::from_interleaved(
            &[0.0_f32, 10.0, 1.0, 11.0, 2.0, 12.0, 3.0, 13.0],
            stereo_spec(4),
        )
        .unwrap();
        {
            let view = buffer.view();
            assert_eq!(view.channels(), 2);
            assert_eq!(view.frames(), 4);
            assert_eq!(view.channel(0), Some(&[0.0, 1.0, 2.0, 3.0][..]));
            assert_eq!(view.channel(1), Some(&[10.0, 11.0, 12.0, 13.0][..]));
            assert_eq!(view.channel(2), None);
        }
        {
            let mut view = buffer.view_mut();
            for sample in view.channel_mut(1).unwrap() {
                *sample += 1.0;
            }
            assert_eq!(view.channel(1), Some(&[11.0, 12.0, 13.0, 14.0][..]));
        }
        assert_eq!(buffer.channel(1), Some(&[11.0, 12.0, 13.0, 14.0][..]));
    }

    /// RT contract: interleaved boundary copies through views are
    /// allocation-free bounded copies — the canonical layout stays planar.
    #[test]
    fn rt_interleaved_roundtrip_via_views() {
        let src = [0.0_f32, 100.0, 1.0, 101.0, 2.0, 102.0];
        let mut buffer = AudioBuffer::from_interleaved(&src, stereo_spec(3)).unwrap();

        let mut packed = [0.0_f32; 6];
        buffer.view().copy_to_interleaved(&mut packed).unwrap();
        assert_eq!(packed, src);

        let other = [5.0_f32, 105.0, 6.0, 106.0, 7.0, 107.0];
        buffer.view_mut().copy_from_interleaved(&other).unwrap();
        let mut out = [0.0_f32; 6];
        buffer.view().copy_to_interleaved(&mut out).unwrap();
        assert_eq!(out, other);
    }

    /// RT contract: sample-index ↔ `MediaTime` conversion is exact
    /// rational math — RT-suitable, allocation-free.
    #[test]
    fn rt_sample_index_media_time_conversion() {
        let rate = SampleRate::new(48_000).unwrap();
        let index = SampleIndex(96_000); // exactly 2 s
        let time = index.to_media_time(rate);
        assert_eq!(
            SampleIndex::from_media_time(time, rate, Rounding::NearestEven),
            Ok(index)
        );
    }
}
