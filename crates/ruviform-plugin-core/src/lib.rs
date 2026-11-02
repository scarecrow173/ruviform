//! `ruviform-plugin-core` — the standard-neutral plugin foundation of
//! the Ruviform media foundation: plugin description
//! ([`PluginDescriptor`]), parameter description/value
//! ([`ParamInfo`], [`ParamValue`], [`ParamFlags`], [`ParamId`]),
//! sample-accurate events ([`Event`], [`EventKind`], [`EventList`]),
//! audio bus description ([`BusDescriptor`], [`BusDirection`]), a
//! standard-neutral transport snapshot ([`TransportInfo`]), the common
//! lifecycle state machine ([`Lifecycle`], [`LifecycleState`],
//! [`LifecycleEvent`], [`LifecycleError`]), the audio processing
//! contract ([`PluginProcessor`]) with its borrowed view contract
//! ([`BufferView`], [`BufferViewMut`]) and support types
//! ([`ProcessConfig`], [`ProcessContext`], [`ProcessResult`],
//! [`ProcessStatus`], [`PrepareError`]), plus the NONRT video plugin
//! contract ([`VideoPluginInstance`], [`VideoProcessConfig`]) and its
//! erased registry mirror ([`ErasedVideoPlugin`],
//! [`VideoPluginFactory`], [`erase_video`]), the
//! typed plain-parameter channel ([`PlainParamValue`],
//! [`PlainParamInfo`] and its owned twin [`PlainParamInfoOwned`],
//! borrowed [`PluginDescriptorView`]), the
//! plugin-side instance contracts ([`Plugin`], [`PluginFactory`],
//! [`ErasedPlugin`], [`erase`], [`erase_sendable`], [`HostInput`],
//! [`HostOutput`]), the shared host-boundary vocabulary
//! ([`PluginHandle`], [`GuiHandle`], [`GuiParentToken`],
//! [`HostError`]) and the L1 out-of-process wire codec ([`wire`]).
//!
//! Standard-specific types and concepts — CLAP, VST3, and any
//! binding-crate (`clack`, `vst3`, `clap-sys`, …) types — are
//! **non-goals**: nothing here names or wraps a plugin standard. The
//! standard adapters (`ruviform-plugin-clap` / `ruviform-plugin-vst3`)
//! translate standard ABI data into these types and implement/drive
//! [`PluginProcessor`]; they never define a processing contract of
//! their own. The concrete per-channel borrowed views are
//! `ruviform_audio_core::PlanarBufferView` / `PlanarBufferViewMut`
//! (per-channel slice lists) and `ruviform_audio_core::AudioBufferView`
//! / `AudioBufferViewMut` (channel-major slabs) — `process` stays
//! generic over the [`BufferView`] contract so adapters may equally
//! wrap host buffers in their own implementors. Factories that can
//! hand a `Send` instance to a cross-thread RT endpoint opt in via
//! [`PluginFactory::create_sendable`] (default `None`) and box it
//! with [`erase_sendable`]; [`HostError::EndpointUnavailable`] is the
//! local-only rejection for instances that cannot supply one — it
//! never travels the wire.
//!
//! ## Allowed dependency layers
//!
//! Extension layer — Domain foundations (`ruviform-audio-core`,
//! `ruviform-video-core`) + Primitives (`ruviform-media-core`) only. `ruviform-rt` is
//! deliberately *not* a dependency: accepting an `RtContext`
//! capability is deferred to the RT-contract implementation issue, so
//! context arguments are the opaque [`ProcessContext`] instead. Same-
//! and upper-tier dependencies are prohibited.
//!
//! ## RT contract
//!
//! `process` is the RT callback — no alloc/dealloc, blocking, I/O,
//! unbounded waits/retries or unbounded work. `prepare` is the non-RT
//! activation hook. Value types are `Copy` and allocation-free;
//! [`EventList::new`] validates with a bounded scan and never
//! allocates. Per-API guarantees are documented as
//! `**RT contract:**` lines on each item, mirroring the RT ADR rule
//! that the guaranteed scope is defined per API. `rt_`-prefixed tests
//! select the RT contract lane (`mise run test-rt`); they exercise the
//! contract paths but cannot observe allocation — until the Issue #22
//! RT-verification lane exists, no-alloc is a documented contract plus
//! review target.
//!
//! Contract source: `docs/designs/ruviform-architecture-design-baseline.md`
//! (crate table A.2, "プラグインアーキテクチャ"),
//! `docs/adr/plugin-architecture-clap-vst3.md` and
//! `docs/adr/realtime-contract-and-verification-policy.md`.

// Public names deliberately echo their modules
// (`lifecycle::LifecycleState`, `param::ParamInfo`, …) while being
// re-exported flat at the crate root, and as a `publish = false`
// workspace-internal value-type crate this API relies on literal
// construction for structs. Every public enum carries enum-level
// `#[non_exhaustive]`: exhaustive matching is reserved for
// crate-internal code while external users (sibling crates included)
// get forward compatibility — see
// `docs/adr/public-enum-non-exhaustive-policy.md`.
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::exhaustive_structs)]

mod boundary;
mod bus;
mod descriptor;
mod event;
mod lifecycle;
mod param;
mod param_write;
mod plugin;
mod processor;
mod transport;

mod plain_param;
mod transport_bridge;
mod video;
mod video_plugin;

pub mod wire;

pub use boundary::{FailureCause, GuiHandle, GuiParentToken, HostError, PluginHandle};
pub use bus::{BusDescriptor, BusDirection, BusRole};
pub use descriptor::{PluginDescriptor, PluginDescriptorView};
pub use event::{
    Event, EventKind, EventList, MAX_MERGE_SOURCES, MergeError, ParamEvent, merge_call_events,
    sort_events_by_frame_offset,
};
pub use lifecycle::{Lifecycle, LifecycleError, LifecycleEvent, LifecycleState};
pub use param::{ParamFlags, ParamId, ParamInfo, ParamValue};
pub use param_write::{
    ParamWriteMeans, ParamWriteRejection, TimedParamEvent, check_param_write,
    check_param_write_for, coalesce_param_events_latest,
};
pub use plain_param::{PlainParamInfo, PlainParamInfoOwned, PlainParamValue};
pub use plugin::{
    ErasedPlugin, HostInput, HostOutput, Plugin, PluginFactory, erase, erase_sendable,
};
pub use processor::{
    BufferView, BufferViewMut, PluginProcessor, PrepareError, ProcessConfig, ProcessContext,
    ProcessResult, ProcessStatus,
};
pub use transport::TransportInfo;
pub use transport_bridge::{
    build_call_event_list, call_transport_info, calls_for_span, contract_from_process_config,
};
pub use video::{VideoPluginInstance, VideoProcessConfig};
pub use video_plugin::{ErasedVideoPlugin, VideoPluginFactory, erase_video};

/// Returns the crate version string.
///
/// **RT contract:** RT-suitable — pure, allocation-free.
///
/// # Examples
///
/// ```
/// assert_ne!(ruviform_plugin_core::version(), "");
/// ```
#[must_use]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruviform_audio_core::SampleRate;
    use ruviform_media_core::{MediaTime, TimeBase, TransportStatus};

    /// `rt_` contract tests in this crate's lib test binary measure
    /// against this allocator (`assert_no_alloc::assert_no_alloc`).
    #[cfg(debug_assertions)]
    #[global_allocator]
    static ALLOC: assert_no_alloc::AllocDisabler = assert_no_alloc::AllocDisabler;

    #[test]
    fn version_is_not_empty() {
        assert_ne!(version(), "");
    }

    /// RT contract: lifecycle transitions are pure table lookups —
    /// allocation-free on the process path, including the per-block
    /// `Process` self-loop.
    #[test]
    fn rt_lifecycle_transition_table() {
        let mut lifecycle = Lifecycle::new();
        for event in [
            LifecycleEvent::Initialize,
            LifecycleEvent::Activate,
            LifecycleEvent::StartProcessing,
            LifecycleEvent::Process,
            LifecycleEvent::Process,
            LifecycleEvent::StopProcessing,
            LifecycleEvent::Deactivate,
            LifecycleEvent::Destroy,
        ] {
            lifecycle.transition(event).unwrap();
        }
        assert_eq!(lifecycle.state(), LifecycleState::Destroyed);
    }

    /// RT contract: `EventList::new` is a bounded `O(n)` scan with no
    /// allocation — sorted, bounded events come back as a borrowed
    /// view.
    #[test]
    fn rt_event_list_validation() {
        let events = [
            Event::new(
                0,
                EventKind::ParamValue(ParamEvent {
                    param_id: ParamId(0),
                    value: ParamValue::new(0.5).unwrap(),
                }),
            ),
            Event::new(
                64,
                EventKind::ParamValue(ParamEvent {
                    param_id: ParamId(1),
                    value: ParamValue::new(1.0).unwrap(),
                }),
            ),
        ];
        let list = EventList::new(&events, 128).unwrap();
        assert_eq!(list.len(), 2);
        assert!(EventList::new(&[events[1], events[0]], 128).is_err());
        assert_eq!(EventList::empty(128).len(), 0);
    }

    /// RT contract: normalized ↔ plain parameter mapping is pure math
    /// — no allocation, defined NaN policy (reject non-finite, clamp
    /// finite).
    #[test]
    fn rt_param_value_mapping() {
        const PARAM: ParamInfo = ParamInfo {
            id: ParamId(0),
            name: "Gain",
            unit: "dB",
            min_value: -60.0,
            max_value: 12.0,
            default_value: 0.0,
            flags: ParamFlags::AUTOMATABLE,
        };
        let normalized = PARAM.normalize(-6.0).unwrap();
        assert_eq!(PARAM.denormalize(normalized), -6.0);
        assert!(PARAM.normalize(f64::NAN).is_err());
        assert_eq!(ParamValue::new(-1.0).unwrap(), ParamValue::ZERO);
    }

    /// RT contract: `process` runs against borrowed views with no
    /// copy — writes land in the host-side storage. (The `rt_` filter
    /// cannot observe allocation; the no-alloc guarantee is a
    /// documented contract + review target — see crate docs.)
    #[test]
    fn rt_process_call_with_borrowed_views() {
        struct In<'a>(&'a [&'a [f32]]);
        impl BufferView for In<'_> {
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
        struct Out<'a>(&'a mut [&'a mut [f32]]);
        impl BufferView for Out<'_> {
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
        impl BufferViewMut for Out<'_> {
            fn channel_mut(&mut self, index: usize) -> Option<&mut [f32]> {
                self.0.get_mut(index).map(|c| &mut c[..])
            }
        }
        struct Copy;
        impl PluginProcessor for Copy {
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
                assert!(events.is_empty());
                if let (Some(i), Some(o)) = (inputs.first(), outputs.first_mut())
                    && let (Some(src), Some(dst)) = (i.channel(0), o.channel_mut(0))
                {
                    dst.copy_from_slice(src);
                }
                ProcessResult::CONTINUE
            }
        }

        let input_data = [1.0_f32, 2.0, 3.0];
        let inputs = [In(&[&input_data[..]])];
        let mut out_data = [0.0_f32; 3];
        let mut slots = [&mut out_data[..]];
        let mut outputs = [Out(&mut slots)];
        let mut context = ProcessContext::new();
        let transport = TransportInfo::new(
            TransportStatus::Playing,
            MediaTime::new(0, TimeBase::sample_rate(48_000).unwrap()),
        );

        let result = Copy.process(
            &inputs,
            &mut outputs,
            EventList::empty(3),
            Some(transport),
            &mut context,
        );
        assert_eq!(result.status, ProcessStatus::Continue);
        assert_eq!(out_data, input_data);
    }

    /// RT contract: `TransportInfo` accessors are `const` pure reads —
    /// no musical time, opaque reserve stays zeroed.
    #[test]
    fn rt_transport_snapshot_accessors() {
        let position = MediaTime::new(48_000, TimeBase::COMMON_90K);
        let info = TransportInfo::new(TransportStatus::Playing, position);
        assert_eq!(info.status(), TransportStatus::Playing);
        assert_eq!(info.position(), position);
        assert_eq!(info.loop_range(), None);
        assert_eq!(info.reserved(), [0; 2]);
    }

    #[test]
    fn process_config_and_context_reserved() {
        let config = ProcessConfig::new(SampleRate::new(48_000).unwrap(), 1, 1_024).unwrap();
        assert_eq!(config.min_frames(), 1);
        assert_eq!(config.max_frames(), 1_024);
        assert_eq!(config.reserved(), [0; 2]);
        assert_eq!(ProcessContext::new().reserved(), [0; 2]);
    }
}
