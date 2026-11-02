//! Declared plugin buses → typed-graph ports (`Issue #196`, M4-02).
//!
//! The consumer declares the bus table it intends to present
//! ([`BusDescriptor`]), the bridge maps each entry onto a [`PortDef`]
//! (bus index `i` in each direction ↔ `NodeIo::input(i)`/`output(i)`),
//! and — when the host can report the plugin's own table — cross-checks
//! it ([`validate_buses`]). Everything happens at attach/compile time:
//! no RT path ever changes bus count or infers a channel conversion.

use ruviform_audio_core::AudioFormat;
use ruviform_graph::{
    Activation, Availability, Direction, Multiplicity, PortDef, PortFormat, PortRole,
};
use ruviform_plugin_core::{BusDescriptor, BusDirection, BusRole};

/// `BusDescriptor` → typed `PortRole`. `#[non_exhaustive]` roles fall
/// back to `PortRole::Aux` so a future plugin role degrades to the
/// least-privileged structural class rather than a lie.
///
/// **RT contract:** RT-suitable — `const`, pure.
#[must_use]
pub const fn role_to_port_role(role: BusRole) -> PortRole {
    match role {
        BusRole::Main => PortRole::Main,
        BusRole::Sidechain => PortRole::Sidechain,
        BusRole::Send => PortRole::Send,
        BusRole::Return => PortRole::Return,
        _ => PortRole::Aux,
    }
}

/// Maps one declared bus onto a typed [`PortDef`]: direction and role
/// from the descriptor, `AudioFormat` supplied by the consumer (the
/// descriptor's `layout` sets the channel count, `format`'s
/// `sample_rate`/`sample_format` are carried through), `enabled` →
/// `Activation`. A `Main` bus is `Availability::Always` (the graph
/// requires its connections); other roles are `Optional`. Multiplicity
/// is `unbounded(0)` — the plugin bus accepts any fan-in the graph
/// decides; per-connection `ChannelMap`s live on `ConnDef`, not here.
///
/// **RT contract:** non-RT — `PortDef.capabilities` is a `Vec`.
#[must_use]
pub fn bus_to_port_def(bus: &BusDescriptor, format: AudioFormat) -> PortDef {
    let mut format = format;
    format.layout = bus.layout;
    PortDef {
        direction: match bus.direction {
            BusDirection::Output => Direction::Output,
            _ => Direction::Input,
        },
        role: role_to_port_role(bus.role),
        format: PortFormat::Audio(format),
        multiplicity: Multiplicity::unbounded(0),
        capabilities: Vec::new(),
        availability: match bus.role {
            BusRole::Main => Availability::Always,
            _ => Availability::Optional,
        },
        activation: if bus.enabled {
            Activation::Enabled
        } else {
            Activation::Disabled
        },
    }
}

/// Defined bus-table failure — every variant names the first offending
/// entry so the caller can reject with evidence.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BusError {
    /// The consumer declared no buses.
    EmptyDecl,
    /// More buses than [`crate::MAX_BUSES`].
    TooManyBuses {
        /// Declared.
        count: usize,
        /// Bound.
        max: usize,
    },
    /// The plugin's reported table has fewer buses of a direction
    /// than the consumer declared.
    MissingPort {
        /// `0` input, `1` output.
        direction: u8,
        /// First missing bus index within the direction.
        index: usize,
    },
    /// Reported bus direction differs from the declaration at `index`
    /// (within the flattened declared order).
    DirectionMismatch {
        /// Flattened declared index.
        index: usize,
    },
    /// Reported bus role differs from the declaration.
    RoleMismatch {
        /// Flattened declared index.
        index: usize,
    },
    /// Reported bus channel layout differs from the declaration.
    LayoutMismatch {
        /// Flattened declared index.
        index: usize,
    },
}

impl core::fmt::Display for BusError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::EmptyDecl => f.write_str("no buses declared"),
            Self::TooManyBuses { count, max } => {
                write!(f, "declared {count} buses exceeds bound {max}")
            }
            Self::MissingPort { direction, index } => write!(
                f,
                "plugin reports no {} bus {index}",
                if *direction == 0 { "input" } else { "output" }
            ),
            Self::DirectionMismatch { index } => {
                write!(f, "reported bus {index} direction differs from declaration")
            }
            Self::RoleMismatch { index } => {
                write!(f, "reported bus {index} role differs from declaration")
            }
            Self::LayoutMismatch { index } => {
                write!(f, "reported bus {index} layout differs from declaration")
            }
        }
    }
}

impl core::error::Error for BusError {}

/// Validates the consumer's declared bus table — and, when the host
/// reported the plugin's own table (`reported: Some`), requires every
/// declared bus to exist in the report with matching direction, role
/// and layout. `reported: None` (L1 / undeclared) skips the
/// cross-check: *unverified*, not failed.
///
/// Ordering rule: `declared` is the flattened plugin-facing order —
/// all input buses first (in `process` array order), then all output
/// buses. The report is checked in the same flattened order, so a
/// standard that interleaves directions must pre-flatten.
///
/// **RT contract:** non-RT only.
///
/// # Errors
///
/// The first [`BusError`] encountered.
pub fn validate_buses(
    declared: &[BusDescriptor],
    reported: Option<&[BusDescriptor]>,
) -> Result<(), BusError> {
    if declared.is_empty() {
        return Err(BusError::EmptyDecl);
    }
    if declared.len() > crate::MAX_BUSES {
        return Err(BusError::TooManyBuses {
            count: declared.len(),
            max: crate::MAX_BUSES,
        });
    }
    let declared_inputs = declared
        .iter()
        .filter(|b| matches!(b.direction, BusDirection::Input))
        .count();
    let declared_outputs = declared.len() - declared_inputs;
    if let Some(reported) = reported {
        let reported_inputs = reported
            .iter()
            .filter(|b| matches!(b.direction, BusDirection::Input))
            .count();
        let reported_outputs = reported.len() - reported_inputs;
        if reported_inputs < declared_inputs {
            return Err(BusError::MissingPort {
                direction: 0,
                index: reported_inputs,
            });
        }
        if reported_outputs < declared_outputs {
            return Err(BusError::MissingPort {
                direction: 1,
                index: reported_outputs,
            });
        }
        // Compare each declared bus against the same-index reported
        // bus of its direction — `MissingPort` already proved every
        // `next()` yields `Some`.
        let mut rep_in = reported
            .iter()
            .filter(|b| matches!(b.direction, BusDirection::Input));
        let mut rep_out = reported
            .iter()
            .filter(|b| matches!(b.direction, BusDirection::Output));
        for (i, decl) in declared.iter().enumerate() {
            let rep = match decl.direction {
                BusDirection::Input => rep_in.next(),
                _ => rep_out.next(),
            };
            let Some(rep) = rep else {
                debug_assert!(false, "MissingPort check precedes comparison");
                continue;
            };
            if decl.direction != rep.direction {
                return Err(BusError::DirectionMismatch { index: i });
            }
            if decl.role != rep.role {
                return Err(BusError::RoleMismatch { index: i });
            }
            if decl.layout != rep.layout {
                return Err(BusError::LayoutMismatch { index: i });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruviform_audio_core::{ChannelLayout, SampleFormat, SampleRate};

    fn fmt() -> AudioFormat {
        AudioFormat {
            sample_rate: SampleRate::new(48_000).unwrap(),
            layout: ChannelLayout::Stereo,
            sample_format: SampleFormat::F32,
        }
    }

    const IN_MAIN: BusDescriptor = BusDescriptor {
        direction: BusDirection::Input,
        layout: ChannelLayout::Stereo,
        name: "in",
        role: BusRole::Main,
        enabled: true,
    };
    const OUT_MAIN: BusDescriptor = BusDescriptor {
        direction: BusDirection::Output,
        layout: ChannelLayout::Stereo,
        name: "out",
        role: BusRole::Main,
        enabled: true,
    };
    const IN_SC: BusDescriptor = BusDescriptor {
        direction: BusDirection::Input,
        layout: ChannelLayout::Mono,
        name: "sc",
        role: BusRole::Sidechain,
        enabled: true,
    };

    #[test]
    fn bus_to_port_def_maps_role_layout_activation() {
        let def = bus_to_port_def(&IN_SC, fmt());
        assert_eq!(def.direction, Direction::Input);
        assert_eq!(def.role, PortRole::Sidechain);
        assert_eq!(def.activation, Activation::Enabled);
        assert_eq!(def.availability, Availability::Optional);
        let PortFormat::Audio(af) = def.format else {
            panic!("audio format expected")
        };
        assert_eq!(af.layout.channels(), 1);
        let main = bus_to_port_def(&OUT_MAIN, fmt());
        assert_eq!(main.availability, Availability::Always);
        let disabled = BusDescriptor {
            enabled: false,
            ..IN_MAIN
        };
        assert_eq!(
            bus_to_port_def(&disabled, fmt()).activation,
            Activation::Disabled
        );
    }

    #[test]
    fn validate_declared_against_report() {
        let declared = [IN_MAIN, IN_SC, OUT_MAIN];
        let reported = [IN_MAIN, IN_SC, OUT_MAIN];
        assert!(validate_buses(&declared, Some(&reported)).is_ok());
        assert!(validate_buses(&declared, None).is_ok());
        let short = [IN_MAIN, OUT_MAIN];
        assert_eq!(
            validate_buses(&declared, Some(&short)),
            Err(BusError::MissingPort {
                direction: 0,
                index: 1
            })
        );
        let wrong_role = [
            IN_MAIN,
            BusDescriptor {
                role: BusRole::Aux,
                ..IN_SC
            },
            OUT_MAIN,
        ];
        assert_eq!(
            validate_buses(&declared, Some(&wrong_role)),
            Err(BusError::RoleMismatch { index: 1 })
        );
        assert_eq!(validate_buses(&[], None), Err(BusError::EmptyDecl));
    }
}
