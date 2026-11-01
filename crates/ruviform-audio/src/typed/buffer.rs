//! Bounded, aligned, prepare-time buffer slots for the typed runtime.
//!
//! Every slot and delay line is allocated once on the non-RT side
//! ([`RtRuntime` construction](crate::typed::RtRuntime)); inside the
//! RT contract interval the arena shape never changes — only
//! per-slot metadata (`channels`, `stamp`, `frames_valid`) is
//! rewritten, so no allocation or reallocation can occur.

/// Default sample alignment in bytes — sized for SIMD-friendly
/// channel bases on mainstream targets.
pub const DEFAULT_ALIGNMENT: usize = 64;

/// A `Vec<f32>`-backed planar sample store whose data slice starts at
/// a requested byte alignment without any `unsafe`: the vector is
/// over-allocated and the aligned start index is computed from the
/// pointer address (index arithmetic only).
#[derive(Debug)]
pub struct AlignedSamples {
    buf: Vec<f32>,
    /// Element offset of the first aligned byte inside `buf`.
    offset: usize,
    /// Requested byte alignment (power of two, `>= 4`).
    align: usize,
    /// Per-channel stride in elements (`frames` padded to `align`).
    stride: usize,
    /// Maximum channel capacity the buffer was sized for.
    channels: usize,
    /// Per-channel frame capacity.
    frames: usize,
}

impl AlignedSamples {
    /// Allocates `channels × frames` planar storage aligned to
    /// `align` bytes. `align` must be a power of two `>= 4`.
    ///
    /// **RT contract:** non-RT only — allocates.
    ///
    /// # Panics
    /// When `align` is not a power of two `>= 4`.
    #[must_use]
    pub fn new(channels: usize, frames: usize, align: usize) -> Self {
        assert!(
            align >= 4 && align.is_power_of_two(),
            "alignment must be a power of two >= 4"
        );
        let align_elems = align / 4;
        let stride = frames.div_ceil(align_elems) * align_elems;
        let extra = align_elems - 1;
        let buf = vec![0.0f32; channels * stride + extra];
        let base = buf.as_ptr() as usize;
        let offset = (base.wrapping_neg() & (align - 1)) / 4;
        Self {
            buf,
            offset,
            align,
            stride,
            channels,
            frames,
        }
    }

    /// The aligned flat region (`channels * stride` elements).
    #[must_use]
    pub fn as_slice(&self) -> &[f32] {
        &self.buf[self.offset..self.offset + self.channels * self.stride]
    }

    /// Mutable view of [`as_slice`](Self::as_slice).
    #[must_use]
    pub fn as_mut_slice(&mut self) -> &mut [f32] {
        let end = self.offset + self.channels * self.stride;
        &mut self.buf[self.offset..end]
    }

    /// The byte alignment of `as_slice().as_ptr()` — always `>=`
    /// the requested `align` by construction.
    #[must_use]
    pub fn alignment(&self) -> usize {
        debug_assert_eq!(self.as_slice().as_ptr() as usize % self.align, 0);
        self.align
    }

    /// Per-channel stride in elements.
    #[must_use]
    pub const fn stride(&self) -> usize {
        self.stride
    }

    /// Channel capacity the buffer was sized for.
    #[must_use]
    pub const fn channels(&self) -> usize {
        self.channels
    }

    /// Per-channel frame capacity.
    #[must_use]
    pub const fn frames(&self) -> usize {
        self.frames
    }

    /// Channel `c`'s full-capacity plane (`stride` elements; the
    /// valid prefix is `frames` long).
    #[must_use]
    pub fn channel(&self, c: usize) -> Option<&[f32]> {
        (c < self.channels).then(|| {
            let start = self.offset + c * self.stride;
            &self.buf[start..start + self.stride]
        })
    }

    /// Mutable view of [`channel`](Self::channel).
    #[must_use]
    pub fn channel_mut(&mut self, c: usize) -> Option<&mut [f32]> {
        if c >= self.channels {
            return None;
        }
        let start = self.offset + c * self.stride;
        Some(&mut self.buf[start..start + self.stride])
    }
}

/// Declared capacity of one arena slot — immutable plan metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlotSpec {
    /// Channels this slot presents for the owning port.
    pub channels: u16,
}

/// One buffer slot: planar channel data plus per-callback metadata.
///
/// `stamp` records the callback sequence number in which the slot was
/// last written; consumers require `stamp == current seq` and treat
/// anything older as an uninitialized read (fault, never UB).
#[derive(Debug)]
pub struct Slot {
    /// Planar data — sized for the arena maximum channel count.
    pub data: AlignedSamples,
    /// Logical channels for the active plan (`<= data.channels()`).
    pub channels: u16,
    /// Frames written this callback (valid prefix of each channel).
    pub frames_valid: u32,
    /// Callback sequence in which this slot was last written.
    stamp: u64,
}

impl Slot {
    /// Allocates a slot for `max_channels × max_frames`.
    ///
    /// **RT contract:** non-RT only — allocates.
    #[must_use]
    pub fn new(max_channels: usize, max_frames: usize, align: usize) -> Self {
        Self {
            data: AlignedSamples::new(max_channels, max_frames, align),
            channels: 0,
            frames_valid: 0,
            stamp: 0,
        }
    }

    /// Callback sequence of the last write.
    #[must_use]
    pub const fn stamp(&self) -> u64 {
        self.stamp
    }

    /// Marks the slot written during callback `seq` with `frames`
    /// valid samples per channel.
    pub fn mark_written(&mut self, seq: u64, frames: u32) {
        self.stamp = seq;
        self.frames_valid = frames;
    }

    /// Zeroes `frames` samples on every logical channel and stamps
    /// the slot for `seq` — used both for deterministic degrade after
    /// an uninitialized-read fault and for delay warm-up.
    pub fn fill_zero(&mut self, seq: u64, frames: usize) {
        for c in 0..usize::from(self.channels) {
            if let Some(plane) = self.data.channel_mut(c) {
                plane[..frames].fill(0.0);
            }
        }
        self.mark_written(seq, u32::try_from(frames).unwrap_or(u32::MAX));
    }

    /// Re-shapes the logical channel count for a new plan
    /// generation. Metadata only — never allocates.
    pub fn configure(&mut self, channels: u16) {
        self.channels = channels;
        self.frames_valid = 0;
    }
}

/// Per-edge PDC ring buffer: `delay + max_frames` per channel so a
/// full callback segment fits without clobbering the read window.
///
/// Write cursors are per-channel and allocated at construction —
/// `configure` only resets them, so RT use never allocates.
#[derive(Debug)]
pub struct DelayLine {
    /// Ring storage — `channels × capacity` planar.
    ring: AlignedSamples,
    /// Per-channel write cursors (built at construction).
    writes: Vec<usize>,
    /// Logical channels for the active plan.
    channels: u16,
    /// Applied delay in frames.
    delay: usize,
    /// Ring capacity per channel (`delay + max_frames` at build).
    capacity: usize,
}

impl DelayLine {
    /// Allocates a ring sized for `max_channels` and
    /// `max_delay + max_frames` frames.
    ///
    /// **RT contract:** non-RT only — allocates.
    #[must_use]
    pub fn new(max_channels: usize, capacity_frames: usize, align: usize) -> Self {
        Self {
            ring: AlignedSamples::new(max_channels, capacity_frames, align),
            writes: vec![0; max_channels],
            channels: 0,
            delay: 0,
            capacity: capacity_frames,
        }
    }

    /// Re-shapes for a new plan generation and clears the ring —
    /// deterministic warm-up (initial silence).
    pub fn configure(&mut self, channels: u16, delay: usize) {
        debug_assert!(delay < self.capacity, "delay exceeds ring capacity");
        self.channels = channels;
        self.delay = delay;
        self.writes.fill(0);
        self.ring.as_mut_slice().fill(0.0);
    }

    /// Applied delay in frames.
    #[must_use]
    pub const fn delay(&self) -> usize {
        self.delay
    }

    /// Streams `src[..frames]` through the ring and *adds* the
    /// signal delayed by [`delay`](Self::delay) into `dst[..frames]`
    /// — the fan-in merge accumulates without a temporary buffer.
    ///
    /// **RT contract:** RT-suitable — bounded copies, no allocation.
    ///
    /// # Panics
    /// When `ch` is outside the configured channel range or `src`/`dst`
    /// are shorter than `frames` — plan/layout invariants established
    /// at activation make both unreachable in a correct schedule.
    pub fn process_add(&mut self, ch: usize, src: &[f32], dst: &mut [f32], frames: usize) {
        debug_assert!(ch < usize::from(self.channels));
        debug_assert!(src.len() >= frames && dst.len() >= frames);
        let ring = self
            .ring
            .channel_mut(ch)
            .expect("channel within configured count");
        let mut write = self.writes[ch];
        for (d, s) in dst[..frames].iter_mut().zip(&src[..frames]) {
            let read = (write + self.capacity - self.delay) % self.capacity;
            *d += ring[read];
            ring[write] = *s;
            write = (write + 1) % self.capacity;
        }
        self.writes[ch] = write;
    }
}
