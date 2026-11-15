//! The `test_gain` signal law and its sample-accurate parameter
//! application — the semantic core of the test plugin.
//!
//! ## Parameter law
//!
//! `Gain` is VST3 parameter id `0` with a plain (host-visible DSP)
//! range of `[0.0, 2.0]` and a default of `1.0`. The normalized
//! domain is the linear `[0.0, 1.0]` mapping maintained by
//! [`ParamInfo`] — deliberately **non-identity** (`normalized 0.5` ↔
//! `plain 1.0`), so a host that confuses the two domains produces
//! measurably wrong output. The conversion is linear and invertible:
//! `plain = min + normalized * (max - min)`.
//!
//! ## Sample-accurate application law
//!
//! Events carry a frame offset — the first frame the change affects.
//! `process` walks the (sorted) event list once: for each gain event
//! it applies the *current* gain over `[cursor, offset)` and then
//! adopts the new value, so the event's own offset frame and all
//! later frames in the block use the new gain. Events at the same
//! offset apply in enumeration order — the last one wins because each
//! subsequent event's empty `[cursor, offset)` segment applies
//! nothing before the newer value takes over. A zero-frame block
//! (`EventList` flush semantics) applies no frames but still updates
//! the persisted gain, so an offset-0 event queued on a flush block
//! affects the *next* block. This is stricter than the `PoC` fixture's
//! whole-block application and is the frame-precision oracle for the
//! issue's offset/off-by-one checks.
//!
//! The DSP itself is the minimal deterministic law
//! `out = in * gain`, applied per channel — no signal generators, no
//! state beyond the gain value itself.
//!
//! [`TestGainProcessor`] implements the shared neutral
//! [`PluginProcessor`] contract — the same contract every Ruviform
//! adapter drives — while the DSP, parameter domain and application
//! law are this fixture's own (independent of the `ruviform-plugin-vst3`
//! fixture's gain implementation).

use ruviform_plugin_core::{
    BufferView, BufferViewMut, EventKind, EventList, ParamFlags, ParamId, ParamInfo,
    PluginProcessor, PrepareError, ProcessConfig, ProcessContext, ProcessResult, TransportInfo,
};

/// VST3 parameter id of the gain parameter.
pub(crate) const GAIN_PARAM_ID: u32 = 0;

/// The `Gain` parameter descriptor — plain range `[0.0, 2.0]`,
/// default `1.0`, automatable, dimensionless.
///
/// The normalized↔plain mapping is the [`ParamInfo`] linear law —
/// see module docs for why it is deliberately non-identity.
pub(crate) const GAIN_PARAM: ParamInfo = ParamInfo {
    id: ParamId(GAIN_PARAM_ID),
    name: "Gain",
    unit: "",
    min_value: 0.0,
    max_value: 2.0,
    default_value: 1.0,
    flags: ParamFlags::AUTOMATABLE,
};

/// Applies `gain` over frames `[start, end)` on every shared channel.
/// `end` is additionally clamped to each channel slice's length —
/// views guarantee uniform lengths, so this is only defensive.
/// `pub(crate)` so the `test_stateful` fixture's identical gain law
/// reuses the one implementation.
pub(crate) fn apply_gain<I: BufferView, O: BufferViewMut>(
    inputs: &[I],
    outputs: &mut [O],
    start: usize,
    end: usize,
    gain: f32,
) {
    let Some((input, output)) = inputs.first().zip(outputs.first_mut()) else {
        return;
    };
    let channels = input.channels().min(output.channels());
    for channel in 0..channels {
        let (Some(src), Some(dst)) = (input.channel(channel), output.channel_mut(channel)) else {
            continue;
        };
        let limit = end.min(src.len()).min(dst.len());
        if start >= limit {
            continue;
        }
        for (dst, src) in dst[start..limit].iter_mut().zip(&src[start..limit]) {
            *dst = *src * gain;
        }
    }
}

/// The `test_gain` processor — implements the shared neutral
/// [`PluginProcessor`] contract with this fixture's own DSP and
/// parameter law.
pub(crate) struct TestGainProcessor {
    /// Current gain in the plain domain (`[0.0, 2.0]`), held as `f64`
    /// for exact parameter-domain precision; cast to `f32` per segment
    /// at apply time. Persists across blocks — events update it at
    /// their offsets and the tail state carries into the next block.
    gain: f64,
}

impl TestGainProcessor {
    /// A fresh processor at the parameter default (`gain = 1.0`).
    ///
    /// **RT contract:** RT-suitable — pure.
    #[must_use]
    pub(crate) const fn new() -> Self {
        Self {
            gain: GAIN_PARAM.default_value,
        }
    }
}

impl PluginProcessor for TestGainProcessor {
    /// Activation hook — resets the gain to the parameter default so
    /// every `setupProcessing` starts from a deterministic state.
    /// Non-RT (may allocate; this implementation does not need to).
    fn prepare(
        &mut self,
        _config: &ProcessConfig,
        _context: &mut ProcessContext,
    ) -> Result<(), PrepareError> {
        self.gain = GAIN_PARAM.default_value;
        Ok(())
    }

    /// Runs one block: applies the persisted gain from `cursor = 0`,
    /// switching at each gain event's frame offset.
    ///
    /// `inputs`/`outputs` are the zero-copy channel views built by the
    /// block layer; `events` is the sorted, block-bounded event list.
    /// `transport`/`context` are unused — the plugin declares no
    /// process-context requirements.
    ///
    /// **RT contract:** RT-suitable — bounded `O(frames × channels +
    /// events)` work; no allocation, locking, waiting or I/O.
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
        let mut cursor = 0_usize;
        let mut gain = self.gain;
        for event in events.iter() {
            let EventKind::ParamValue(param) = event.kind() else {
                continue;
            };
            if param.param_id != GAIN_PARAM.id {
                continue;
            }
            let offset = usize::try_from(event.frame_offset())
                .unwrap_or(usize::MAX)
                .min(frames);
            #[expect(clippy::cast_possible_truncation)]
            let segment_gain = gain as f32;
            apply_gain(inputs, outputs, cursor, offset, segment_gain);
            gain = GAIN_PARAM.denormalize(param.value);
            cursor = offset;
        }
        #[expect(clippy::cast_possible_truncation)]
        let tail_gain = gain as f32;
        apply_gain(inputs, outputs, cursor, frames, tail_gain);
        self.gain = gain;
        if inputs.is_empty() || outputs.is_empty() {
            ProcessResult::SLEEP
        } else {
            ProcessResult::CONTINUE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruviform_audio_core::{PlanarBufferView, PlanarBufferViewMut};
    use ruviform_plugin_core::{Event, ParamEvent, ParamId, ParamValue};

    fn param_event(offset: u32, normalized: f64) -> Event {
        Event::new(
            offset,
            EventKind::ParamValue(ParamEvent {
                param_id: GAIN_PARAM.id,
                value: ParamValue::new(normalized).unwrap(),
            }),
        )
    }

    /// Runs one block through the neutral contract — the same call
    /// shape the component's `process` makes.
    fn run_block(
        processor: &mut TestGainProcessor,
        input: &[Vec<f32>],
        output: &mut [Vec<f32>],
        events: &[Event],
    ) {
        let frames = u32::try_from(input.first().map_or(0, Vec::len)).unwrap_or(u32::MAX);
        let in_views: Vec<_> = input.iter().map(|c| &c[..]).collect();
        let inputs = [PlanarBufferView::new(&in_views).unwrap()];
        let mut out_views: Vec<_> = output.iter_mut().map(|c| &mut c[..]).collect();
        let mut outputs = [PlanarBufferViewMut::new(&mut out_views).unwrap()];
        let list = EventList::new(events, frames).unwrap();
        let mut context = ProcessContext::new();
        processor.process(&inputs, &mut outputs, list, None, &mut context);
    }

    #[test]
    fn default_gain_is_unity() {
        let mut processor = TestGainProcessor::new();
        let input = [vec![0.25_f32; 8], vec![-0.5; 8]];
        let mut output = [vec![0.0_f32; 8], vec![0.0; 8]];
        run_block(&mut processor, &input, &mut output, &[]);
        assert_eq!(output[0], input[0]);
        assert_eq!(output[1], input[1]);
    }

    #[test]
    fn normalized_maps_to_plain_non_identically() {
        // normalized 0.25 → plain 0.5; normalized 0.75 → plain 1.5.
        assert_eq!(GAIN_PARAM.denormalize(ParamValue::new(0.25).unwrap()), 0.5);
        assert_eq!(GAIN_PARAM.denormalize(ParamValue::new(0.75).unwrap()), 1.5);
        // Round-trip through the neutral mapping.
        for normalized in [0.0, 0.1, 0.5, 0.9, 1.0] {
            let plain = GAIN_PARAM.denormalize(ParamValue::new(normalized).unwrap());
            let back = GAIN_PARAM.normalize(plain).unwrap();
            assert!((back.get() - normalized).abs() < 1e-12);
        }
    }

    #[test]
    fn offset_zero_event_applies_to_whole_block() {
        let mut processor = TestGainProcessor::new();
        // normalized 0.25 → plain 0.5.
        let events = [param_event(0, 0.25)];
        let input = [vec![1.0_f32; 4]];
        let mut output = [vec![0.0_f32; 4]];
        run_block(&mut processor, &input, &mut output, &events);
        assert_eq!(output[0], vec![0.5_f32; 4]);
    }

    #[test]
    fn mid_block_event_switches_at_its_offset() {
        let mut processor = TestGainProcessor::new();
        // normalized 0.75 → plain 1.5, effective at frame 2.
        let events = [param_event(2, 0.75)];
        let input = [vec![1.0_f32; 4]];
        let mut output = [vec![0.0_f32; 4]];
        run_block(&mut processor, &input, &mut output, &events);
        assert_eq!(output[0], vec![1.0, 1.0, 1.5, 1.5]);
    }

    #[test]
    fn same_offset_events_last_wins() {
        let mut processor = TestGainProcessor::new();
        // Two events at offset 1: normalized 0.25 (→0.5) then 1.0 (→2.0).
        let events = [param_event(1, 0.25), param_event(1, 1.0)];
        let input = [vec![1.0_f32; 3]];
        let mut output = [vec![0.0_f32; 3]];
        run_block(&mut processor, &input, &mut output, &events);
        // Frame 0 uses default 1.0; frames 1-2 use the last event → 2.0.
        assert_eq!(output[0], vec![1.0, 2.0, 2.0]);
    }

    #[test]
    fn zero_frame_block_flushes_events_and_persists_gain() {
        let mut processor = TestGainProcessor::new();
        // Flush block: offset-0 event only, no frames — normalized 0.0
        // → plain 0.0.
        let flush = [param_event(0, 0.0)];
        processor.process::<PlanarBufferView, PlanarBufferViewMut>(
            &[],
            &mut [],
            EventList::new(&flush, 0).unwrap(),
            None,
            &mut ProcessContext::new(),
        );
        // Next block: gain persists → silence.
        let input = [vec![1.0_f32; 2]];
        let mut output = [vec![9.0_f32; 2]];
        run_block(&mut processor, &input, &mut output, &[]);
        assert_eq!(output[0], vec![0.0, 0.0]);
    }

    #[test]
    fn gain_persists_across_blocks() {
        let mut processor = TestGainProcessor::new();
        let events = [param_event(0, 0.75)]; // → plain 1.5
        let input = [vec![1.0_f32; 2]];
        let mut output = [vec![0.0_f32; 2]];
        run_block(&mut processor, &input, &mut output, &events);
        assert_eq!(output[0], vec![1.5, 1.5]);
        // Second block with no events keeps the gain.
        output[0].fill(0.0);
        run_block(&mut processor, &input, &mut output, &[]);
        assert_eq!(output[0], vec![1.5, 1.5]);
    }

    #[test]
    fn stereo_applies_per_channel() {
        let mut processor = TestGainProcessor::new();
        let events = [param_event(0, 0.25)]; // → 0.5
        let input = [vec![2.0_f32; 4], vec![-1.0; 4]];
        let mut output = [vec![0.0_f32; 4], vec![0.0; 4]];
        run_block(&mut processor, &input, &mut output, &events);
        assert_eq!(output[0], vec![1.0_f32; 4]);
        assert_eq!(output[1], vec![-0.5_f32; 4]);
    }

    #[test]
    fn unrelated_param_events_are_ignored() {
        let mut processor = TestGainProcessor::new();
        let events = [Event::new(
            0,
            EventKind::ParamValue(ParamEvent {
                param_id: ParamId(99),
                value: ParamValue::new(1.0).unwrap(),
            }),
        )];
        let input = [vec![1.0_f32; 2]];
        let mut output = [vec![0.0_f32; 2]];
        run_block(&mut processor, &input, &mut output, &events);
        assert_eq!(output[0], vec![1.0, 1.0]);
    }
}
