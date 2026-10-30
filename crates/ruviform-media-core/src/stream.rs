//! Container-level, codec-agnostic stream description: [`StreamId`],
//! [`StreamKind`] and [`StreamDescriptor`].
//!
//! Codec- and format-specific details belong to the domain crates' own
//! format types (`ruviform-audio-core`, `ruviform-video-core`), not to
//! this descriptor. Ports consuming `Packet`/`StreamDescriptor` are a
//! later `ruviform-media-io` concern and are intentionally not defined
//! here.

use crate::fact::Fact;
use crate::time::{MediaDuration, TimeBase};

/// Identifier of a stream inside a container.
///
/// **RT contract:** RT-suitable — `Copy` value type.
///
/// # Examples
///
/// ```
/// use ruviform_media_core::StreamId;
///
/// let id = StreamId(7);
/// assert_eq!(id.0, 7);
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StreamId(pub u32);

/// The media kind of a stream.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy — new variants may be added without breaking downstream
/// `match` arms.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum StreamKind {
    /// Audio stream.
    Audio,
    /// Video stream.
    Video,
    /// Data stream — subtitles, metadata and other non-A/V payloads.
    Data,
}

/// Container-level, codec-agnostic description of one stream.
///
/// Only stream-level facts live here. Codec- and format-specific details
/// (sample format, pixel format, …) are carried by the domain crates' own
/// format types.
///
/// **RT contract:** RT-suitable — `Copy` value type.
///
/// # Examples
///
/// ```
/// use ruviform_media_core::{Fact, MediaDuration, StreamDescriptor, StreamId, StreamKind, TimeBase};
///
/// let stream = StreamDescriptor {
///     id: StreamId(0),
///     kind: StreamKind::Audio,
///     time_base: Fact::known(TimeBase::COMMON_90K),
///     duration: Some(MediaDuration::new(90_000, TimeBase::COMMON_90K)),
/// };
/// assert_eq!(stream.duration.unwrap().value(), 90_000);
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct StreamDescriptor {
    /// Stream identifier.
    pub id: StreamId,
    /// Media kind.
    pub kind: StreamKind,
    /// Time base of the stream's PTS units as carried by the container.
    ///
    /// `Fact` rather than a bare `TimeBase`: when the container does not
    /// declare a base or the backend cannot supply one, the fact is
    /// reported as unavailable — never replaced with a fabricated
    /// default. Per-unit timestamps are carried as raw ticks
    /// ([`TickTimestamp`](crate::TickTimestamp)) precisely so they stay
    /// observable when this fact is unavailable.
    pub time_base: Fact<TimeBase>,
    /// Declared stream duration, when the container provides one.
    pub duration: Option<MediaDuration>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptor_is_a_copy_value_type() {
        let descriptor = StreamDescriptor {
            id: StreamId(7),
            kind: StreamKind::Audio,
            time_base: Fact::known(TimeBase::COMMON_90K),
            duration: Some(MediaDuration::new(90_000, TimeBase::COMMON_90K)),
        };
        let copy = descriptor; // `Copy` — the original stays usable.
        assert_eq!(descriptor, copy);
        assert_eq!(descriptor.id, StreamId(7));
        assert_eq!(descriptor.kind, StreamKind::Audio);
        assert_eq!(descriptor.time_base, Fact::Known(TimeBase::COMMON_90K));
        assert!(descriptor.duration.is_some());
    }

    #[test]
    fn descriptor_may_omit_duration() {
        let descriptor = StreamDescriptor {
            id: StreamId(1),
            kind: StreamKind::Data,
            time_base: Fact::known(TimeBase::new(1, 1_000).unwrap()),
            duration: None,
        };
        assert_eq!(descriptor.duration, None);
    }

    /// An unavailable time base is represented, never fabricated: a
    /// stream without a declared base reports `Fact::NotDeclared` rather
    /// than a guessed value such as `COMMON_90K`.
    #[test]
    fn descriptor_reports_unavailable_time_base() {
        let descriptor = StreamDescriptor {
            id: StreamId(2),
            kind: StreamKind::Audio,
            time_base: Fact::NotDeclared,
            duration: None,
        };
        assert!(descriptor.time_base.is_unavailable());
        assert_eq!(descriptor.time_base.value(), None);
    }
}
