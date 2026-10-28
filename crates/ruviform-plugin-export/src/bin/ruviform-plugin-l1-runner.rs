//! Reference L1 runner binary (`ruviform-plugin-l1-runner`) — the
//! spawn fixture for the `ruviform-plugin-export` E2E tests
//! (`tests/l1_runner_e2e.rs`, via `CARGO_BIN_EXE_*`).
//!
//! It embeds three mock factories:
//!
//! - `GainPlugin` (`dev.ruviform.mock.gain`) — a deterministic gain
//!   plugin: planar `f32` input × a single normalized gain parameter —
//!   each `ParamValue` event sets the block-wide gain (the latest
//!   event in the block wins; there is no per-offset ramp).
//!   Implements the full `Plugin` surface — `init`/
//!   `start_processing`/`stop_processing`/`save_state`/
//!   `restore_state` — so the E2E can drive the lifecycle and the
//!   state round-trip.
//! - `PanicPlugin` (`dev.ruviform.mock.panic`) — panics inside `init`;
//!   the deterministic fixture for the panic →
//!   `Err(HostError::PluginFailed)` contract (the child must survive
//!   and keep serving commands). Its *factory* `descriptor()` also
//!   panics when `RUVIFORM_PANIC_DESCRIPTOR` is set, and `create()`
//!   panics when `RUVIFORM_PANIC_CREATE` is set — the fixtures for
//!   guarded factory calls (`Scan`/`CreateInstance` →
//!   `Err(HostError::PluginFailed)` on the wire, child alive).
//! - `UnderwritePlugin` (`dev.ruviform.mock.underwrite`) — writes
//!   every output channel on its first `process` call but only
//!   channel 0 afterwards; the fixture proving the runner re-zeroes
//!   `staging_out` each block (a legal under-write must not leak the
//!   previous block's samples into the out-slot).
//!
//! Wire v2 takes no spawn arguments — the binary is spawned bare and
//! the host picks a plugin id per `CreateInstance`.

use std::process::ExitCode;

use ruviform_plugin_core::{
    BufferView, BufferViewMut, EventKind, EventList, HostError, ParamFlags, ParamId, ParamInfo,
    Plugin, PluginDescriptor, PluginFactory, PluginProcessor, PrepareError, ProcessConfig,
    ProcessContext, ProcessResult, TransportInfo, erase,
};

fn main() -> ExitCode {
    ruviform_plugin_export::run(vec![
        Box::new(GainFactory),
        Box::new(PanicFactory),
        Box::new(UnderwriteFactory),
    ])
}

const GAIN_DESCRIPTOR: PluginDescriptor = PluginDescriptor {
    id: "dev.ruviform.mock.gain",
    name: "Gain",
    vendor: "Ruviform",
    version: "1.0.0",
    features: &["audio-effect"],
};

/// The `Gain` parameter: plain range `[0.0, 2.0]` (linear
/// multiplier), default `1.0` — normalized `ParamValue` events are
/// denormalized through this table.
const GAIN_PARAMS: &[ParamInfo] = &[ParamInfo {
    id: ParamId(0),
    name: "Gain",
    unit: "",
    min_value: 0.0,
    max_value: 2.0,
    default_value: 1.0,
    flags: ParamFlags::AUTOMATABLE,
}];

/// The `save_state` blob: `"RFG1"` magic + `f32` LE gain.
const STATE_MAGIC: &[u8; 4] = b"RFG1";

/// The mock gain plugin — multiplies every input channel by the
/// current gain.
struct GainPlugin {
    /// Plain-domain gain multiplier.
    gain: f32,
}

impl GainPlugin {
    fn new() -> Self {
        Self { gain: 1.0 }
    }
}

impl PluginProcessor for GainPlugin {
    fn prepare(
        &mut self,
        _config: &ProcessConfig,
        _context: &mut ProcessContext,
    ) -> Result<(), PrepareError> {
        Ok(())
    }

    fn process<I, O>(
        &mut self,
        inputs: &[I],
        outputs: &mut [O],
        events: EventList<'_>,
        _transport: Option<TransportInfo>,
        _context: &mut ProcessContext,
    ) -> ProcessResult
    where
        I: BufferView,
        O: BufferViewMut,
    {
        for event in events.iter() {
            if let EventKind::ParamValue(param) = event.kind()
                && param.param_id == ParamId(0)
            {
                // The latest event wins and applies to the whole
                // block — no per-offset ramp. Plain range [0.0, 2.0]
                // fits an f32 losslessly at the f64→f32 boundary the
                // mock needs; denormalize is the ParamInfo contract.
                #[expect(clippy::cast_possible_truncation)]
                {
                    self.gain = GAIN_PARAMS[0].denormalize(param.value) as f32;
                }
            }
        }
        let (Some(input), Some(output)) = (inputs.first(), outputs.first_mut()) else {
            return ProcessResult::CONTINUE;
        };
        for channel in 0..input.channels().min(output.channels()) {
            let (Some(src), Some(dst)) = (input.channel(channel), output.channel_mut(channel))
            else {
                continue;
            };
            for (input_sample, output_sample) in src.iter().zip(dst.iter_mut()) {
                *output_sample = *input_sample * self.gain;
            }
        }
        ProcessResult::CONTINUE
    }
}

impl Plugin for GainPlugin {
    fn descriptor(&self) -> PluginDescriptor {
        GAIN_DESCRIPTOR
    }

    fn params(&self) -> &[ParamInfo] {
        GAIN_PARAMS
    }

    fn save_state(&mut self) -> Result<Vec<u8>, HostError> {
        let mut bytes = Vec::with_capacity(STATE_MAGIC.len() + 4);
        bytes.extend_from_slice(STATE_MAGIC);
        bytes.extend_from_slice(&self.gain.to_le_bytes());
        Ok(bytes)
    }

    fn restore_state(&mut self, state: &[u8]) -> Result<(), HostError> {
        let Some(gain_bytes) = state
            .strip_prefix(STATE_MAGIC.as_slice())
            .and_then(|rest| <[u8; 4]>::try_from(rest).ok())
        else {
            return Err(HostError::PluginFailed);
        };
        let gain = f32::from_le_bytes(gain_bytes);
        if !gain.is_finite() {
            return Err(HostError::PluginFailed);
        }
        self.gain = gain;
        Ok(())
    }
}

struct GainFactory;

impl PluginFactory for GainFactory {
    fn descriptor(&self) -> PluginDescriptor {
        GAIN_DESCRIPTOR
    }

    fn create(&self) -> Box<dyn ruviform_plugin_core::ErasedPlugin> {
        erase(GainPlugin::new())
    }
}

const PANIC_DESCRIPTOR: PluginDescriptor = PluginDescriptor {
    id: "dev.ruviform.mock.panic",
    name: "Panic",
    vendor: "Ruviform",
    version: "1.0.0",
    features: &["test-fixture"],
};

/// Deterministic panic fixture: `init` panics so the E2E can verify
/// the panic → `Err(HostError::PluginFailed)` mapping and that the
/// child keeps serving commands afterwards.
struct PanicPlugin;

impl PluginProcessor for PanicPlugin {
    fn prepare(
        &mut self,
        _config: &ProcessConfig,
        _context: &mut ProcessContext,
    ) -> Result<(), PrepareError> {
        Ok(())
    }

    fn process<I, O>(
        &mut self,
        _inputs: &[I],
        _outputs: &mut [O],
        _events: EventList<'_>,
        _transport: Option<TransportInfo>,
        _context: &mut ProcessContext,
    ) -> ProcessResult
    where
        I: BufferView,
        O: BufferViewMut,
    {
        ProcessResult::CONTINUE
    }
}

impl Plugin for PanicPlugin {
    fn descriptor(&self) -> PluginDescriptor {
        PANIC_DESCRIPTOR
    }

    fn params(&self) -> &[ParamInfo] {
        &[]
    }

    fn init(&mut self) -> Result<(), HostError> {
        panic!("ruviform-plugin-l1-runner: PanicPlugin init (deterministic E2E fixture)");
    }
}

struct PanicFactory;

impl PluginFactory for PanicFactory {
    fn descriptor(&self) -> PluginDescriptor {
        // Env-gated fixture for the guarded `descriptor()` paths:
        // `Scan` and the `CreateInstance` factory lookup both catch
        // this → `Err(HostError::PluginFailed)` on the wire.
        assert!(
            std::env::var_os("RUVIFORM_PANIC_DESCRIPTOR").is_none(),
            "ruviform-plugin-l1-runner: PanicFactory::descriptor (deterministic E2E fixture)"
        );
        PANIC_DESCRIPTOR
    }

    fn create(&self) -> Box<dyn ruviform_plugin_core::ErasedPlugin> {
        // Env-gated fixture for the guarded `create()` path inside
        // `CreateInstance` — same `PluginFailed` normalization as a
        // hook panic; the registry entry is never inserted.
        assert!(
            std::env::var_os("RUVIFORM_PANIC_CREATE").is_none(),
            "ruviform-plugin-l1-runner: PanicFactory::create (deterministic E2E fixture)"
        );
        erase(PanicPlugin)
    }
}

const UNDERWRITE_DESCRIPTOR: PluginDescriptor = PluginDescriptor {
    id: "dev.ruviform.mock.underwrite",
    name: "Underwrite",
    vendor: "Ruviform",
    version: "1.0.0",
    features: &["test-fixture"],
};

/// Under-write fixture: copies input to output but only for channel 0
/// after the first `process` call — the runner must re-zero the
/// staging buffer every block so unwritten channels come out as
/// silence, never as the previous block's leftovers.
struct UnderwritePlugin {
    processed: u32,
}

impl PluginProcessor for UnderwritePlugin {
    fn prepare(
        &mut self,
        _config: &ProcessConfig,
        _context: &mut ProcessContext,
    ) -> Result<(), PrepareError> {
        Ok(())
    }

    fn process<I, O>(
        &mut self,
        inputs: &[I],
        outputs: &mut [O],
        _events: EventList<'_>,
        _transport: Option<TransportInfo>,
        _context: &mut ProcessContext,
    ) -> ProcessResult
    where
        I: BufferView,
        O: BufferViewMut,
    {
        self.processed += 1;
        // First call fills every channel (seeding stale content into
        // the runner's staging buffer); later calls under-write.
        let channels_to_write = if self.processed == 1 { usize::MAX } else { 1 };
        let (Some(input), Some(output)) = (inputs.first(), outputs.first_mut()) else {
            return ProcessResult::CONTINUE;
        };
        for channel in 0..input
            .channels()
            .min(output.channels())
            .min(channels_to_write)
        {
            let (Some(src), Some(dst)) = (input.channel(channel), output.channel_mut(channel))
            else {
                continue;
            };
            dst.copy_from_slice(src);
        }
        ProcessResult::CONTINUE
    }
}

impl Plugin for UnderwritePlugin {
    fn descriptor(&self) -> PluginDescriptor {
        UNDERWRITE_DESCRIPTOR
    }

    fn params(&self) -> &[ParamInfo] {
        &[]
    }
}

struct UnderwriteFactory;

impl PluginFactory for UnderwriteFactory {
    fn descriptor(&self) -> PluginDescriptor {
        UNDERWRITE_DESCRIPTOR
    }

    fn create(&self) -> Box<dyn ruviform_plugin_core::ErasedPlugin> {
        erase(UnderwritePlugin { processed: 0 })
    }
}
