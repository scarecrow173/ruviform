//! The evaluation projection — [`Timeline::evaluate`] and the
//! [`Evaluation`]/[`ActiveClip`] read-only snapshots it returns
//! (`docs/designs/edit-data-model.md` "評価").
//!
//! `evaluate(at)` projects the stored model into engine-neutral data:
//! at most one [`ActiveClip`] per track — the winner of the defined
//! overlap resolution — carrying the residual reachable source span.
//! Gaps and disabled tracks contribute no element; generating
//! silence/black for them is the consumer's job (ADR D3-a: the
//! projection stays neutral data — no engine types appear here).
//!
//! The scan is O(tracks × clips-per-track): a plain linear sweep with
//! no interval index, per the design's Non-Functional Constraints —
//! an index stays deferred until a large-timeline need is proven.
//! Rounding sites on this path are fixed: `at` converts into
//! `timeline.base()` once ([`Rounding::NearestEven`]), and each
//! [`MediaTime::checked_add`] applies at most one conversion into the
//! source base (two calls — `source_time` and the residual end).

use ruviform_media_core::{MediaDuration, MediaRange, MediaTime, Rounding};

use crate::error::{Error, Result};
use crate::model::{ClipAttributes, ClipId, ClipSource, Timeline, TrackId, TrackKind};

/// One track's winning clip at the evaluated position — a read-only
/// snapshot.
///
/// Plain data with public fields (no invariant held). `kind` is the
/// track kind so the consumer can dispatch to the right engine;
/// `source_range` is the residual reachable source span
/// `[source_time, min(out, in + range.duration()))` expressed in the
/// source-specific base — the tail of a `source_range` longer than
/// the occupied `range` is not reported, because the 1:1 mapping
/// cannot reach it. The residual may be **empty (`end == start`)**
/// when the remaining reachable span rounds below one source tick:
/// `source_time` and the residual end are produced by independent
/// `NearestEven` conversions into the source base and can collide.
/// Consumers must tolerate a zero-duration `source_range`.
///
/// **RT contract:** non-RT only — `Copy` value type.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ActiveClip {
    /// The track the winning clip lives on.
    pub track: TrackId,
    /// The winning clip's [`Timeline`]-minted identifier.
    pub clip: ClipId,
    /// The track kind — the consumer's engine-dispatch key.
    pub kind: TrackKind,
    /// The opaque source handle — the model never resolves it.
    pub source: ClipSource,
    /// The residual reachable source span `[source_time, end)` in the
    /// source-specific base — possibly empty (`end == start`) when
    /// the reachable span rounds below one source tick; consumers
    /// must tolerate zero duration.
    pub source_range: MediaRange,
    /// The stored per-kind attribute snapshot.
    pub attributes: ClipAttributes,
}

/// The projection result for position `at` — a read-only snapshot.
///
/// Plain data with public fields (no invariant held). `active` holds
/// at most one [`ActiveClip`] per track, in track insertion order —
/// the same order [`Timeline::tracks`] reports, which consumers may
/// read as the neutral stacking order for video.
///
/// **RT contract:** non-RT only — owns a `Vec`.
#[derive(Clone, Debug, PartialEq)]
pub struct Evaluation {
    /// The evaluated position, normalized into `timeline.base()` —
    /// it may differ from the `evaluate` argument by at most half a
    /// tick after the `NearestEven` conversion.
    pub at: MediaTime,
    /// Per-track winners in track insertion order; gaps and disabled
    /// tracks contribute no element.
    pub active: Vec<ActiveClip>,
}

impl Timeline {
    /// Projects the timeline at `at` into an [`Evaluation`].
    ///
    /// `at` converts into `timeline.base()` once
    /// ([`Rounding::NearestEven`]); every later comparison and offset
    /// runs on same-base `i64` values. For each enabled track, the
    /// **last** clip whose half-open `range`
    /// (`start <= at < start + duration`) contains `at` wins: the
    /// sorted clip order — `(range.start(), ClipId)`, and `ClipId`
    /// grows with insertion order — makes that the latest start, and
    /// a start tie resolves to the later insertion — the defined
    /// overlap resolution.
    ///
    /// The winner maps into source time as
    /// `source_time = source_range.start() + (at - range.start())`
    /// via [`MediaTime::checked_add`] (one `NearestEven` conversion
    /// into the source base), and the emitted
    /// [`ActiveClip::source_range`] is the residual reachable span
    /// `[source_time, min(out, in + range.duration()))`. Because
    /// `source_time` and the residual end come from independent
    /// `NearestEven` conversions, the residual can collapse to an
    /// empty range (`end == start`) when less than one source tick
    /// remains reachable; consumers must tolerate that zero-duration
    /// result. The total cost is O(tracks × clips-per-track): a
    /// linear scan.
    ///
    /// **RT contract:** non-RT only — allocates the `active` vector.
    ///
    /// # Errors
    ///
    /// [`Error::TimeOverflow`] when the `at` conversion or any checked
    /// offset/residual arithmetic overflows the `i64` range.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_edit::{
    ///     ClipAttributes, ClipSource, ClipSpec, Timeline, TrackKind,
    /// };
    /// use ruviform_media_core::{
    ///     MediaDuration, MediaRange, MediaTime, TimeBase,
    /// };
    ///
    /// let base = TimeBase::COMMON_90K;
    /// let mut timeline = Timeline::new(base);
    /// let track = timeline.add_track(TrackKind::Audio);
    /// timeline
    ///     .insert_clip(
    ///         track,
    ///         ClipSpec {
    ///             source: ClipSource(7),
    ///             range: MediaRange::new(
    ///                 MediaTime::new(90_000, base),
    ///                 MediaDuration::new(90_000, base),
    ///             ),
    ///             source_range: MediaRange::new(
    ///                 MediaTime::new(0, base),
    ///                 MediaDuration::new(90_000, base),
    ///             ),
    ///             attributes: ClipAttributes::Audio {
    ///                 gain: 1.0,
    ///                 mute: false,
    ///             },
    ///         },
    ///     )
    ///     .unwrap();
    ///
    /// // Half a second into the clip -> source_time 45_000, residual
    /// // reaching the clip's source end.
    /// let eval = timeline.evaluate(MediaTime::new(135_000, base)).unwrap();
    /// assert_eq!(eval.active.len(), 1);
    /// let active = eval.active[0];
    /// assert_eq!(active.track, track);
    /// assert_eq!(active.kind, TrackKind::Audio);
    /// assert_eq!(
    ///     active.source_range.start(),
    ///     MediaTime::new(45_000, base)
    /// );
    /// assert_eq!(active.source_range.end(), MediaTime::new(90_000, base));
    ///
    /// // A gap position contributes no element.
    /// assert!(
    ///     timeline
    ///         .evaluate(MediaTime::new(0, base))
    ///         .unwrap()
    ///         .active
    ///         .is_empty()
    /// );
    /// ```
    pub fn evaluate(&self, at: MediaTime) -> Result<Evaluation> {
        // The single conversion on this path: every comparison below
        // runs on same-base i64 values (stored `range`s share
        // `timeline.base()` by invariant).
        let at = at.convert(self.base(), Rounding::NearestEven)?;
        let mut active = Vec::with_capacity(self.tracks().len());
        for track in self.tracks() {
            if !track.enabled() {
                continue;
            }
            // The last containing clip in track order is the defined
            // winner: the `(range.start(), ClipId)` sort puts the
            // latest start last, and a start tie resolves to the
            // later-minted (later-inserted) clip. A gap emits no
            // element.
            let Some(winner) = track.clips().iter().rev().find(|clip| {
                clip.range().start().value() <= at.value()
                    && at.value() < clip.range().end().value()
            }) else {
                continue;
            };
            // `at >= range.start()` is guaranteed by the containment
            // test; `checked_sub` only guards the theoretical full-
            // width case per the documented path.
            let offset_ticks = at
                .value()
                .checked_sub(winner.range().start().value())
                .ok_or(Error::TimeOverflow)?;
            let offset = MediaDuration::new(offset_ticks, self.base());
            let source = winner.source_range();
            let source_time = source.start().checked_add(offset)?;
            // The furthest reachable source instant under the 1:1
            // mapping is `in + range.duration()`; `out` bounds it from
            // above. `reachable <= out` is already provable — the
            // invariant `range.duration() <= source_range.duration()`
            // is an exact cross-base comparison and `NearestEven`
            // conversion is monotonic — so `min` is a defensive clamp
            // kept for robustness, not a required bound.
            // `source.end()` reproduces the `checked_end` value
            // validation established — the stored invariants keep it
            // exact and `source_time <= end`.
            let reachable = source.start().checked_add(winner.range().duration())?;
            let end = reachable.min(source.end());
            let residual = end
                .value()
                .checked_sub(source_time.value())
                .ok_or(Error::TimeOverflow)?;
            active.push(ActiveClip {
                track: track.id(),
                clip: winner.id(),
                kind: track.kind(),
                source: winner.source(),
                source_range: MediaRange::new(
                    source_time,
                    MediaDuration::new(residual, source_time.base()),
                ),
                attributes: winner.attributes(),
            });
        }
        Ok(Evaluation { at, active })
    }
}

#[cfg(test)]
mod tests {
    use ruviform_media_core::TimeBase;

    use super::*;
    use crate::ClipSpec;

    fn base() -> TimeBase {
        TimeBase::COMMON_90K
    }

    fn time(value: i64) -> MediaTime {
        MediaTime::new(value, base())
    }

    fn range(start: i64, duration: i64) -> MediaRange {
        MediaRange::new(time(start), MediaDuration::new(duration, base()))
    }

    fn audio_clip(start: i64, duration: i64, source_in: i64, source_duration: i64) -> ClipSpec {
        ClipSpec {
            source: ClipSource(1),
            range: range(start, duration),
            source_range: range(source_in, source_duration),
            attributes: ClipAttributes::Audio {
                gain: 1.0,
                mute: false,
            },
        }
    }

    fn video_clip(start: i64, duration: i64, source_in: i64, source_duration: i64) -> ClipSpec {
        ClipSpec {
            source: ClipSource(1),
            range: range(start, duration),
            source_range: range(source_in, source_duration),
            attributes: ClipAttributes::Video { opacity: 1.0 },
        }
    }

    #[test]
    fn empty_timeline_evaluates_to_no_active_clips() {
        let timeline = Timeline::new(base());
        let eval = timeline.evaluate(time(1_000)).unwrap();
        assert_eq!(eval.at, time(1_000));
        assert_eq!(eval.active, Vec::new());
        // A track holding no clips is a gap, not an element.
        let mut timeline = Timeline::new(base());
        let _track = timeline.add_track(TrackKind::Audio);
        assert_eq!(timeline.evaluate(time(1_000)).unwrap().active, []);
    }

    #[test]
    fn gap_positions_and_half_open_bounds_emit_nothing() {
        let mut timeline = Timeline::new(base());
        let track = timeline.add_track(TrackKind::Audio);
        timeline
            .insert_clip(track, audio_clip(10, 10, 0, 10))
            .unwrap(); // [10, 20)
        assert_eq!(timeline.evaluate(time(9)).unwrap().active, []);
        assert_eq!(timeline.evaluate(time(10)).unwrap().active.len(), 1);
        assert_eq!(timeline.evaluate(time(19)).unwrap().active.len(), 1);
        assert_eq!(timeline.evaluate(time(20)).unwrap().active, []);
        assert_eq!(timeline.evaluate(time(-5)).unwrap().active, []);
    }

    #[test]
    fn overlap_resolves_to_the_latest_start() {
        let mut timeline = Timeline::new(base());
        let track = timeline.add_track(TrackKind::Audio);
        let wide = timeline
            .insert_clip(track, audio_clip(0, 30, 0, 30))
            .unwrap();
        let late = timeline
            .insert_clip(track, audio_clip(10, 10, 0, 10))
            .unwrap();
        // Both contain 15 — the later start wins.
        assert_eq!(timeline.evaluate(time(15)).unwrap().active[0].clip, late);
        // Only `wide` contains 5 — a later-start clip that does not
        // contain `at` cannot win.
        assert_eq!(timeline.evaluate(time(5)).unwrap().active[0].clip, wide);
    }

    #[test]
    fn overlap_on_equal_start_resolves_to_later_insertion() {
        let mut timeline = Timeline::new(base());
        let track = timeline.add_track(TrackKind::Audio);
        timeline
            .insert_clip(track, audio_clip(10, 10, 0, 10))
            .unwrap();
        let later = timeline
            .insert_clip(track, audio_clip(10, 20, 0, 20))
            .unwrap();
        assert_eq!(timeline.evaluate(time(15)).unwrap().active[0].clip, later);
    }

    #[test]
    fn move_clip_onto_equal_start_resolves_to_later_insertion() {
        // The `(range.start(), ClipId)` sort key makes the tie-break
        // uniform: whichever clip moved, the later insertion (the
        // larger `ClipId`) wins — a start-only stable sort would keep
        // the moved clip's stale vec position instead.
        let mut timeline = Timeline::new(base());
        let track = timeline.add_track(TrackKind::Audio);
        // Insert the t=20 clip first: the sorted vec is
        // [later@10, first@20], so a start-only stable sort would
        // leave the moved clip ahead of the earlier-inserted one.
        let first = timeline
            .insert_clip(track, audio_clip(20, 10, 0, 10))
            .unwrap();
        let later = timeline
            .insert_clip(track, audio_clip(10, 10, 0, 10))
            .unwrap();
        timeline.move_clip(track, later, time(20)).unwrap();
        let clips = timeline.track(track).unwrap().clips();
        assert_eq!(
            clips.iter().map(|clip| clip.id()).collect::<Vec<_>>(),
            [first, later]
        );
        assert_eq!(timeline.evaluate(time(25)).unwrap().active[0].clip, later);

        // Reverse direction: the earlier-inserted clip sits later in
        // the vec (it had the larger start) — moving it onto the same
        // start must not make it win.
        let mut timeline = Timeline::new(base());
        let track = timeline.add_track(TrackKind::Audio);
        let first = timeline
            .insert_clip(track, audio_clip(30, 10, 0, 10))
            .unwrap();
        let later = timeline
            .insert_clip(track, audio_clip(10, 10, 0, 10))
            .unwrap();
        timeline.move_clip(track, first, time(10)).unwrap();
        let clips = timeline.track(track).unwrap().clips();
        assert_eq!(
            clips.iter().map(|clip| clip.id()).collect::<Vec<_>>(),
            [first, later]
        );
        assert_eq!(timeline.evaluate(time(15)).unwrap().active[0].clip, later);
    }

    #[test]
    fn trim_clip_onto_equal_start_resolves_to_later_insertion() {
        // The same tie-break through `trim_clip`: re-specifying the
        // start onto an existing start keeps "later insertion wins"
        // in both directions.
        let mut timeline = Timeline::new(base());
        let track = timeline.add_track(TrackKind::Audio);
        let first = timeline
            .insert_clip(track, audio_clip(20, 10, 0, 10))
            .unwrap();
        let later = timeline
            .insert_clip(track, audio_clip(10, 10, 0, 10))
            .unwrap();
        timeline
            .trim_clip(track, later, range(20, 10), range(0, 10))
            .unwrap();
        let clips = timeline.track(track).unwrap().clips();
        assert_eq!(
            clips.iter().map(|clip| clip.id()).collect::<Vec<_>>(),
            [first, later]
        );
        assert_eq!(timeline.evaluate(time(25)).unwrap().active[0].clip, later);

        // The earlier-inserted clip trimmed onto a later insertion's
        // start stays the loser.
        let mut timeline = Timeline::new(base());
        let track = timeline.add_track(TrackKind::Audio);
        let first = timeline
            .insert_clip(track, audio_clip(30, 10, 0, 10))
            .unwrap();
        let later = timeline
            .insert_clip(track, audio_clip(10, 10, 0, 10))
            .unwrap();
        timeline
            .trim_clip(track, first, range(10, 10), range(0, 10))
            .unwrap();
        let clips = timeline.track(track).unwrap().clips();
        assert_eq!(
            clips.iter().map(|clip| clip.id()).collect::<Vec<_>>(),
            [first, later]
        );
        assert_eq!(timeline.evaluate(time(15)).unwrap().active[0].clip, later);
    }

    #[test]
    fn residual_may_collapse_to_an_empty_range() {
        // Pinning the documented edge: `source_time` and the residual
        // end are independent `NearestEven` conversions into the
        // source base; when less than one source tick remains
        // reachable they collide and the emitted `source_range` is
        // empty (`end == start`). Consumers must tolerate that.
        let fine = TimeBase::new(1, 48_000).unwrap();
        let mut timeline = Timeline::new(base());
        let track = timeline.add_track(TrackKind::Audio);
        timeline
            .insert_clip(
                track,
                ClipSpec {
                    source: ClipSource(1),
                    range: range(0, 4), // [0, 4) @90k
                    source_range: MediaRange::new(
                        MediaTime::new(0, fine),
                        MediaDuration::new(3, fine), // [0, 3) @48k
                    ),
                    attributes: ClipAttributes::Audio {
                        gain: 1.0,
                        mute: false,
                    },
                },
            )
            .unwrap();
        // at = 3 ticks @90k -> offset 3/90000 s -> 1.6 -> 2 ticks
        // @48k; reachable = 4/90000 s -> 2.13 -> 2 ticks @48k;
        // end = min(2, out 3) = 2 -> residual [2, 2).
        let active = timeline.evaluate(time(3)).unwrap().active[0];
        assert_eq!(active.source_range.start(), MediaTime::new(2, fine));
        assert_eq!(active.source_range.end(), MediaTime::new(2, fine));
        assert_eq!(active.source_range.duration().value(), 0);
    }

    #[test]
    fn disabled_track_contributes_no_element() {
        let mut timeline = Timeline::new(base());
        let audio = timeline.add_track(TrackKind::Audio);
        let video = timeline.add_track(TrackKind::Video);
        timeline
            .insert_clip(audio, audio_clip(0, 30, 0, 30))
            .unwrap();
        timeline
            .insert_clip(video, video_clip(0, 30, 0, 30))
            .unwrap();
        timeline.set_track_enabled(audio, false).unwrap();
        let eval = timeline.evaluate(time(5)).unwrap();
        assert_eq!(eval.active.len(), 1);
        assert_eq!(eval.active[0].track, video);
    }

    #[test]
    fn source_time_maps_through_nonzero_in_and_residual_clamps() {
        // The test-spec anchor case: range [10, 20), source_range
        // [100, 200), at 15 -> source_time 105, residual
        // [105, min(200, 100 + 10)) = [105, 110).
        let mut timeline = Timeline::new(base());
        let track = timeline.add_track(TrackKind::Audio);
        timeline
            .insert_clip(track, audio_clip(10, 10, 100, 100))
            .unwrap();
        let active = timeline.evaluate(time(15)).unwrap().active[0];
        assert_eq!(active.source_range.start(), time(105));
        assert_eq!(active.source_range.end(), time(110));

        // An equal-length source selection: the residual reaches `out`.
        let mut timeline = Timeline::new(base());
        let track = timeline.add_track(TrackKind::Audio);
        timeline
            .insert_clip(track, audio_clip(10, 10, 100, 10))
            .unwrap();
        let active = timeline.evaluate(time(15)).unwrap().active[0];
        assert_eq!(active.source_range.start(), time(105));
        assert_eq!(active.source_range.end(), time(110));

        // At the clip's last tick the residual shrinks to one tick.
        let active = timeline.evaluate(time(19)).unwrap().active[0];
        assert_eq!(active.source_range.start(), time(109));
        assert_eq!(active.source_range.end(), time(110));
    }

    #[test]
    fn source_mapping_respects_the_source_base() {
        // Source selection on a 1/48_000 base: offset and residual are
        // converted into the source base, and the residual end clamps
        // to `in + range.duration()` instead of `out`.
        let fine = TimeBase::new(1, 48_000).unwrap();
        let mut timeline = Timeline::new(base());
        let track = timeline.add_track(TrackKind::Audio);
        let mut spec = audio_clip(90_000, 90_000, 0, 90_000);
        spec.source_range = MediaRange::new(
            MediaTime::new(4_800, fine),
            MediaDuration::new(96_000, fine),
        );
        timeline.insert_clip(track, spec).unwrap();
        // at = 1.5 s -> offset 0.5 s -> source_time = 4_800 + 24_000.
        let active = timeline.evaluate(time(135_000)).unwrap().active[0];
        assert_eq!(active.source_range.start(), MediaTime::new(28_800, fine));
        // reachable = 4_800 + 48_000 < out = 100_800 -> residual to 52_800.
        assert_eq!(active.source_range.end(), MediaTime::new(52_800, fine));
        assert_eq!(active.source_range.start().base(), fine);
    }

    #[test]
    fn cross_base_at_converts_once_into_timeline_base() {
        let mut timeline = Timeline::new(base());
        let track = timeline.add_track(TrackKind::Audio);
        let clip = timeline
            .insert_clip(track, audio_clip(90_000, 90_000, 0, 90_000))
            .unwrap();
        // 1.5 s expressed on a half-second base: `at` is stored in the
        // evaluation normalized into the timeline base.
        let half_seconds = TimeBase::new(1, 2).unwrap();
        let eval = timeline.evaluate(MediaTime::new(3, half_seconds)).unwrap();
        assert_eq!(eval.at, time(135_000));
        assert_eq!(eval.at.base(), base());
        assert_eq!(eval.active[0].clip, clip);
        assert_eq!(eval.active[0].source_range.start(), time(45_000));
    }

    #[test]
    fn at_conversion_overflow_returns_time_overflow() {
        let mut timeline = Timeline::new(base());
        let _track = timeline.add_track(TrackKind::Audio);
        // i64::MAX seconds cannot be expressed in 1/90_000 ticks.
        let huge = MediaTime::new(i64::MAX, TimeBase::new(1, 1).unwrap());
        assert_eq!(timeline.evaluate(huge), Err(Error::TimeOverflow));
    }

    #[test]
    fn active_clips_follow_track_insertion_order() {
        let mut timeline = Timeline::new(base());
        let video = timeline.add_track(TrackKind::Video);
        let audio = timeline.add_track(TrackKind::Audio);
        let video_clip_id = timeline
            .insert_clip(video, video_clip(0, 30, 5, 30))
            .unwrap();
        let mut spec = audio_clip(0, 30, 5, 30);
        spec.source = ClipSource(42);
        spec.attributes = ClipAttributes::Audio {
            gain: 2.5,
            mute: true,
        };
        let audio_clip_id = timeline.insert_clip(audio, spec).unwrap();

        let eval = timeline.evaluate(time(10)).unwrap();
        assert_eq!(eval.active.len(), 2);
        // Track insertion order — not track kind or clip order — wins.
        assert_eq!(eval.active[0].track, video);
        assert_eq!(eval.active[1].track, audio);

        let video_active = eval.active[0];
        assert_eq!(video_active.clip, video_clip_id);
        assert_eq!(video_active.kind, TrackKind::Video);

        let audio_active = eval.active[1];
        assert_eq!(audio_active.clip, audio_clip_id);
        assert_eq!(audio_active.kind, TrackKind::Audio);
        assert_eq!(audio_active.source, ClipSource(42));
        assert_eq!(
            audio_active.attributes,
            ClipAttributes::Audio {
                gain: 2.5,
                mute: true
            }
        );
        // offset = 10 - 0 -> source_time = 5 + 10.
        assert_eq!(audio_active.source_range.start(), time(15));
        assert_eq!(audio_active.source_range.end(), time(35));
    }

    #[test]
    fn per_track_gaps_are_independent() {
        let mut timeline = Timeline::new(base());
        let video = timeline.add_track(TrackKind::Video);
        let audio = timeline.add_track(TrackKind::Audio);
        timeline
            .insert_clip(video, video_clip(0, 30, 0, 30))
            .unwrap();
        timeline
            .insert_clip(audio, audio_clip(100, 10, 0, 10))
            .unwrap();
        let eval = timeline.evaluate(time(10)).unwrap();
        assert_eq!(eval.active.len(), 1);
        assert_eq!(eval.active[0].track, video);
    }

    #[test]
    fn removed_clip_and_removed_track_contribute_nothing() {
        let mut timeline = Timeline::new(base());
        let track = timeline.add_track(TrackKind::Audio);
        let clip = timeline
            .insert_clip(track, audio_clip(0, 30, 0, 30))
            .unwrap();
        timeline.remove_clip(track, clip).unwrap();
        assert_eq!(timeline.evaluate(time(10)).unwrap().active, []);
        let clip = timeline
            .insert_clip(track, audio_clip(0, 30, 0, 30))
            .unwrap();
        assert_eq!(timeline.evaluate(time(10)).unwrap().active[0].clip, clip);
        timeline.remove_track(track).unwrap();
        assert_eq!(timeline.evaluate(time(10)).unwrap().active, []);
    }

    #[test]
    fn evaluation_is_deterministic() {
        let mut timeline = Timeline::new(base());
        let track = timeline.add_track(TrackKind::Audio);
        timeline
            .insert_clip(track, audio_clip(0, 30, 0, 30))
            .unwrap();
        timeline
            .insert_clip(track, audio_clip(10, 10, 0, 10))
            .unwrap();
        let first = timeline.evaluate(time(15)).unwrap();
        let second = timeline.evaluate(time(15)).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn foreign_base_clip_evaluates_in_timeline_base() {
        // The stored `range` was normalized at insertion; evaluation
        // still maps into the source-specific base correctly.
        let fine = TimeBase::new(1, 48_000).unwrap();
        let mut timeline = Timeline::new(base());
        let track = timeline.add_track(TrackKind::Audio);
        let clip = timeline
            .insert_clip(
                track,
                ClipSpec {
                    source: ClipSource(1),
                    range: MediaRange::new(
                        MediaTime::new(48_000, fine),
                        MediaDuration::new(48_000, fine),
                    ),
                    source_range: MediaRange::new(
                        MediaTime::new(0, fine),
                        MediaDuration::new(48_000, fine),
                    ),
                    attributes: ClipAttributes::Audio {
                        gain: 1.0,
                        mute: false,
                    },
                },
            )
            .unwrap();
        let eval = timeline.evaluate(time(135_000)).unwrap();
        assert_eq!(eval.active[0].clip, clip);
        // offset 45_000 timeline ticks = 0.5 s = 24_000 source ticks.
        assert_eq!(
            eval.active[0].source_range.start(),
            MediaTime::new(24_000, fine)
        );
        assert_eq!(
            eval.active[0].source_range.end(),
            MediaTime::new(48_000, fine)
        );
    }
}
