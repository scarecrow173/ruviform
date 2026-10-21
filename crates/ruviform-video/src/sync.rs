//! A/V sync — audio master + PTS follow (ADR `video-av-sync-policy`).
//!
//! [`FrameScheduler`] compares a frame's PTS against the master clock
//! and returns a [`SyncDecision`]. The engine decides; the caller
//! (presentation scheduler) executes waits.

use ruviform_media_core::MediaTime;
use ruviform_rt::MasterClock;

/// What the engine decided for one frame.
///
/// **RT contract:** non-RT only — `Copy` value type.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SyncDecision {
    /// Frame PTS is within sync tolerance — present it.
    Present,
    /// Frame is early — present at the carried target time.
    Wait(MediaTime),
    /// Frame is too late (or predates the last seek target) — drop it
    /// and keep following the audio master.
    Drop,
}

/// Sync tolerances in seconds.
///
/// `early`: positive lateness beyond which the frame waits.
/// `drop_late`: negative lateness magnitude beyond which the frame is
/// dropped instead of presented late. Callers typically derive both
/// from the frame rate (e.g. one frame period each).
///
/// **RT contract:** non-RT only — `Copy` value type.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SyncThresholds {
    /// Seconds ahead of the master that still counts as on-time.
    pub early: f64,
    /// Seconds behind the master beyond which a frame is dropped.
    pub drop_late: f64,
}

impl Default for SyncThresholds {
    /// ±40 ms — roughly one frame at 25 fps.
    fn default() -> Self {
        Self {
            early: 0.040,
            drop_late: 0.040,
        }
    }
}

/// Decides per-frame present/wait/drop against the audio master.
///
/// `floor` (set by [`Self::resync_at`]) drops frames that predate the
/// latest seek target — presentation starts at the seeked position.
///
/// **RT contract:** non-RT only. Reading a [`MasterClock`] is a
/// non-blocking value load; scheduling itself is pure arithmetic.
#[non_exhaustive]
pub struct FrameScheduler {
    thresholds: SyncThresholds,
    floor: Option<MediaTime>,
}

impl FrameScheduler {
    /// Scheduler with the given thresholds.
    #[must_use]
    pub const fn new(thresholds: SyncThresholds) -> Self {
        Self {
            thresholds,
            floor: None,
        }
    }

    /// Clears the resync floor.
    pub fn reset(&mut self) {
        self.floor = None;
    }

    /// Drops everything before `target` — called after a seek so
    /// stale decoder output cannot present ahead of the seek point.
    pub fn resync_at(&mut self, target: MediaTime) {
        self.floor = Some(target);
    }

    /// Classifies `pts` against `master_now`.
    ///
    /// `lateness = pts − master`: larger than `early` waits, more
    /// negative than `drop_late` drops, otherwise presents.
    #[must_use]
    pub fn decide(&self, pts: MediaTime, master_now: MediaTime) -> SyncDecision {
        if let Some(floor) = self.floor {
            if pts < floor {
                return SyncDecision::Drop;
            }
        }
        let lateness = pts.to_seconds_f64() - master_now.to_seconds_f64();
        if lateness > self.thresholds.early {
            SyncDecision::Wait(pts)
        } else if lateness < -self.thresholds.drop_late {
            SyncDecision::Drop
        } else {
            SyncDecision::Present
        }
    }

    /// Current master time as a [`MediaTime`] (reads the clock once).
    ///
    /// The clock's timebase was validated at construction, so the
    /// `TimeBase` rebuild cannot fail; the fallback is unreachable.
    #[must_use]
    pub fn master_time(clock: &MasterClock) -> MediaTime {
        let (num, den) = clock.timebase();
        let base = ruviform_media_core::TimeBase::new(num, den)
            .unwrap_or(ruviform_media_core::TimeBase::COMMON_90K);
        MediaTime::new(i64::try_from(clock.tick()).unwrap_or(i64::MAX), base)
    }
}
