//! Standard-neutral transport state: the [`TransportInfo`] snapshot
//! handed to processing.
//!
//! The snapshot is deliberately free of musical time — no bars, beats,
//! PPQN or tempo. Positions are [`MediaTime`] (a [`TimeBase`] plus an
//! integer count) only, the same rule as the design baseline's time
//! model. The opaque `reserved` extension field absorbs a future
//! musical-time mapping without changing this type's shape — the same
//! policy as `TransportSnapshot`'s `reserved` in
//! `ruviform-media-core`.
//!
//! [`TimeBase`]: ruviform_media_core::TimeBase

use ruviform_media_core::{MediaRange, MediaTime, TransportStatus};

/// An immutable, standard-neutral transport snapshot for one
/// processing call: playback status, position and optional loop range.
///
/// Contains no musical time — `position`/`loop_range` are media times
/// (`TimeBase` + integer count). `reserved` is an opaque extension
/// field, always `[0; 2]` today, reserved for the future musical-time
/// mapping; callers must not interpret its contents.
///
/// **RT contract:** RT-suitable — `Copy` value type.
///
/// # Examples
///
/// ```
/// use ruviform_media_core::{MediaTime, TimeBase, TransportStatus};
/// use ruviform_plugin_core::TransportInfo;
///
/// let pos = MediaTime::new(96_000, TimeBase::sample_rate(48_000).unwrap());
/// let info = TransportInfo::new(TransportStatus::Playing, pos);
/// assert_eq!(info.status(), TransportStatus::Playing);
/// assert_eq!(info.position(), pos);
/// assert_eq!(info.loop_range(), None);
/// assert_eq!(info.reserved(), [0; 2]);
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TransportInfo {
    status: TransportStatus,
    position: MediaTime,
    loop_range: Option<MediaRange>,
    reserved: [u64; 2],
}

impl TransportInfo {
    /// Creates a snapshot without a loop range.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn new(status: TransportStatus, position: MediaTime) -> Self {
        Self {
            status,
            position,
            loop_range: None,
            reserved: [0; 2],
        }
    }

    /// Returns a copy of this snapshot carrying `loop_range` — the
    /// active cycle region when the host is looping.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_media_core::{MediaDuration, MediaRange, MediaTime, TimeBase, TransportStatus};
    /// use ruviform_plugin_core::TransportInfo;
    ///
    /// let base = TimeBase::sample_rate(48_000).unwrap();
    /// let loop_range = MediaRange::new(
    ///     MediaTime::new(0, base),
    ///     MediaDuration::new(48_000, base),
    /// );
    /// let info = TransportInfo::new(TransportStatus::Playing, MediaTime::ZERO)
    ///     .with_loop_range(loop_range);
    /// assert_eq!(info.loop_range(), Some(loop_range));
    /// ```
    #[must_use]
    pub const fn with_loop_range(self, loop_range: MediaRange) -> Self {
        Self {
            loop_range: Some(loop_range),
            ..self
        }
    }

    /// The playback status.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn status(self) -> TransportStatus {
        self.status
    }

    /// The transport position at the start of the processing block.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn position(self) -> MediaTime {
        self.position
    }

    /// The active loop range, when the host is cycling a region.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn loop_range(self) -> Option<MediaRange> {
        self.loop_range
    }

    /// Opaque extension reserve for the future musical-time mapping
    /// (currently always `[0; 2]`). Callers must not interpret its
    /// contents.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn reserved(&self) -> [u64; 2] {
        self.reserved
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruviform_media_core::TimeBase;

    #[test]
    fn snapshot_carries_status_position_and_loop() {
        let base = TimeBase::sample_rate(44_100).unwrap();
        let position = MediaTime::new(44_100, base);
        let info =
            TransportInfo::new(TransportStatus::Paused, position).with_loop_range(MediaRange::new(
                MediaTime::ZERO,
                ruviform_media_core::MediaDuration::new(88_200, base),
            ));
        assert_eq!(info.status(), TransportStatus::Paused);
        assert_eq!(info.position(), position);
        assert!(info.loop_range().is_some());
        assert_eq!(info.reserved(), [0; 2]);
    }
}
