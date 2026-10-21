//! The `PoC` plugin: a minimal gain `PluginProcessor` — one stereo
//! input/output bus pair and one normalized gain parameter — driven by
//! the VST3 adapter to prove the zero-copy process path end to end.
//!
//! The processor is deliberately trivial: it applies parameter events
//! first (the last [`ParamValue`] event in the block wins, matching the
//! upstream `vst3` gain example's tail-point semantics), then multiplies
//! every input channel frame into the matching output channel. A
//! missing bus pair yields `Sleep`, mirroring the plugin-core test
//! processor contract.

use ruviform_plugin_core::{
    BufferView, BufferViewMut, EventKind, EventList, ParamFlags, ParamId, ParamInfo,
    PluginProcessor, PrepareError, ProcessConfig, ProcessContext, ProcessResult, TransportInfo,
};

/// VST3 `ParamID` of the single gain parameter.
pub(crate) const GAIN_PARAM_ID: u32 = 0;

/// The normalized parameter maps linearly onto `[0.0, 1.0]` — the
/// normalized value *is* the gain multiplier, so plain and normalized
/// domains coincide (`normalizedParamToPlain`/`plainParamToNormalized`
/// are the identity).
pub(crate) const GAIN_PARAM: ParamInfo = ParamInfo {
    id: ParamId(GAIN_PARAM_ID),
    name: "Gain",
    unit: "",
    min_value: 0.0,
    max_value: 1.0,
    default_value: 1.0,
    flags: ParamFlags::AUTOMATABLE,
};

/// Minimal gain processor: `out[i][n] = in[i][n] * gain` per channel.
///
/// **RT contract:** `process` is RT-suitable — bounded per-sample loop,
/// no allocation; `prepare` is the non-RT activation hook.
pub(crate) struct GainProcessor {
    gain: f32,
}

impl GainProcessor {
    /// A processor at the parameter default (unity gain).
    ///
    /// **RT contract:** RT-suitable — pure.
    #[must_use]
    pub(crate) fn new() -> Self {
        Self { gain: 1.0 }
    }
}

impl PluginProcessor for GainProcessor {
    fn prepare(
        &mut self,
        _config: &ProcessConfig,
        _context: &mut ProcessContext,
    ) -> Result<(), PrepareError> {
        self.gain = 1.0;
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
            // `EventKind` is `#[non_exhaustive]` — only the parameter
            // variant exists today; unknown future kinds are ignored.
            if let EventKind::ParamValue(param) = event.kind() {
                if param.param_id == GAIN_PARAM.id {
                    // Normalized [0,1] `ParamValue` → f32 is a defined
                    // narrowing within the normalized range.
                    #[expect(clippy::cast_possible_truncation)]
                    {
                        self.gain = param.value.get() as f32;
                    }
                }
            }
        }
        let (Some(input), Some(output)) = (inputs.first(), outputs.first_mut()) else {
            return ProcessResult::SLEEP;
        };
        for channel in 0..input.channels().min(output.channels()) {
            let (Some(src), Some(dst)) = (input.channel(channel), output.channel_mut(channel))
            else {
                continue;
            };
            for (i, o) in src.iter().zip(dst.iter_mut()) {
                *o = *i * self.gain;
            }
        }
        ProcessResult::CONTINUE
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruviform_audio_core::{PlanarBufferView, PlanarBufferViewMut, SampleRate};
    use ruviform_plugin_core::{Event, ParamEvent, ParamValue, ProcessStatus};

    #[test]
    fn gain_applies_param_event_then_scales() {
        let input_a = [0.5_f32; 4];
        let input_b = [1.0_f32; 4];
        let in_channels = [&input_a[..], &input_b[..]];
        let in_view = PlanarBufferView::new(&in_channels).unwrap();

        let mut out_a = [0.0_f32; 4];
        let mut out_b = [0.0_f32; 4];
        let mut out_channels = [&mut out_a[..], &mut out_b[..]];
        let out_view = PlanarBufferViewMut::new(&mut out_channels).unwrap();

        let events = [Event::new(
            0,
            EventKind::ParamValue(ParamEvent {
                param_id: ParamId(GAIN_PARAM_ID),
                value: ParamValue::new(0.5).unwrap(),
            }),
        )];

        let mut processor = GainProcessor::new();
        let mut context = ProcessContext::new();
        let config = ProcessConfig::new(SampleRate::new(48_000).unwrap(), 1, 512).unwrap();
        processor.prepare(&config, &mut context).unwrap();

        let inputs = [in_view];
        let mut outputs = [out_view];
        let result = processor.process(
            &inputs,
            &mut outputs,
            EventList::new(&events, 4).unwrap(),
            None,
            &mut context,
        );
        assert_eq!(result.status, ProcessStatus::Continue);
        assert_eq!(result.tail_frames, ProcessResult::TAIL_NONE);
        assert_eq!(out_a, [0.25; 4]);
        assert_eq!(out_b, [0.5; 4]);
    }
}
