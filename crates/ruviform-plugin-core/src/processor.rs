//! The standard-neutral plugin processing contract: [`PluginProcessor`]
//! — the trait plugin authors implement and adapters drive — plus its
//! support types.
//!
//! ## Ownership of the contract
//!
//! `PluginProcessor` is owned by `ruviform-plugin-core`. Standard
//! adapters (`ruviform-plugin-clap`, `ruviform-plugin-vst3`) *implement
//! and drive* it; they never define a contract of their own.
//!
//! ## Borrowed buffer views (zero-copy)
//!
//! [`PluginProcessor::process`] receives *borrowed* input/output views
//! through the [`BufferView`] / [`BufferViewMut`] accessor traits —
//! deliberately a *trait contract*, not a concrete view type: the
//! concrete borrowed views implemented below —
//! `ruviform_audio_core::PlanarBufferView` / `PlanarBufferViewMut`
//! (per-channel slice lists) and `ruviform_audio_core::AudioBufferView`
//! / `AudioBufferViewMut` (channel-major slabs) — satisfy them, and
//! adapters may wrap host buffers in their own implementors. Because
//! `process` takes views rather than owning buffers,
//! the zero-copy rule is enforced by the signature itself; the concrete
//! view's lifetime parameter binds the borrow to the callback scope, so
//! a view can never outlive the `process` call that produced it.
//!
//! Channel contract: every channel slice in one view shares a single
//! frame length; channel order follows the negotiated bus channel
//! order; and all views in one `process` call share the block frame
//! count. Views for disabled buses are built as empty channel lists.
//!
//! ## Context
//!
//! [`ProcessContext`] is an opaque reserved type so that a future
//! capability injection (e.g. `RtContext` from `ruviform-rt`, deferred
//! to the RT-contract implementation issue) stays non-breaking.
//!
//! ## RT boundary
//!
//! `prepare` is the non-RT activation hook (allocation allowed);
//! `process` is the RT callback — no alloc/dealloc, no blocking, no
//! I/O, no unbounded waits or retries, no unbounded work. The `rt_`
//! tests exercise the contract but cannot *observe* allocation; the
//! no-alloc guarantee is a documented contract plus review target until
//! the Issue #22 RT-verification lane lands.

use ruviform_audio_core::SampleRate;
use ruviform_media_core::{Error, Result};

use crate::event::{Event, EventList};
use crate::transport::TransportInfo;

/// Read-side view contract over a per-channel planar `f32` buffer.
///
/// Implemented by the borrowed view type(s) adapters wrap host buffers
/// in — `ruviform_audio_core::PlanarBufferView` /
/// `PlanarBufferViewMut` (per-channel slice lists) and
/// `ruviform_audio_core::AudioBufferView` / `AudioBufferViewMut`
/// (channel-major slabs) are the canonical implementors. One
/// implementor = one bus: `channels()` is the bus's channel count and
/// every channel slice shares [`BufferView::frames`].
///
/// **RT contract:** RT-suitable — all methods are borrows or pure
/// reads; implementations must not allocate, block or do I/O.
///
/// # Examples
///
/// ```
/// use ruviform_plugin_core::BufferView;
///
/// // A minimal borrowed view shape — the same shape the concrete
/// // view lands in a follow-up task.
/// struct Channels<'a>(&'a [&'a [f32]]);
///
/// impl BufferView for Channels<'_> {
///     fn channels(&self) -> usize {
///         self.0.len()
///     }
///     fn frames(&self) -> usize {
///         self.0.first().map_or(0, |c| c.len())
///     }
///     fn channel(&self, index: usize) -> Option<&[f32]> {
///         self.0.get(index).copied()
///     }
/// }
///
/// let left = [0.0_f32; 4];
/// let right = [1.0_f32; 4];
/// let inputs = Channels(&[&left[..], &right[..]]);
/// assert_eq!(inputs.channels(), 2);
/// assert_eq!(inputs.channel(1), Some(&right[..]));
/// ```
pub trait BufferView {
    /// Number of channels in this view.
    ///
    /// **RT contract:** RT-suitable — pure read.
    fn channels(&self) -> usize;

    /// Number of frames — the shared length of every channel slice.
    ///
    /// **RT contract:** RT-suitable — pure read.
    fn frames(&self) -> usize;

    /// Channel `index` as a slice, or `None` when out of range.
    /// Every returned slice has [`BufferView::frames`] elements.
    ///
    /// **RT contract:** RT-suitable — borrow, no allocation.
    fn channel(&self, index: usize) -> Option<&[f32]>;
}

/// Write-side extension of [`BufferView`].
///
/// **RT contract:** RT-suitable — all methods are borrows;
/// implementations must not allocate, block or do I/O.
///
/// # Examples
///
/// ```
/// use ruviform_plugin_core::{BufferView, BufferViewMut};
///
/// struct ChannelsMut<'a>(&'a mut [&'a mut [f32]]);
///
/// impl BufferView for ChannelsMut<'_> {
///     fn channels(&self) -> usize {
///         self.0.len()
///     }
///     fn frames(&self) -> usize {
///         self.0.first().map_or(0, |c| c.len())
///     }
///     fn channel(&self, index: usize) -> Option<&[f32]> {
///         self.0.get(index).map(|c| &c[..])
///     }
/// }
///
/// impl BufferViewMut for ChannelsMut<'_> {
///     fn channel_mut(&mut self, index: usize) -> Option<&mut [f32]> {
///         self.0.get_mut(index).map(|c| &mut c[..])
///     }
/// }
///
/// let mut out = [0.0_f32; 4];
/// {
///     let mut slot = [&mut out[..]];
///     let mut outputs = ChannelsMut(&mut slot);
///     outputs.channel_mut(0).unwrap().fill(2.0);
///     assert_eq!(outputs.channel(0), Some(&[2.0; 4][..]));
/// }
/// assert_eq!(out, [2.0; 4]);
/// ```
pub trait BufferViewMut: BufferView {
    /// Channel `index` as a mutable slice, or `None` when out of
    /// range.
    ///
    /// **RT contract:** RT-suitable — borrow, no allocation.
    fn channel_mut(&mut self, index: usize) -> Option<&mut [f32]>;
}

/// Processing configuration negotiated at activation time
/// (`prepare`): sample rate plus the block-size bounds a `process`
/// call may carry.
///
/// `reserved` is an opaque extension field — always `[0; 2]` — kept so
/// future negotiation fields (e.g. processing precision) extend the
/// config without breaking its shape.
///
/// **RT contract:** RT-suitable — `Copy` value type.
///
/// # Examples
///
/// ```
/// use ruviform_audio_core::SampleRate;
/// use ruviform_plugin_core::ProcessConfig;
///
/// let config = ProcessConfig::new(SampleRate::new(48_000).unwrap(), 1, 512).unwrap();
/// assert_eq!(config.max_frames(), 512);
/// assert_eq!(config.reserved(), [0; 2]);
/// assert!(ProcessConfig::new(SampleRate::new(48_000).unwrap(), 512, 1).is_err());
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ProcessConfig {
    sample_rate: SampleRate,
    min_frames: u32,
    max_frames: u32,
    reserved: [u64; 2],
}

impl ProcessConfig {
    /// Creates a config; `min_frames` must not exceed `max_frames`.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    ///
    /// # Errors
    ///
    /// `Error::InvalidValue("frames")` when `min_frames > max_frames`.
    pub const fn new(sample_rate: SampleRate, min_frames: u32, max_frames: u32) -> Result<Self> {
        if min_frames > max_frames {
            return Err(Error::InvalidValue("frames"));
        }
        Ok(Self {
            sample_rate,
            min_frames,
            max_frames,
            reserved: [0; 2],
        })
    }

    /// The processing sample rate.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn sample_rate(self) -> SampleRate {
        self.sample_rate
    }

    /// Minimum block length a `process` call may carry.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn min_frames(self) -> u32 {
        self.min_frames
    }

    /// Maximum block length a `process` call may carry; adapters
    /// pre-allocate per-instance scratch (channel lists, bounce
    /// buffers) for this bound so `process` never allocates.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn max_frames(self) -> u32 {
        self.max_frames
    }

    /// Opaque extension reserve (currently always `[0; 2]`). Callers
    /// must not interpret its contents.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn reserved(&self) -> [u64; 2] {
        self.reserved
    }
}

/// Opaque per-instance context passed to [`PluginProcessor`] calls.
///
/// Carries no usable data today; the private `reserved` field lets a
/// future revision inject capabilities (e.g. `RtContext` from
/// `ruviform-rt`) without breaking the [`PluginProcessor`] signatures.
/// Plugins must not interpret its contents.
///
/// **RT contract:** RT-suitable — `Copy` value type; borrowing it
/// inside `process` is allocation-free.
///
/// # Examples
///
/// ```
/// use ruviform_plugin_core::ProcessContext;
///
/// let context = ProcessContext::new();
/// assert_eq!(context.reserved(), [0; 2]);
/// ```
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ProcessContext {
    reserved: [u64; 2],
}

impl ProcessContext {
    /// An empty context.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn new() -> Self {
        Self { reserved: [0; 2] }
    }

    /// Opaque extension reserve (currently always `[0; 2]`). Callers
    /// must not interpret its contents.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn reserved(&self) -> [u64; 2] {
        self.reserved
    }
}

/// Why [`PluginProcessor::prepare`] failed.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy — new variants may be added without breaking downstream
/// `match` arms.
///
/// **RT contract:** `prepare` is a non-RT hook; the value itself is
/// `Copy` and allocation-free.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum PrepareError {
    /// The supplied [`ProcessConfig`] cannot be honored (unsupported
    /// rate or frame bounds).
    UnsupportedConfig,
    /// Activation-time setup (resource allocation, DSP reset) failed.
    ActivationFailed,
}

impl core::fmt::Display for PrepareError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::UnsupportedConfig => f.write_str("unsupported processing configuration"),
            Self::ActivationFailed => f.write_str("activation-time setup failed"),
        }
    }
}

impl core::error::Error for PrepareError {}

/// Neutral processing status a [`PluginProcessor::process`] call ends
/// with; adapters map it to each standard's own status codes.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy — new variants may be added without breaking downstream
/// `match` arms.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ProcessStatus {
    /// Keep calling `process`.
    Continue,
    /// The plugin is finished until input changes — the host may stop
    /// calling `process` until then.
    Sleep,
    /// Processing failed; the adapter maps this to the standard's
    /// error status.
    Error,
}

/// The neutral result of one `process` call: a [`ProcessStatus`] plus
/// the plugin's tail length in frames.
///
/// `tail_frames` is the number of frames of meaningful output the
/// plugin still produces after input goes silent (`TAIL_NONE` =
/// already silent, `TAIL_INFINITE` = unbounded / unknown). Adapters
/// map it to each standard's tail reporting.
///
/// **RT contract:** RT-suitable — `Copy` value type.
///
/// # Examples
///
/// ```
/// use ruviform_plugin_core::{ProcessResult, ProcessStatus};
///
/// let result = ProcessResult::new(ProcessStatus::Continue, 48_000);
/// assert_eq!(result.tail_frames, 48_000);
/// assert_eq!(ProcessResult::CONTINUE.status, ProcessStatus::Continue);
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ProcessResult {
    /// How processing ended.
    pub status: ProcessStatus,
    /// Remaining tail in frames after the input goes silent.
    pub tail_frames: u32,
}

impl ProcessResult {
    /// Tail value for "no tail" — the plugin is already silent.
    pub const TAIL_NONE: u32 = 0;
    /// Tail value for an unbounded or unknown tail.
    pub const TAIL_INFINITE: u32 = u32::MAX;

    /// A `Continue` result with [`ProcessResult::TAIL_NONE`].
    pub const CONTINUE: Self = Self::new(ProcessStatus::Continue, Self::TAIL_NONE);
    /// A `Sleep` result.
    pub const SLEEP: Self = Self::new(ProcessStatus::Sleep, Self::TAIL_NONE);

    /// Creates a result from its parts.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn new(status: ProcessStatus, tail_frames: u32) -> Self {
        Self {
            status,
            tail_frames,
        }
    }
}

impl BufferView for ruviform_audio_core::PlanarBufferView<'_, f32> {
    fn channels(&self) -> usize {
        Self::channels(self)
    }
    fn frames(&self) -> usize {
        Self::frames(self)
    }
    fn channel(&self, index: usize) -> Option<&[f32]> {
        Self::channel(self, index)
    }
}

impl BufferView for ruviform_audio_core::PlanarBufferViewMut<'_, f32> {
    fn channels(&self) -> usize {
        Self::channels(self)
    }
    fn frames(&self) -> usize {
        Self::frames(self)
    }
    fn channel(&self, index: usize) -> Option<&[f32]> {
        Self::channel(self, index)
    }
}

impl BufferViewMut for ruviform_audio_core::PlanarBufferViewMut<'_, f32> {
    fn channel_mut(&mut self, index: usize) -> Option<&mut [f32]> {
        Self::channel_mut(self, index)
    }
}

impl BufferView for ruviform_audio_core::AudioBufferView<'_, f32> {
    fn channels(&self) -> usize {
        Self::channels(self)
    }
    fn frames(&self) -> usize {
        Self::frames(self)
    }
    fn channel(&self, index: usize) -> Option<&[f32]> {
        Self::channel(self, index)
    }
}

impl BufferView for ruviform_audio_core::AudioBufferViewMut<'_, f32> {
    fn channels(&self) -> usize {
        Self::channels(self)
    }
    fn frames(&self) -> usize {
        Self::frames(self)
    }
    fn channel(&self, index: usize) -> Option<&[f32]> {
        Self::channel(self, index)
    }
}

impl BufferViewMut for ruviform_audio_core::AudioBufferViewMut<'_, f32> {
    fn channel_mut(&mut self, index: usize) -> Option<&mut [f32]> {
        Self::channel_mut(self, index)
    }
}

/// The standard-neutral plugin processing contract owned by
/// `ruviform-plugin-core`.
///
/// Plugin authors implement this trait; standard adapters drive it
/// through the common [`Lifecycle`](crate::Lifecycle) machine —
/// `prepare` is called from the `activate` transition and `process`
/// only while the machine is in `Processing`.
///
/// The trait is deliberately not object-safe (`process` is generic
/// over the view contract): adapters call it on concrete plugin types.
///
/// **RT contract:** `prepare` is **non-RT only** (activation-time
/// allocation is allowed); `process` is **RT-suitable** — no
/// alloc/dealloc, blocking, I/O, unbounded waits/retries or unbounded
/// work — see the module docs.
pub trait PluginProcessor {
    /// Activation hook (non-RT): allocate and configure for
    /// processing under `config`.
    ///
    /// Called once per `activate` transition before the first
    /// `process`; resources set up here (including per-instance
    /// scratch sized to `config.max_frames()`) are what the RT path
    /// uses.
    ///
    /// **RT contract:** non-RT only — may allocate and block.
    ///
    /// # Errors
    ///
    /// [`PrepareError`] when the config cannot be honored or setup
    /// fails; the lifecycle stays in `Initialized`
    /// ([`LifecycleEvent::ActivateFailed`](crate::LifecycleEvent)).
    fn prepare(
        &mut self,
        config: &ProcessConfig,
        context: &mut ProcessContext,
    ) -> core::result::Result<(), PrepareError>;

    /// RT processing callback: read `inputs`, write `outputs`, apply
    /// `events` at their frame offsets.
    ///
    /// `inputs`/`outputs` hold one borrowed view per enabled bus, in
    /// bus order; the views borrow host buffers for the duration of
    /// this call only (the concrete view type binds that lifetime —
    /// see the module docs). `events` is the validated in-block event
    /// list; `transport` is `None` when the host supplies no transport
    /// state.
    ///
    /// **RT contract:** RT-suitable — no alloc/dealloc, blocking, I/O,
    /// unbounded waits/retries or unbounded work.
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
        O: BufferViewMut;

    /// Drains plugin→host reports queued by `process` calls since the
    /// last drain, appending them to `out`.
    ///
    /// A *report* is a canonical [`Event`] flowing in the reverse
    /// direction: [`EventKind::ParamValue`](crate::event::EventKind)
    /// reports a parameter value
    /// the plugin wants the host to observe (gesture confirmation,
    /// output parameter queues in standards that carry them), and
    /// `EventKind::Midi` reports a plugin-generated channel-voice
    /// event. Reports are produced during `process` into a bounded
    /// per-instance buffer; this pull-based drain gives the consumer
    /// natural back-pressure — undrained reports accumulate only up
    /// to the producer's declared bound and are dropped (counted) past
    /// it.
    ///
    /// The default implementation reports nothing — plugins without
    /// a reporting surface keep the previous behavior unchanged.
    ///
    /// **RT contract:** non-RT only — the `out` append may allocate
    /// (the caller owns and may reuse the buffer); implementations
    /// must not call it from inside `process`.
    fn drain_output_reports(&mut self, out: &mut Vec<Event>) {
        let _ = out;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{Event, EventKind, ParamEvent};
    use crate::param::{ParamId, ParamValue};

    struct SliceIn<'a>(&'a [&'a [f32]]);

    impl BufferView for SliceIn<'_> {
        fn channels(&self) -> usize {
            self.0.len()
        }
        fn frames(&self) -> usize {
            self.0.first().map_or(0, |c| c.len())
        }
        fn channel(&self, index: usize) -> Option<&[f32]> {
            self.0.get(index).copied()
        }
    }

    struct SliceOut<'a>(&'a mut [&'a mut [f32]]);

    impl BufferView for SliceOut<'_> {
        fn channels(&self) -> usize {
            self.0.len()
        }
        fn frames(&self) -> usize {
            self.0.first().map_or(0, |c| c.len())
        }
        fn channel(&self, index: usize) -> Option<&[f32]> {
            self.0.get(index).map(|c| &c[..])
        }
    }

    impl BufferViewMut for SliceOut<'_> {
        fn channel_mut(&mut self, index: usize) -> Option<&mut [f32]> {
            self.0.get_mut(index).map(|c| &mut c[..])
        }
    }

    /// Gain plugin: per-channel gain applied in place, driven by a
    /// single normalized parameter event.
    struct Gain {
        gain: f32,
    }

    impl PluginProcessor for Gain {
        fn prepare(
            &mut self,
            config: &ProcessConfig,
            _context: &mut ProcessContext,
        ) -> core::result::Result<(), PrepareError> {
            if config.sample_rate().get() == 0 {
                return Err(PrepareError::UnsupportedConfig);
            }
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
                let EventKind::ParamValue(ParamEvent { value, .. }) = event.kind() else {
                    continue;
                };
                // Test processor: normalized [0,1] `ParamValue` → f32 is a
                // defined narrowing within the normalized range.
                #[expect(clippy::cast_possible_truncation)]
                let gain = value.get() as f32;
                self.gain = gain;
            }
            let (Some(input), Some(output)) = (inputs.first(), outputs.first_mut()) else {
                return ProcessResult::new(ProcessStatus::Sleep, ProcessResult::TAIL_NONE);
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

    #[test]
    fn process_copies_through_views_zero_copy() {
        let input_data = [1.0_f32, 2.0, 3.0, 4.0];
        let inputs = [SliceIn(&[&input_data[..]])];
        let mut out_data = [0.0_f32; 4];
        let events = [Event::new(
            0,
            EventKind::ParamValue(ParamEvent {
                param_id: ParamId(0),
                value: ParamValue::new(0.5).unwrap(),
            }),
        )];

        let mut gain = Gain { gain: 0.0 };
        let mut context = ProcessContext::new();
        let config = ProcessConfig::new(SampleRate::new(48_000).unwrap(), 1, 512).unwrap();
        gain.prepare(&config, &mut context).unwrap();

        let mut slots = [&mut out_data[..]];
        let mut outputs = [SliceOut(&mut slots)];
        let result = gain.process(
            &inputs,
            &mut outputs,
            EventList::new(&events, 4).unwrap(),
            None,
            &mut context,
        );
        assert_eq!(result.status, ProcessStatus::Continue);
        assert_eq!(out_data, [0.5, 1.0, 1.5, 2.0]);
    }

    #[test]
    fn process_config_validates_bounds() {
        let rate = SampleRate::new(48_000).unwrap();
        assert!(ProcessConfig::new(rate, 128, 512).is_ok());
        assert!(ProcessConfig::new(rate, 512, 512).is_ok());
        assert_eq!(
            ProcessConfig::new(rate, 513, 512),
            Err(Error::InvalidValue("frames"))
        );
    }

    /// `AudioBufferView` satisfies `BufferView` with channel-major
    /// offsets: `channel(i)` is `&data[i*frames .. (i+1)*frames]`.
    #[test]
    fn audio_buffer_view_satisfies_buffer_view_channel_major() {
        use ruviform_audio_core::{AudioBuffer, BufferSpec, ChannelLayout};

        let spec = BufferSpec {
            layout: ChannelLayout::Stereo,
            frames: 4,
            sample_rate: SampleRate::new(48_000).unwrap(),
        };
        let mut buffer = AudioBuffer::<f32>::try_new(spec).unwrap();
        // Channel-major slab: [ch0 f0..f3 | ch1 f0..f3].
        buffer
            .data_mut()
            .copy_from_slice(&[1.0, 2.0, 3.0, 4.0, 10.0, 11.0, 12.0, 13.0]);

        let view = buffer.view();
        assert_eq!(BufferView::channels(&view), 2);
        assert_eq!(BufferView::frames(&view), 4);
        assert_eq!(
            BufferView::channel(&view, 0),
            Some(&[1.0, 2.0, 3.0, 4.0][..])
        );
        assert_eq!(
            BufferView::channel(&view, 1),
            Some(&[10.0, 11.0, 12.0, 13.0][..])
        );
        assert_eq!(BufferView::channel(&view, 2), None);
    }

    /// `AudioBufferViewMut` satisfies `BufferView`/`BufferViewMut`;
    /// `channel_mut` writes land in the underlying channel-major slab.
    #[test]
    fn audio_buffer_view_mut_writes_reach_the_slab() {
        use ruviform_audio_core::{AudioBufferViewMut, ChannelLayout};

        // 2 channels × 4 frames channel-major slab.
        let mut slab = [1.0_f32, 2.0, 3.0, 4.0, 10.0, 11.0, 12.0, 13.0];
        {
            let mut view = AudioBufferViewMut::try_new(&mut slab, 2, 4).unwrap();
            assert_eq!(BufferView::channels(&view), 2);
            assert_eq!(BufferView::frames(&view), 4);
            assert_eq!(
                BufferView::channel(&view, 1),
                Some(&[10.0, 11.0, 12.0, 13.0][..])
            );
            assert_eq!(BufferView::channel(&view, 2), None);
            BufferViewMut::channel_mut(&mut view, 0).unwrap()[0] = 42.0;
            assert!(BufferViewMut::channel_mut(&mut view, 2).is_none());
        }
        assert_eq!(slab[0], 42.0);
        assert_eq!(slab[4], 10.0);

        // `AudioBuffer::view_mut` builds the same channel-major view.
        let spec = ruviform_audio_core::BufferSpec {
            layout: ChannelLayout::Mono,
            frames: 2,
            sample_rate: SampleRate::new(48_000).unwrap(),
        };
        let mut buffer = ruviform_audio_core::AudioBuffer::<f32>::try_new(spec).unwrap();
        {
            let mut view = buffer.view_mut();
            BufferViewMut::channel_mut(&mut view, 0).unwrap().fill(0.5);
        }
        assert_eq!(buffer.channel(0), Some(&[0.5, 0.5][..]));
    }
}
