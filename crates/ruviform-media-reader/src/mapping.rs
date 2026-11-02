//! Frame vocabulary: the source-frame / engine-frame distinction and
//! the caller-supplied mapping contract between them.
//!
//! A *source frame* is one sample instant of a decoded stream —
//! `SourceFrame(0)` is the first decoded sample. An *engine frame* is
//! one sample instant of the caller's processing clock. Clip offset
//! ("where inside the media does this clip begin"), project placement
//! ("where on the timeline does the clip start") and playback-rate
//! ratio together map engine frames onto source frames. Looping and
//! stretch are *caller semantics*: a loop restart is expressed by the
//! caller issuing a new mapping and a [`SourceReader`] seek — the
//! reader never invents media positions.
//!
//! [`SourceReader`]: crate::SourceReader
//!
//! **RT contract:** RT-suitable — all types are `Copy` value types
//! and every method is allocation-free.

/// A decoded-sample index inside one source stream — `SourceFrame(0)`
/// is the stream's first decoded sample.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceFrame(pub u64);

/// A sample index on the caller's processing clock — the engine-side
/// coordinate of [`FrameMap`].
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EngineFrame(pub u64);

/// A half-open span of source frames — `[start, start + frames)`.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct RequestedRange {
    /// First requested source frame.
    pub start: SourceFrame,
    /// Number of source frames requested.
    pub frames: u64,
}

/// The caller-supplied `EngineFrame → SourceFrame` mapping.
///
/// `engine_to_source(e) = clip_offset + (e - placement) * num / den`
/// under exact integer arithmetic:
///
/// * `clip_offset` — signed offset into the source where the clip
///   starts (negative is legal: the clip begins before the first
///   delivered sample; mapped results that land below zero are
///   reported `None`, never clamped).
/// * `placement` — the engine frame at which the clip's first mapped
///   output sits.
/// * `num/den` — playback-rate ratio (`1/1` nominal speed; `den`
///   must be non-zero).
///
/// **RT contract:** RT-suitable — `Copy` value type, `i128` math,
/// no allocation.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct FrameMap {
    /// Signed source-frame offset of the clip start.
    pub clip_offset: i64,
    /// Engine frame at which the clip begins producing mapped source
    /// frames.
    pub placement: i64,
    /// Rate-ratio numerator.
    pub num: u32,
    /// Rate-ratio denominator (non-zero).
    pub den: u32,
}

impl FrameMap {
    /// A 1:1 mapping starting source frame `clip_offset` at engine
    /// frame `placement`.
    ///
    /// **RT contract:** RT-suitable.
    #[must_use]
    pub const fn linear(clip_offset: i64, placement: i64) -> Self {
        Self {
            clip_offset,
            placement,
            num: 1,
            den: 1,
        }
    }

    /// Maps one engine frame to its source frame, or `None` when the
    /// result lies outside the representable source range (below zero
    /// or arithmetic overflow) — out-of-range is reported, never
    /// clamped.
    ///
    /// **RT contract:** RT-suitable — allocation-free.
    #[must_use]
    pub fn engine_to_source(&self, engine: EngineFrame) -> Option<SourceFrame> {
        let rel = i128::from(engine.0) - i128::from(self.placement);
        let mapped =
            i128::from(self.clip_offset) + rel * i128::from(self.num) / i128::from(self.den);
        u64::try_from(mapped).ok().map(SourceFrame)
    }

    /// The half-open source range covering `frames` engine frames
    /// starting at `engine`. `None` when the range start is not
    /// representable (same rule as [`engine_to_source`](Self::engine_to_source)).
    ///
    /// **RT contract:** RT-suitable — allocation-free.
    #[must_use]
    pub fn source_range(&self, engine: EngineFrame, frames: u64) -> Option<RequestedRange> {
        let start = self.engine_to_source(engine)?;
        let end = self.engine_to_source(EngineFrame(engine.0 + frames))?;
        Some(RequestedRange {
            start,
            frames: end.0 - start.0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_map_applies_clip_offset_and_placement() {
        let map = FrameMap::linear(1024, 2048);
        assert_eq!(
            map.engine_to_source(EngineFrame(2048)),
            Some(SourceFrame(1024))
        );
        assert_eq!(
            map.engine_to_source(EngineFrame(2560)),
            Some(SourceFrame(1536))
        );
        // Before placement the map still resolves — the clip's bounds
        // live on the caller's timeline, not inside the mapping.
        assert_eq!(
            map.engine_to_source(EngineFrame(1024)),
            Some(SourceFrame(0))
        );
        // Mapped below source frame 0 → unrepresentable → None.
        assert_eq!(map.engine_to_source(EngineFrame(1023)), None);
    }

    #[test]
    fn ratio_maps_engine_frames() {
        let map = FrameMap {
            clip_offset: 0,
            placement: 0,
            num: 1,
            den: 2,
        };
        assert_eq!(
            map.engine_to_source(EngineFrame(100)),
            Some(SourceFrame(50))
        );
    }

    #[test]
    fn source_range_computes_span() {
        let map = FrameMap::linear(10, 0);
        let range = map.source_range(EngineFrame(0), 8).unwrap();
        assert_eq!(range.start, SourceFrame(10));
        assert_eq!(range.frames, 8);
    }
}
