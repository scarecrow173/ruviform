//! The `test_editor` signal law: the suite's `IPlugView` editor
//! fixture (see [`crate::editor_view`]) carrying the shared `Fault`
//! parameter — the same in-band armed-fault law `test_fault` runs
//! (Issue #885).
//!
//! `test_editor` declares the [`FAULT_PARAM`] descriptor (`ParamId`
//! `0`, plain `[0.0, 8.0]`, stepped — eight modes) so the component's
//! `wanted` filter admits `Fault` `IParamValueQueue` points and the
//! controller reports `getParameterCount == 1`. Unarmed (`none`) the
//! class is pure pass-through; an in-band `ParamValue` event arms
//! `abort`/`exit`/`hang`/`error`/`rt_alloc`/`bad_time`/`invalid`
//! inside the arming block's `process()` — the fault law itself is
//! [`crate::fault`]'s single source of truth, consumed here through
//! [`TestFaultProcessor`].
//!
//! [`TestEditorProcessor`] implements the shared neutral
//! [`PluginProcessor`] contract by delegation — the editor class
//! keeps its own processor type so [`TestEditorComponent`] holds the
//! class's own `CID`/`new` surface (`TestComponent<TestFaultProcessor>`
//! is already `test_fault`'s concrete type and cannot carry a second
//! `CID`/`new` pair).

use ruviform_plugin_core::{
    BufferView, BufferViewMut, EventList, PluginProcessor, PrepareError, ProcessConfig,
    ProcessContext, ProcessResult, TransportInfo,
};
use vst3::Steinberg::TUID;
use vst3::uid;

use crate::component::TestComponent;
use crate::controller::EDITOR_CONTROLLER_CID;
use crate::fault::{FAULT_PARAM, TestFaultProcessor};

/// The `test_editor` processor — the [`TestFaultProcessor`] law
/// (pass-through DSP + armed-fault dispatch) behind the editor
/// fixture's own type, so `TestEditorComponent` keeps the
/// associated `CID`/`new` surface every suite class shares.
pub(crate) struct TestEditorProcessor(TestFaultProcessor);

impl TestEditorProcessor {
    /// A fresh processor at the parameter default (`armed = 0.0` —
    /// fully inert).
    ///
    /// **RT contract:** RT-suitable — pure.
    #[must_use]
    pub(crate) const fn new() -> Self {
        Self(TestFaultProcessor::new())
    }
}

impl PluginProcessor for TestEditorProcessor {
    /// Activation hook — resets the armed value to the parameter
    /// default (the delegated [`TestFaultProcessor::prepare`] law).
    /// Non-RT (may allocate; this implementation does not need to).
    fn prepare(
        &mut self,
        config: &ProcessConfig,
        context: &mut ProcessContext,
    ) -> Result<(), PrepareError> {
        self.0.prepare(config, context)
    }

    /// Runs one block through the delegated [`TestFaultProcessor`]
    /// law: pass-through copy, then the armed-value update from the
    /// block's `Fault` events, then the mode dispatch.
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
        transport: Option<TransportInfo>,
        context: &mut ProcessContext,
    ) -> ProcessResult
    where
        I: BufferView,
        O: BufferViewMut,
    {
        self.0.process(inputs, outputs, events, transport, context)
    }
}

/// `test_editor`'s component — the shared [`TestComponent`] shell over
/// [`TestEditorProcessor`]. The associated `CID`/`new` keep the same
/// surface shape as [`crate::component::TestGainComponent`] and
/// [`crate::fault::TestFaultComponent`].
pub(crate) type TestEditorComponent = TestComponent<TestEditorProcessor>;

impl TestComponent<TestEditorProcessor> {
    /// The component class ID — a fresh `uid` for the test suite,
    /// distinct from `test_gain`/`test_fault` component CIDs and the
    /// adapter fixture's CIDs.
    pub(crate) const CID: TUID = uid(0x3F8A_4C62, 0x9D1E_7B05, 0xA6F3_48D9, 0xC2E7_5B81);

    pub(crate) fn new() -> Self {
        Self::with(
            EDITOR_CONTROLLER_CID,
            FAULT_PARAM.id,
            TestEditorProcessor::new(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruviform_audio_core::{PlanarBufferView, PlanarBufferViewMut};
    use ruviform_plugin_core::{Event, EventKind, ParamEvent, ParamId, ParamValue, ProcessStatus};

    /// One `Fault` `ParamValue` event at frame offset 0 — the same
    /// builder `fault.rs`'s tests use (normalized × 8.0 = plain).
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
    /// shape the component's `process` makes. Only `none`/`error`/
    /// `rt_alloc`/`bad_time`/`invalid` are exercised here:
    /// `abort`/`exit`/`hang` would kill or wedge the test binary
    /// itself (the `fault.rs` law — L1-only modes).
    fn run_block(
        processor: &mut TestEditorProcessor,
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

    /// `test_editor` unarmed is pure pass-through — `out == in`
    /// bitwise, status `Continue` — and a `ParamValue` event on a
    /// foreign parameter id (anything but `FAULT_PARAM.id`) arms
    /// nothing and changes nothing.
    #[test]
    fn editor_block_is_pass_through() {
        let mut processor = TestEditorProcessor::new();
        let input = [vec![0.25_f32; 8], vec![-0.5; 8]];
        let mut output = [vec![0.0_f32; 8], vec![0.0; 8]];
        let foreign = Event::new(
            0,
            EventKind::ParamValue(ParamEvent {
                param_id: ParamId(1),
                value: ParamValue::new(1.0).unwrap(),
            }),
        );
        let result = run_block(&mut processor, &input, &mut output, &[foreign]);
        assert_eq!(result.status, ProcessStatus::Continue);
        assert_eq!(output[0], input[0]);
        assert_eq!(output[1], input[1]);
    }

    /// An explicit `none` arming event (plain `0.0` → normalized
    /// `0.0`) keeps the block inert — pass-through, `Continue`.
    #[test]
    fn none_mode_event_stays_inert() {
        let mut processor = TestEditorProcessor::new();
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
        let mut processor = TestEditorProcessor::new();
        let input = [vec![0.25_f32; 8]];
        let mut output = [vec![0.0_f32; 8]];
        // plain 4.0 → normalized 0.5 → `error`.
        let result = run_block(&mut processor, &input, &mut output, &[fault_event(0.5)]);
        assert_eq!(result.status, ProcessStatus::Error);
    }

    /// `rt_alloc` allocates inside `process()` and writes the heap
    /// byte (`200.0`) into `output[0][0]` — the sample proves the
    /// allocation round-trip ran. The rest of the block stays
    /// pass-through.
    #[test]
    fn alloc_fault_marks_armed_block() {
        let mut processor = TestEditorProcessor::new();
        let input = [vec![0.25_f32; 8], vec![-0.5; 8]];
        let mut output = [vec![0.0_f32; 8], vec![0.0; 8]];
        // plain 5.0 → normalized 0.625 → `rt_alloc`.
        let result = run_block(&mut processor, &input, &mut output, &[fault_event(0.625)]);
        assert_eq!(result.status, ProcessStatus::Continue);
        assert_eq!(output[0][0], 200.0);
        assert_eq!(output[0][1..], input[0][1..]);
        assert_eq!(output[1], input[1]);
    }

    /// `bad_time` returns `Continue` with `TAIL_INFINITE` — the
    /// zero-tail fixture's timing lie surfaces on the neutral
    /// `tail_frames` a host reads back.
    #[test]
    fn bad_time_mode_claims_infinite_tail() {
        let mut processor = TestEditorProcessor::new();
        let input = [vec![0.25_f32; 8]];
        let mut output = [vec![0.0_f32; 8]];
        // plain 6.0 → normalized 0.75 → `bad_time`.
        let result = run_block(&mut processor, &input, &mut output, &[fault_event(0.75)]);
        assert_eq!(result.status, ProcessStatus::Continue);
        assert_eq!(result.tail_frames, ProcessResult::TAIL_INFINITE);
    }

    /// `invalid` fills every output sample with NaN — the invalid
    /// audio state the host oracle detects on the delivered samples.
    #[test]
    fn invalid_mode_fills_nan_outputs() {
        let mut processor = TestEditorProcessor::new();
        let input = [vec![0.25_f32; 8], vec![-0.5; 8]];
        let mut output = [vec![0.0_f32; 8], vec![0.0; 8]];
        // plain 7.0 → normalized 0.875 → `invalid`.
        let result = run_block(&mut processor, &input, &mut output, &[fault_event(0.875)]);
        assert_eq!(result.status, ProcessStatus::Continue);
        assert!(output[0].iter().all(|s| s.is_nan()));
        assert!(output[1].iter().all(|s| s.is_nan()));
    }

    /// The armed modes persist until disarmed: after `invalid`, a
    /// `none` event restores pure pass-through — the fault mode's
    /// recovery half.
    #[test]
    fn disarm_restores_pass_through() {
        let mut processor = TestEditorProcessor::new();
        let input = [vec![0.25_f32; 8]];
        let mut output = [vec![0.0_f32; 8]];
        // plain 7.0 → normalized 0.875 → `invalid`.
        run_block(&mut processor, &input, &mut output, &[fault_event(0.875)]);
        // plain 0.0 → normalized 0.0 → `none`.
        let result = run_block(&mut processor, &input, &mut output, &[fault_event(0.0)]);
        assert_eq!(result.status, ProcessStatus::Continue);
        assert_eq!(output[0], input[0]);
    }

    /// RT proof: the unarmed/`none` path never allocates — the armed
    /// `rt_alloc` path is the deliberate violation; the inert law is
    /// untouched. Views and the event list are built outside the
    /// `assert_no_alloc` window so only `process()` itself is
    /// measured (the `rt_` name selects this for `mise run test-rt`).
    #[test]
    fn rt_unarmed_block_is_alloc_free() {
        let mut processor = TestEditorProcessor::new();
        let input = [vec![0.25_f32; 8], vec![-0.5; 8]];
        let mut output = [vec![0.0_f32; 8], vec![0.0; 8]];
        let in_views: Vec<_> = input.iter().map(|c| &c[..]).collect();
        let inputs = [PlanarBufferView::new(&in_views).unwrap()];
        let mut out_views: Vec<_> = output.iter_mut().map(|c| &mut c[..]).collect();
        let mut outputs = [PlanarBufferViewMut::new(&mut out_views).unwrap()];
        let events = [fault_event(0.0)];
        let list = EventList::new(&events, 8).unwrap();
        let mut context = ProcessContext::new();
        assert_no_alloc::assert_no_alloc(|| {
            processor.process(&inputs, &mut outputs, list, None, &mut context)
        });
    }
}
