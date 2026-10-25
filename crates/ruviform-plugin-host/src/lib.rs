//! `ruviform-plugin-host` — the plugin host layer of the Ruviform
//! media foundation: the standard-neutral boundary between the host
//! application and plugin instances, defined by the plugin-isolation
//! design (Issue #31 → `docs/designs/plugin-isolation.md`,
//! `docs/adr/plugin-isolation-strategy.md`) and implemented here at
//! level L0 (in-process, the default path).
//!
//! ## Boundary
//!
//! Callers see plain data: [`HostCommand`]/[`HostReply`]/[`HostError`]
//! plus the opaque [`PluginHandle`]. Non-RT operations — scan,
//! instantiate, the [`LifecycleEvent`]-driven lifecycle verbs, state
//! save/restore and GUI open/close — all travel this one surface, so
//! an out-of-process (L1) backend can serve the identical commands
//! later without callers changing. Per the isolation design `process`
//! is **not** a command: in L0 it stays a direct call into
//! [`PluginProcessor`], which [`InProcessHost::process`] /
//! [`InProcessHost::plugin_mut`] expose without IPC or blocking.
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
//! foundations it uses) only. No standard-specific or binding-crate
//! types appear here; `ruviform-plugin-clap` / `-vst3` adapt to
//! [`Plugin`]/[`PluginFactory`].
//!
//! ## RT contract
//!
//! [`InProcessHost::process`], [`InProcessHost::plugin_mut`] and
//! [`InProcessHost::lifecycle_state`] are RT-suitable — a `const`
//! lifecycle table lookup plus a direct call, no allocation, no
//! blocking, no IPC. Command dispatch (`execute`,
//! `register_factory`) is **non-RT only**.
//!
//! Contract source: `docs/designs/plugin-isolation.md`,
//! `docs/designs/plugin-host-l0.md`, `docs/adr/plugin-isolation-strategy.md`.
//!
//! [`LifecycleEvent`]: ruviform_plugin_core::LifecycleEvent
//! [`PluginProcessor`]: ruviform_plugin_core::PluginProcessor

#![allow(clippy::module_name_repetitions)]

mod command;
mod host;
mod plugin;

pub use command::{GuiHandle, GuiParentToken, HostCommand, HostError, HostReply, PluginHandle};
pub use host::InProcessHost;
pub use plugin::{ErasedPlugin, HostInput, HostOutput, Plugin, PluginFactory, erase};

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
