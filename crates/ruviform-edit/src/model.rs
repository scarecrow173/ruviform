//! The stored edit model — [`Timeline`], [`Track`], [`Clip`] — plus
//! the [`ClipSpec`] input bundle, the ID newtypes and the
//! kind/attribute enums.
//!
//! Invariants owned here (re-established by every clip-storing
//! mutation; `docs/designs/edit-data-model.md` "検証規則"):
//!
//! - every stored `Clip::range` is expressed in `timeline.base()` —
//!   mutations normalize `start`/`duration` via
//!   [`Rounding::NearestEven`] conversion at the boundary, while
//!   `source_range` keeps its source-specific base;
//! - `Track::clips` stays sorted by `(range.start(), ClipId)` —
//!   `ClipId` is minted monotonically in insertion order, so a
//!   start-position tie keeps insertion order;
//! - both `range` and `source_range` have positive duration, a
//!   resolvable `end` and `end > start` (no base-collapse);
//! - `range.duration() <= source_range.duration()` (exact cross-base
//!   comparison — the 1:1 mapping never overshoots `out`);
//! - the `ClipAttributes` variant matches `Track::kind` and
//!   `gain`/`opacity` stay in range.
//!
//! `TrackId`/`ClipId` are minted by [`Timeline`] as monotonically
//! increasing `u64`s — numbers are never reused, so a stale ID simply
//! fails lookup. Generational handles are deliberately not used
//! (`PluginHandle`-only convention; over-engineering otherwise).

use ruviform_media_core::{MediaRange, MediaTime, Rounding, TimeBase};

use crate::error::{Error, Result};

/// Identifier of a [`Track`], minted by [`Timeline`] — monotonically
/// increasing, never reused (a stale ID fails lookup as
/// [`Error::TrackNotFound`] / `None`).
///
/// **RT contract:** non-RT only — `Copy` value type.
///
/// # Examples
///
/// ```
/// use ruviform_edit::TrackId;
///
/// let id = TrackId(7);
/// assert_eq!(id.0, 7);
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TrackId(pub u64);

/// Identifier of a [`Clip`], minted by [`Timeline`] — same contract as
/// [`TrackId`] (monotonic, never reused, stale IDs fail lookup as
/// [`Error::ClipNotFound`] / `None`).
///
/// **RT contract:** non-RT only — `Copy` value type.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ClipId(pub u64);

/// Opaque reference to source media — an external key the application
/// maps onto its media-io/asset registry. The model never resolves or
/// validates the referent.
///
/// **RT contract:** non-RT only — `Copy` value type.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ClipSource(pub u64);

/// The media kind of a [`Track`] — constrains which
/// [`ClipAttributes`] variant its clips may carry, and lets consumers
/// dispatch evaluation results to the right engine.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy — new variants may be added without breaking downstream
/// `match` arms.
///
/// **RT contract:** non-RT only — `Copy` value type.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum TrackKind {
    /// Audio track — clips carry [`ClipAttributes::Audio`].
    Audio,
    /// Video track — clips carry [`ClipAttributes::Video`].
    Video,
}

/// Per-kind clip attributes — the minimal interpreted value set.
///
/// `gain` is a linear multiplier (finite, `[0.0, ∞)`); `opacity` is an
/// alpha factor (finite, `[0.0, 1.0]`). The value domain is re-checked
/// by every clip-storing mutation, as is the variant ↔
/// [`Track::kind`] match.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy — new variants may be added without breaking downstream
/// `match` arms.
///
/// **RT contract:** non-RT only — `Copy` value type.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ClipAttributes {
    /// Audio clip attributes.
    Audio {
        /// Linear gain multiplier — finite and `>= 0.0`.
        gain: f32,
        /// Whether the clip is muted.
        mute: bool,
    },
    /// Video clip attributes.
    Video {
        /// Opacity factor — finite and inside `[0.0, 1.0]`.
        opacity: f32,
    },
}

/// One clip on a [`Track`] — the timeline occupancy `range` `[start,
/// start + duration)`, the source selection `source_range` `[in, out)`
/// and per-kind `attributes`.
///
/// `range` is always stored in `timeline.base()`; `source_range` keeps
/// the source-specific base (its `start`/`duration` may even use
/// different internal bases). All fields are private — read through
/// the accessors (media-core stored-model convention).
///
/// **RT contract:** non-RT only — `Copy` value type.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Clip {
    id: ClipId,
    source: ClipSource,
    range: MediaRange,
    source_range: MediaRange,
    attributes: ClipAttributes,
}

impl Clip {
    /// The [`Timeline`]-minted identifier.
    ///
    /// **RT contract:** non-RT only — `const`, pure.
    #[must_use]
    pub const fn id(self) -> ClipId {
        self.id
    }

    /// The opaque source handle.
    ///
    /// **RT contract:** non-RT only — `const`, pure.
    #[must_use]
    pub const fn source(self) -> ClipSource {
        self.source
    }

    /// The timeline occupancy `[start, start + duration)` — always in
    /// `timeline.base()`.
    ///
    /// **RT contract:** non-RT only — `const`, pure.
    #[must_use]
    pub const fn range(self) -> MediaRange {
        self.range
    }

    /// The source selection `[in, out)` — kept in the source-specific
    /// base.
    ///
    /// **RT contract:** non-RT only — `const`, pure.
    #[must_use]
    pub const fn source_range(self) -> MediaRange {
        self.source_range
    }

    /// The per-kind attribute snapshot.
    ///
    /// **RT contract:** non-RT only — `const`, pure.
    #[must_use]
    pub const fn attributes(self) -> ClipAttributes {
        self.attributes
    }
}

/// Input bundle for [`Timeline::insert_clip`] — the clip definition
/// before ID minting.
///
/// Plain data with public fields: no invariant is held here — the
/// full rule set runs inside `insert_clip`. `range` may carry any
/// base: it is normalized into `timeline.base()` with
/// [`Rounding::NearestEven`] on insertion, so the stored value may
/// shift from the input by at most half a tick per endpoint.
/// `source_range` is *not* normalized — it keeps its source-specific
/// base.
///
/// **RT contract:** non-RT only — `Copy` value type.
#[derive(Copy, Clone, Debug)]
pub struct ClipSpec {
    /// Opaque source handle — the model never resolves it.
    pub source: ClipSource,
    /// Timeline occupancy `[start, start + duration)` — any base.
    pub range: MediaRange,
    /// Source selection `[in, out)` — kept in the source-specific base.
    pub source_range: MediaRange,
    /// Kind attributes — the variant must match the target track's
    /// `kind`.
    pub attributes: ClipAttributes,
}

/// One track — an ordered [`Clip`] list kept sorted by
/// `(range.start(), ClipId)` (`ClipId` grows with insertion order,
/// so a start-position tie keeps insertion order).
///
/// Overlapping clips are legal data — evaluation resolves them with a
/// defined rule. `clips()` doubles as the application's pre-insertion
/// overlap-inspection surface. `enabled = false` deactivates every
/// clip at evaluation time; newly added tracks are enabled.
///
/// All fields are private — read through the accessors.
///
/// **RT contract:** non-RT only — owns a `Vec`.
#[derive(Clone, Debug, PartialEq)]
pub struct Track {
    id: TrackId,
    kind: TrackKind,
    clips: Vec<Clip>,
    enabled: bool,
}

impl Track {
    /// The [`Timeline`]-minted identifier.
    ///
    /// **RT contract:** non-RT only — `const`, pure.
    #[must_use]
    pub const fn id(&self) -> TrackId {
        self.id
    }

    /// The track kind — constrains the clip attribute variants.
    ///
    /// **RT contract:** non-RT only — `const`, pure.
    #[must_use]
    pub const fn kind(&self) -> TrackKind {
        self.kind
    }

    /// The clips sorted by `(range.start(), ClipId)` — a
    /// start-position tie keeps insertion order, because `ClipId`
    /// is minted monotonically.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn clips(&self) -> &[Clip] {
        &self.clips
    }

    /// Whether the track contributes to evaluation — `false`
    /// deactivates every clip.
    ///
    /// **RT contract:** non-RT only — `const`, pure.
    #[must_use]
    pub const fn enabled(&self) -> bool {
        self.enabled
    }
}

/// The authoritative mutation entry point — owns a single
/// [`TimeBase`] and the ordered [`Track`] list, and mints all IDs.
///
/// Every stored `Clip::range` shares `base`. Track order is the
/// evaluation-enumeration order (consumers may read it as the neutral
/// stacking order for video); `add_track` appends at the end.
///
/// **RT contract:** non-RT only — mutations allocate and re-sort.
#[derive(Clone, Debug)]
pub struct Timeline {
    base: TimeBase,
    tracks: Vec<Track>,
    next_track: u64,
    next_clip: u64,
}

impl Timeline {
    /// An empty timeline owning `base` — every stored `Clip::range` is
    /// normalized into this base.
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_edit::{Timeline, TrackKind};
    /// use ruviform_media_core::TimeBase;
    ///
    /// let mut timeline = Timeline::new(TimeBase::COMMON_90K);
    /// let track = timeline.add_track(TrackKind::Audio);
    /// assert_eq!(timeline.tracks().len(), 1);
    /// assert_eq!(timeline.track(track).unwrap().kind(), TrackKind::Audio);
    /// ```
    #[must_use]
    pub const fn new(base: TimeBase) -> Self {
        Self {
            base,
            tracks: Vec::new(),
            next_track: 0,
            next_clip: 0,
        }
    }

    /// The [`TimeBase`] all stored `Clip::range`s share.
    ///
    /// **RT contract:** non-RT only — `const`, pure.
    #[must_use]
    pub const fn base(&self) -> TimeBase {
        self.base
    }

    /// The tracks in insertion order — also the
    /// evaluation-enumeration order.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn tracks(&self) -> &[Track] {
        &self.tracks
    }

    /// Looks up a track by ID — `None` for a stale or unknown ID.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn track(&self, id: TrackId) -> Option<&Track> {
        self.tracks.iter().find(|track| track.id == id)
    }

    /// Looks up a clip by track + clip ID — `None` for stale or
    /// unknown IDs.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn clip(&self, track: TrackId, clip: ClipId) -> Option<&Clip> {
        self.track(track)?
            .clips
            .iter()
            .find(|candidate| candidate.id == clip)
    }

    /// The largest resolved clip end across all tracks — `None` when
    /// no clip exists.
    ///
    /// The `filter_map` never drops a real clip: every stored `range`
    /// re-validated `checked_end(Rounding::NearestEven)` at the
    /// mutation boundary, so resolution is invariant-guaranteed.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn end(&self) -> Option<MediaTime> {
        self.tracks
            .iter()
            .flat_map(|track| track.clips().iter())
            .filter_map(|clip| clip.range().checked_end(Rounding::NearestEven).ok())
            .max()
    }

    /// Appends a new track of `kind` at the end of the track list and
    /// returns its minted ID — monotonically increasing, never reused.
    /// New tracks are `enabled`.
    ///
    /// **RT contract:** non-RT only — allocates.
    ///
    /// # Panics
    ///
    /// Panics when the `u64` track-ID space is exhausted — unreachable
    /// in practice (2⁶⁴ mints); panicking keeps the never-reuse
    /// invariant where wrapping would mint a duplicate ID.
    #[must_use]
    pub fn add_track(&mut self, kind: TrackKind) -> TrackId {
        let id = TrackId(self.next_track);
        // Monotonic, never reused: `checked_add` refuses to wrap the
        // counter into a duplicate `TrackId`.
        self.next_track = self
            .next_track
            .checked_add(1)
            .expect("u64 track id space exhausted");
        self.tracks.push(Track {
            id,
            kind,
            clips: Vec::new(),
            enabled: true,
        });
        id
    }

    /// Removes a track and returns it — the clips on it are dropped
    /// with the track; their [`ClipId`]s are never reused.
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// [`Error::TrackNotFound`] for a stale or unknown `id`.
    pub fn remove_track(&mut self, id: TrackId) -> Result<Track> {
        let index = self
            .tracks
            .iter()
            .position(|track| track.id == id)
            .ok_or(Error::TrackNotFound)?;
        Ok(self.tracks.remove(index))
    }

    /// Enables or disables a track — `enabled = false` deactivates
    /// every clip at evaluation time.
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// [`Error::TrackNotFound`] for a stale or unknown `id`.
    pub fn set_track_enabled(&mut self, id: TrackId, enabled: bool) -> Result<()> {
        self.track_mut(id)?.enabled = enabled;
        Ok(())
    }

    /// Validates `spec` and stores a new clip on `track`, keeping
    /// `Track::clips` sorted by `(range.start(), ClipId)` (a tie puts
    /// the newly minted clip last — insertion order).
    ///
    /// `spec.range` is normalized into `timeline.base()` first: both
    /// `start` and `duration` convert with [`Rounding::NearestEven`],
    /// so a base mismatch is accepted by rounding in — the stored
    /// value may shift from the input by at most half a tick per
    /// endpoint. `spec.source_range` keeps its own base. The full
    /// validation rule set then runs; a failed call changes nothing.
    ///
    /// **RT contract:** non-RT only — allocates and sorts.
    ///
    /// # Errors
    ///
    /// - [`Error::TrackNotFound`] for a stale or unknown `track`;
    /// - [`Error::TimeOverflow`] when base conversion or `end`
    ///   resolution overflows `i64`;
    /// - [`Error::InvalidRange`] for a non-positive duration, a
    ///   base-collapsed `end <= start`, or
    ///   `range.duration() > source_range.duration()`;
    /// - [`Error::KindMismatch`] when the attribute variant does not
    ///   match the track kind;
    /// - [`Error::InvalidValue`] for out-of-domain `gain`/`opacity`,
    ///   or `"id_space"` on `u64` clip-ID exhaustion (practically
    ///   unreachable — the call fails rather than wrapping into a
    ///   duplicate ID).
    pub fn insert_clip(&mut self, track: TrackId, spec: ClipSpec) -> Result<ClipId> {
        let index = self
            .tracks
            .iter()
            .position(|candidate| candidate.id == track)
            .ok_or(Error::TrackNotFound)?;
        let range = normalize_range(spec.range, self.base)?;
        validate_clip(
            self.tracks[index].kind,
            range,
            spec.source_range,
            spec.attributes,
        )?;
        let id = ClipId(self.next_clip);
        // Monotonic, never reused: on `u64` exhaustion the call fails
        // rather than wrapping into a duplicate `ClipId`.
        // `InvalidValue` is the least-wrong existing variant — a
        // dedicated variant is not worth the API surface for a
        // practically unreachable condition.
        self.next_clip = self
            .next_clip
            .checked_add(1)
            .ok_or(Error::InvalidValue("id_space"))?;
        let clips = &mut self.tracks[index].clips;
        clips.push(Clip {
            id,
            source: spec.source,
            range,
            source_range: spec.source_range,
            attributes: spec.attributes,
        });
        sort_clips(clips);
        Ok(id)
    }

    /// Removes a clip from `track` and returns it — the [`ClipId`] is
    /// never reused.
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// [`Error::TrackNotFound`] / [`Error::ClipNotFound`] for stale or
    /// unknown IDs.
    pub fn remove_clip(&mut self, track: TrackId, clip: ClipId) -> Result<Clip> {
        let track = self.track_mut(track)?;
        let index = track
            .clips
            .iter()
            .position(|candidate| candidate.id == clip)
            .ok_or(Error::ClipNotFound)?;
        Ok(track.clips.remove(index))
    }

    /// Moves a clip to `new_start`, keeping its duration, and re-sorts
    /// the track's clip list.
    ///
    /// `new_start` is converted into `timeline.base()` with
    /// [`Rounding::NearestEven`] first — a base mismatch is accepted by
    /// rounding in, never by saturation. Cross-track moves are
    /// expressed as `remove_clip` + `insert_clip` (the kind may
    /// change), so no dedicated op exists.
    ///
    /// **RT contract:** non-RT only — sorts.
    ///
    /// # Errors
    ///
    /// - [`Error::TrackNotFound`] / [`Error::ClipNotFound`] for stale
    ///   or unknown IDs;
    /// - [`Error::TimeOverflow`] when the `new_start` conversion or
    ///   the resolved `end` overflows `i64`;
    /// - [`Error::InvalidRange`] for a collapsed or out-of-source
    ///   range.
    ///
    /// [`Error::KindMismatch`] / [`Error::InvalidValue`] are not
    /// reachable here: the re-checked attributes are the stored ones,
    /// which were validated at insertion and are left unchanged.
    pub fn move_clip(&mut self, track: TrackId, clip: ClipId, new_start: MediaTime) -> Result<()> {
        let base = self.base;
        let track = self.track_mut(track)?;
        let index = track
            .clips
            .iter()
            .position(|candidate| candidate.id == clip)
            .ok_or(Error::ClipNotFound)?;
        let start = new_start.convert(base, Rounding::NearestEven)?;
        let stored = track.clips[index];
        let range = MediaRange::new(start, stored.range().duration());
        validate_clip(
            track.kind,
            range,
            stored.source_range(),
            stored.attributes(),
        )?;
        track.clips[index].range = range;
        sort_clips(&mut track.clips);
        Ok(())
    }

    /// Re-specifies a clip's `range` and `source_range` atomically —
    /// `new_range` is normalized into `timeline.base()`, the full
    /// validation rule set re-runs, and when `range.start()` changed
    /// the track's clip order is re-established. A failed call changes
    /// nothing.
    ///
    /// **RT contract:** non-RT only — sorts.
    ///
    /// # Errors
    ///
    /// Same set as [`Timeline::insert_clip`], plus
    /// [`Error::ClipNotFound`] for a stale or unknown `clip` — the
    /// full rule set re-runs on the re-specified ranges and the stored
    /// attributes, so [`Error::KindMismatch`] /
    /// [`Error::InvalidValue`] remain reachable in principle.
    pub fn trim_clip(
        &mut self,
        track: TrackId,
        clip: ClipId,
        new_range: MediaRange,
        new_source_range: MediaRange,
    ) -> Result<()> {
        let base = self.base;
        let track = self.track_mut(track)?;
        let index = track
            .clips
            .iter()
            .position(|candidate| candidate.id == clip)
            .ok_or(Error::ClipNotFound)?;
        let range = normalize_range(new_range, base)?;
        let stored = track.clips[index];
        validate_clip(track.kind, range, new_source_range, stored.attributes())?;
        track.clips[index].range = range;
        track.clips[index].source_range = new_source_range;
        sort_clips(&mut track.clips);
        Ok(())
    }

    fn track_mut(&mut self, id: TrackId) -> Result<&mut Track> {
        self.tracks
            .iter_mut()
            .find(|track| track.id == id)
            .ok_or(Error::TrackNotFound)
    }
}

/// Converts `range`'s `start` and `duration` into `base`, rounding
/// each once with `NearestEven` — the two conversions are
/// independent, so `start` and `duration` may each shift by at most
/// half a tick and the resolved `end` (`start + duration`) can drift
/// up to ~1 tick.
fn normalize_range(range: MediaRange, base: TimeBase) -> Result<MediaRange> {
    let start = range.start().convert(base, Rounding::NearestEven)?;
    let duration = range.duration().convert(base, Rounding::NearestEven)?;
    Ok(MediaRange::new(start, duration))
}

/// The full clip-invariant rule set — executed by every mutation that
/// stores or changes a clip (`insert_clip` / `move_clip` /
/// `trim_clip`).
fn validate_clip(
    kind: TrackKind,
    range: MediaRange,
    source_range: MediaRange,
    attributes: ClipAttributes,
) -> Result<()> {
    validate_range(range, "range")?;
    validate_range(source_range, "source_range")?;
    // Exact cross-base ordering: the 1:1 mapping must not overshoot
    // `out`.
    if range.duration() > source_range.duration() {
        return Err(Error::InvalidRange("source_range"));
    }
    let kind_matches = match attributes {
        ClipAttributes::Audio { .. } => kind == TrackKind::Audio,
        ClipAttributes::Video { .. } => kind == TrackKind::Video,
    };
    if !kind_matches {
        return Err(Error::KindMismatch);
    }
    match attributes {
        ClipAttributes::Audio { gain, .. } if !(gain.is_finite() && gain >= 0.0) => {
            Err(Error::InvalidValue("gain"))
        }
        ClipAttributes::Video { opacity }
            if !(opacity.is_finite() && (0.0..=1.0).contains(&opacity)) =>
        {
            Err(Error::InvalidValue("opacity"))
        }
        _ => Ok(()),
    }
}

/// Positive duration, a resolvable `end` and `end > start` — the
/// base-collapse guard shared by `range` (already timeline-base
/// normalized) and `source_range` (source-specific base; its
/// `start`/`duration` may use different internal bases, which
/// `checked_end` reconciles with one `NearestEven` conversion).
fn validate_range(range: MediaRange, name: &'static str) -> Result<()> {
    if range.duration().value() <= 0 {
        return Err(Error::InvalidRange(name));
    }
    let end = range.checked_end(Rounding::NearestEven)?;
    if end <= range.start() {
        return Err(Error::InvalidRange(name));
    }
    Ok(())
}

/// Re-establishes the `(range.start(), ClipId)` order — `ClipId` is
/// minted monotonically in insertion order, so a start-position tie
/// resolves to the later insertion no matter which mutation
/// (`insert_clip` / `move_clip` / `trim_clip`) last re-sorted the
/// list.
fn sort_clips(clips: &mut [Clip]) {
    clips.sort_by_key(|clip| (clip.range().start(), clip.id()));
}

#[cfg(test)]
mod tests {
    use ruviform_media_core::MediaDuration;

    use super::*;

    fn base() -> TimeBase {
        TimeBase::COMMON_90K
    }

    fn time(value: i64) -> MediaTime {
        MediaTime::new(value, base())
    }

    fn duration(value: i64) -> MediaDuration {
        MediaDuration::new(value, base())
    }

    fn range(start: i64, duration: i64) -> MediaRange {
        MediaRange::new(time(start), MediaDuration::new(duration, base()))
    }

    fn audio_spec(start: i64, duration: i64) -> ClipSpec {
        ClipSpec {
            source: ClipSource(1),
            range: range(start, duration),
            source_range: range(0, duration),
            attributes: ClipAttributes::Audio {
                gain: 1.0,
                mute: false,
            },
        }
    }

    #[test]
    fn new_timeline_is_empty() {
        let timeline = Timeline::new(base());
        assert_eq!(timeline.base(), base());
        assert_eq!(timeline.tracks(), []);
        assert_eq!(timeline.end(), None);
    }

    #[test]
    fn add_track_appends_and_mints_monotonic_ids() {
        let mut timeline = Timeline::new(base());
        let audio = timeline.add_track(TrackKind::Audio);
        let video = timeline.add_track(TrackKind::Video);
        assert!(video > audio);
        assert_eq!(
            timeline.tracks().iter().map(Track::id).collect::<Vec<_>>(),
            [audio, video]
        );
        assert!(timeline.track(audio).unwrap().enabled());
        assert_eq!(timeline.track(audio).unwrap().kind(), TrackKind::Audio);
        assert_eq!(timeline.track(TrackId(999)), None);
    }

    #[test]
    fn remove_track_returns_it_and_ids_stay_stale() {
        let mut timeline = Timeline::new(base());
        let track = timeline.add_track(TrackKind::Audio);
        let removed = timeline.remove_track(track).unwrap();
        assert_eq!(removed.id(), track);
        assert_eq!(timeline.track(track), None);
        assert_eq!(
            timeline.remove_track(track).unwrap_err(),
            Error::TrackNotFound
        );
        // Re-adding mints a fresh, larger ID — no reuse.
        let readded = timeline.add_track(TrackKind::Audio);
        assert!(readded > track);
    }

    #[test]
    fn insert_clip_sorts_by_start_and_normalizes_range() {
        let mut timeline = Timeline::new(base());
        let track = timeline.add_track(TrackKind::Audio);
        let later = timeline.insert_clip(track, audio_spec(100, 10)).unwrap();
        let earlier = timeline.insert_clip(track, audio_spec(0, 10)).unwrap();
        assert!(earlier > later); // ClipIds stay monotonic.
        assert_eq!(
            timeline
                .track(track)
                .unwrap()
                .clips()
                .iter()
                .map(|clip| clip.id())
                .collect::<Vec<_>>(),
            [earlier, later]
        );

        // A spec in a foreign base is accepted and stored normalized
        // into the timeline base — `source_range` keeps its own base.
        let fine = TimeBase::new(1, 48_000).unwrap();
        let clip = timeline
            .insert_clip(
                track,
                ClipSpec {
                    source: ClipSource(2),
                    range: MediaRange::new(
                        MediaTime::new(48_000, fine),
                        MediaDuration::new(48_000, fine),
                    ),
                    source_range: MediaRange::new(
                        MediaTime::ZERO,
                        MediaDuration::new(48_000, fine),
                    ),
                    attributes: ClipAttributes::Audio {
                        gain: 1.0,
                        mute: false,
                    },
                },
            )
            .unwrap();
        let stored = timeline.clip(track, clip).unwrap();
        assert_eq!(stored.range().start().base(), base());
        assert_eq!(stored.range().duration().base(), base());
        assert_eq!(stored.range().start(), time(90_000));
        assert_eq!(stored.source_range().duration().base(), fine);
        assert_eq!(stored.source(), ClipSource(2));
    }

    #[test]
    fn insert_clip_rejects_non_positive_duration() {
        let mut timeline = Timeline::new(base());
        let track = timeline.add_track(TrackKind::Audio);
        for duration in [0, -10] {
            assert_eq!(
                timeline
                    .insert_clip(track, audio_spec(0, duration))
                    .unwrap_err(),
                Error::InvalidRange("range")
            );
        }
        // A negative-duration source_range is rejected too.
        let mut spec = audio_spec(0, 10);
        spec.source_range = range(0, -10);
        assert_eq!(
            timeline.insert_clip(track, spec).unwrap_err(),
            Error::InvalidRange("source_range")
        );
        assert_eq!(timeline.track(track).unwrap().clips(), []);
    }

    #[test]
    fn insert_clip_rejects_base_collapsed_ranges() {
        let mut timeline = Timeline::new(base());
        let track = timeline.add_track(TrackKind::Audio);
        // Timeline base is 1/1 s: a one-tick duration on a fine base
        // normalizes to zero ticks — `end == start` must not slip in.
        let mut coarse = Timeline::new(TimeBase::new(1, 1).unwrap());
        let coarse_track = coarse.add_track(TrackKind::Audio);
        let fine = TimeBase::new(1, 48_000).unwrap();
        let collapsed = ClipSpec {
            source: ClipSource(1),
            range: MediaRange::new(MediaTime::ZERO, MediaDuration::new(1, fine)),
            source_range: range(0, 10),
            attributes: ClipAttributes::Audio {
                gain: 1.0,
                mute: false,
            },
        };
        assert_eq!(
            coarse.insert_clip(coarse_track, collapsed).unwrap_err(),
            Error::InvalidRange("range")
        );
        // The same collapse on `source_range` (`out == in` after the
        // internal conversion) is rejected as well.
        let mut spec = audio_spec(0, 10);
        spec.source_range = MediaRange::new(MediaTime::ZERO, MediaDuration::new(1, fine));
        assert_eq!(
            timeline.insert_clip(track, spec).unwrap_err(),
            Error::InvalidRange("source_range")
        );
    }

    #[test]
    fn insert_clip_rejects_range_longer_than_source() {
        let mut timeline = Timeline::new(base());
        let track = timeline.add_track(TrackKind::Audio);
        let mut spec = audio_spec(0, 20);
        spec.source_range = range(0, 10); // source selection shorter than the span
        assert_eq!(
            timeline.insert_clip(track, spec).unwrap_err(),
            Error::InvalidRange("source_range")
        );
    }

    #[test]
    fn insert_clip_rejects_kind_mismatch_and_bad_attributes() {
        let mut timeline = Timeline::new(base());
        let audio = timeline.add_track(TrackKind::Audio);
        let video = timeline.add_track(TrackKind::Video);

        let mut spec = audio_spec(0, 10);
        spec.attributes = ClipAttributes::Video { opacity: 1.0 };
        assert_eq!(
            timeline.insert_clip(audio, spec).unwrap_err(),
            Error::KindMismatch
        );
        spec.attributes = ClipAttributes::Audio {
            gain: 1.0,
            mute: false,
        };
        assert_eq!(
            timeline.insert_clip(video, spec).unwrap_err(),
            Error::KindMismatch
        );

        for gain in [-1.0, f32::NAN, f32::INFINITY] {
            let mut spec = audio_spec(0, 10);
            spec.attributes = ClipAttributes::Audio { gain, mute: false };
            assert_eq!(
                timeline.insert_clip(audio, spec).unwrap_err(),
                Error::InvalidValue("gain")
            );
        }
        for opacity in [-0.1, 1.5, f32::NAN] {
            let mut spec = audio_spec(0, 10);
            spec.attributes = ClipAttributes::Video { opacity };
            assert_eq!(
                timeline.insert_clip(video, spec).unwrap_err(),
                Error::InvalidValue("opacity")
            );
        }
    }

    #[test]
    fn insert_clip_propagates_time_overflow() {
        let mut timeline = Timeline::new(base());
        let track = timeline.add_track(TrackKind::Audio);
        // `start + duration` overflows i64 -> `checked_end` fails.
        assert_eq!(
            timeline
                .insert_clip(track, audio_spec(i64::MAX - 10, 100))
                .unwrap_err(),
            Error::TimeOverflow
        );
        // Unknown track wins over spec validation.
        assert_eq!(
            timeline
                .insert_clip(TrackId(999), audio_spec(0, 10))
                .unwrap_err(),
            Error::TrackNotFound
        );
    }

    #[test]
    fn remove_clip_returns_it_and_rejects_stale_ids() {
        let mut timeline = Timeline::new(base());
        let track = timeline.add_track(TrackKind::Audio);
        let clip = timeline.insert_clip(track, audio_spec(0, 10)).unwrap();
        let removed = timeline.remove_clip(track, clip).unwrap();
        assert_eq!(removed.id(), clip);
        assert_eq!(timeline.clip(track, clip), None);
        assert_eq!(
            timeline.remove_clip(track, clip).unwrap_err(),
            Error::ClipNotFound
        );
        assert_eq!(
            timeline.remove_clip(TrackId(999), clip).unwrap_err(),
            Error::TrackNotFound
        );
        // The next minted ClipId is larger — no reuse.
        let next = timeline.insert_clip(track, audio_spec(20, 10)).unwrap();
        assert!(next > clip);
    }

    #[test]
    fn move_clip_resorts_and_keeps_duration() {
        let mut timeline = Timeline::new(base());
        let track = timeline.add_track(TrackKind::Audio);
        let first = timeline.insert_clip(track, audio_spec(0, 10)).unwrap();
        let second = timeline.insert_clip(track, audio_spec(100, 10)).unwrap();
        timeline.move_clip(track, first, time(200)).unwrap();
        let clips = timeline.track(track).unwrap().clips();
        assert_eq!(
            clips.iter().map(|clip| clip.id()).collect::<Vec<_>>(),
            [second, first]
        );
        assert_eq!(clips[1].range().start(), time(200));
        assert_eq!(clips[1].range().duration(), duration(10));

        // A foreign-base start is converted in; overflow propagates.
        let fine = TimeBase::new(1, 48_000).unwrap();
        timeline
            .move_clip(track, first, MediaTime::new(0, fine))
            .unwrap();
        assert_eq!(
            timeline.clip(track, first).unwrap().range().start(),
            time(0)
        );
        assert_eq!(
            timeline
                .move_clip(track, first, time(i64::MAX))
                .unwrap_err(),
            Error::TimeOverflow
        );
        // A rejected move changes nothing.
        assert_eq!(timeline.clip(track, first).unwrap().range(), range(0, 10));
        assert_eq!(
            timeline.move_clip(track, ClipId(999), time(0)).unwrap_err(),
            Error::ClipNotFound
        );
    }

    #[test]
    fn trim_clip_respecifies_atomically_and_resorts() {
        let mut timeline = Timeline::new(base());
        let track = timeline.add_track(TrackKind::Audio);
        let first = timeline.insert_clip(track, audio_spec(0, 10)).unwrap();
        let second = timeline.insert_clip(track, audio_spec(100, 10)).unwrap();
        // Start change -> re-sort; source_range is re-specified too.
        timeline
            .trim_clip(track, first, range(200, 5), range(0, 5))
            .unwrap();
        let clips = timeline.track(track).unwrap().clips();
        assert_eq!(
            clips.iter().map(|clip| clip.id()).collect::<Vec<_>>(),
            [second, first]
        );
        assert_eq!(clips[1].range(), range(200, 5));
        assert_eq!(clips[1].source_range(), range(0, 5));

        // A rejected trim changes nothing.
        assert_eq!(
            timeline
                .trim_clip(track, first, range(300, 0), range(0, 5))
                .unwrap_err(),
            Error::InvalidRange("range")
        );
        assert_eq!(
            timeline.trim_clip(track, first, range(300, 50), range(0, 5)),
            Err(Error::InvalidRange("source_range"))
        );
        assert_eq!(timeline.clip(track, first).unwrap().range(), range(200, 5));
    }

    #[test]
    fn set_track_enabled_toggles_the_flag() {
        let mut timeline = Timeline::new(base());
        let track = timeline.add_track(TrackKind::Audio);
        timeline.set_track_enabled(track, false).unwrap();
        assert!(!timeline.track(track).unwrap().enabled());
        timeline.set_track_enabled(track, true).unwrap();
        assert!(timeline.track(track).unwrap().enabled());
        assert_eq!(
            timeline.set_track_enabled(TrackId(999), true).unwrap_err(),
            Error::TrackNotFound
        );
    }

    #[test]
    fn end_is_the_max_resolved_clip_end() {
        let mut timeline = Timeline::new(base());
        let audio = timeline.add_track(TrackKind::Audio);
        let video = timeline.add_track(TrackKind::Video);
        timeline.insert_clip(audio, audio_spec(0, 10)).unwrap();
        timeline.insert_clip(audio, audio_spec(100, 10)).unwrap();
        let mut spec = audio_spec(50, 10);
        spec.attributes = ClipAttributes::Video { opacity: 1.0 };
        timeline.insert_clip(video, spec).unwrap();
        assert_eq!(timeline.end(), Some(time(110)));
    }
}
