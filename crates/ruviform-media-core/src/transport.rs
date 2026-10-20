//! Transport state value types: the [`TransportStatus`] machine states and
//! the immutable [`TransportSnapshot`] position snapshot.

use crate::time::MediaTime;

/// Playback state of a transport.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum TransportStatus {
    /// Not running; position is held or reset per caller policy.
    Stopped,
    /// Position advances with the media clock.
    Playing,
    /// Position is held.
    Paused,
}

/// An immutable transport state snapshot: status plus position.
///
/// Snapshots are read-side value types; the producing transport and its
/// update mechanism are defined elsewhere.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TransportSnapshot {
    status: TransportStatus,
    position: MediaTime,
    reserved: [u64; 2],
}

impl TransportSnapshot {
    /// Creates a snapshot; the `reserved` extension field is always
    /// `[0; 2]` today.
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
    /// ```
    #[must_use]
    pub const fn new(status: TransportStatus, position: MediaTime) -> Self {
        Self {
            status,
            position,
            reserved: [0; 2],
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

    /// Opaque extension reserve for future musical time (currently always
    /// `[0; 2]`). Callers must not interpret its contents.
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
