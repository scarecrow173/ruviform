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

/// The role a bus plays in the plugin's routing topology.
///
/// Maps onto the standard-neutral roles a plugin standard exposes:
/// CLAP's main/aux audio-ports and note-ports, VST3's `kMain`/`kAux`
/// bus categories, and the generic send/return concept. `#[non_exhaustive]`
/// per the unified public-enum API policy.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum BusRole {
    /// The plugin's primary bus of its direction (CLAP `is_main`,
    /// VST3 `kMain`).
    Main,
    /// A secondary auxiliary bus (additional input or output path).
    Aux,
    /// A sidechain input bus — audio that modulates processing
    /// without being the primary material.
    Sidechain,
    /// A send bus (output that feeds an external chain).
    Send,
    /// A return bus (input fed back from an external chain).
    Return,
}

/// Standard-neutral audio bus descriptor.
///
/// `layout` both fixes the channel count ([`BusDescriptor::channels`])
/// and describes speaker placement. `role` names the standard-neutral
/// bus role each standard defines (CLAP main/aux, VST3 `kMain`/`kAux`);
/// [`BusRole::Main`] is the main bus of its direction. `enabled`
/// declares host-visible availability: a bus the host leaves
/// disabled is *not* presented to processing — adapters build empty
/// views for it instead of touching its pointers.
///
/// **RT contract:** RT-suitable — `Copy` value type.
///
/// # Examples
///
/// ```
/// use ruviform_audio_core::ChannelLayout;
/// use ruviform_plugin_core::{BusDescriptor, BusDirection, BusRole};
///
/// const MAIN_OUT: BusDescriptor = BusDescriptor {
///     direction: BusDirection::Output,
///     layout: ChannelLayout::Stereo,
///     name: "Main Out",
///     role: BusRole::Main,
///     enabled: true,
/// };
/// assert_eq!(MAIN_OUT.channels(), 2);
/// assert!(MAIN_OUT.is_main());
/// ```
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct BusDescriptor {
    /// Whether this is an input or output bus.
    pub direction: BusDirection,
    /// Channel layout — fixes the channel count.
    pub layout: ChannelLayout,
    /// Human-readable bus name.
    pub name: &'static str,
    /// The routing role of this bus ([`BusRole::Main`] marks the
    /// plugin's main bus of its direction).
    pub role: BusRole,
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

    /// Whether this is the plugin's main bus of its direction
    /// ([`BusRole::Main`]).
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn is_main(self) -> bool {
        matches!(self.role, BusRole::Main)
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
            role: BusRole::Main,
            enabled: true,
        };
        assert_eq!(BUS.channels(), 6);
        assert!(BUS.is_main());
        let aux = BusDescriptor {
            role: BusRole::Aux,
            ..BUS
        };
        assert!(!aux.is_main());
        let off = BusDescriptor {
            enabled: false,
            ..BUS
        };
        assert!(!off.enabled);
        assert_eq!(off.direction, BusDirection::Input);
    }
}
