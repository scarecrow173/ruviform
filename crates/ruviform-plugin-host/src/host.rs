//! [`InProcessHost`] — the L0 backend: factory registry, instance
//! slots, command dispatch and the direct-call RT `process` path.

use ruviform_plugin_core::{
    EventList, Lifecycle, LifecycleError, LifecycleEvent, LifecycleState, ProcessConfig,
    ProcessContext, ProcessResult, TransportInfo,
};

use crate::{
    ErasedPlugin, GuiHandle, HostCommand, HostError, HostInput, HostOutput, HostReply,
    PluginFactory, PluginHandle,
};

/// One live instance slot: the boxed plugin plus the host-side state
/// the command surface drives.
struct Instance {
    plugin: Box<dyn ErasedPlugin>,
    lifecycle: Lifecycle,
    /// Stamped into [`PluginHandle`]s; bumped on reuse so a stale
    /// handle is rejected instead of aliasing a new instance.
    generation: u16,
    /// Opaque per-instance context forwarded to `PluginProcessor`.
    context: ProcessContext,
    /// Whether a GUI surface is currently open.
    gui_open: bool,
}

impl Instance {
    fn new(plugin: Box<dyn ErasedPlugin>, generation: u16) -> Self {
        Self {
            plugin,
            lifecycle: Lifecycle::new(),
            generation,
            context: ProcessContext::new(),
            gui_open: false,
        }
    }
}

/// The L0 host: executes [`HostCommand`]s in-process against
/// registered factories and exposes the direct-call RT processing
/// path.
///
/// Per the plugin-isolation design (`docs/designs/plugin-isolation.md`)
/// L0 is the default path: callers use the same command surface an L1
/// out-of-process backend will serve later, and `process` stays a
/// direct `&mut dyn PluginProcessor` call — no IPC, no blocking.
///
/// **RT contract:** command dispatch (`execute`, `register_factory`)
/// is **non-RT only**. [`InProcessHost::process`] and
/// [`InProcessHost::processor`] are the RT surface — allocation-free
/// and non-blocking by construction.
pub struct InProcessHost {
    factories: Vec<Box<dyn PluginFactory>>,
    /// `None` marks a destroyed slot available for reuse.
    instances: Vec<Option<Instance>>,
    /// Indices of reusable `None` slots.
    free_slots: Vec<u32>,
}

impl Default for InProcessHost {
    fn default() -> Self {
        Self::new()
    }
}

impl InProcessHost {
    /// An empty host with no registered factories.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn new() -> Self {
        Self {
            factories: Vec::new(),
            instances: Vec::new(),
            free_slots: Vec::new(),
        }
    }

    /// Registers one plugin type for `Scan`/`Instantiate`.
    ///
    /// **RT contract:** non-RT only.
    pub fn register_factory(&mut self, factory: Box<dyn PluginFactory>) {
        self.factories.push(factory);
    }

    /// Executes one non-RT command against the registry and the
    /// instance it names.
    ///
    /// **RT contract:** non-RT only — may allocate and call non-RT
    /// plugin hooks.
    ///
    /// # Errors
    ///
    /// [`HostError`] — see each variant for the failure condition.
    pub fn execute(&mut self, command: HostCommand) -> Result<HostReply, HostError> {
        match command {
            HostCommand::Scan => Ok(HostReply::Scanned(
                self.factories
                    .iter()
                    .map(|factory| factory.descriptor())
                    .collect(),
            )),
            HostCommand::Instantiate { plugin_id } => self.instantiate(&plugin_id),
            HostCommand::Initialize { handle } => self.initialize(handle),
            HostCommand::Activate { handle, config } => self.activate(handle, &config),
            HostCommand::StartProcessing { handle } => self.start_processing(handle),
            HostCommand::StopProcessing { handle } => self.stop_processing(handle),
            HostCommand::Deactivate { handle } => {
                self.transition(handle, LifecycleEvent::Deactivate)
            }
            HostCommand::Destroy { handle } => self.destroy(handle),
            HostCommand::SaveState { handle } => self
                .instance_mut(handle)?
                .plugin
                .save_state()
                .map(|bytes| HostReply::State { bytes }),
            HostCommand::RestoreState { handle, bytes } => self
                .instance_mut(handle)?
                .plugin
                .restore_state(&bytes)
                .map(|()| HostReply::Ok),
            HostCommand::OpenGui { handle, parent } => self.open_gui(handle, parent),
            HostCommand::CloseGui { handle } => self.close_gui(handle),
        }
    }

    /// Direct access to an instance for the L0 RT path (e.g. bespoke
    /// [`ErasedPlugin::process_host`] dispatch).
    ///
    /// Returns `None` for an invalid handle; callers must only invoke
    /// processing while [`lifecycle_state`] reports `Processing` — or
    /// use [`InProcessHost::process`], which checks it for them.
    ///
    /// **RT contract:** RT-suitable — no allocation, no blocking.
    ///
    /// [`lifecycle_state`]: InProcessHost::lifecycle_state
    pub fn plugin_mut(&mut self, handle: PluginHandle) -> Option<&mut dyn ErasedPlugin> {
        self.instances
            .get_mut(handle.slot() as usize)
            .and_then(Option::as_mut)
            .filter(|instance| instance.generation == handle.generation())
            .map(|instance| &mut *instance.plugin as &mut dyn ErasedPlugin)
    }

    /// The instance's current lifecycle state, `None` for an invalid
    /// handle.
    ///
    /// **RT contract:** RT-suitable — no allocation, no blocking.
    #[must_use]
    pub fn lifecycle_state(&self, handle: PluginHandle) -> Option<LifecycleState> {
        self.instances
            .get(handle.slot() as usize)
            .and_then(Option::as_ref)
            .filter(|instance| instance.generation == handle.generation())
            .map(|instance| instance.lifecycle.state())
    }

    /// The RT processing path: validates `Processing` state (feeding
    /// the lifecycle `Process` event) then calls the plugin's erased
    /// [`Plugin::process_host`] directly.
    ///
    /// This is the L0 branch of the isolation design — a plain
    /// in-process call, no IPC and no blocking. `inputs`/`outputs`
    /// carry one erased view per enabled bus; the caller builds the
    /// slices (stack array or reusable scratch — nothing here
    /// allocates).
    ///
    /// **RT contract:** RT-suitable — the lifecycle transition is a
    /// `const` table lookup; nothing here allocates or blocks.
    ///
    /// # Errors
    ///
    /// [`HostError::InvalidHandle`] for a stale/unknown handle;
    /// [`HostError::Lifecycle`] when the instance is not `Processing`.
    pub fn process(
        &mut self,
        handle: PluginHandle,
        inputs: &[HostInput<'_>],
        outputs: &mut [HostOutput<'_>],
        events: EventList<'_>,
        transport: Option<TransportInfo>,
    ) -> Result<ProcessResult, HostError> {
        let instance = self.instance_mut(handle)?;
        instance
            .lifecycle
            .transition(LifecycleEvent::Process)
            .map_err(HostError::Lifecycle)?;
        Ok(instance
            .plugin
            .process_host(inputs, outputs, events, transport, &mut instance.context))
    }

    fn instantiate(&mut self, plugin_id: &str) -> Result<HostReply, HostError> {
        let position = self
            .factories
            .iter()
            .position(|factory| factory.descriptor().id == plugin_id)
            .ok_or(HostError::UnknownPluginId)?;
        let plugin = self.factories[position].create();
        if let Some(slot) = self.free_slots.pop() {
            let index = slot as usize;
            let generation = self.instances[index]
                .as_ref()
                .map_or(0, |instance| instance.generation)
                .wrapping_add(1);
            self.instances[index] = Some(Instance::new(plugin, generation));
            Ok(HostReply::Instantiated {
                handle: PluginHandle::new(slot, generation),
            })
        } else {
            let slot = u32::try_from(self.instances.len()).expect("instance slots fit u32");
            self.instances.push(Some(Instance::new(plugin, 0)));
            Ok(HostReply::Instantiated {
                handle: PluginHandle::new(slot, 0),
            })
        }
    }

    fn initialize(&mut self, handle: PluginHandle) -> Result<HostReply, HostError> {
        let instance = self.instance_mut(handle)?;
        // Validate before the hook: `InitFailed` is only reachable
        // from `Created`, so the success transition commits last.
        instance
            .lifecycle
            .state()
            .transition(LifecycleEvent::Initialize)
            .map_err(HostError::Lifecycle)?;
        if let Err(error) = instance.plugin.init() {
            // Reported failure destination: the machine stays Created.
            let _ = instance.lifecycle.transition(LifecycleEvent::InitFailed);
            return Err(error);
        }
        instance
            .lifecycle
            .transition(LifecycleEvent::Initialize)
            .map_err(HostError::Lifecycle)?;
        Ok(HostReply::Ok)
    }

    fn activate(
        &mut self,
        handle: PluginHandle,
        config: &ProcessConfig,
    ) -> Result<HostReply, HostError> {
        let instance = self.instance_mut(handle)?;
        // Same shape: `ActivateFailed` is only reachable from
        // `Initialized`, so `Activate` commits after `prepare`.
        instance
            .lifecycle
            .state()
            .transition(LifecycleEvent::Activate)
            .map_err(HostError::Lifecycle)?;
        if let Err(error) = instance.plugin.prepare(config, &mut instance.context) {
            // Reported failure destination: back to Initialized, retryable.
            let _ = instance
                .lifecycle
                .transition(LifecycleEvent::ActivateFailed);
            return Err(HostError::Prepare(error));
        }
        instance
            .lifecycle
            .transition(LifecycleEvent::Activate)
            .map_err(HostError::Lifecycle)?;
        Ok(HostReply::Ok)
    }

    fn start_processing(&mut self, handle: PluginHandle) -> Result<HostReply, HostError> {
        let instance = self.instance_mut(handle)?;
        instance
            .lifecycle
            .transition(LifecycleEvent::StartProcessing)
            .map_err(HostError::Lifecycle)?;
        if let Err(error) = instance.plugin.start_processing() {
            // Roll back: the plugin never started processing.
            let _ = instance
                .lifecycle
                .transition(LifecycleEvent::StopProcessing);
            return Err(error);
        }
        Ok(HostReply::Ok)
    }

    fn stop_processing(&mut self, handle: PluginHandle) -> Result<HostReply, HostError> {
        let instance = self.instance_mut(handle)?;
        instance
            .lifecycle
            .transition(LifecycleEvent::StopProcessing)
            .map_err(HostError::Lifecycle)?;
        instance.plugin.stop_processing()?;
        Ok(HostReply::Ok)
    }

    fn transition(
        &mut self,
        handle: PluginHandle,
        event: LifecycleEvent,
    ) -> Result<HostReply, HostError> {
        self.instance_mut(handle)?
            .lifecycle
            .transition(event)
            .map_err(HostError::Lifecycle)?;
        Ok(HostReply::Ok)
    }

    fn destroy(&mut self, handle: PluginHandle) -> Result<HostReply, HostError> {
        let instance = self.instance_mut(handle)?;
        instance
            .lifecycle
            .transition(LifecycleEvent::Destroy)
            .map_err(HostError::Lifecycle)?;
        self.instances[handle.slot() as usize] = None;
        self.free_slots.push(handle.slot());
        Ok(HostReply::Ok)
    }

    fn open_gui(
        &mut self,
        handle: PluginHandle,
        parent: crate::GuiParentToken,
    ) -> Result<HostReply, HostError> {
        let instance = self.instance_mut(handle)?;
        let gui: GuiHandle = instance.plugin.open_gui(parent)?;
        instance.gui_open = true;
        Ok(HostReply::Opened { gui })
    }

    fn close_gui(&mut self, handle: PluginHandle) -> Result<HostReply, HostError> {
        let instance = self.instance_mut(handle)?;
        instance.plugin.close_gui()?;
        instance.gui_open = false;
        Ok(HostReply::Ok)
    }

    fn instance_mut(&mut self, handle: PluginHandle) -> Result<&mut Instance, HostError> {
        self.instances
            .get_mut(handle.slot() as usize)
            .and_then(Option::as_mut)
            .filter(|instance| instance.generation == handle.generation())
            .ok_or(HostError::InvalidHandle)
    }
}

impl From<LifecycleError> for HostError {
    fn from(error: LifecycleError) -> Self {
        Self::Lifecycle(error)
    }
}
