//! `attach()` — the non-RT seam that validates a declared bus table,
//! cross-checks the host's bus report, mints the RT endpoint and wires
//! up the node + control pair.
//!
//! Invariants the caller must have established first (the host
//! enforces them on `prepare_endpoint`):
//! `Initialize` → `Activate(config)` → `StartProcessing` have already
//! run, so the endpoint attaches against a `Processing` instance.

use std::sync::Arc;

use ruviform_audio_core::midi_input::TimedMidiInput;
use ruviform_media_core::{DeliveryPolicy, InBlockTransportSupport, ScheduleBudget, TimeBase};
use ruviform_plugin_core::{BusDescriptor, ProcessConfig, contract_from_process_config};
use ruviform_plugin_host::{BusReport, EndpointBounds, HostError, PluginHandle, PluginHost};
use ruviform_rt::channel;

use crate::bus::{BusError, bus_to_port_def, validate_buses};
use crate::control::{NodeControl, NodeReport, PluginNodeControl};
use crate::input::{INPUT_STAGE_CAPACITY, InputConfig, InputStage, InputTx};
use crate::node::{CONTROL_CHANNEL_CAPACITY, PluginNode, REPORT_CHANNEL_CAPACITY};
use crate::status::NodeStatus;
use ruviform_graph::PortDef;

/// Everything `attach` needs to mint a [`PluginNode`].
#[derive(Clone, Debug)]
pub struct AttachConfig {
    /// The consumer-declared bus table — the exact `process` array
    /// order (inputs and outputs may interleave; the node preserves
    /// per-direction declaration order against `NodeIo` port index).
    pub buses: Vec<BusDescriptor>,
    /// The audio format the graph binds on every port — the descriptor
    /// supplies the layout; `sample_rate`/`sample_format` come from
    /// the graph's format.
    pub port_format: ruviform_audio_core::AudioFormat,
    /// Endpoint attach bounds — `max_frames` must be `>=`
    /// `process_config.max_frames`; every declared bus's channel count
    /// must be `<= bounds.channels`.
    pub bounds: EndpointBounds,
    /// The activated `ProcessConfig` — source of the `BlockContract`
    /// (`min_frames`/`max_frames`) the node's physical calls obey.
    pub process_config: ProcessConfig,
    /// Engine frame base (one tick = one frame).
    pub frame_base: TimeBase,
    /// Exact vs Degraded delivery policy.
    pub policy: DeliveryPolicy,
    /// Bounded-work budgets.
    pub budget: ScheduleBudget,
    /// Enables the live-MIDI input leg: the node gains an
    /// [`InputStage`] fed through a bounded channel whose non-RT
    /// producer half comes back as [`Attached::input_tx`].
    /// `None` — no input path (plan/chase/release still run).
    pub input: Option<InputConfig>,
}

/// What `attach` returns: the RT node, its non-RT control twin and the
/// typed port declarations for the graph `NodeDef`.
pub struct Attached {
    /// The RT-side node — move into the graph via the factory.
    pub node: PluginNode,
    /// The non-RT control handle.
    pub control: PluginNodeControl,
    /// `PortDef`s in `buses` declaration order — build the graph node
    /// spec with exactly these (index = same-direction bus ordinal).
    pub ports: Vec<PortDef>,
    /// The non-RT pump half of the input leg — `Some` only when
    /// [`AttachConfig::input`] enabled it. Drive it once per engine
    /// block (or faster) with [`InputTx::pump`] against an open
    /// `MidiInput`.
    pub input_tx: Option<InputTx>,
}

/// Defined attach failure.
#[non_exhaustive]
#[derive(Debug)]
pub enum AttachError {
    /// The declared/reported bus tables disagree or are malformed.
    Bus(BusError),
    /// A declared bus's channel count exceeds `bounds.channels`, or
    /// `bounds.max_frames < process_config.max_frames`.
    BoundsMismatch,
    /// `AttachConfig::input.channel_capacity` is `0` or exceeds
    /// [`INPUT_STAGE_CAPACITY`].
    InputConfig,
    /// The host rejected (`prepare_endpoint` / `buses`).
    Host(HostError),
}

impl core::fmt::Display for AttachError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Bus(e) => write!(f, "bus table invalid: {e}"),
            Self::BoundsMismatch => f.write_str("endpoint bounds incompatible with config/buses"),
            Self::InputConfig => f.write_str("input channel capacity out of range"),
            Self::Host(e) => write!(f, "host error: {e}"),
        }
    }
}

impl core::error::Error for AttachError {}

/// Validates the bus table (against the host report when the layer can
/// produce one — `Unavailable` is recorded as *unverified*, not an
/// error), mints the endpoint, and returns the node/control/ports
/// triple.
///
/// **RT contract:** non-RT only.
///
/// # Errors
///
/// [`AttachError::Bus`], [`AttachError::BoundsMismatch`] or
/// [`AttachError::Host`].
pub fn attach(
    host: &mut PluginHost,
    handle: PluginHandle,
    cfg: &AttachConfig,
) -> Result<Attached, AttachError> {
    validate_buses(&cfg.buses, None).map_err(AttachError::Bus)?;
    // L1 reports `Unavailable` (no wire verb) — unverified, not
    // failed; the capability matrix records it and validation
    // continues on the declared table alone. Future `BusReport`
    // variants are likewise treated as unverified, never as failure.
    if let BusReport::Reported(reported) = host.buses(handle).map_err(AttachError::Host)? {
        validate_buses(&cfg.buses, Some(&reported)).map_err(AttachError::Bus)?;
    }
    let cfg_max = cfg.process_config.max_frames();
    let buses_fit = cfg
        .buses
        .iter()
        .all(|b| b.channels() <= cfg.bounds.channels);
    if cfg.bounds.max_frames < cfg_max || !buses_fit {
        return Err(AttachError::BoundsMismatch);
    }
    let ports: Vec<PortDef> = cfg
        .buses
        .iter()
        .map(|b| bus_to_port_def(b, cfg.port_format))
        .collect();
    let endpoint = host
        .prepare_endpoint(handle, cfg.bounds)
        .map_err(AttachError::Host)?;
    let contract =
        contract_from_process_config(&cfg.process_config, InBlockTransportSupport::Unknown);
    let (ctrl_tx, ctrl_rx) = channel::<NodeControl>(CONTROL_CHANNEL_CAPACITY);
    let (report_tx, report_rx) = channel::<NodeReport>(REPORT_CHANNEL_CAPACITY);
    let status = Arc::new(NodeStatus::default());
    let epoch = ruviform_media_core::ClockEpochId::new(0);
    let (input_stage, input_rx, input_tx) = match cfg.input {
        Some(config) => {
            if config.channel_capacity == 0 || config.channel_capacity > INPUT_STAGE_CAPACITY {
                return Err(AttachError::InputConfig);
            }
            let (tx, rx) = channel::<TimedMidiInput>(config.channel_capacity);
            (
                Some(InputStage::new(cfg.frame_base)),
                Some(rx),
                Some(InputTx::new(tx)),
            )
        }
        None => (None, None, None),
    };
    let node = PluginNode::new(
        endpoint,
        &cfg.buses,
        contract,
        cfg.policy,
        cfg.budget,
        cfg.frame_base,
        epoch,
        ctrl_rx,
        report_tx,
        Arc::clone(&status),
        cfg.bounds.max_frames as usize,
        input_stage,
        input_rx,
    );
    // Single origin: `cfg.frame_base` feeds both halves, so
    // `control.frame_base()` and `node.frame_base()` can never diverge.
    let control = PluginNodeControl::new(ctrl_tx, report_rx, status, cfg.frame_base);
    Ok(Attached {
        node,
        control,
        ports,
        input_tx,
    })
}
