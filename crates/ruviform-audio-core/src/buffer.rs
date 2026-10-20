//! The canonical audio buffer: channel-major contiguous planar storage.
//!
//! Planar `f32` ([`AudioBuffer<f32>`], the default type parameter) is the
//! canonical processing layout: channel `c` occupies
//! `data[c * frames .. (c + 1) * frames]` of one contiguous `Vec`.
//! Interleaved sample order exists only at device/file boundaries — the
//! sanctioned conversion points are [`AudioBuffer::from_interleaved`],
//! [`AudioBuffer::copy_to_interleaved`],
//! [`AudioBuffer::copy_from_interleaved`] and their view counterparts.
//!
//! A buffer never grows after construction: there is deliberately no
//! `push`/`reserve`-style API. Allocating constructors are non-RT; views
//! ([`AudioBufferView`], [`AudioBufferViewMut`]) and all accessors are
//! RT-suitable.

use ruviform_media_core::{Error, Result};

use crate::layout::ChannelLayout;
use crate::sample::Sample;
use crate::time::SampleRate;

/// Slice bounds `[start, end)` of channel `index` inside a channel-major
/// slab — `None` when `index` is out of range.
fn channel_range(index: usize, channels: usize, frames: usize) -> Option<(usize, usize)> {
    if index < channels {
        let start = index * frames;
        Some((start, start + frames))
    } else {
        None
    }
}

/// Packs a channel-major slab into interleaved frame-major `dst`.
/// `dst.len()` must equal `data.len()` (== `channels * frames`).
fn pack_interleaved<S: Sample>(
    data: &[S],
    channels: usize,
    frames: usize,
    dst: &mut [S],
) -> Result<()> {
    if dst.len() != data.len() {
        return Err(Error::InvalidValue("dst"));
    }
    for (frame_index, frame) in dst.chunks_exact_mut(channels).enumerate() {
        for (channel_index, slot) in frame.iter_mut().enumerate() {
            *slot = data[channel_index * frames + frame_index];
        }
    }
    Ok(())
}

/// Unpacks interleaved frame-major `src` into a channel-major slab.
/// `src.len()` must equal `data.len()` (== `channels * frames`).
fn unpack_interleaved<S: Sample>(
    src: &[S],
    channels: usize,
    frames: usize,
    data: &mut [S],
) -> Result<()> {
    if src.len() != data.len() {
        return Err(Error::InvalidValue("src"));
    }
    for (frame_index, frame) in src.chunks_exact(channels).enumerate() {
        for (channel_index, sample) in frame.iter().enumerate() {
            data[channel_index * frames + frame_index] = *sample;
        }
    }
    Ok(())
}

/// Shape check shared by the buffer constructors: returns the slab
/// length `channels * frames`, or why it is not representable. The
/// `isize::MAX` byte bound (the maximum single-allocation size) belongs
/// to the shape check — `vec![S::default(); len]` would otherwise hit
/// the same failure at allocation time.
fn checked_slab_len<S: Sample>(spec: &BufferSpec) -> Result<usize> {
    let len = spec
        .channels()
        .checked_mul(spec.frames)
        .ok_or(Error::InvalidValue("frames"))?;
    len.checked_mul(size_of::<S>())
        .filter(|byte_len| isize::try_from(*byte_len).is_ok())
        .ok_or(Error::InvalidValue("frames"))?;
    Ok(len)
}

/// Geometry of an [`AudioBuffer`]: channel layout (which carries the
/// channel count), frame count and sample rate.
///
/// `frames` is the number of sample instants per channel; the buffer
/// holds `channels * frames` samples in total.
///
/// **RT contract:** RT-suitable — `Copy` value type.
///
/// # Examples
///
/// ```
/// use ruviform_audio_core::{BufferSpec, ChannelLayout, SampleRate};
///
/// let spec = BufferSpec {
///     layout: ChannelLayout::Stereo,
///     frames: 512,
///     sample_rate: SampleRate::new(48_000).unwrap(),
/// };
/// assert_eq!(spec.channels(), 2);
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BufferSpec {
    /// Channel layout — owns the channel count via
    /// [`ChannelLayout::channels`].
    pub layout: ChannelLayout,
    /// Frames (sample instants) per channel.
    pub frames: usize,
    /// Nominal sample rate.
    pub sample_rate: SampleRate,
}

impl BufferSpec {
    /// Channel count implied by [`BufferSpec::layout`].
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn channels(&self) -> usize {
        self.layout.channels() as usize
    }
}

/// Channel-major contiguous planar audio buffer.
///
/// Channel `c` occupies `data[c * frames .. (c + 1) * frames]` of one
/// contiguous `Vec<S>`; `f32` (the default) is the canonical RT sample
/// type. A buffer never grows after construction — there is no
/// `push`/`reserve`-style API. Every layout carries at least one
/// channel — [`ChannelLayout::Custom`] holds a validated non-empty
/// mask set — so a zero-channel buffer cannot be built.
///
/// **RT contract:** the storage for RT paths — accessors and views are
/// allocation-free borrows; the allocating constructors
/// ([`AudioBuffer::new`], [`AudioBuffer::try_new`],
/// [`AudioBuffer::from_interleaved`]) are
/// non-RT-only. See the per-method labels.
///
/// # Examples
///
/// ```
/// use ruviform_audio_core::{AudioBuffer, BufferSpec, ChannelLayout, SampleRate};
///
/// let spec = BufferSpec {
///     layout: ChannelLayout::Stereo,
///     frames: 256,
///     sample_rate: SampleRate::new(48_000).unwrap(),
/// };
/// let mut buffer = AudioBuffer::<f32>::new(spec);
/// assert_eq!(buffer.channel(0), Some(&vec![0.0; 256][..]));
/// buffer.channel_mut(1).unwrap()[0] = 0.5;
/// assert_eq!(buffer.channel(1).unwrap()[0], 0.5);
/// ```
#[derive(Debug)]
pub struct AudioBuffer<S: Sample = f32> {
    data: Vec<S>,
    frames: usize,
    layout: ChannelLayout,
    sample_rate: SampleRate,
}

impl<S: Sample> AudioBuffer<S> {
    /// Allocates a zeroed channel-major planar buffer of
    /// `channels * frames` samples.
    ///
    /// Panicking sibling of [`AudioBuffer::try_new`] — use `try_new` on
    /// paths that must not panic (e.g. format negotiation).
    ///
    /// **RT contract:** non-RT only — allocates.
    ///
    /// # Panics
    ///
    /// Panics when `channels * frames` overflows `usize`, or when the
    /// slab's byte size (`channels * frames * size_of::<S>()`) exceeds
    /// `isize::MAX` — the maximum single-allocation size (same class of
    /// failure as the allocation itself). [`AudioBuffer::try_new`]
    /// surfaces the same shape failures as `Err` instead.
    #[must_use]
    pub fn new(spec: BufferSpec) -> Self {
        Self::try_new(spec)
            .expect("BufferSpec channels * frames overflows usize or exceeds isize::MAX bytes")
    }

    /// Fallible variant of [`AudioBuffer::new`]: the same zeroed
    /// channel-major slab, with the shape validation surfaced as `Err`
    /// instead of a panic — the constructor for paths that must not
    /// panic (e.g. format negotiation).
    ///
    /// **RT contract:** non-RT only — allocates.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] when `channels * frames` overflows
    /// `usize`, or when the slab's byte size
    /// (`channels * frames * size_of::<S>()`) exceeds `isize::MAX` —
    /// the maximum single-allocation size.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_audio_core::{AudioBuffer, BufferSpec, ChannelLayout, SampleRate};
    ///
    /// let spec = BufferSpec {
    ///     layout: ChannelLayout::Stereo,
    ///     frames: 256,
    ///     sample_rate: SampleRate::new(48_000).unwrap(),
    /// };
    /// assert!(AudioBuffer::<f32>::try_new(spec).is_ok());
    /// ```
    pub fn try_new(spec: BufferSpec) -> Result<Self> {
        let len = checked_slab_len::<S>(&spec)?;
        Ok(Self {
            data: vec![S::default(); len],
            frames: spec.frames,
            layout: spec.layout,
            sample_rate: spec.sample_rate,
        })
    }

    /// Allocates a buffer and de-interleaves `src` (frame-major,
    /// `src.len() == channels * frames`) into it.
    ///
    /// **RT contract:** non-RT only — allocates.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] when `src.len() != channels * frames`,
    /// when `channels * frames` overflows `usize`, or when the slab's
    /// byte size exceeds `isize::MAX` (the same shape check as
    /// [`AudioBuffer::try_new`]).
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_audio_core::{AudioBuffer, BufferSpec, ChannelLayout, SampleRate};
    ///
    /// let spec = BufferSpec {
    ///     layout: ChannelLayout::Stereo,
    ///     frames: 2,
    ///     sample_rate: SampleRate::new(48_000).unwrap(),
    /// };
    /// // L R | L R → planar [L L | R R].
    /// let buffer = AudioBuffer::from_interleaved(&[1.0_f32, 10.0, 2.0, 20.0], spec).unwrap();
    /// assert_eq!(buffer.channel(0), Some(&[1.0, 2.0][..]));
    /// assert_eq!(buffer.channel(1), Some(&[10.0, 20.0][..]));
    /// ```
    pub fn from_interleaved(src: &[S], spec: BufferSpec) -> Result<Self> {
        let channels = spec.channels();
        if src.len() != checked_slab_len::<S>(&spec)? {
            return Err(Error::InvalidValue("src"));
        }
        // The slab length was just validated; `try_new` re-runs the
        // same shape check (two integer multiplies) and cannot fail.
        let mut buffer = Self::try_new(spec)?;
        // `src.len()` was just checked against the slab length.
        unpack_interleaved(src, channels, spec.frames, &mut buffer.data)?;
        Ok(buffer)
    }

    /// Channel count — from [`BufferSpec::layout`].
    ///
    /// **RT contract:** RT-suitable — pure, allocation-free.
    #[must_use]
    pub fn channels(&self) -> usize {
        self.layout.channels() as usize
    }

    /// Frames (sample instants) per channel.
    ///
    /// **RT contract:** RT-suitable — pure, allocation-free.
    #[must_use]
    pub fn frames(&self) -> usize {
        self.frames
    }

    /// Nominal sample rate carried by the buffer.
    ///
    /// **RT contract:** RT-suitable — `Copy`, pure.
    #[must_use]
    pub fn sample_rate(&self) -> SampleRate {
        self.sample_rate
    }

    /// Channel layout carried by the buffer.
    ///
    /// **RT contract:** RT-suitable — `Copy`, pure.
    #[must_use]
    pub fn layout(&self) -> ChannelLayout {
        self.layout
    }

    /// The whole channel-major slab (`channels * frames` samples).
    ///
    /// **RT contract:** RT-suitable — borrow only.
    #[must_use]
    pub fn data(&self) -> &[S] {
        self.data.as_slice()
    }

    /// The whole channel-major slab, mutably.
    ///
    /// **RT contract:** RT-suitable — borrow only.
    pub fn data_mut(&mut self) -> &mut [S] {
        self.data.as_mut_slice()
    }

    /// Channel `index` as a contiguous `frames`-long slice — `None` when
    /// out of range.
    ///
    /// **RT contract:** RT-suitable — borrow only, no allocation.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_audio_core::{AudioBuffer, BufferSpec, ChannelLayout, SampleRate};
    ///
    /// let spec = BufferSpec {
    ///     layout: ChannelLayout::Mono,
    ///     frames: 4,
    ///     sample_rate: SampleRate::new(48_000).unwrap(),
    /// };
    /// let buffer = AudioBuffer::<f32>::new(spec);
    /// assert_eq!(buffer.channel(0), Some(&[0.0; 4][..]));
    /// assert_eq!(buffer.channel(1), None);
    /// ```
    #[must_use]
    pub fn channel(&self, index: usize) -> Option<&[S]> {
        channel_range(index, self.channels(), self.frames)
            .map(|(start, end)| &self.data[start..end])
    }

    /// Channel `index` as a contiguous mutable slice — `None` when out of
    /// range.
    ///
    /// **RT contract:** RT-suitable — borrow only, no allocation.
    #[must_use]
    pub fn channel_mut(&mut self, index: usize) -> Option<&mut [S]> {
        channel_range(index, self.channels(), self.frames)
            .map(|(start, end)| &mut self.data[start..end])
    }

    /// Borrowed read view over this buffer.
    ///
    /// **RT contract:** RT-suitable — borrow only, no allocation.
    #[must_use]
    pub fn view(&self) -> AudioBufferView<'_, S> {
        AudioBufferView {
            data: self.data.as_slice(),
            channels: self.channels(),
            frames: self.frames,
        }
    }

    /// Borrowed mutable view over this buffer.
    ///
    /// **RT contract:** RT-suitable — borrow only, no allocation.
    pub fn view_mut(&mut self) -> AudioBufferViewMut<'_, S> {
        let channels = self.channels();
        AudioBufferViewMut {
            data: self.data.as_mut_slice(),
            channels,
            frames: self.frames,
        }
    }

    /// Boundary write: packs the planar buffer into interleaved
    /// frame-major `dst` (`dst.len() == channels * frames` required).
    ///
    /// **RT contract:** RT-suitable — bounded element-wise copy, no
    /// allocation.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] when `dst.len() != channels * frames`.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_audio_core::{AudioBuffer, BufferSpec, ChannelLayout, SampleRate};
    ///
    /// let spec = BufferSpec {
    ///     layout: ChannelLayout::Stereo,
    ///     frames: 2,
    ///     sample_rate: SampleRate::new(48_000).unwrap(),
    /// };
    /// let buffer = AudioBuffer::from_interleaved(&[1.0_f32, 10.0, 2.0, 20.0], spec).unwrap();
    /// let mut dst = [0.0_f32; 4];
    /// buffer.copy_to_interleaved(&mut dst).unwrap();
    /// assert_eq!(dst, [1.0, 10.0, 2.0, 20.0]);
    /// ```
    pub fn copy_to_interleaved(&self, dst: &mut [S]) -> Result<()> {
        self.view().copy_to_interleaved(dst)
    }

    /// Boundary read: de-interleaves frame-major `src` into the planar
    /// buffer (`src.len() == channels * frames` required).
    ///
    /// **RT contract:** RT-suitable — bounded element-wise copy, no
    /// allocation.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] when `src.len() != channels * frames`.
    pub fn copy_from_interleaved(&mut self, src: &[S]) -> Result<()> {
        self.view_mut().copy_from_interleaved(src)
    }
}

/// Borrowed read-only view over planar buffer storage.
///
/// Obtained from [`AudioBuffer::view`]. `Copy` — channel slices borrow
/// the underlying data for `'a`, outliving the view itself.
///
/// **RT contract:** RT-suitable — all operations are allocation-free
/// borrows or bounded copies.
#[derive(Copy, Clone, Debug)]
pub struct AudioBufferView<'a, S: Sample = f32> {
    data: &'a [S],
    channels: usize,
    frames: usize,
}

impl<'a, S: Sample> AudioBufferView<'a, S> {
    /// Channel count of the viewed buffer.
    ///
    /// **RT contract:** RT-suitable — pure, allocation-free.
    #[must_use]
    pub fn channels(&self) -> usize {
        self.channels
    }

    /// Frames (sample instants) per channel.
    ///
    /// **RT contract:** RT-suitable — pure, allocation-free.
    #[must_use]
    pub fn frames(&self) -> usize {
        self.frames
    }

    /// The whole channel-major slab.
    ///
    /// **RT contract:** RT-suitable — borrow only.
    #[must_use]
    pub fn data(&self) -> &'a [S] {
        self.data
    }

    /// Channel `index` as a contiguous `frames`-long slice — `None` when
    /// out of range. The slice borrows the underlying data for `'a`, not
    /// the view.
    ///
    /// **RT contract:** RT-suitable — borrow only, no allocation.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_audio_core::{AudioBuffer, BufferSpec, ChannelLayout, SampleRate};
    ///
    /// let spec = BufferSpec {
    ///     layout: ChannelLayout::Stereo,
    ///     frames: 2,
    ///     sample_rate: SampleRate::new(48_000).unwrap(),
    /// };
    /// let buffer = AudioBuffer::from_interleaved(&[1.0_f32, 10.0, 2.0, 20.0], spec).unwrap();
    /// let view = buffer.view();
    /// let left = view.channel(0).unwrap(); // outlives `view` if needed
    /// assert_eq!(left, &[1.0, 2.0]);
    /// ```
    #[must_use]
    pub fn channel(&self, index: usize) -> Option<&'a [S]> {
        channel_range(index, self.channels, self.frames).map(|(start, end)| &self.data[start..end])
    }

    /// Boundary write: packs the viewed planar data into interleaved
    /// frame-major `dst` (`dst.len() == channels * frames` required).
    ///
    /// **RT contract:** RT-suitable — bounded element-wise copy, no
    /// allocation.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] when `dst.len() != channels * frames`.
    pub fn copy_to_interleaved(&self, dst: &mut [S]) -> Result<()> {
        pack_interleaved(self.data, self.channels, self.frames, dst)
    }
}

/// Borrowed mutable view over planar buffer storage.
///
/// Obtained from [`AudioBuffer::view_mut`]. Not `Copy` — mutable borrows
/// are unique.
///
/// **RT contract:** RT-suitable — all operations are allocation-free
/// borrows or bounded copies.
#[derive(Debug)]
pub struct AudioBufferViewMut<'a, S: Sample = f32> {
    data: &'a mut [S],
    channels: usize,
    frames: usize,
}

impl<S: Sample> AudioBufferViewMut<'_, S> {
    /// Channel count of the viewed buffer.
    ///
    /// **RT contract:** RT-suitable — pure, allocation-free.
    #[must_use]
    pub fn channels(&self) -> usize {
        self.channels
    }

    /// Frames (sample instants) per channel.
    ///
    /// **RT contract:** RT-suitable — pure, allocation-free.
    #[must_use]
    pub fn frames(&self) -> usize {
        self.frames
    }

    /// The whole channel-major slab.
    ///
    /// **RT contract:** RT-suitable — borrow only.
    #[must_use]
    pub fn data(&self) -> &[S] {
        &self.data[..]
    }

    /// The whole channel-major slab, mutably.
    ///
    /// **RT contract:** RT-suitable — borrow only.
    pub fn data_mut(&mut self) -> &mut [S] {
        &mut self.data[..]
    }

    /// Channel `index` as a contiguous `frames`-long slice — `None` when
    /// out of range.
    ///
    /// **RT contract:** RT-suitable — borrow only, no allocation.
    #[must_use]
    pub fn channel(&self, index: usize) -> Option<&[S]> {
        channel_range(index, self.channels, self.frames).map(|(start, end)| &self.data[start..end])
    }

    /// Channel `index` as a contiguous mutable slice — `None` when out of
    /// range.
    ///
    /// **RT contract:** RT-suitable — borrow only, no allocation.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_audio_core::{AudioBuffer, BufferSpec, ChannelLayout, SampleRate};
    ///
    /// let spec = BufferSpec {
    ///     layout: ChannelLayout::Stereo,
    ///     frames: 2,
    ///     sample_rate: SampleRate::new(48_000).unwrap(),
    /// };
    /// let mut buffer = AudioBuffer::<f32>::new(spec);
    /// buffer.view_mut().channel_mut(0).unwrap()[0] = 7.0;
    /// assert_eq!(buffer.channel(0), Some(&[7.0, 0.0][..]));
    /// ```
    #[must_use]
    pub fn channel_mut(&mut self, index: usize) -> Option<&mut [S]> {
        channel_range(index, self.channels, self.frames)
            .map(|(start, end)| &mut self.data[start..end])
    }

    /// Boundary write: packs the viewed planar data into interleaved
    /// frame-major `dst` (`dst.len() == channels * frames` required).
    ///
    /// **RT contract:** RT-suitable — bounded element-wise copy, no
    /// allocation.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] when `dst.len() != channels * frames`.
    pub fn copy_to_interleaved(&self, dst: &mut [S]) -> Result<()> {
        pack_interleaved(&self.data[..], self.channels, self.frames, dst)
    }

    /// Boundary read: de-interleaves frame-major `src` into the viewed
    /// planar storage (`src.len() == channels * frames` required).
    ///
    /// **RT contract:** RT-suitable — bounded element-wise copy, no
    /// allocation.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] when `src.len() != channels * frames`.
    pub fn copy_from_interleaved(&mut self, src: &[S]) -> Result<()> {
        unpack_interleaved(src, self.channels, self.frames, &mut self.data[..])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stereo_spec(frames: usize) -> BufferSpec {
        BufferSpec {
            layout: ChannelLayout::Stereo,
            frames,
            sample_rate: SampleRate::new(48_000).unwrap(),
        }
    }

    #[test]
    fn spec_channels_comes_from_layout() {
        assert_eq!(stereo_spec(10).channels(), 2);
        let mono = BufferSpec {
            layout: ChannelLayout::Mono,
            ..stereo_spec(10)
        };
        assert_eq!(mono.channels(), 1);
    }

    #[test]
    fn new_allocates_zeroed_channel_major_slab() {
        let buffer = AudioBuffer::<f32>::new(stereo_spec(3));
        assert_eq!(buffer.channels(), 2);
        assert_eq!(buffer.frames(), 3);
        assert_eq!(buffer.data(), &[0.0; 6]);
        assert_eq!(buffer.channel(0), Some(&[0.0; 3][..]));
        assert_eq!(buffer.channel(1), Some(&[0.0; 3][..]));
        assert_eq!(buffer.channel(2), None);
        // Channel-major contiguity: channel 1 follows channel 0 directly.
        assert_eq!(&buffer.data()[3..6], buffer.channel(1).unwrap());
    }

    #[test]
    fn from_interleaved_validates_source_length() {
        let spec = stereo_spec(2);
        // len must be exactly channels * frames.
        assert_eq!(
            AudioBuffer::from_interleaved(&[0.0_f32; 3], spec).map(|_| ()),
            Err(Error::InvalidValue("src"))
        );
        assert_eq!(
            AudioBuffer::from_interleaved(&[0.0_f32; 5], spec).map(|_| ()),
            Err(Error::InvalidValue("src"))
        );
        assert!(AudioBuffer::from_interleaved(&[0.0_f32; 4], spec).is_ok());
    }

    #[test]
    fn from_interleaved_places_frame_major_samples() {
        // L R | L R | L R → planar [L L L | R R R].
        let buffer =
            AudioBuffer::from_interleaved(&[1.0_f32, 10.0, 2.0, 20.0, 3.0, 30.0], stereo_spec(3))
                .unwrap();
        assert_eq!(buffer.channel(0), Some(&[1.0, 2.0, 3.0][..]));
        assert_eq!(buffer.channel(1), Some(&[10.0, 20.0, 30.0][..]));
    }

    #[test]
    fn interleaved_copies_validate_slice_length() {
        let mut buffer = AudioBuffer::<f32>::new(stereo_spec(2));
        assert_eq!(
            buffer.copy_from_interleaved(&[0.0; 3]),
            Err(Error::InvalidValue("src"))
        );
        let mut dst = [0.0_f32; 3];
        assert_eq!(
            buffer.copy_to_interleaved(&mut dst),
            Err(Error::InvalidValue("dst"))
        );
        // View variants share the same validation.
        assert_eq!(
            buffer.view().copy_to_interleaved(&mut dst),
            Err(Error::InvalidValue("dst"))
        );
        assert_eq!(
            buffer.view_mut().copy_from_interleaved(&[0.0; 3]),
            Err(Error::InvalidValue("src"))
        );
    }

    #[test]
    fn copy_from_interleaved_updates_planar_slab() {
        let mut buffer = AudioBuffer::<f32>::new(stereo_spec(2));
        buffer
            .copy_from_interleaved(&[7.0_f32, 70.0, 8.0, 80.0])
            .unwrap();
        assert_eq!(buffer.channel(0), Some(&[7.0, 8.0][..]));
        assert_eq!(buffer.channel(1), Some(&[70.0, 80.0][..]));
    }

    #[test]
    fn channel_mut_writes_through() {
        let mut buffer = AudioBuffer::<f32>::new(stereo_spec(2));
        buffer.channel_mut(1).unwrap()[1] = 9.0;
        assert_eq!(buffer.channel(1), Some(&[0.0, 9.0][..]));
        assert!(buffer.channel_mut(2).is_none());
    }

    #[test]
    fn buffer_and_views_expose_spec_and_slab() {
        let spec = stereo_spec(2);
        let mut buffer = AudioBuffer::from_interleaved(&[1.0_f32, 10.0, 2.0, 20.0], spec).unwrap();
        assert_eq!(buffer.sample_rate(), spec.sample_rate);
        assert_eq!(buffer.layout(), spec.layout);

        // `data_mut` writes through to the channel-major slab.
        buffer.data_mut()[2] = 99.0; // first sample of channel 1
        assert_eq!(buffer.channel(1), Some(&[99.0, 20.0][..]));

        // The read view exposes the same slab.
        assert_eq!(buffer.view().data(), &[1.0, 2.0, 99.0, 20.0]);

        {
            let mut view = buffer.view_mut();
            assert_eq!(view.channels(), 2);
            assert_eq!(view.frames(), 2);
            assert_eq!(view.data(), &[1.0, 2.0, 99.0, 20.0]);
            view.data_mut()[0] = 5.0;
            // Planar [5, 2 | 99, 20] packs to frame-major [5, 99, 2, 20].
            let mut dst = [0.0_f32; 4];
            view.copy_to_interleaved(&mut dst).unwrap();
            assert_eq!(dst, [5.0, 99.0, 2.0, 20.0]);
        }
        assert_eq!(buffer.channel(0), Some(&[5.0, 2.0][..]));
    }

    #[test]
    fn f64_buffers_use_the_same_layout() {
        let buffer =
            AudioBuffer::<f64>::from_interleaved(&[1.0_f64, 10.0, 2.0, 20.0], stereo_spec(2))
                .unwrap();
        assert_eq!(buffer.channel(1), Some(&[10.0, 20.0][..]));
    }

    #[test]
    fn empty_specs_are_legal() {
        // Zero frames → zero-length slab; channel access still bounds
        // against the channel count.
        let buffer = AudioBuffer::<f32>::new(stereo_spec(0));
        assert_eq!(buffer.channel(0), Some(&[][..]));
        assert!(buffer.copy_to_interleaved(&mut []).is_ok());
    }

    #[test]
    fn try_new_returns_buffer_for_valid_spec() {
        let buffer = AudioBuffer::<f32>::try_new(stereo_spec(3)).unwrap();
        assert_eq!(buffer.channels(), 2);
        assert_eq!(buffer.frames(), 3);
        assert_eq!(buffer.data(), &[0.0; 6]);
        // Zero frames stays legal — a zero-length slab.
        assert!(AudioBuffer::<f32>::try_new(stereo_spec(0)).is_ok());
    }

    #[test]
    fn try_new_rejects_oversized_shapes() {
        // channels * frames overflows usize.
        assert_eq!(
            AudioBuffer::<f32>::try_new(stereo_spec(usize::MAX)).map(|_| ()),
            Err(Error::InvalidValue("frames"))
        );
        // The slab fits usize but its byte size exceeds isize::MAX.
        assert_eq!(
            AudioBuffer::<f32>::try_new(stereo_spec(isize::MAX as usize)).map(|_| ()),
            Err(Error::InvalidValue("frames"))
        );
    }

    #[test]
    #[should_panic(expected = "BufferSpec channels * frames overflows usize or exceeds isize::MAX")]
    fn new_keeps_its_panic_contract() {
        // Same rejected shape as `try_new` — panic by contract.
        let _ = AudioBuffer::<f32>::new(stereo_spec(usize::MAX));
    }
}
