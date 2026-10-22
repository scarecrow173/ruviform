//! Transport state value types: the [`TransportStatus`] machine states and
//! the immutable [`TransportSnapshot`] position snapshot.

use crate::{music::MusicalTimeInfo, time::MediaTime};

/// Playback state of a transport.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy — new variants may be added without breaking downstream
/// `match` arms.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum TransportStatus {
    /// Not running; position is held or reset per caller policy.
    Stopped,
    /// Position advances with the media clock.
    Playing,
    /// Position is held.
    Paused,
}

/// An immutable transport state snapshot: status plus position, with an
/// optional [`MusicalTimeInfo`] musical-time snapshot.
///
/// Snapshots are read-side value types; the producing transport and its
/// update mechanism are defined elsewhere. `musical` is `None` when the
/// producing transport has no musical data — no implicit 4/4 or tempo
/// is assumed.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TransportSnapshot {
    status: TransportStatus,
    position: MediaTime,
    musical: Option<MusicalTimeInfo>,
    reserved: [u64; 2],
}

impl TransportSnapshot {
    /// Creates a snapshot without musical data; the `reserved`
    /// extension field is always `[0; 2]` today.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_media_core::{MediaTime, TransportSnapshot, TransportStatus};
    ///
    /// let snap = TransportSnapshot::new(TransportStatus::Playing, MediaTime::ZERO);
    /// assert_eq!(snap.status(), TransportStatus::Playing);
    /// assert_eq!(snap.position(), MediaTime::ZERO);
    /// assert_eq!(snap.musical(), None);
    /// ```
    #[must_use]
    pub const fn new(status: TransportStatus, position: MediaTime) -> Self {
        Self {
            status,
            position,
            musical: None,
            reserved: [0; 2],
        }
    }

    /// Returns a copy of this snapshot carrying `musical` — the
    /// musical-time snapshot from the producing transport's tempo map.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn with_musical(self, musical: MusicalTimeInfo) -> Self {
        Self {
            musical: Some(musical),
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

    /// The transport position.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn position(self) -> MediaTime {
        self.position
    }

    /// The musical-time snapshot, when the producing transport has
    /// musical data.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn musical(self) -> Option<MusicalTimeInfo> {
        self.musical
    }

    /// Opaque extension reserve (currently always `[0; 2]`). Callers
    /// must not interpret its contents.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_media_core::{MediaTime, TransportSnapshot, TransportStatus};
    ///
    /// let snap = TransportSnapshot::new(TransportStatus::Stopped, MediaTime::ZERO);
    /// assert_eq!(snap.reserved(), [0; 2]);
    /// ```
    #[must_use]
    pub const fn reserved(&self) -> [u64; 2] {
        self.reserved
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_carries_status_and_position() {
        let position = MediaTime::new(90_000, crate::TimeBase::COMMON_90K);
        let snap = TransportSnapshot::new(TransportStatus::Paused, position);
        assert_eq!(snap.status(), TransportStatus::Paused);
        assert_eq!(snap.position(), position);
        assert_eq!(snap.reserved(), [0; 2]);
    }
}
