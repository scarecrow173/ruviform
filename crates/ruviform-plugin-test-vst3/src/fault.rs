//! The `test_fault` signal law: an in-band armed-fault parameter and
//! a pass-through processor that executes the armed fault inside the
//! `process()` call that carries the arming event.
//!
//! ## Fault parameter law
//!
//! `Fault` is parameter id `0` with a plain (host-visible) range of
//! `[0.0, 4.0]`, default `0.0`, stepped (`stepCount = 4` → five
//! discrete modes). The normalized domain is the [`ParamInfo`] linear
//! law (`plain = normalized * 4.0`), so the armed modes are reached
//! through ordinary `ParamValue` events — no argv, no env vars, no
//! out-of-band channel:
//!
//! | plain value | mode | `process()` behavior |
//! | ----------- | ---- | -------------------- |
//! | `[0.0, 1.0)` | `none` | pass-through only — fully inert |
//! | `[1.0, 2.0)` | `abort` | `std::process::abort()` |
//! | `[2.0, 3.0)` | `exit` | `std::process::exit(code)` — `code = round((plain - 2.0) * 100)` clamped to `[0, 100]` |
//! | `[3.0, 4.0)` | `hang` | never returns (bounded spin) |
//! | `4.0` | `error` | returns `ProcessStatus::Error` — a defined rejection, not a crash |
//!
//! The mode is the *floor* of the plain value — any value inside a
//! mode's unit band selects that mode, so automation jitter inside a
//! band cannot change the fault. `exit` additionally carries its code
//! in the fractional part, giving `plain 2.42` → `exit(42)` while
//! `plain 2.0` → `exit(0)` (a voluntary clean exit is still a plugin
//! failure to the host).
//!
//! ## Arming contract
//!
//! The armed value is the *last* `Fault` `ParamValue` event of the
//! block, persisted across blocks like `test_gain`'s gain. The fault
//! executes at the **end** of the `process()` call of the block that
//! carried the arming event — the armed block's audio is still
//! written (pass-through) before the mode dispatches. Mode `none`
//! (the default) never dispatches: unarmed blocks are pure
//! pass-through, safe under `full_series` and `rt_` tests.
//!
//! `abort`/`exit`/`hang` intentionally kill or wedge the *host
//! process* — they must only ever run inside the L1 out-of-process
//! child. Unit tests exercise only `none`/`error` in-process.
//!
//! [`TestFaultProcessor`] implements the shared neutral
//! [`PluginProcessor`] contract; the fault law is this fixture's own.

use ruviform_plugin_core::{
    BufferView, BufferViewMut, EventKind, EventList, ParamFlags, ParamId, ParamInfo,
    PluginProcessor, PrepareError, ProcessConfig, ProcessContext, ProcessResult, ProcessStatus,
    TransportInfo,
};
use vst3::Steinberg::TUID;
use vst3::uid;

use crate::component::TestComponent;

/// VST3 parameter id of the fault parameter.
pub(crate) const FAULT_PARAM_ID: u32 = 0;

/// The `Fault` parameter descriptor — plain range `[0.0, 4.0]`,
/// default `0.0` (`none`), automatable and stepped (five modes).
pub(crate) const FAULT_PARAM: ParamInfo = ParamInfo {
    id: ParamId(FAULT_PARAM_ID),
    name: "Fault",
    unit: "",
    min_value: 0.0,
    max_value: 4.0,
    default_value: 0.0,
    flags: ParamFlags::AUTOMATABLE.union(ParamFlags::STEPPED),
};

/// VST3 `ParameterInfo::stepCount` for the fault parameter — four
/// steps means five discrete modes (`none`/`abort`/`exit`/`hang`/
/// `error`).
pub(crate) const FAULT_STEP_COUNT: i32 = 4;

/// `test_fault`'s controller class ID — a fresh `uid` for the test
/// suite, distinct from both `test_gain`'s controller and the adapter
/// fixture's controller CIDs.
pub(crate) const FAULT_CONTROLLER_CID: TUID =
    uid(0x4B9D_2F73, 0x8E1C_6A05, 0xB3F8_4D97, 0xE5A1_0C46);

/// The armed fault mode — the unit band of the plain `Fault` value.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum FaultMode {
    /// `[0.0, 1.0)` — pass-through only; the unarmed default.
    None,
    /// `[1.0, 2.0)` — `std::process::abort()`.
    Abort,
    /// `[2.0, 3.0)` — `std::process::exit(code)`.
    Exit,
    /// `[3.0, 4.0)` — never returns.
    Hang,
    /// `4.0` — defined `ProcessStatus::Error` rejection.
    Error,
}

/// Decodes a plain `Fault` value into `(mode, exit_code)`.
///
/// The mode is `floor(plain)` clamped to `[0, 4]`; `exit_code` is
/// `round((plain - 2.0) * 100)` clamped to `[0, 100]` and meaningful
/// only for [`FaultMode::Exit`]. Pure and total — safe to unit test
/// for every input including values outside the declared domain.
#[must_use]
pub(crate) fn decode_fault(plain: f64) -> (FaultMode, u32) {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to [0.0, 4.0] before the cast — values are small non-negative integers"
    )]
    let index = plain.floor().clamp(0.0, 4.0) as u32;
    let mode = match index {
        0 => FaultMode::None,
        1 => FaultMode::Abort,
        2 => FaultMode::Exit,
        3 => FaultMode::Hang,
        _ => FaultMode::Error,
    };
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the result is clamped to [0.0, 100.0] before the cast"
    )]
    let exit_code = if mode == FaultMode::Exit {
        (((plain - 2.0) * 100.0).round().clamp(0.0, 100.0)) as u32
    } else {
        0
    };
    (mode, exit_code)
}

/// Copies `input` to `output` channel-for-channel — the trivial
/// pass-through DSP law shared by `test_fault` (unarmed/`none`) and
/// `test_editor`. `frames` is bounded by the shorter side; missing
/// channels are skipped (the views guarantee uniform lengths, so
/// this is only defensive).
pub(crate) fn pass_through<I: BufferView, O: BufferViewMut>(
    inputs: &[I],
    outputs: &mut [O],
    frames: usize,
) {
    let Some((input, output)) = inputs.first().zip(outputs.first_mut()) else {
        return;
    };
    let channels = input.channels().min(output.channels());
    for channel in 0..channels {
        let (Some(src), Some(dst)) = (input.channel(channel), output.channel_mut(channel)) else {
            continue;
        };
        let limit = frames.min(src.len()).min(dst.len());
        dst[..limit].copy_from_slice(&src[..limit]);
    }
}

/// The `test_fault` processor — pass-through DSP plus the armed-fault
/// dispatch. The armed plain value persists across blocks (the last
/// `Fault` event wins); the mode decodes at the end of every block.
pub(crate) struct TestFaultProcessor {
    /// The armed plain `Fault` value (`[0.0, 4.0]`), held as `f64`.
    /// `0.0` (`none`) is the unarmed default.
    armed: f64,
}

impl TestFaultProcessor {
    /// A fresh processor at the parameter default (`armed = 0.0` —
    /// fully inert).
    ///
    /// **RT contract:** RT-suitable — pure.
    #[must_use]
    pub(crate) const fn new() -> Self {
        Self {
            armed: FAULT_PARAM.default_value,
        }
    }
}

impl PluginProcessor for TestFaultProcessor {
    /// Activation hook — resets the armed value to the parameter
    /// default so every `setupProcessing` starts unarmed.
    /// Non-RT (may allocate; this implementation does not need to).
    fn prepare(
        &mut self,
        _config: &ProcessConfig,
        _context: &mut ProcessContext,
    ) -> Result<(), PrepareError> {
        self.armed = FAULT_PARAM.default_value;
        Ok(())
    }

    /// Runs one block: pass-through copy, then armed-value update from
    /// the block's `Fault` events, then the mode dispatch.
    ///
    /// The dispatch runs *after* the audio is written so the armed
    /// block's output is defined before `abort`/`exit`/`hang`/`error`
    /// takes effect. `abort`/`exit`/`hang` diverge — they never
    /// return a `ProcessResult`.
    ///
    /// **RT contract:** the `none` path is RT-suitable — bounded
    /// `O(frames × channels + events)` work; no allocation, locking,
    /// waiting or I/O. The armed paths intentionally violate RT
    /// rules (that is the fault under test).
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
        let frames = inputs
            .first()
            .map_or(0, BufferView::frames)
            .min(outputs.first().map_or(0, BufferView::frames));
        pass_through(inputs, outputs, frames);

        for event in events.iter() {
            let EventKind::ParamValue(param) = event.kind() else {
                continue;
            };
            if param.param_id != FAULT_PARAM.id {
                continue;
            }
            // Last event wins — the armed value is the final `Fault`
            // point of the block regardless of frame offset.
            self.armed = FAULT_PARAM.denormalize(param.value);
        }

        let result = if inputs.is_empty() || outputs.is_empty() {
            ProcessResult::SLEEP
        } else {
            ProcessResult::CONTINUE
        };
        let (mode, exit_code) = decode_fault(self.armed);
        match mode {
            FaultMode::None => result,
            FaultMode::Error => ProcessResult::new(ProcessStatus::Error, ProcessResult::TAIL_NONE),
            FaultMode::Abort => std::process::abort(),
            FaultMode::Exit => {
                std::process::exit(i32::try_from(exit_code).unwrap_or(i32::MAX));
            }
            FaultMode::Hang => loop {
                std::hint::spin_loop();
            },
        }
    }
}

/// `test_fault`'s component — the shared [`TestComponent`] shell over
/// [`TestFaultProcessor`]. The associated `CID`/`new` keep the same
/// surface shape as [`crate::component::TestGainComponent`].
pub(crate) type TestFaultComponent = TestComponent<TestFaultProcessor>;

impl TestComponent<TestFaultProcessor> {
    /// The component class ID — a fresh `uid` for the test suite,
    /// distinct from `test_gain`'s component CID.
    pub(crate) const CID: TUID = uid(0x7C3A_91E4, 0x5F2B_8D61, 0xA4E7_0C93, 0xD8B5_6F2A);

    pub(crate) fn new() -> Self {
        Self::with(
            FAULT_CONTROLLER_CID,
            FAULT_PARAM.id,
            TestFaultProcessor::new(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruviform_audio_core::{PlanarBufferView, PlanarBufferViewMut};
    use ruviform_plugin_core::{Event, ParamEvent, ParamValue};

    fn fault_event(normalized: f64) -> Event {
        Event::new(
            0,
            EventKind::ParamValue(ParamEvent {
                param_id: FAULT_PARAM.id,
                value: ParamValue::new(normalized).unwrap(),
            }),
        )
    }

    /// Runs one block through the neutral contract — the same call
    /// shape the component's `process` makes. Only `none`/`error`
    /// modes are exercised here: `abort`/`exit`/`hang` would kill or
    /// wedge the test binary itself.
    fn run_block(
        processor: &mut TestFaultProcessor,
        input: &[Vec<f32>],
        output: &mut [Vec<f32>],
        events: &[Event],
    ) -> ProcessResult {
        let frames = u32::try_from(input.first().map_or(0, Vec::len)).unwrap_or(u32::MAX);
        let in_views: Vec<_> = input.iter().map(|c| &c[..]).collect();
        let inputs = [PlanarBufferView::new(&in_views).unwrap()];
        let mut out_views: Vec<_> = output.iter_mut().map(|c| &mut c[..]).collect();
        let mut outputs = [PlanarBufferViewMut::new(&mut out_views).unwrap()];
        let list = EventList::new(events, frames).unwrap();
        let mut context = ProcessContext::new();
        processor.process(&inputs, &mut outputs, list, None, &mut context)
    }

    /// `decode_fault` maps every plain band to its mode; `exit` codes
    /// decode from the fractional band; out-of-domain values clamp.
    #[test]
    fn decode_fault_maps_bands_and_exit_codes() {
        assert_eq!(decode_fault(0.0), (FaultMode::None, 0));
        assert_eq!(decode_fault(0.99), (FaultMode::None, 0));
        assert_eq!(decode_fault(1.0), (FaultMode::Abort, 0));
        assert_eq!(decode_fault(1.99), (FaultMode::Abort, 0));
        assert_eq!(decode_fault(2.0), (FaultMode::Exit, 0));
        assert_eq!(decode_fault(2.42), (FaultMode::Exit, 42));
        assert_eq!(decode_fault(2.99), (FaultMode::Exit, 99));
        assert_eq!(decode_fault(3.0), (FaultMode::Hang, 0));
        assert_eq!(decode_fault(3.99), (FaultMode::Hang, 0));
        assert_eq!(decode_fault(4.0), (FaultMode::Error, 0));
        // Out-of-domain plain values clamp deterministically.
        assert_eq!(decode_fault(-1.0).0, FaultMode::None);
        assert_eq!(decode_fault(9.0).0, FaultMode::Error);
    }

    /// The unarmed default is pure pass-through — `out == in`
    /// bitwise, status `Continue`.
    #[test]
    fn unarmed_block_is_pass_through() {
        let mut processor = TestFaultProcessor::new();
        let input = [vec![0.25_f32; 8], vec![-0.5; 8]];
        let mut output = [vec![0.0_f32; 8], vec![0.0; 8]];
        let result = run_block(&mut processor, &input, &mut output, &[]);
        assert_eq!(result.status, ProcessStatus::Continue);
        assert_eq!(output[0], input[0]);
        assert_eq!(output[1], input[1]);
    }

    /// An explicit `none` arming event (plain `0.0` → normalized
    /// `0.0`) keeps the block inert — pass-through, `Continue`.
    #[test]
    fn none_mode_event_stays_inert() {
        let mut processor = TestFaultProcessor::new();
        let input = [vec![0.25_f32; 8]];
        let mut output = [vec![0.0_f32; 8]];
        let result = run_block(&mut processor, &input, &mut output, &[fault_event(0.0)]);
        assert_eq!(result.status, ProcessStatus::Continue);
        assert_eq!(output[0], input[0]);
    }

    /// The `error` mode returns `ProcessStatus::Error` — a defined
    /// rejection the host observes as a failed result, not a crash.
    /// Safe in-process: it never diverges.
    #[test]
    fn error_mode_returns_process_error() {
        let mut processor = TestFaultProcessor::new();
        let input = [vec![0.25_f32; 8]];
        let mut output = [vec![0.0_f32; 8]];
        // plain 4.0 → normalized 1.0 → `error`.
        let result = run_block(&mut processor, &input, &mut output, &[fault_event(1.0)]);
        assert_eq!(result.status, ProcessStatus::Error);
    }

    /// Foreign parameter ids never arm the fault — the armed value
    /// stays `none` and the block stays inert.
    #[test]
    fn foreign_param_events_do_not_arm() {
        let mut processor = TestFaultProcessor::new();
        let input = [vec![0.25_f32; 8]];
        let mut output = [vec![0.0_f32; 8]];
        let foreign = Event::new(
            0,
            EventKind::ParamValue(ParamEvent {
                param_id: ParamId(9),
                value: ParamValue::new(1.0).unwrap(),
            }),
        );
        let result = run_block(&mut processor, &input, &mut output, &[foreign]);
        assert_eq!(result.status, ProcessStatus::Continue);
        assert_eq!(output[0], input[0]);
    }
}
