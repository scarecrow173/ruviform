//! `ruviform-plugin-host` — the plugin host layer of the Ruviform
//! media foundation: the standard-neutral boundary between the host
//! application and plugin instances, defined by the plugin-isolation
//! design (Issue #31 → `docs/designs/plugin-isolation.md`,
//! `docs/adr/plugin-isolation-strategy.md`) and implemented here at
//! levels L0 (in-process, the default path) and L1 (opt-in
//! out-of-process, Issue #76 → `docs/designs/plugin-l1-out-of-process.md`).
//!
//! ## Boundary
//!
//! Callers see plain data: [`HostCommand`]/[`HostReply`]/[`HostError`]
//! plus the opaque [`PluginHandle`]. Non-RT operations — scan,
//! instantiate, the [`LifecycleEvent`]-driven lifecycle verbs, state
//! save/restore and GUI open/close — all travel this one surface.
//! Per the isolation design `process` is **not** a command: in L0 it
//! stays a direct call into [`PluginProcessor`], which
//! [`InProcessHost::process`] / [`InProcessHost::plugin_mut`] expose
//! without IPC or blocking.
//!
//! ## Backends
//!
//! - **L0 (default)** — [`InProcessHost`]: factories registered via
//!   `register_factory` run in-process; `process` is a direct
//!   [`PluginProcessor`] call.
//! - **L1 (opt-in, Issue #76)** — child-process hosting behind a
//!   framed stdio control plane, a `shared_memory` data plane and
//!   `ruviform-rt` SPSC try-legs so the caller thread never touches
//!   pipes, shm or blocking I/O. Instances are placed per
//!   [`OutOfProcessSpec::isolation`]: `Bundled` (the default) packs
//!   them into shared children from the route's pool, `Dedicated`
//!   gives each its own child — see [`OutOfProcessIsolation`] for
//!   the crash/liveness trade-off. A live-exit child converges its
//!   instances to `Failed` and reports each once through
//!   [`PluginHost::try_recv_notification`] as
//!   [`HostNotification::InstanceFailed`]; a `Failed` instance
//!   rejects every command but `Destroy` (`Err(HostError::Failed)`)
//!   and its `process` leg emits silence — `Ok(CONTINUE)`, never a
//!   hang.
//!
//! [`PluginHost`] is the composite that serves both behind one
//! unified [`PluginHandle`] space: `register_factory` claims a
//! `plugin_id` for L0, [`PluginHost::register_out_of_process`]
//! ([`OutOfProcessSpec`]) claims one for L1, and
//! `Instantiate { plugin_id }` keeps its shape — the caller never
//! names a layer. Duplicate `plugin_id` claims across routes are the
//! defined rejection `Err(HostError::DuplicatePluginId)`.
//!
//! **Latency contract (L1):** an L1-routed block is answered
//! `lookahead_blocks` blocks later — latency =
//! `lookahead_blocks × activated max_frames` frames (default
//! 2 × 512 = 1024 frames ≈ 21.3 ms at 48 kHz; per-spec
//! [`OutOfProcessSpec::lookahead_blocks`]; divide `frames` by the
//! activated sample rate for seconds). Underruns pass through
//! the last received block (silence before the first), never error.
//!
//! ## Video surface (Issue #118)
//!
//! [`PluginHost::video`] borrows the host into a
//! [`VideoPluginHost`] — the NONRT sibling surface for
//! out-of-process video plugins: [`VideoHostCommand`]/
//! [`VideoHostReply`] carry `Scan`, `Instantiate` (with a
//! [`VideoProcessConfig`](ruviform_plugin_core::VideoProcessConfig)),
//! the lifecycle verbs, `Render` (the video `process` equivalent
//! — one whole [`VideoFrame`](ruviform_video_core::VideoFrame) per
//! call) and the plain-parameter verbs over the same unified
//! [`PluginHandle`] space. Registration takes a
//! [`VideoOutOfProcessSpec`] — **dedicated-only by construction**
//! (no `isolation`, no `lookahead_blocks`): every video instance
//! gets its own child and a host-owned `vshm` shared-memory
//! segment transports its frames. Audio and video share the
//! plugin-id namespace (`DuplicatePluginId` fires either
//! direction) but never cross: a video `Instantiate` on an audio
//! id is `UnknownPluginId`, an audio `HostCommand` on a video
//! handle is `Unsupported`, and the audio RT/endpoint surface
//! (`process`/`prepare_endpoint`) does not exist for video
//! handles (`Unsupported`/`EndpointUnavailable`). Every video
//! operation is non-RT.
//!
//! ## Endpoint surface (Issue #113)
//!
//! [`PluginHost::prepare_endpoint`] detaches one activated
//! instance's processing leg into a [`PluginEndpoint`] — a `Send`
//! object moved once into the RT-side owner (e.g. a graph node
//! closure), carrying the same `process`/`latency_frames`
//! vocabulary as the host path. The endpoint surface exists on the
//! composite only: a standalone [`InProcessHost`] keeps its
//! direct-call `process`/`plugin_mut` path. One instance carries at
//! most one endpoint at a time — a second attach, an L0 slot whose
//! factory skipped [`PluginFactory::create_sendable`], or an L1 leg
//! still checked out is the defined rejection
//! [`HostError::EndpointUnavailable`] (a host-local topology fact —
//! it deliberately carries no wire encoding). `PluginHost` issues
//! handles for audio instances only: video plugin instances
//! ([`VideoPluginInstance`](ruviform_plugin_core::VideoPluginInstance))
//! are **non-RT in every operation** — the endpoint/RT route simply
//! does not exist for them.
//!
//! The pipeline-latency query is [`PluginHost::latency_frames`]:
//! `Some(0)` for an activated L0 instance,
//! `Some(lookahead_blocks × activated max_frames)` for L1, `None`
//! before `Activate` or for an invalid handle — the same value a
//! detached [`PluginEndpoint::latency_frames`] keeps from attach
//! (a re-`Activate` may change the true latency; the endpoint's
//! stored integer does not track it).
//!
//! ## Supervisor util (Issue #116, opt-in)
//!
//! [`Supervisor`] is an opt-in crash-recovery driver layered on the
//! public surface only — the core host stays restart-free (AC4).
//! `watch` a handle, then call [`Supervisor::poll`] from the caller's
//! non-RT loop: `poll` becomes the **sole drain point** for
//! [`PluginHost::try_recv_notification`], replays the recovery
//! (`Destroy` → `Instantiate` → lifecycle → optional `RestoreState`)
//! and reports [`SupervisionEvent`]s for the caller to re-wire
//! endpoints and handle bookkeeping. All operations are non-RT.
//!
//! ## Plugin side
//!
//! [`Plugin`] extends [`PluginProcessor`] with the non-RT instance
//! hooks the command surface drives (`init`, processing notifications,
//! state, GUI — all with defaults so minimal plugins implement only
//! the descriptor, params and processing contract). [`PluginFactory`]
//! is the registration unit a standard adapter hands to
//! [`InProcessHost::register_factory`].
//!
//! ## Allowed dependency layers
//!
//! Extension layer — `ruviform-plugin-core` (and transitively the
//! foundations it uses), `ruviform-rt` (Primitives — the L1 RT legs)
//! and `shared_memory` (confined to the internal `l1::shm` module —
//! never exposed on the public API or feature names). No
//! standard-specific or binding-crate types appear here;
//! `ruviform-plugin-clap` / `-vst3` adapt to [`Plugin`]/[`PluginFactory`].
//!
//! ## RT contract
//!
//! [`InProcessHost::process`], [`InProcessHost::plugin_mut`] and
//! [`InProcessHost::lifecycle_state`] are RT-suitable — a `const`
//! lifecycle table lookup plus a direct call, no allocation, no
//! blocking, no IPC. [`PluginHost::process`] is RT-suitable on both
//! routes (L1 = `try_push`/`try_pop` + bounded staging copies only);
//! [`PluginHost::plugin_mut`] is RT-suitable but returns `None` for
//! L1 slots (and for L0 `Shared` slots — the plugin sits behind the
//! shared mutex). [`PluginHost::lifecycle_state`] and
//! [`PluginHost::latency_frames`] are const-cheap for L0 slots; for
//! L1 slots they take the instance's shared mutex — treat them as
//! non-RT. [`PluginEndpoint::process`] is the endpoint's
//! RT leg — `Send`, `try_lock`-only and allocation-free on the
//! block path (the `rt_`-prefixed tests pin that), and
//! [`PluginEndpoint::process_audio`] is its graph-facing sibling
//! — the single-bus `pull`-shaped shell over `process` under
//! the same RT contract. The attach-time getters
//! ([`PluginEndpoint::bounds`], [`PluginEndpoint::max_frames`],
//! [`PluginEndpoint::channels`], [`PluginEndpoint::latency_frames`])
//! are const-cheap; [`PluginHost::prepare_endpoint`] itself is the
//! non-RT mint.
//! Command dispatch (`execute`, `register_*`), `prepare_endpoint`
//! and `try_recv_notification` are **non-RT only**.
//!
//! Contract source: `docs/designs/plugin-isolation.md`,
//! `docs/designs/plugin-host-l0.md`,
//! `docs/designs/plugin-l1-out-of-process.md`,
//! `docs/adr/plugin-isolation-strategy.md`.
//!
//! [`LifecycleEvent`]: ruviform_plugin_core::LifecycleEvent
//! [`PluginProcessor`]: ruviform_plugin_core::PluginProcessor

#![allow(clippy::module_name_repetitions)]

mod command;
mod composite;
mod endpoint;
mod host;
// The `pub(crate)` L1 out-of-process instance machinery (Issue #76):
// spawn + stdio handshake, driver/reader threads, the shm data plane
// and the idempotent `Failed` convergence. `composite::PluginHost` is
// its only in-crate consumer; items kept solely for the `#[path]`
// test include (`tests/l1_proxy_e2e.rs`) carry their own
// `dead_code` allowances instead of a blanket module-level one.
pub(crate) mod l1;
mod plugin;
mod supervisor;
mod video;

pub use command::{
    FailureCause, GuiHandle, GuiParentToken, HostCommand, HostError, HostReply, PluginHandle,
};
pub use composite::{
    AudioCounterSnapshot, ChildExit, HostNotification, OutOfProcessIsolation, OutOfProcessSpec,
    PluginHost,
};
// The diagnostics query surface (Issue #121): `PluginHost`'s
// `diagnostics_*` methods return these — re-exported here because
// `mod l1` is `pub(crate)` (public consumers must be able to name
// the result types).
pub use endpoint::{EndpointBounds, PluginEndpoint};
pub use host::{BusReport, InProcessHost};
pub use l1::diag::{DiagRegistry, DiagnosticQuery, DiagnosticRecord, SpawnPhase, StderrSnapshot};
pub use plugin::{
    ErasedPlugin, HostInput, HostOutput, Plugin, PluginFactory, erase, erase_sendable,
};
pub use supervisor::{
    RestartPolicy, RestartRecipe, RestoreOutcome, SupervisedId, SupervisionError, SupervisionEvent,
    Supervisor,
};
pub use video::{VideoHostCommand, VideoHostReply, VideoOutOfProcessSpec, VideoPluginHost};

/// Returns the crate version string.
///
/// **RT contract:** RT-suitable — pure, allocation-free.
///
/// # Examples
///
/// ```
/// assert_ne!(ruviform_plugin_host::version(), "");
/// ```
#[must_use]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use ruviform_audio_core::SampleRate;
    use ruviform_plugin_core::{
        BufferView, BufferViewMut, Event, EventKind, EventList, LifecycleState, ParamEvent,
        ParamId, ParamInfo, ParamValue, PluginDescriptor, PrepareError, ProcessConfig,
        ProcessContext, ProcessResult, ProcessStatus,
    };

    use super::*;

    const GAIN_DESCRIPTOR: PluginDescriptor = PluginDescriptor {
        id: "org.ruviform.test.gain",
        name: "Test Gain",
        vendor: "Ruviform",
        version: "0.1.0",
        features: &[],
    };

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

    /// Minimal gain plugin mirroring the plugin-core test fixture,
    /// extended with state save/restore for the host contract.
    struct Gain {
        gain: f32,
        fail_prepare: bool,
    }

    impl ruviform_plugin_core::PluginProcessor for Gain {
        fn prepare(
            &mut self,
            _config: &ProcessConfig,
            _context: &mut ProcessContext,
        ) -> Result<(), PrepareError> {
            if self.fail_prepare {
                return Err(PrepareError::ActivationFailed);
            }
            Ok(())
        }

        fn process<I, O>(
            &mut self,
            inputs: &[I],
            outputs: &mut [O],
            events: EventList<'_>,
            _transport: Option<ruviform_plugin_core::TransportInfo>,
            _context: &mut ProcessContext,
        ) -> ProcessResult
        where
            I: BufferView,
            O: BufferViewMut,
        {
            for event in events.iter() {
                if let EventKind::ParamValue(ParamEvent { param_id, value }) = event.kind()
                    && param_id == ParamId(0)
                {
                    #[expect(clippy::cast_possible_truncation)]
                    let gain = value.get() as f32;
                    self.gain = gain;
                }
            }
            for (input, output) in inputs.iter().zip(outputs.iter_mut()) {
                for channel in 0..output.channels() {
                    if let (Some(src), Some(dst)) =
                        (input.channel(channel), output.channel_mut(channel))
                    {
                        for (s, d) in src.iter().zip(dst.iter_mut()) {
                            *d = *s * self.gain;
                        }
                    }
                }
            }
            ProcessResult::new(ProcessStatus::Continue, ProcessResult::TAIL_NONE)
        }
    }

    impl Plugin for Gain {
        fn descriptor(&self) -> PluginDescriptor {
            GAIN_DESCRIPTOR
        }

        fn params(&self) -> &[ParamInfo] {
            &[]
        }

        fn save_state(&mut self) -> Result<Vec<u8>, HostError> {
            Ok(self.gain.to_le_bytes().to_vec())
        }

        fn restore_state(&mut self, state: &[u8]) -> Result<(), HostError> {
            let bytes: [u8; 4] = state.try_into().map_err(|_| HostError::PluginFailed)?;
            self.gain = f32::from_le_bytes(bytes);
            Ok(())
        }
    }

    struct GainFactory {
        fail_prepare: bool,
    }

    impl PluginFactory for GainFactory {
        fn descriptor(&self) -> PluginDescriptor {
            GAIN_DESCRIPTOR
        }

        fn create(&self) -> Box<dyn ErasedPlugin> {
            erase(Gain {
                gain: 1.0,
                fail_prepare: self.fail_prepare,
            })
        }
    }

    fn host_with_gain() -> InProcessHost {
        let mut host = InProcessHost::new();
        host.register_factory(Box::new(GainFactory {
            fail_prepare: false,
        }));
        host
    }

    fn instantiate(host: &mut InProcessHost) -> PluginHandle {
        match host
            .execute(HostCommand::Instantiate {
                plugin_id: GAIN_DESCRIPTOR.id.to_owned(),
            })
            .unwrap()
        {
            HostReply::Instantiated { handle } => handle,
            _ => panic!("unexpected reply"),
        }
    }

    fn config() -> ProcessConfig {
        ProcessConfig::new(SampleRate::new(48_000).unwrap(), 1, 512).unwrap()
    }

    fn drive_to_processing(host: &mut InProcessHost, handle: PluginHandle) {
        host.execute(HostCommand::Initialize { handle }).unwrap();
        host.execute(HostCommand::Activate {
            handle,
            config: config(),
        })
        .unwrap();
        host.execute(HostCommand::StartProcessing { handle })
            .unwrap();
    }

    #[test]
    fn scan_lists_registered_factories() {
        let mut host = host_with_gain();
        match host.execute(HostCommand::Scan).unwrap() {
            HostReply::Scanned(descriptors) => {
                assert_eq!(descriptors.len(), 1);
                assert_eq!(descriptors[0].id, GAIN_DESCRIPTOR.id);
            }
            _ => panic!("unexpected reply"),
        }
    }

    #[test]
    fn instantiate_unknown_id_rejected() {
        let mut host = host_with_gain();
        assert_eq!(
            host.execute(HostCommand::Instantiate {
                plugin_id: "org.example.missing".to_owned(),
            }),
            Err(HostError::UnknownPluginId)
        );
    }

    #[test]
    fn lifecycle_command_chain() {
        let mut host = host_with_gain();
        let handle = instantiate(&mut host);
        assert_eq!(host.lifecycle_state(handle), Some(LifecycleState::Created));

        host.execute(HostCommand::Initialize { handle }).unwrap();
        assert_eq!(
            host.lifecycle_state(handle),
            Some(LifecycleState::Initialized)
        );

        host.execute(HostCommand::Activate {
            handle,
            config: config(),
        })
        .unwrap();
        host.execute(HostCommand::StartProcessing { handle })
            .unwrap();
        assert_eq!(
            host.lifecycle_state(handle),
            Some(LifecycleState::Processing)
        );

        // One block through the RT path.
        let input = [1.0f32, 0.5, -0.5, -1.0];
        let input_view = SliceIn(&[&input[..]]);
        let inputs = [HostInput(&input_view)];
        let mut out = [0.0f32; 4];
        let mut out_ch = [&mut out[..]];
        let mut out_view = SliceOut(&mut out_ch);
        let mut outputs = [HostOutput(&mut out_view)];
        let events = EventList::new(&[], 4).unwrap();
        let result = host
            .process(handle, &inputs, &mut outputs, events, None)
            .unwrap();
        assert_eq!(result.status, ProcessStatus::Continue);
        assert_eq!(out, input);

        host.execute(HostCommand::StopProcessing { handle })
            .unwrap();
        host.execute(HostCommand::Deactivate { handle }).unwrap();
        assert_eq!(
            host.lifecycle_state(handle),
            Some(LifecycleState::Initialized)
        );

        host.execute(HostCommand::Destroy { handle }).unwrap();
        assert_eq!(host.lifecycle_state(handle), None);
    }

    #[test]
    fn illegal_transitions_rejected() {
        let mut host = host_with_gain();
        let handle = instantiate(&mut host);
        drive_to_processing(&mut host, handle);

        // Deactivate while Processing.
        assert!(matches!(
            host.execute(HostCommand::Deactivate { handle }),
            Err(HostError::Lifecycle(_))
        ));
        // Still Processing — the rejection left the machine alone.
        assert_eq!(
            host.lifecycle_state(handle),
            Some(LifecycleState::Processing)
        );

        host.execute(HostCommand::StopProcessing { handle })
            .unwrap();
        // Destroy while Activated (needs Deactivate first).
        assert!(matches!(
            host.execute(HostCommand::Destroy { handle }),
            Err(HostError::Lifecycle(_))
        ));
        // Double Initialize.
        host.execute(HostCommand::Deactivate { handle }).unwrap();
        assert!(matches!(
            host.execute(HostCommand::Initialize { handle }),
            Err(HostError::Lifecycle(_))
        ));
    }

    #[test]
    fn activate_failure_reports_activate_failed() {
        let mut host = InProcessHost::new();
        host.register_factory(Box::new(GainFactory { fail_prepare: true }));
        let handle = instantiate(&mut host);
        host.execute(HostCommand::Initialize { handle }).unwrap();

        assert!(matches!(
            host.execute(HostCommand::Activate {
                handle,
                config: config(),
            }),
            Err(HostError::Prepare(PrepareError::ActivationFailed))
        ));
        // Reported failure destination: still Initialized, retryable.
        assert_eq!(
            host.lifecycle_state(handle),
            Some(LifecycleState::Initialized)
        );
    }

    #[test]
    fn state_roundtrip() {
        let mut host = host_with_gain();
        let handle = instantiate(&mut host);
        drive_to_processing(&mut host, handle);

        // Push gain to 0.5 via a param event through the RT path.
        let input = [1.0f32; 4];
        let input_view = SliceIn(&[&input[..]]);
        let inputs = [HostInput(&input_view)];
        let mut out = [0.0f32; 4];
        let mut out_ch = [&mut out[..]];
        let mut out_view = SliceOut(&mut out_ch);
        let mut outputs = [HostOutput(&mut out_view)];
        let event = Event::new(
            0,
            EventKind::ParamValue(ParamEvent {
                param_id: ParamId(0),
                value: ParamValue::new(0.5).unwrap(),
            }),
        );
        let event_list = [event];
        let events = EventList::new(&event_list, 4).unwrap();
        host.process(handle, &inputs, &mut outputs, events, None)
            .unwrap();
        assert_eq!(out, [0.5; 4]);

        // Save, restore a second instance, verify the byte roundtrip.
        let HostReply::State { bytes } = host.execute(HostCommand::SaveState { handle }).unwrap()
        else {
            panic!("unexpected reply");
        };

        let handle2 = instantiate(&mut host);
        drive_to_processing(&mut host, handle2);
        host.execute(HostCommand::RestoreState {
            handle: handle2,
            bytes,
        })
        .unwrap();

        let mut restored = [0.0f32; 4];
        let mut restored_ch = [&mut restored[..]];
        let mut restored_view = SliceOut(&mut restored_ch);
        let mut restored_outputs = [HostOutput(&mut restored_view)];
        let events = EventList::new(&[], 4).unwrap();
        host.process(handle2, &inputs, &mut restored_outputs, events, None)
            .unwrap();
        assert_eq!(restored, [0.5; 4]);
    }

    #[test]
    fn gui_unsupported_by_default() {
        let mut host = host_with_gain();
        let handle = instantiate(&mut host);
        drive_to_processing(&mut host, handle);
        assert_eq!(
            host.execute(HostCommand::OpenGui {
                handle,
                parent: GuiParentToken::new(0),
            }),
            Err(HostError::Unsupported)
        );
        assert_eq!(
            host.execute(HostCommand::CloseGui { handle }),
            Err(HostError::Unsupported)
        );
    }

    #[test]
    fn stale_handle_rejected() {
        let mut host = host_with_gain();
        let handle = instantiate(&mut host);
        host.execute(HostCommand::Initialize { handle }).unwrap();
        host.execute(HostCommand::Destroy { handle }).unwrap();

        // The destroyed handle is dead.
        assert_eq!(
            host.execute(HostCommand::Initialize { handle }),
            Err(HostError::InvalidHandle)
        );

        // Slot reuse yields a new generation; the stale handle stays dead.
        let handle2 = instantiate(&mut host);
        assert_eq!(handle2.slot(), handle.slot());
        assert_ne!(handle2.generation(), handle.generation());
        assert_eq!(
            host.execute(HostCommand::Initialize { handle }),
            Err(HostError::InvalidHandle)
        );
    }

    #[test]
    fn reused_slot_generations_do_not_alias() {
        let mut host = host_with_gain();
        // Churn one slot: each destroy frees it for the next
        // instantiate, and the generation must keep bumping — a
        // generation that reset on reuse would let a stale handle
        // validate against a different live instance.
        let handle0 = instantiate(&mut host);
        host.execute(HostCommand::Destroy { handle: handle0 })
            .unwrap();
        let handle1 = instantiate(&mut host);
        host.execute(HostCommand::Destroy { handle: handle1 })
            .unwrap();
        let handle2 = instantiate(&mut host);

        assert_eq!(handle0.slot(), handle1.slot());
        assert_eq!(handle1.slot(), handle2.slot());
        assert_eq!(handle1.generation(), 1);
        assert_eq!(handle2.generation(), 2);

        // The second stale handle must not alias the live instance;
        // the live handle still works.
        assert_eq!(
            host.execute(HostCommand::Initialize { handle: handle1 }),
            Err(HostError::InvalidHandle)
        );
        host.execute(HostCommand::Initialize { handle: handle2 })
            .unwrap();
        assert_eq!(
            host.lifecycle_state(handle2),
            Some(LifecycleState::Initialized)
        );
    }

    #[test]
    fn process_requires_processing_state() {
        let mut host = host_with_gain();
        let handle = instantiate(&mut host);
        host.execute(HostCommand::Initialize { handle }).unwrap();

        let input = [1.0f32; 4];
        let input_view = SliceIn(&[&input[..]]);
        let inputs = [HostInput(&input_view)];
        let mut out = [0.0f32; 4];
        let mut out_ch = [&mut out[..]];
        let mut out_view = SliceOut(&mut out_ch);
        let mut outputs = [HostOutput(&mut out_view)];
        let events = EventList::new(&[], 4).unwrap();
        assert!(matches!(
            host.process(handle, &inputs, &mut outputs, events, None),
            Err(HostError::Lifecycle(_))
        ));
    }

    #[cfg(debug_assertions)]
    mod alloc_probe {
        #[global_allocator]
        static ALLOC: assert_no_alloc::AllocDisabler = assert_no_alloc::AllocDisabler;
    }

    /// Issue #22 lane: the L0 RT path (handle lookup + lifecycle
    /// transition + direct process call) must not allocate.
    #[test]
    fn rt_process_no_alloc() {
        let mut host = host_with_gain();
        let handle = instantiate(&mut host);
        drive_to_processing(&mut host, handle);

        let input = [0.25f32; 8];
        let input_view = SliceIn(&[&input[..]]);
        let inputs = [HostInput(&input_view)];
        let mut out = [0.0f32; 8];
        let mut out_ch = [&mut out[..]];
        let mut out_view = SliceOut(&mut out_ch);
        let mut outputs = [HostOutput(&mut out_view)];
        assert_no_alloc::assert_no_alloc(|| {
            for _ in 0..64 {
                let events = EventList::new(&[], 8).unwrap();
                host.process(handle, &inputs, &mut outputs, events, None)
                    .unwrap();
            }
        });
    }
}
