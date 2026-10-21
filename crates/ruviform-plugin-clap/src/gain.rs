//! The `PoC` plugin's DSP core: [`GainProcessor`], a real
//! [`PluginProcessor`] implementation living on the *neutral* side of
//! the adapter boundary — no CLAP or `clack` types appear here.
//!
//! ## Processing semantics
//!
//! The plugin applies a single gain multiplier (`[0.0, 2.0]` plain,
//! normalized `[0.0, 1.0]` on the wire) to every output channel.
//! Parameter events are applied **sample-accurately**: the block is
//! processed in segments delimited by event frame offsets, so a value
//! change at offset `o` affects samples `o..` of the same block.
//!
//! ## In-place semantics (adapter contract)
//!
//! For each output channel the input view either supplies a
//! matching input channel or does not:
//!
//! - `input.channel(c)` present — `out[c] = in[c] * gain` (normal
//!   paired path).
//! - `input.channel(c)` absent — the output channel itself carries
//!   the input data and is scaled in place (`out[c] *= gain`). The
//!   CLAP adapter uses this branch when `clack` reports
//!   `ChannelPair::InPlace`, so the shared and mutable views never
//!   alias the same memory.

use ruviform_plugin_core::{
    BufferView, BufferViewMut, EventKind, EventList, ParamEvent, ParamFlags, ParamId, ParamInfo,
    PluginProcessor, PrepareError, ProcessConfig, ProcessContext, ProcessResult, TransportInfo,
};

/// The single gain parameter this `PoC` exposes: plain range
/// `[0.0, 2.0]` as a linear multiplier, default `1.0` (unity).
///
/// **RT contract:** RT-suitable — `Copy` descriptor value.
///
/// # Examples
///
/// ```
/// use ruviform_plugin_clap::GAIN_PARAM;
///
/// // Unity gain (1.0) sits at the middle of the normalized range.
/// assert_eq!(GAIN_PARAM.default_normalized().unwrap().get(), 0.5);
/// assert_eq!(GAIN_PARAM.denormalize(ruviform_plugin_core::ParamValue::ZERO), 0.0);
/// ```
pub const GAIN_PARAM: ParamInfo = ParamInfo {
    id: ParamId(0),
    name: "Gain",
    unit: "x",
    min_value: 0.0,
    max_value: 2.0,
    default_value: 1.0,
    flags: ParamFlags::AUTOMATABLE,
};

/// Minimal gain [`PluginProcessor`]: multiplies every output channel
/// by [`GAIN_PARAM`]'s plain value, driven by normalized
/// [`ParamEvent`]s applied at their frame offsets.
///
/// **RT contract:** `process` is RT-suitable — bounded work over
/// `channels × frames`, no allocation, no blocking. `prepare` is the
/// non-RT hook.
///
/// # Examples
///
/// ```
/// use ruviform_audio_core::PlanarBufferView;
/// use ruviform_audio_core::SampleRate;
/// use ruviform_plugin_clap::{GAIN_PARAM, GainProcessor};
/// use ruviform_plugin_core::{
///     Event, EventKind, EventList, ParamEvent, ParamValue, PluginProcessor, ProcessConfig,
///     ProcessContext, ProcessStatus,
/// };
///
/// let input_data = [0.5_f32, -0.5, 1.0];
/// let in_channels = [&input_data[..]];
/// let input_view = PlanarBufferView::new(&in_channels).unwrap();
/// let mut out_data = [0.0_f32; 3];
/// let mut out_channels = [&mut out_data[..]];
/// let mut output_view = ruviform_audio_core::PlanarBufferViewMut::new(&mut out_channels).unwrap();
///
/// // Set gain to 0.5 at frame offset 1: sample 0 keeps gain 1.0.
/// let events = [Event::new(1, EventKind::ParamValue(ParamEvent {
///     param_id: GAIN_PARAM.id,
///     value: ParamValue::new(0.25).unwrap(), // plain = 0.25 * 2.0 = 0.5
/// }))];
///
/// let mut processor = GainProcessor::new();
/// let mut context = ProcessContext::new();
/// processor
///     .prepare(&ProcessConfig::new(SampleRate::new(48_000).unwrap(), 1, 512).unwrap(), &mut context)
///     .unwrap();
/// let mut outputs = [output_view];
/// let result = processor.process(
///     &[input_view],
///     &mut outputs,
///     EventList::new(&events, 3).unwrap(),
///     None,
///     &mut context,
/// );
/// assert_eq!(result.status, ProcessStatus::Continue);
/// assert_eq!(out_data, [0.5, -0.25, 0.5]);
/// ```
#[derive(Debug)]
pub struct GainProcessor {
    /// Current gain as a plain multiplier in `[0.0, 2.0]`.
    gain: f32,
}

impl GainProcessor {
    /// A processor at the parameter's default value (unity gain).
    ///
    /// **RT contract:** RT-suitable — pure, allocation-free.
    #[must_use]
    pub const fn new() -> Self {
        // `GAIN_PARAM.default_value` (1.0) is exactly representable as
        // f32; keep the field type f32 for the RT sample loop.
        Self { gain: 1.0 }
    }

    /// The current gain as a plain multiplier.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn gain(&self) -> f32 {
        self.gain
    }

    /// The current gain as a plain `f64` — the parameter mirror the
    /// adapter reports through its params extension.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn gain_plain(&self) -> f64 {
        self.gain as f64
    }

    /// The plain gain for a normalized value — `denormalize` yields
    /// `[0.0, 2.0]`; the cast to f32 is a defined narrowing inside
    /// that bounded range.
    fn denorm_gain(value: ruviform_plugin_core::ParamValue) -> f32 {
        #[expect(clippy::cast_possible_truncation)]
        {
            GAIN_PARAM.denormalize(value) as f32
        }
    }

    /// Applies a normalized parameter event; returns `true` when the
    /// event targeted [`GAIN_PARAM`].
    ///
    /// **RT contract:** RT-suitable — pure math, allocation-free.
    pub(crate) fn apply_param(&mut self, event: ParamEvent) -> bool {
        if event.param_id != GAIN_PARAM.id {
            return false;
        }
        self.gain = Self::denorm_gain(event.value);
        true
    }

    /// Sets the gain directly from a plain `f64` — used by the
    /// adapter to sync the parameter mirror, not by `process`.
    ///
    /// **RT contract:** RT-suitable — bounded cast, allocation-free.
    pub(crate) fn set_gain_plain(&mut self, plain: f64) {
        #[expect(clippy::cast_possible_truncation)]
        {
            self.gain = plain.clamp(GAIN_PARAM.min_value, GAIN_PARAM.max_value) as f32;
        }
    }

    /// Scales `dst[start..end]` by `gain`, reading from `src` when
    /// present or treating `dst` itself as the input (in-place) when
    /// absent. `src` shorter than `dst` is impossible per the adapter
    /// contract; the `min` is a defensive bound, never a silent
    /// channel-index shift.
    fn apply_gain_segment(
        src: Option<&[f32]>,
        dst: &mut [f32],
        start: usize,
        end: usize,
        gain: f32,
    ) {
        let end = end.min(dst.len());
        match src {
            Some(s) => {
                let end = end.min(s.len());
                for i in start..end {
                    dst[i] = s[i] * gain;
                }
            }
            None => {
                for sample in &mut dst[start..end] {
                    *sample *= gain;
                }
            }
        }
    }
}

impl Default for GainProcessor {
    /// [`GainProcessor::new`] — unity gain.
    fn default() -> Self {
        Self::new()
    }
}

impl PluginProcessor for GainProcessor {
    fn prepare(
        &mut self,
        config: &ProcessConfig,
        _context: &mut ProcessContext,
    ) -> Result<(), PrepareError> {
        // Zero-frame configs are rejected: the sample loop would never
        // run and downstream contracts assume ≥1-frame channels.
        if config.max_frames() == 0 {
            return Err(PrepareError::UnsupportedConfig);
        }
        // Deliberately no parameter reset: parameter values persist
        // across activate/deactivate cycles — the adapter seeds the
        // gain from its param mirror after each activation.
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
        let start_gain = self.gain;
        let mut processed_any = false;
        for (bus, out) in outputs.iter_mut().enumerate() {
            let input = inputs.get(bus);
            for channel in 0..out.channels() {
                let Some(dst) = out.channel_mut(channel) else {
                    continue;
                };
                let src = input.and_then(|view| view.channel(channel));
                let frames = dst.len();
                // The param timeline is processor-global — replay it
                // per channel from the block-start value instead of
                // mutating `self.gain` once per channel.
                let mut gain = start_gain;
                let mut position = 0_usize;
                for event in events.iter() {
                    let offset = (event.frame_offset() as usize).min(frames);
                    if offset > position {
                        Self::apply_gain_segment(src, dst, position, offset, gain);
                        processed_any = true;
                    }
                    if let EventKind::ParamValue(param_event) = event.kind()
                        && param_event.param_id == GAIN_PARAM.id
                    {
                        gain = Self::denorm_gain(param_event.value);
                    }
                    position = offset;
                }
                if position < frames {
                    Self::apply_gain_segment(src, dst, position, frames, gain);
                    processed_any = true;
                }
            }
        }
        // Parameter state is processor-global and must flush exactly
        // once per block — including zero-channel/zero-frame blocks
        // where the channel loop above never ran.
        for event in events.iter() {
            if let EventKind::ParamValue(param_event) = event.kind() {
                self.apply_param(param_event);
            }
        }
        if processed_any {
            ProcessResult::CONTINUE
        } else {
            // No channel carried a single frame — the block was empty
            // of audio (events may still have been applied above).
            ProcessResult::SLEEP
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruviform_audio_core::{PlanarBufferView, PlanarBufferViewMut, SampleRate};
    use ruviform_plugin_core::{Event, EventKind, ParamId, ParamValue, ProcessStatus};

    fn config() -> ProcessConfig {
        ProcessConfig::new(SampleRate::new(48_000).unwrap(), 1, 512).unwrap()
    }

    fn param_event(offset: u32, normalized: f64) -> Event {
        Event::new(
            offset,
            EventKind::ParamValue(ParamEvent {
                param_id: GAIN_PARAM.id,
                value: ParamValue::new(normalized).unwrap(),
            }),
        )
    }

    #[test]
    fn param_events_apply_sample_accurately() {
        let in_data = [1.0_f32; 8];
        let mut out_data = [0.0_f32; 8];
        let mut processor = GainProcessor::new();
        let mut context = ProcessContext::new();
        processor.prepare(&config(), &mut context).unwrap();

        let in_list = [&in_data[..]];
        let input_view = PlanarBufferView::new(&in_list).unwrap();
        let mut out_list = [&mut out_data[..]];
        let output_view = PlanarBufferViewMut::new(&mut out_list).unwrap();
        // Gain 1.0 until offset 4, then 0.5 (normalized 0.25 → plain 0.5).
        let events = [param_event(4, 0.25)];
        let mut outputs = [output_view];
        let result = processor.process(
            &[input_view],
            &mut outputs,
            EventList::new(&events, 8).unwrap(),
            None,
            &mut context,
        );
        assert_eq!(result.status, ProcessStatus::Continue);
        assert_eq!(
            out_data,
            [1.0, 1.0, 1.0, 1.0, 0.5, 0.5, 0.5, 0.5],
            "gain change must take effect exactly at the event offset"
        );
    }

    #[test]
    fn in_place_mode_scales_output_as_input() {
        // Adapter in-place contract: no input channel for the bus →
        // output channels hold the input data and are scaled in place.
        let mut io_data = [0.5_f32, 1.0, -0.5];
        let mut out_list = [&mut io_data[..]];
        let output_view = PlanarBufferViewMut::new(&mut out_list).unwrap();
        let empty_input = PlanarBufferView::empty();
        let mut processor = GainProcessor::new();
        let mut context = ProcessContext::new();
        processor.prepare(&config(), &mut context).unwrap();
        processor.apply_param(ParamEvent {
            param_id: GAIN_PARAM.id,
            value: ParamValue::new(0.25).unwrap(),
        });
        let mut outputs = [output_view];
        let result = processor.process(
            &[empty_input],
            &mut outputs,
            EventList::empty(3),
            None,
            &mut context,
        );
        assert_eq!(result.status, ProcessStatus::Continue);
        assert_eq!(io_data, [0.25, 0.5, -0.25]);
    }

    #[test]
    fn empty_block_flushes_events_and_sleeps() {
        let mut processor = GainProcessor::new();
        let mut context = ProcessContext::new();
        processor.prepare(&config(), &mut context).unwrap();
        let empty_in = PlanarBufferView::empty();
        let empty_out = PlanarBufferViewMut::empty();
        let events = [param_event(0, 0.25)];
        let mut outputs = [empty_out];
        let result = processor.process(
            &[empty_in],
            &mut outputs,
            EventList::new(&events, 0).unwrap(),
            None,
            &mut context,
        );
        // Zero frames: the offset-0 event still applied (flush).
        assert_eq!(processor.gain(), 0.5);
        assert_eq!(result.status, ProcessStatus::Sleep);
    }

    #[test]
    fn unknown_param_ids_are_ignored() {
        let mut processor = GainProcessor::new();
        assert!(!processor.apply_param(ParamEvent {
            param_id: ParamId(99),
            value: ParamValue::new(0.5).unwrap(),
        }));
        assert_eq!(processor.gain(), 1.0);
    }

    /// RT contract: `process` performs bounded scans and segment
    /// writes only — no allocation. (`rt_` tests cannot *observe*
    /// allocation; the no-alloc guarantee is a documented contract +
    /// review target until the Issue #22 lane lands.)
    #[test]
    fn rt_process_full_stereo_block() {
        let in_left = [0.25_f32; 64];
        let in_right = [-0.25_f32; 64];
        let mut out_left = [0.0_f32; 64];
        let mut out_right = [0.0_f32; 64];
        let in_list = [&in_left[..], &in_right[..]];
        let input_view = PlanarBufferView::new(&in_list).unwrap();
        let mut out_list = [&mut out_left[..], &mut out_right[..]];
        let output_view = PlanarBufferViewMut::new(&mut out_list).unwrap();
        let mut processor = GainProcessor::new();
        let mut context = ProcessContext::new();
        processor.prepare(&config(), &mut context).unwrap();
        let mut outputs = [output_view];
        let result = processor.process(
            &[input_view],
            &mut outputs,
            EventList::empty(64),
            None,
            &mut context,
        );
        assert_eq!(result.status, ProcessStatus::Continue);
        assert_eq!(out_left, [0.25; 64]);
        assert_eq!(out_right, [-0.25; 64]);
    }
}
