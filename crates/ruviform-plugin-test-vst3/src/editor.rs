//! The `test_editor` signal law: a trivial pass-through processor —
//! the class exists to carry the suite's `IPlugView` editor fixture
//! (see [`crate::editor_view`]), not to exercise DSP or parameters.
//!
//! `test_editor` declares **no parameters**: the component admits no
//! `IParamValueQueue` points (its `param_id` is a sentinel outside
//! every real id, so `collect_param_events` rejects all queues) and
//! its controller reports `getParameterCount == 0`. The processor
//! therefore ignores the event list entirely and copies input to
//! output channel-for-channel — the same [`pass_through`] law
//! `test_fault` uses when unarmed.
//!
//! [`TestEditorProcessor`] implements the shared neutral
//! [`PluginProcessor`] contract; the pass-through law itself lives in
//! [`crate::fault::pass_through`].

use ruviform_plugin_core::{
    BufferView, BufferViewMut, EventList, ParamId, PluginProcessor, PrepareError, ProcessConfig,
    ProcessContext, ProcessResult, TransportInfo,
};
use vst3::Steinberg::TUID;
use vst3::uid;

use crate::component::TestComponent;
use crate::controller::EDITOR_CONTROLLER_CID;
use crate::fault::pass_through;

/// Sentinel parameter id — `test_editor` declares no parameters, so
/// the component admits no queue points: every `IParamValueQueue`
/// id fails the `wanted` filter in `collect_param_events`. `u32::MAX`
/// cannot collide with a real VST3 `ParamID`.
const EDITOR_NO_PARAM: ParamId = ParamId(u32::MAX);

/// The `test_editor` processor — pure pass-through; every event is
/// ignored (the class has no parameter surface).
pub(crate) struct TestEditorProcessor;

impl PluginProcessor for TestEditorProcessor {
    /// Activation hook — nothing to reset (stateless).
    /// Non-RT (may allocate; this implementation does not need to).
    fn prepare(
        &mut self,
        _config: &ProcessConfig,
        _context: &mut ProcessContext,
    ) -> Result<(), PrepareError> {
        Ok(())
    }

    /// Runs one block: channel-for-channel pass-through copy. The
    /// event list is ignored — `test_editor` has no parameters.
    ///
    /// **RT contract:** RT-suitable — bounded `O(frames × channels)`
    /// work; no allocation, locking, waiting or I/O.
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
        let frames = inputs
            .first()
            .map_or(0, BufferView::frames)
            .min(outputs.first().map_or(0, BufferView::frames));
        pass_through(inputs, outputs, frames);

        if inputs.is_empty() || outputs.is_empty() {
            ProcessResult::SLEEP
        } else {
            ProcessResult::CONTINUE
        }
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
        Self::with(EDITOR_CONTROLLER_CID, EDITOR_NO_PARAM, TestEditorProcessor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruviform_audio_core::{PlanarBufferView, PlanarBufferViewMut};
    use ruviform_plugin_core::{Event, EventKind, ParamEvent, ParamValue, ProcessStatus};

    /// Runs one block through the neutral contract — the same call
    /// shape the component's `process` makes.
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

    /// `test_editor` is pure pass-through — `out == in` bitwise,
    /// status `Continue`, and parameter events (which the class
    /// cannot produce) change nothing.
    #[test]
    fn editor_block_is_pass_through() {
        let mut processor = TestEditorProcessor;
        let input = [vec![0.25_f32; 8], vec![-0.5; 8]];
        let mut output = [vec![0.0_f32; 8], vec![0.0; 8]];
        let foreign = Event::new(
            0,
            EventKind::ParamValue(ParamEvent {
                param_id: ParamId(0),
                value: ParamValue::new(1.0).unwrap(),
            }),
        );
        let result = run_block(&mut processor, &input, &mut output, &[foreign]);
        assert_eq!(result.status, ProcessStatus::Continue);
        assert_eq!(output[0], input[0]);
        assert_eq!(output[1], input[1]);
    }
}
