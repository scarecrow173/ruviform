//! Independent per-channel planar buffer views — zero-copy borrows of
//! *disjoint* channel slice lists supplied at processing boundaries.
//!
//! Unlike [`AudioBuffer`]'s channel-major contiguous slab, these views
//! wrap a *list* of independent per-channel slices — the shape plugin
//! process APIs (CLAP `audio_buffer.data32`, VST3 `channelBuffers32`)
//! and device/host callbacks supply. Wrapping borrows without copying:
//! for the canonical `f32` path a view performs **zero** sample
//! conversions — identical layout in, identical layout out.
//!
//! ## Lifetime contract
//!
//! The lifetime `'a` binds a view to the scope that supplied the
//! channel list — at plugin boundaries that is the process callback,
//! so a view can never outlive the callback that produced it. The
//! borrow checker enforces this (see the `compile_fail` examples); the
//! provider-side contract — the host must not free or reallocate the
//! channel memory while the view is alive — is documented on each
//! constructor.
//!
//! ## Allocation contract
//!
//! Construction borrows a fully-formed channel slice list. The caller
//! (e.g. a plugin adapter) fills a channel list it pre-allocated at
//! activate/prepare time, so building a view never allocates.
//!
//! ## Channel contract
//!
//! - Every channel slice in one view shares a single frame length —
//!   heterogeneous lengths are rejected at construction.
//! - Channel order follows the negotiated bus channel index order.
//! - Zero channels, zero frames and inactive-bus views are legal: the
//!   view then exposes no samples (`channel(i)` is `None`).
//! - Views are never built over null pointers: constructors take valid
//!   `&[S]` / `&mut [S]` slices; boundary code that turns raw pointers
//!   into slices owns that obligation.
//!
//! ## In-place / aliasing rule
//!
//! A [`PlanarBufferViewMut`] may never expose the same memory region as
//! a concurrently-live input view — building a `&[f32]` and a
//! `&mut [f32]` (or two `&mut [f32]`) over one region is undefined
//! behavior, and the safe constructor cannot see raw pointers.
//! Boundary code therefore detects input/output pointer equality *per
//! channel pair* before building views and either routes the pair
//! through a dedicated in-place handling branch or bounces the input
//! through pre-allocated per-instance scratch. A `debug_assert!` in
//! [`PlanarBufferViewMut::new`] additionally rejects duplicated
//! *output* channel regions in debug builds.
//!
//! The chosen in-place strategy is recorded as binding-method ADR
//! input (task T5 of the plugin-foundation plan): bounce-scratch is
//! the uniform safe default for any alias pattern; a dedicated
//! in-place fast path is viable only where the binding classifies
//! in-place pairs itself (e.g. `clack`'s `ChannelPair::InPlace`).

use ruviform_media_core::{Error, Result};

use crate::sample::Sample;

/// Borrowed read view over an independent per-channel planar `S`
/// buffer list — one `&[S]` per channel, no ownership transfer, no
/// copy.
///
/// `S` defaults to the canonical `f32`; the parameter keeps the shape
/// open for the reserved 64-bit sample path (`S = f64`) without a new
/// type.
///
/// **RT contract:** RT-suitable — borrows only; no allocation.
///
/// # Examples
///
/// ```
/// use ruviform_audio_core::PlanarBufferView;
///
/// let left = [0.0_f32; 4];
/// let right = [1.0_f32; 4];
/// let channels = [&left[..], &right[..]];
/// let view = PlanarBufferView::new(&channels).unwrap();
/// assert_eq!(view.channels(), 2);
/// assert_eq!(view.frames(), 4);
/// assert_eq!(view.channel(1), Some(&right[..]));
/// assert_eq!(view.channel(2), None);
/// ```
///
/// The lifetime binds the view to the channel list's scope — a view
/// cannot escape the callback that produced it:
///
/// ```compile_fail
/// use ruviform_audio_core::PlanarBufferView;
///
/// let view = {
///     let data = [0.0_f32; 4];
///     let channels = [&data[..]];
///     PlanarBufferView::new(&channels).unwrap()
///     // `channels` is dropped here — the view may not escape.
/// };
/// let _ = view.channels(); // borrow-escapes-scope: does not compile
/// ```
#[derive(Copy, Clone, Debug)]
pub struct PlanarBufferView<'a, S: Sample = f32> {
    channels: &'a [&'a [S]],
    frames: usize,
}

impl<'a, S: Sample> PlanarBufferView<'a, S> {
    /// Borrows a channel slice list as a view.
    ///
    /// `channels[i]` is channel `i` in bus channel index order; every
    /// slice must share one frame length. An empty list yields a
    /// zero-channel view (legal — inactive buses and zero-frame blocks
    /// are built this way).
    ///
    /// **Provider contract:** the memory behind every channel slice
    /// must stay valid and un-reallocated for `'a` — the caller
    /// guarantees it by building the slices over live host storage.
    ///
    /// **RT contract:** RT-suitable — `O(channels)` bounded validation
    /// scan, allocation-free.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] (`"planar_frames"`) when channel slices
    /// do not share a single frame length.
    pub fn new(channels: &'a [&'a [S]]) -> Result<Self> {
        let frames = channels.first().map_or(0, |c| c.len());
        if channels.iter().any(|c| c.len() != frames) {
            return Err(Error::InvalidValue("planar_frames"));
        }
        Ok(Self { channels, frames })
    }

    /// An empty view — the zero-channel / inactive-bus shape.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            channels: &[],
            frames: 0,
        }
    }

    /// Number of channels in this view.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn channels(&self) -> usize {
        self.channels.len()
    }

    /// Number of frames — the shared length of every channel slice.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn frames(&self) -> usize {
        self.frames
    }

    /// Channel `index` as a slice bound to `'a`, or `None` when out of
    /// range. Every returned slice has [`PlanarBufferView::frames`]
    /// elements.
    ///
    /// **RT contract:** RT-suitable — borrow, no allocation.
    #[must_use]
    pub fn channel(&self, index: usize) -> Option<&'a [S]> {
        self.channels.get(index).copied()
    }

    /// The borrowed channel slice list itself.
    ///
    /// **RT contract:** RT-suitable — `const` borrow, pure.
    #[must_use]
    pub const fn as_slice(&self) -> &'a [&'a [S]] {
        self.channels
    }
}

/// Borrowed write view over an independent per-channel planar `S`
/// buffer list — one `&mut [S]` per channel, no ownership transfer, no
/// copy.
///
/// Carries the same lifetime/allocation/channel contracts as
/// [`PlanarBufferView`]. Construction also checks (debug builds) that
/// no two non-empty output channels share a data pointer — two `&mut`
/// over one region would be instant UB, and debug assertion turns an
/// adapter-side alias bug into a test-visible panic.
///
/// **RT contract:** RT-suitable — borrows only; no allocation.
///
/// # Examples
///
/// ```
/// use ruviform_audio_core::PlanarBufferViewMut;
///
/// let mut out = [0.0_f32; 4];
/// let mut channels = [&mut out[..]];
/// {
///     let mut view = PlanarBufferViewMut::new(&mut channels).unwrap();
///     assert_eq!(view.channels(), 1);
///     view.channel_mut(0).unwrap().fill(2.0);
///     assert_eq!(view.channel(0), Some(&[2.0; 4][..]));
/// }
/// assert_eq!(out, [2.0; 4]);
/// ```
///
/// The lifetime binds the view to the channel list's scope:
///
/// ```compile_fail
/// use ruviform_audio_core::PlanarBufferViewMut;
///
/// let view = {
///     let mut data = [0.0_f32; 4];
///     let mut channels = [&mut data[..]];
///     PlanarBufferViewMut::new(&mut channels).unwrap()
///     // `channels` is dropped here — the view may not escape.
/// };
/// let _ = view.channels(); // borrow-escapes-scope: does not compile
/// ```
#[derive(Debug)]
pub struct PlanarBufferViewMut<'a, S: Sample = f32> {
    channels: &'a mut [&'a mut [S]],
    frames: usize,
}

impl<'a, S: Sample> PlanarBufferViewMut<'a, S> {
    /// Borrows a mutable channel slice list as a view.
    ///
    /// `channels[i]` is channel `i` in bus channel index order; every
    /// slice must share one frame length. An empty list yields a
    /// zero-channel view.
    ///
    /// **Provider contract:** the memory behind every channel slice
    /// must stay valid, un-reallocated and *unaliased* for `'a` — the
    /// caller guarantees all three when it builds the slices.
    ///
    /// **RT contract:** RT-suitable — `O(channels)` bounded validation
    /// scan (plus an `O(channels²)` duplicate-pointer check in debug
    /// builds), allocation-free.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] (`"planar_frames"`) when channel slices
    /// do not share a single frame length.
    pub fn new(channels: &'a mut [&'a mut [S]]) -> Result<Self> {
        let frames = channels.first().map_or(0, |c| c.len());
        if channels.iter().any(|c| c.len() != frames) {
            return Err(Error::InvalidValue("planar_frames"));
        }
        debug_assert!(
            !has_duplicate_channel_region(channels),
            "duplicate output channel region in PlanarBufferViewMut"
        );
        Ok(Self { channels, frames })
    }

    /// An empty view — the zero-channel / inactive-bus shape.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            channels: &mut [],
            frames: 0,
        }
    }

    /// Number of channels in this view.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn channels(&self) -> usize {
        self.channels.len()
    }

    /// Number of frames — the shared length of every channel slice.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn frames(&self) -> usize {
        self.frames
    }

    /// Channel `index` as an immutable slice, or `None` when out of
    /// range.
    ///
    /// **RT contract:** RT-suitable — borrow, no allocation.
    #[must_use]
    pub fn channel(&self, index: usize) -> Option<&[S]> {
        self.channels.get(index).map(|c| &c[..])
    }

    /// Channel `index` as a mutable slice, or `None` when out of
    /// range.
    ///
    /// **RT contract:** RT-suitable — borrow, no allocation.
    pub fn channel_mut(&mut self, index: usize) -> Option<&mut [S]> {
        self.channels.get_mut(index).map(|c| &mut c[..])
    }
}

/// Whether any two *non-empty* channel slices overlap in memory —
/// i.e. the list would expose two `&mut` over one region. Zero-length
/// slices carry dangling pointers and are skipped.
///
/// `O(channels²)` — called from `debug_assert!` only (channel counts
/// are small).
fn has_duplicate_channel_region<S: Sample>(channels: &[&mut [S]]) -> bool {
    fn range<T>(slice: &[T]) -> (usize, usize) {
        let range = slice.as_ptr_range();
        (range.start.addr(), range.end.addr())
    }
    for (i, a) in channels.iter().enumerate() {
        if a.is_empty() {
            continue;
        }
        let (a_start, a_end) = range(a);
        for b in &channels[i + 1..] {
            if b.is_empty() {
                continue;
            }
            let (b_start, b_end) = range(b);
            if a_start < b_end && b_start < a_end {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_view_borrows_independent_channels() {
        let a = [1.0_f32, 2.0, 3.0];
        let b = [4.0_f32, 5.0, 6.0];
        let channels = [&a[..], &b[..]];
        let view = PlanarBufferView::new(&channels).unwrap();
        assert_eq!(view.channels(), 2);
        assert_eq!(view.frames(), 3);
        assert_eq!(view.channel(0), Some(&a[..]));
        assert_eq!(view.channel(1), Some(&b[..]));
        assert_eq!(view.channel(2), None);
        assert_eq!(view.as_slice(), &channels[..]);
    }

    #[test]
    fn read_view_rejects_heterogeneous_frame_lengths() {
        let a = [0.0_f32; 4];
        let b = [0.0_f32; 8];
        let channels = [&a[..], &b[..]];
        assert!(PlanarBufferView::new(&channels).is_err());
    }

    #[test]
    fn empty_views_expose_no_samples() {
        let view = PlanarBufferView::<f32>::empty();
        assert_eq!(view.channels(), 0);
        assert_eq!(view.frames(), 0);
        assert_eq!(view.channel(0), None);

        let data = [0.0_f32; 0];
        let channels = [&data[..]];
        let zero_frame = PlanarBufferView::new(&channels).unwrap();
        assert_eq!(zero_frame.channels(), 1);
        assert_eq!(zero_frame.frames(), 0);
        assert_eq!(zero_frame.channel(0), Some(&[][..]));

        let mut out = PlanarBufferViewMut::<f32>::empty();
        assert_eq!(out.channels(), 0);
        assert_eq!(out.frames(), 0);
        assert!(out.channel_mut(0).is_none());
    }

    #[test]
    fn write_view_borrows_and_writes_independent_channels() {
        let mut a = [0.0_f32; 3];
        let mut b = [0.0_f32; 3];
        let mut channels = [&mut a[..], &mut b[..]];
        {
            let mut view = PlanarBufferViewMut::new(&mut channels).unwrap();
            assert_eq!(view.channels(), 2);
            assert_eq!(view.frames(), 3);
            view.channel_mut(0).unwrap().fill(1.0);
            view.channel_mut(1)
                .unwrap()
                .copy_from_slice(&[7.0, 8.0, 9.0]);
            assert_eq!(view.channel(1), Some(&[7.0, 8.0, 9.0][..]));
            assert_eq!(view.channel(2), None);
        }
        assert_eq!(a, [1.0; 3]);
        assert_eq!(b, [7.0, 8.0, 9.0]);
    }

    #[test]
    fn write_view_rejects_heterogeneous_frame_lengths() {
        let mut a = [0.0_f32; 4];
        let mut b = [0.0_f32; 8];
        let mut channels = [&mut a[..], &mut b[..]];
        assert!(PlanarBufferViewMut::new(&mut channels).is_err());
    }

    /// RT contract: read-view construction and access are bounded
    /// borrows over caller-supplied lists — no allocation on the
    /// process path (`rt_` cannot observe allocation; the contract is
    /// documented + review target).
    #[test]
    fn rt_planar_read_view_construction_and_access() {
        let a = [1.0_f32; 64];
        let b = [2.0_f32; 64];
        let channels = [&a[..], &b[..]];
        let view = PlanarBufferView::new(&channels).unwrap();
        let mut sum = 0.0_f32;
        for c in 0..view.channels() {
            for s in view.channel(c).unwrap() {
                sum += s;
            }
        }
        assert_eq!(sum, 64.0 + 128.0);
    }

    /// RT contract: write-view construction and per-channel writes are
    /// bounded borrows — allocation-free.
    #[test]
    fn rt_planar_write_view_construction_and_access() {
        let mut a = [0.0_f32; 64];
        let mut b = [0.0_f32; 64];
        let mut channels = [&mut a[..], &mut b[..]];
        {
            let mut view = PlanarBufferViewMut::new(&mut channels).unwrap();
            for c in 0..view.channels() {
                if let Some(ch) = view.channel_mut(c) {
                    ch.fill(0.5);
                }
            }
        }
        assert_eq!(a, [0.5; 64]);
        assert_eq!(b, [0.5; 64]);
    }
}
