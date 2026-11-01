//! Port attribute model — orthogonal direction/domain/role/format/
//! multiplicity/capability/availability/activation attributes plus the
//! explicit [`ChannelMap`] conversion declaration.

use ruviform_audio_core::AudioFormat;

/// Whether a port consumes (`Input`) or produces (`Output`) signal.
///
/// Connections run `Output` → `Input`; any other direction pair is
/// rejected as [`crate::ValidationError::InvalidDirection`].
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Direction {
    /// Consumes signal produced elsewhere.
    Input,
    /// Produces signal consumed elsewhere.
    Output,
}

/// The signal domain a port carries, derived from its
/// [`PortFormat`] — never an independent field (single source of
/// truth).
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum SignalDomain {
    /// Sampled audio stream (PCM over an [`AudioFormat`]).
    Audio,
    /// Timestamped event stream (MIDI-like; payload shape is M3+).
    TimedEvent,
    /// Sampled/quantized control-rate stream.
    Control,
}

/// Semantic role of a bus port — purely structural metadata.
///
/// Roles carry no direction or DAW-specific meaning: `Sidechain` is an
/// independent input, `Send`/`Return` name send/return buses without
/// assigning send-level semantics.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum PortRole {
    /// Primary bus.
    Main,
    /// Auxiliary bus.
    Aux,
    /// Independent key/sidechain input.
    Sidechain,
    /// Send bus tap.
    Send,
    /// Return bus feed.
    Return,
}

/// The payload format of a port — and thereby its
/// [`SignalDomain`].
///
/// Audio ports reuse [`ruviform_audio_core::AudioFormat`] so sample
/// rate / channel layout / sample format are the foundation type's
/// fields; event and control payloads carry no M1 format payload
/// (typed payload shapes are M3+).
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum PortFormat {
    /// Sampled audio with a concrete stream format.
    Audio(AudioFormat),
    /// Timestamped event stream; no typed payload in M1.
    TimedEvent,
    /// Control-rate stream; no typed payload in M1.
    Control,
}

impl PortFormat {
    /// The domain this format belongs to — the single source of truth
    /// for domain checks.
    #[must_use]
    pub const fn domain(&self) -> SignalDomain {
        match self {
            Self::Audio(_) => SignalDomain::Audio,
            Self::TimedEvent => SignalDomain::TimedEvent,
            Self::Control => SignalDomain::Control,
        }
    }
}

/// Declared connection-count bounds of a port: at least `min`
/// connections may be required and at most `max` are allowed.
///
/// `max == u16::MAX` means *unbounded*; `min <= max` is enforced by
/// the constructor.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Multiplicity {
    /// Required lower bound — enforced only when the port is
    /// [`Activation::Enabled`] and [`Availability::Always`].
    pub min: u16,
    /// Upper bound (`u16::MAX` = unbounded).
    pub max: u16,
}

impl Multiplicity {
    /// Unbounded upper bound sentinel.
    pub const UNBOUNDED: u16 = u16::MAX;

    /// Builds bounds; returns `None` when `min > max`.
    #[must_use]
    pub const fn new(min: u16, max: u16) -> Option<Self> {
        if min > max {
            None
        } else {
            Some(Self { min, max })
        }
    }

    /// Exactly `n` connections required and allowed.
    #[must_use]
    pub const fn exactly(n: u16) -> Self {
        Self { min: n, max: n }
    }

    /// `min` required, unbounded allowed.
    #[must_use]
    pub const fn unbounded(min: u16) -> Self {
        Self {
            min,
            max: Self::UNBOUNDED,
        }
    }
}

/// Optional capabilities a port advertises. M1 records them; runtime
/// behavior (latency reporting, compensation, probing) belongs to the
/// M2+ consumer.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum PortCapability {
    /// Port reports latency for downstream delay compensation.
    LatencyReporting,
    /// Port participates in plugin delay compensation.
    DelayCompensation,
    /// Port may carry an observation probe/tap (#185 boundary).
    ProbePoint,
}

/// Whether a port's presence is mandatory for the node's operation.
///
/// `Optional` ports that stay unconnected do not raise
/// [`crate::ValidationError::RequiredConnections`]; `Always` ports
/// enforce `Multiplicity::min` when enabled.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Availability {
    /// Always present — `min` connections are required when enabled.
    Always,
    /// May be absent — `min` is waived entirely.
    Optional,
}

/// Whether a declared port participates in the graph.
///
/// A `Disabled` port remains declared (its `PortSpec` exists) but its
/// connection requirements — both `min` and incoming/outgoing
/// connection checks tied to requirement — are waived. Existing
/// connections to a disabled port still validate direction/format
/// normally.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Activation {
    /// Port is active; multiplicity rules apply.
    Enabled,
    /// Port is declared but inactive; `min` requirement is waived.
    Disabled,
}

/// Builder-side port declaration — ID-free. The owning
/// [`crate::GraphBuilder::add_node`] issues the stable `PortId`.
#[derive(Clone, Debug)]
pub struct PortDef {
    /// Input or output.
    pub direction: Direction,
    /// Structural bus role.
    pub role: PortRole,
    /// Payload format (carries the domain).
    pub format: PortFormat,
    /// Connection-count bounds.
    pub multiplicity: Multiplicity,
    /// Advertised capabilities.
    pub capabilities: Vec<PortCapability>,
    /// Mandatory vs. optional presence.
    pub availability: Availability,
    /// Enabled vs. declared-but-inactive.
    pub activation: Activation,
}

/// Explicit source→destination channel mapping declared on a
/// connection between audio ports whose layouts differ.
///
/// Entries are `(source_channel, destination_channel)` index pairs.
/// Validation requires: all indices in range of the respective
/// layouts, every destination channel covered exactly once, no
/// destination duplicated; sources may repeat (duplication permitted),
/// partial maps rejected — implicit upmix/downmix is never inferred.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChannelMap {
    /// `(src_ch, dst_ch)` pairs.
    pub entries: Vec<(u16, u16)>,
}
