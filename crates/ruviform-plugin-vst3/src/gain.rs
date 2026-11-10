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

use ruviform_media_core::MidiKind;
use ruviform_plugin_core::{
    BufferView, BufferViewMut, Event, EventKind, EventList, ParamFlags, ParamId, ParamInfo,
    PluginProcessor, PrepareError, ProcessConfig, ProcessContext, ProcessResult, TransportInfo,
};

use crate::block::MAX_BLOCK_EVENTS;

/// VST3 `ParamID` of the single gain parameter.
pub(crate) const GAIN_PARAM_ID: u32 = 0;

/// Per-block plugin→host report bound — one report per consumed
/// input event plus headroom; capacity is reserved at `prepare` and
/// pushes past it are defined drops (the report channel is
/// best-effort, never an RT allocation).
const REPORT_CAPACITY: usize = MAX_BLOCK_EVENTS + 8;

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
    /// Plugin→host reports queued during `process` — drained by the
    /// adapter through [`PluginProcessor::drain_output_reports`]
    /// after each block. Bounded at [`REPORT_CAPACITY`]; a full
    /// buffer drops further reports by definition.
    pending_reports: Vec<Event>,
}

impl GainProcessor {
    /// A processor at the parameter default (unity gain).
    ///
    /// **RT contract:** RT-suitable — pure.
    #[must_use]
    pub(crate) fn new() -> Self {
        Self {
            gain: 1.0,
            pending_reports: Vec::new(),
        }
    }

    /// Queues one report — `false`-equivalent defined drop past
    /// [`REPORT_CAPACITY`] (no allocation: capacity was reserved at
    /// `prepare`).
    ///
    /// **RT contract:** RT-suitable — bounded push inside reserved
    /// capacity.
    fn push_report(&mut self, event: Event) {
        if self.pending_reports.len() < REPORT_CAPACITY {
            self.pending_reports.push(event);
        }
    }
}

impl PluginProcessor for GainProcessor {
    fn prepare(
        &mut self,
        _config: &ProcessConfig,
        _context: &mut ProcessContext,
    ) -> Result<(), PrepareError> {
        self.gain = 1.0;
        self.pending_reports.clear();
        self.pending_reports.reserve(REPORT_CAPACITY);
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
        // Per-block report semantics: only this block's emissions are
        // visible to the post-process drain.
        self.pending_reports.clear();
        for event in events.iter() {
            match event.kind() {
                // `EventKind` is `#[non_exhaustive]` — only the
                // parameter and MIDI variants exist today; unknown
                // future kinds are ignored.
                EventKind::ParamValue(param) if param.param_id == GAIN_PARAM.id => {
                    // Normalized [0,1] `ParamValue` → f32 is a defined
                    // narrowing within the normalized range.
                    #[expect(clippy::cast_possible_truncation)]
                    {
                        self.gain = param.value.get() as f32;
                    }
                    // Fixture report: confirm each applied value back
                    // to the host through the plugin→host channel —
                    // exists to prove the reporting path, not a
                    // musical feature.
                    self.push_report(Event::new(
                        event.frame_offset(),
                        EventKind::ParamValue(param),
                    ));
                }
                EventKind::Midi(midi) => {
                    // Fixture report: echo each `NoteOn` as an output
                    // channel-voice event — the `outputEvents` proof.
                    if matches!(midi.kind, MidiKind::NoteOn { .. }) {
                        self.push_report(Event::new(event.frame_offset(), EventKind::Midi(midi)));
                    }
                }
                _ => {}
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

    /// Moves the reports queued by the last `process` call into `out`
    /// — the adapter calls this once per block after `process`
    /// returns.
    fn drain_output_reports(&mut self, out: &mut Vec<Event>) {
        out.append(&mut self.pending_reports);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruviform_audio_core::{PlanarBufferView, PlanarBufferViewMut, SampleRate};
    use ruviform_media_core::{MidiChannel, MidiEvent, MidiGroup, MidiKind, MidiValue, NoteKey};
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

    /// The fixture's plugin→host reporting proof: a consumed
    /// `ParamValue` echoes back as a `ParamValue` report and a
    /// `NoteOn` echoes as a `Midi` report — both visible through
    /// `drain_output_reports` after `process`.
    #[test]
    fn process_queues_and_drains_output_reports() {
        let input_a = [0.5_f32; 4];
        let in_channels = [&input_a[..]];
        let in_view = PlanarBufferView::new(&in_channels).unwrap();
        let mut out_a = [0.0_f32; 4];
        let mut out_channels = [&mut out_a[..]];
        let out_view = PlanarBufferViewMut::new(&mut out_channels).unwrap();

        let note_on = MidiEvent {
            group: MidiGroup::ZERO,
            channel: MidiChannel::new(0).unwrap(),
            kind: MidiKind::NoteOn {
                key: NoteKey::new(60).unwrap(),
                velocity: MidiValue::from_u7(100),
            },
        };
        let events = [
            Event::new(
                0,
                EventKind::ParamValue(ParamEvent {
                    param_id: ParamId(GAIN_PARAM_ID),
                    value: ParamValue::new(0.5).unwrap(),
                }),
            ),
            Event::new(1, EventKind::Midi(note_on)),
        ];

        let mut processor = GainProcessor::new();
        let mut context = ProcessContext::new();
        let config = ProcessConfig::new(SampleRate::new(48_000).unwrap(), 1, 512).unwrap();
        processor.prepare(&config, &mut context).unwrap();

        let inputs = [in_view];
        let mut outputs = [out_view];
        processor.process(
            &inputs,
            &mut outputs,
            EventList::new(&events, 4).unwrap(),
            None,
            &mut context,
        );

        let mut reports = Vec::new();
        processor.drain_output_reports(&mut reports);
        assert_eq!(reports.len(), 2);
        assert_eq!(reports[0].frame_offset(), 0);
        assert_eq!(reports[1].frame_offset(), 1);
        let EventKind::ParamValue(param) = reports[0].kind() else {
            panic!("first report must be the ParamValue echo");
        };
        assert_eq!(param.param_id, ParamId(GAIN_PARAM_ID));
        assert_eq!(param.value.get(), 0.5);
        let EventKind::Midi(midi) = reports[1].kind() else {
            panic!("second report must be the Midi echo");
        };
        assert!(matches!(midi.kind, MidiKind::NoteOn { .. }));

        // The drain empties the queue — a second drain reports
        // nothing until the next `process`.
        let mut again = Vec::new();
        processor.drain_output_reports(&mut again);
        assert_eq!(again, Vec::<Event>::new());
    }

    /// A `process` with no reportable events leaves the drain empty.
    #[test]
    fn process_without_events_reports_nothing() {
        let input_a = [0.5_f32; 4];
        let in_channels = [&input_a[..]];
        let in_view = PlanarBufferView::new(&in_channels).unwrap();
        let mut out_a = [0.0_f32; 4];
        let mut out_channels = [&mut out_a[..]];
        let out_view = PlanarBufferViewMut::new(&mut out_channels).unwrap();

        let mut processor = GainProcessor::new();
        let mut context = ProcessContext::new();
        let config = ProcessConfig::new(SampleRate::new(48_000).unwrap(), 1, 512).unwrap();
        processor.prepare(&config, &mut context).unwrap();
        let inputs = [in_view];
        let mut outputs = [out_view];
        processor.process(
            &inputs,
            &mut outputs,
            EventList::empty(4),
            None,
            &mut context,
        );
        let mut reports = Vec::new();
        processor.drain_output_reports(&mut reports);
        assert_eq!(reports, Vec::<Event>::new());
    }
}
