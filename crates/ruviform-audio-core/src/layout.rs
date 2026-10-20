//! Channel layouts and speaker masks.
//!
//! [`SpeakerMask`] is a bit set of speaker positions using the
//! `WAVEFORMATEX`-style low-order-bit convention. [`ChannelLayout`] names
//! the standard layouts; the variant — not a separate field — owns the
//! channel count (`channels` counts set mask bits). [`SpeakerMaskSet`]
//! wraps `SpeakerMask` with a non-empty invariant — the
//! [`ChannelLayout::Custom`] payload, so a zero-channel layout cannot
//! be built.

use ruviform_media_core::{Error, Result};

/// Bit set of speaker positions, `WAVEFORMATEX` channel-mask convention.
///
/// Bits beyond the named positions are preserved as-is by
/// [`SpeakerMask::from_bits`] (reserved positions) and still count toward
/// [`ChannelLayout::channels`].
///
/// **RT contract:** RT-suitable — `Copy` value type; all operations are
/// `const` integer math.
///
/// # Examples
///
/// ```
/// use ruviform_audio_core::SpeakerMask;
///
/// let stereo = SpeakerMask::FRONT_LEFT.union(SpeakerMask::FRONT_RIGHT);
/// assert!(stereo.contains(SpeakerMask::FRONT_LEFT));
/// assert!(!stereo.contains(SpeakerMask::LOW_FREQUENCY));
/// assert_eq!(stereo.bits(), 0x3);
/// ```
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct SpeakerMask(u32);

impl SpeakerMask {
    /// Front left speaker.
    pub const FRONT_LEFT: Self = Self(0x1);
    /// Front right speaker.
    pub const FRONT_RIGHT: Self = Self(0x2);
    /// Front center speaker.
    pub const FRONT_CENTER: Self = Self(0x4);
    /// Low-frequency effects (subwoofer) channel.
    pub const LOW_FREQUENCY: Self = Self(0x8);
    /// Back left speaker.
    pub const BACK_LEFT: Self = Self(0x10);
    /// Back right speaker.
    pub const BACK_RIGHT: Self = Self(0x20);
    /// Side left speaker.
    pub const SIDE_LEFT: Self = Self(0x40);
    /// Side right speaker.
    pub const SIDE_RIGHT: Self = Self(0x80);

    /// The empty mask — no speakers.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Wraps raw mask bits, preserving bits beyond the named positions.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn from_bits(bits: u32) -> Self {
        Self(bits)
    }

    /// The raw mask bits.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn bits(self) -> u32 {
        self.0
    }

    /// Whether no speaker bit is set.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Whether every speaker in `other` is also in `self` — the empty mask
    /// is contained in every mask.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Bitwise union of two masks.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// A non-empty [`SpeakerMask`] — the [`ChannelLayout::Custom`] payload.
///
/// `Custom` layouts must name at least one channel; this newtype makes
/// the invariant structural so a zero-channel `ChannelLayout` cannot
/// be built. [`SpeakerMask`] itself is unchanged — the empty mask
/// remains a legal mask value outside `Custom`.
///
/// **RT contract:** RT-suitable — `Copy` value type; `const` ops.
///
/// # Examples
///
/// ```
/// use ruviform_audio_core::{SpeakerMask, SpeakerMaskSet};
/// use ruviform_media_core::Error;
///
/// let set = SpeakerMaskSet::new(SpeakerMask::FRONT_LEFT).unwrap();
/// assert_eq!(set.mask(), SpeakerMask::FRONT_LEFT);
/// assert_eq!(
///     SpeakerMaskSet::new(SpeakerMask::empty()),
///     Err(Error::InvalidValue("mask"))
/// );
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SpeakerMaskSet(SpeakerMask);

impl SpeakerMaskSet {
    /// Validated non-empty mask.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] when `mask` is empty.
    pub const fn new(mask: SpeakerMask) -> Result<Self> {
        if mask.is_empty() {
            Err(Error::InvalidValue("mask"))
        } else {
            Ok(Self(mask))
        }
    }

    /// The wrapped mask — always non-empty.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn mask(self) -> SpeakerMask {
        self.0
    }
}

/// A named channel layout, or a custom speaker mask set.
///
/// The variant owns the channel count: [`ChannelLayout::channels`]
/// derives it from the speaker mask. `Custom` carries a
/// [`SpeakerMaskSet`] — non-empty by construction — so a zero-channel
/// `ChannelLayout` cannot exist. Building a `Custom` layout from a raw
/// [`SpeakerMask`] goes through [`ChannelLayout::custom`], which
/// returns `Err` on an empty mask.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy — new variants may be added without breaking downstream
/// `match` arms.
///
/// **RT contract:** RT-suitable — `Copy` value type.
///
/// # Examples
///
/// ```
/// use ruviform_audio_core::{ChannelLayout, SpeakerMask};
///
/// assert_eq!(ChannelLayout::Stereo.channels(), 2);
/// assert_eq!(ChannelLayout::FivePointOne.channels(), 6);
/// assert!(ChannelLayout::custom(SpeakerMask::empty()).is_err());
/// ```
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ChannelLayout {
    /// One channel: front center.
    Mono,
    /// Two channels: front left + front right.
    Stereo,
    /// Four channels: front left/right + back left/right.
    Quad,
    /// Six channels: stereo + front center + low frequency + back
    /// left/right.
    FivePointOne,
    /// Eight channels: 5.1 + side left/right.
    SevenPointOne,
    /// Custom speaker mask set — non-empty by construction
    /// ([`SpeakerMaskSet`]); build from a [`SpeakerMask`] via
    /// [`ChannelLayout::custom`].
    Custom(SpeakerMaskSet),
}

impl ChannelLayout {
    /// Number of channels — the number of set bits in
    /// [`ChannelLayout::speaker_mask`].
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn channels(self) -> u32 {
        self.speaker_mask().bits().count_ones()
    }

    /// The speaker mask of this layout.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn speaker_mask(self) -> SpeakerMask {
        match self {
            Self::Mono => SpeakerMask::FRONT_CENTER,
            Self::Stereo => SpeakerMask::FRONT_LEFT.union(SpeakerMask::FRONT_RIGHT),
            Self::Quad => SpeakerMask::FRONT_LEFT
                .union(SpeakerMask::FRONT_RIGHT)
                .union(SpeakerMask::BACK_LEFT)
                .union(SpeakerMask::BACK_RIGHT),
            Self::FivePointOne => SpeakerMask::FRONT_LEFT
                .union(SpeakerMask::FRONT_RIGHT)
                .union(SpeakerMask::FRONT_CENTER)
                .union(SpeakerMask::LOW_FREQUENCY)
                .union(SpeakerMask::BACK_LEFT)
                .union(SpeakerMask::BACK_RIGHT),
            Self::SevenPointOne => SpeakerMask::FRONT_LEFT
                .union(SpeakerMask::FRONT_RIGHT)
                .union(SpeakerMask::FRONT_CENTER)
                .union(SpeakerMask::LOW_FREQUENCY)
                .union(SpeakerMask::BACK_LEFT)
                .union(SpeakerMask::BACK_RIGHT)
                .union(SpeakerMask::SIDE_LEFT)
                .union(SpeakerMask::SIDE_RIGHT),
            Self::Custom(mask_set) => mask_set.mask(),
        }
    }

    /// Validated custom layout — rejects an empty mask (zero channels)
    /// by wrapping it in [`SpeakerMaskSet`].
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] when `mask` is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_audio_core::{ChannelLayout, SpeakerMask};
    /// use ruviform_media_core::Error;
    ///
    /// let layout = ChannelLayout::custom(SpeakerMask::FRONT_LEFT).unwrap();
    /// assert_eq!(layout.channels(), 1);
    /// assert_eq!(
    ///     ChannelLayout::custom(SpeakerMask::empty()),
    ///     Err(Error::InvalidValue("mask"))
    /// );
    /// ```
    pub const fn custom(mask: SpeakerMask) -> Result<Self> {
        match SpeakerMaskSet::new(mask) {
            Ok(mask_set) => Ok(Self::Custom(mask_set)),
            Err(error) => Err(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speaker_mask_bit_operations() {
        let mask = SpeakerMask::FRONT_LEFT.union(SpeakerMask::LOW_FREQUENCY);
        assert_eq!(mask.bits(), 0x9);
        assert!(mask.contains(SpeakerMask::FRONT_LEFT));
        assert!(!mask.contains(SpeakerMask::BACK_LEFT));
        // The empty mask is contained in every mask.
        assert!(mask.contains(SpeakerMask::empty()));
        assert!(SpeakerMask::empty().is_empty());
        assert!(!mask.is_empty());
        // Bits beyond the named set are preserved.
        assert_eq!(SpeakerMask::from_bits(0x100).bits(), 0x100);
    }

    #[test]
    fn known_layouts_map_to_their_masks() {
        assert_eq!(
            ChannelLayout::Mono.speaker_mask(),
            SpeakerMask::FRONT_CENTER
        );
        assert_eq!(ChannelLayout::Mono.channels(), 1);
        assert_eq!(ChannelLayout::Stereo.speaker_mask().bits(), 0x3);
        assert_eq!(ChannelLayout::Stereo.channels(), 2);
        assert_eq!(ChannelLayout::Quad.speaker_mask().bits(), 0x33);
        assert_eq!(ChannelLayout::Quad.channels(), 4);
        assert_eq!(ChannelLayout::FivePointOne.speaker_mask().bits(), 0x3F);
        assert_eq!(ChannelLayout::FivePointOne.channels(), 6);
        assert_eq!(ChannelLayout::SevenPointOne.speaker_mask().bits(), 0xFF);
        assert_eq!(ChannelLayout::SevenPointOne.channels(), 8);
    }

    #[test]
    fn custom_layout_keeps_its_mask() {
        let mask = SpeakerMask::FRONT_CENTER
            .union(SpeakerMask::BACK_LEFT)
            .union(SpeakerMask::from_bits(0x100));
        let layout = ChannelLayout::custom(mask).unwrap();
        assert_eq!(
            layout,
            ChannelLayout::Custom(SpeakerMaskSet::new(mask).unwrap())
        );
        assert_eq!(layout.speaker_mask(), mask);
        assert_eq!(layout.channels(), 3);
    }

    #[test]
    fn custom_layout_rejects_empty_mask() {
        assert_eq!(
            ChannelLayout::custom(SpeakerMask::empty()),
            Err(Error::InvalidValue("mask"))
        );
    }

    #[test]
    fn custom_payload_is_structurally_non_empty() {
        // `SpeakerMaskSet::new` rejects the empty mask, so no
        // `ChannelLayout::Custom` value can be zero-channel — the
        // degenerate layout the variant doc used to warn about is now
        // unrepresentable.
        assert_eq!(
            SpeakerMaskSet::new(SpeakerMask::empty()),
            Err(Error::InvalidValue("mask"))
        );
        let set = SpeakerMaskSet::new(SpeakerMask::BACK_RIGHT).unwrap();
        assert_eq!(set.mask(), SpeakerMask::BACK_RIGHT);
        assert_eq!(ChannelLayout::Custom(set).channels(), 1);
    }
}
