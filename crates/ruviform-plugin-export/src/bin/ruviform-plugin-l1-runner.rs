//! Reference L1 runner binary (`ruviform-plugin-l1-runner`) — the
//! spawn fixture for the `ruviform-plugin-export` E2E tests
//! (`tests/l1_runner_e2e.rs`, via `CARGO_BIN_EXE_*`).
//!
//! It embeds three mock factories:
//!
//! - `GainPlugin` (`dev.ruviform.mock.gain`) — a deterministic gain
//!   plugin: planar `f32` input × a single normalized gain parameter —
//!   each `ParamValue` event sets the block-wide gain (the latest
//!   event in the block wins; there is no per-offset ramp). Every
//!   input event is echoed into its output-report backlog in arrival
//!   order and each processed block closes with one deterministic
//!   `Midi` marker, so `drain_output_reports` (wire v4) returns both
//!   `EventKind`s oldest-first. Implements the full `Plugin` surface
//!   — `init`/`start_processing`/`stop_processing`/`save_state`/
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
//! - `GuiPlugin` (`dev.ruviform.mock.gui`) — window-independent GUI
//!   fixture (wire v6): `open_gui` returns a minted `GuiHandle` token
//!   (no HWND involved — the marshal/reply round-trip is what the
//!   E2E pins), the `close_gui`/`set_gui_*`/`send_gui_key` hooks
//!   answer `Ok` while a surface is logically open — the same
//!   `view.is_some()` vocabulary the VST3 adapter carries — and
//!   `RUVIFORM_GUI_WEDGE` parks `open_gui` forever, the wedged-hook
//!   convergence fixture (the marshal reply never arrives, so the
//!   child's command loop stalls the way a wedged plugin GUI does).
//!
//! Wire v2 takes no spawn arguments — the binary is spawned bare and
//! the host picks a plugin id per `CreateInstance`.

use std::process::ExitCode;

use ruviform_media_core::{MidiChannel, MidiEvent, MidiGroup, MidiKind, NoteKey};
use ruviform_plugin_core::{
    BufferView, BufferViewMut, Event, EventKind, EventList, GuiHandle, GuiKeyEvent, GuiSize,
    HostError, ParamFlags, ParamId, ParamInfo, ParentLease, Plugin, PluginDescriptor,
    PluginFactory, PluginProcessor, PrepareError, ProcessConfig, ProcessContext, ProcessResult,
    TransportInfo, erase,
};

fn main() -> ExitCode {
    ruviform_plugin_export::run(vec![
        Box::new(GainFactory),
        Box::new(PanicFactory),
        Box::new(UnderwriteFactory),
        Box::new(GuiFactory),
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
    /// Reports queued for the next `drain_output_reports` pull:
    /// every input event echoed in arrival order, plus one
    /// deterministic `Midi` marker per processed block.
    pending_reports: Vec<Event>,
    /// Blocks processed so far — seeds the marker's note key and
    /// velocity so the E2E can predict the exact report stream.
    blocks: u32,
}

impl GainPlugin {
    fn new() -> Self {
        Self {
            gain: 1.0,
            pending_reports: Vec::new(),
            blocks: 0,
        }
    }

    /// The deterministic per-block `Midi` marker — `None` only if a
    /// constructor rejects an in-range literal (unreachable; the
    /// fixture never panics inside `process`).
    fn report_marker(blocks: u32) -> Option<Event> {
        // Key cycles 60..=71, velocity counts processed blocks.
        let key = NoteKey::new(u8::try_from(60 + (blocks - 1) % 12).ok()?).ok()?;
        let channel = MidiChannel::new(0).ok()?;
        Some(Event::new(
            0,
            EventKind::Midi(MidiEvent {
                group: MidiGroup::ZERO,
                channel,
                kind: MidiKind::NoteOn {
                    key,
                    velocity: blocks,
                },
            }),
        ))
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
            // Report fixture (wire v4): echo every input event into
            // the output-report backlog in arrival order — both
            // `EventKind`s reach `drain_output_reports` unchanged.
            self.pending_reports.push(event);
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
        self.blocks += 1;
        if let Some(marker) = Self::report_marker(self.blocks) {
            self.pending_reports.push(marker);
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

    fn drain_output_reports(&mut self, out: &mut Vec<Event>) {
        out.append(&mut self.pending_reports);
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

const GUI_DESCRIPTOR: PluginDescriptor = PluginDescriptor {
    id: "dev.ruviform.mock.gui",
    name: "Gui",
    vendor: "Ruviform",
    version: "1.0.0",
    features: &["test-fixture"],
};

/// The wedge switch: `RUVIFORM_GUI_WEDGE` in the child env parks
/// `open_gui` forever — the wedged-hook fixture.
const GUI_WEDGE_ENV: &str = "RUVIFORM_GUI_WEDGE";

/// Window-independent GUI fixture (wire v6): the hooks mirror the
/// VST3 adapter's vocabulary — `open_gui` mints a `GuiHandle` token
/// and remembers it as the logical surface (`view`), the other GUI
/// hooks require it (`view.is_none()` → `Err(Unsupported)`), and a
/// second `open_gui` is the defined decline `Err(PluginFailed)` —
/// the "one live surface per instance" rule lives in the hook
/// contract, never in a runner gate. No window is ever created:
/// what the E2E pins is the marshal/reply round-trip and the reply
/// vocabulary, not the HWND — that stays with the Windows lane.
struct GuiPlugin {
    /// The logical surface token — `Some` while a surface is open.
    view: Option<GuiHandle>,
    /// `GuiHandle` mint sequence — 1, 2, … per accepted open.
    gui_seq: u32,
}

impl PluginProcessor for GuiPlugin {
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

impl Plugin for GuiPlugin {
    fn descriptor(&self) -> PluginDescriptor {
        GUI_DESCRIPTOR
    }

    fn params(&self) -> &[ParamInfo] {
        &[]
    }

    fn open_gui(&mut self, _parent: ParentLease) -> Result<GuiHandle, HostError> {
        if std::env::var_os(GUI_WEDGE_ENV).is_some() {
            // Wedged hook — parks forever inside `open_gui`; the
            // marshal reply never arrives, so the child's command
            // loop stalls and the parent's `command_timeout` → kill
            // is what converges it (the E2E kills the child itself).
            loop {
                std::thread::park();
            }
        }
        if self.view.is_some() {
            // One live surface per instance — the adapter contract's
            // decline (the VST3 `view.is_some()` → `PluginFailed`
            // precedent).
            return Err(HostError::PluginFailed);
        }
        self.gui_seq = self.gui_seq.wrapping_add(1);
        let gui = GuiHandle::new(self.gui_seq);
        self.view = Some(gui);
        Ok(gui)
    }

    fn close_gui(&mut self) -> Result<(), HostError> {
        if self.view.take().is_none() {
            return Err(HostError::Unsupported);
        }
        Ok(())
    }

    fn set_gui_size(&mut self, _size: GuiSize) -> Result<(), HostError> {
        if self.view.is_none() {
            return Err(HostError::Unsupported);
        }
        Ok(())
    }

    fn set_gui_visibility(&mut self, _visible: bool) -> Result<(), HostError> {
        if self.view.is_none() {
            return Err(HostError::Unsupported);
        }
        Ok(())
    }

    fn set_gui_focus(&mut self, _focused: bool) -> Result<(), HostError> {
        if self.view.is_none() {
            return Err(HostError::Unsupported);
        }
        Ok(())
    }

    fn send_gui_key(&mut self, _event: GuiKeyEvent) -> Result<(), HostError> {
        if self.view.is_none() {
            return Err(HostError::Unsupported);
        }
        Ok(())
    }
}

struct GuiFactory;

impl PluginFactory for GuiFactory {
    fn descriptor(&self) -> PluginDescriptor {
        GUI_DESCRIPTOR
    }

    fn create(&self) -> Box<dyn ruviform_plugin_core::ErasedPlugin> {
        erase(GuiPlugin {
            view: None,
            gui_seq: 0,
        })
    }
}
