//! Standard-neutral audio bus / port description: [`BusDirection`]
//! and [`BusDescriptor`].
//!
//! A descriptor declares one bus: its direction, channel layout (which
//! implies the channel count), display name and enablement. How a host
//! activates or arranges buses is adapter state, not part of the
//! descriptor.

use ruviform_audio_core::ChannelLayout;

/// Whether a bus carries audio into or out of the plugin.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy — new variants may be added without breaking downstream
/// `match` arms.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum BusDirection {
    /// Host → plugin.
    Input,
    /// Plugin → host.
    Output,
}

/// Standard-neutral audio bus descriptor.
///
/// `layout` both fixes the channel count ([`BusDescriptor::channels`])
/// and describes speaker placement. `main` marks the standard-neutral
/// "main" bus each standard defines (CLAP `is_main`, VST3 `kMain`).
/// `enabled` declares host-visible availability: a bus the host leaves
/// disabled is *not* presented to processing — adapters build empty
/// views for it instead of touching its pointers.
///
/// **RT contract:** RT-suitable — `Copy` value type.
///
/// # Examples
///
/// ```
/// use ruviform_audio_core::ChannelLayout;
/// use ruviform_plugin_core::{BusDescriptor, BusDirection};
///
/// const MAIN_OUT: BusDescriptor = BusDescriptor {
///     direction: BusDirection::Output,
///     layout: ChannelLayout::Stereo,
///     name: "Main Out",
///     main: true,
///     enabled: true,
/// };
/// assert_eq!(MAIN_OUT.channels(), 2);
/// ```
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct BusDescriptor {
    /// Whether this is an input or output bus.
    pub direction: BusDirection,
    /// Channel layout — fixes the channel count.
    pub layout: ChannelLayout,
    /// Human-readable bus name.
    pub name: &'static str,
    /// Whether this is the plugin's main bus of its direction.
    pub main: bool,
    /// Whether the bus is available for the host to activate.
    pub enabled: bool,
}

impl BusDescriptor {
    /// Number of channels — [`ChannelLayout::channels`].
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn channels(self) -> u32 {
        self.layout.channels()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channels_comes_from_layout() {
        const BUS: BusDescriptor = BusDescriptor {
            direction: BusDirection::Input,
            layout: ChannelLayout::FivePointOne,
            name: "in",
            main: true,
            enabled: true,
        };
        assert_eq!(BUS.channels(), 6);
        let off = BusDescriptor {
            enabled: false,
            ..BUS
        };
        assert!(!off.enabled);
        assert_eq!(off.direction, BusDirection::Input);
    }
}
