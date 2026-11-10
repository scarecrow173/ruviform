//! The plugin-side contracts a host drives: [`Plugin`] (one
//! instance), [`PluginFactory`] (one registered plugin type) and the
//! erased I/O views [`HostInput`]/[`HostOutput`] the dyn dispatch
//! uses.
//!
//! Standard adapters (`ruviform-plugin-clap` / `-vst3`) implement
//! these traits over their wrapped plugin objects and register a
//! factory with the host; this crate never names a plugin standard.
//! The contracts live in `ruviform-plugin-core` (not the host crate)
//! so that both the host and an out-of-process plugin runner can name
//! them without a host↔runner dependency.

use crate::boundary::{GuiHandle, GuiParentToken, HostError};
use crate::{
    BufferView, BufferViewMut, BusDescriptor, Event, EventList, ParamInfo, PluginDescriptor,
    PluginProcessor, PrepareError, ProcessConfig, ProcessContext, ProcessResult, TransportInfo,
};

/// An erased input-bus view: one `&dyn BufferView` per bus, passed to
/// the plugin's [`Plugin::process_host`] so heterogeneous instances
/// stay behind `dyn Plugin` without per-call genericity.
///
/// **RT contract:** RT-suitable — `Copy` borrowed view; the caller
/// builds the `&[HostInput]` slice (stack array or reusable scratch).
pub struct HostInput<'a>(pub &'a dyn BufferView);

impl BufferView for HostInput<'_> {
    fn channels(&self) -> usize {
        self.0.channels()
    }
    fn frames(&self) -> usize {
        self.0.frames()
    }
    fn channel(&self, index: usize) -> Option<&[f32]> {
        self.0.channel(index)
    }
}

/// An erased output-bus view — the mutable counterpart of
/// [`HostInput`].
///
/// **RT contract:** RT-suitable — borrowed view; the caller builds
/// the `&mut [HostOutput]` slice (stack array or reusable scratch).
pub struct HostOutput<'a>(pub &'a mut dyn BufferViewMut);

impl BufferView for HostOutput<'_> {
    fn channels(&self) -> usize {
        self.0.channels()
    }
    fn frames(&self) -> usize {
        self.0.frames()
    }
    fn channel(&self, index: usize) -> Option<&[f32]> {
        self.0.channel(index)
    }
}

impl BufferViewMut for HostOutput<'_> {
    fn channel_mut(&mut self, index: usize) -> Option<&mut [f32]> {
        self.0.channel_mut(index)
    }
}

/// One hosted plugin instance, driven through the host's command and
/// RT surfaces.
///
/// `Plugin` extends [`PluginProcessor`] — the standard-neutral RT
/// contract from `ruviform-plugin-core` — with the non-RT instance
/// hooks the host's command surface needs. Hooks that not every
/// plugin implements (`init`, processing notifications, state, GUI)
/// carry default implementations that decline or no-op, so a minimal
/// plugin only fills in `descriptor`, `params` and the processing
/// contract.
///
/// Because [`PluginProcessor::process`] is generic, `Plugin` itself is
/// not dyn-compatible; factories hand the host an [`ErasedPlugin`]
/// produced by [`erase()`], which wraps any `P: Plugin` and exposes a
/// dyn-safe surface (including [`Plugin::process_host`]).
///
/// **RT contract:** [`Plugin::process_host`] is the RT entry point —
/// no allocation, no blocking. Every other method here is **non-RT
/// only**.
pub trait Plugin: PluginProcessor {
    /// The plugin's standard-neutral descriptor.
    ///
    /// **RT contract:** non-RT only.
    fn descriptor(&self) -> PluginDescriptor;

    /// The plugin's parameter table, in stable index order.
    ///
    /// **RT contract:** non-RT only.
    fn params(&self) -> &[ParamInfo];

    /// The plugin's declared buses, in the order the plugin presents
    /// them to processing (input buses first, in the standard's port
    /// order, then output buses in the same convention — or whichever
    /// stable order the adapter declares; the bridge maps each
    /// declared bus onto a typed-graph port by index).
    ///
    /// The default returns an empty slice, meaning "not declared":
    /// callers that cannot enumerate buses report that capability as
    /// unavailable rather than inferring a single stereo bus.
    ///
    /// **RT contract:** non-RT only.
    fn buses(&self) -> &[BusDescriptor] {
        &[]
    }

    /// The dyn-safe processing entry point the host invokes per
    /// block. The default forwards the erased views into
    /// [`PluginProcessor::process`]; overriding is only useful for
    /// bespoke dispatch.
    ///
    /// **RT contract:** RT-suitable — the default performs a direct
    /// call with borrowed views, no allocation, no blocking.
    fn process_host(
        &mut self,
        inputs: &[HostInput<'_>],
        outputs: &mut [HostOutput<'_>],
        events: EventList<'_>,
        transport: Option<TransportInfo>,
        context: &mut ProcessContext,
    ) -> ProcessResult {
        self.process(inputs, outputs, events, transport, context)
    }

    /// `Initialize` hook — plugin-side setup before activation.
    ///
    /// **RT contract:** non-RT only — may allocate and block.
    ///
    /// # Errors
    ///
    /// [`HostError::PluginFailed`] (or a more specific variant) when
    /// initialization fails; the host reports `InitFailed` to the
    /// lifecycle and the instance stays `Created`.
    fn init(&mut self) -> Result<(), HostError> {
        Ok(())
    }

    /// `StartProcessing` hook — notified after the instance enters
    /// `Processing`.
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// [`HostError::PluginFailed`] when the plugin cannot start; the
    /// host rolls the lifecycle back to `Activated`.
    fn start_processing(&mut self) -> Result<(), HostError> {
        Ok(())
    }

    /// `StopProcessing` hook — notified when the instance leaves
    /// `Processing`.
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// [`HostError::PluginFailed`] — the lifecycle has already left
    /// `Processing` at that point; the error is advisory.
    fn stop_processing(&mut self) -> Result<(), HostError> {
        Ok(())
    }

    /// Serializes plugin state into an opaque byte stream
    /// (plugin-defined encoding).
    ///
    /// **RT contract:** non-RT only — may allocate.
    ///
    /// # Errors
    ///
    /// [`HostError::Unsupported`] by default — a plugin without state
    /// support declines the command.
    fn save_state(&mut self) -> Result<Vec<u8>, HostError> {
        Err(HostError::Unsupported)
    }

    /// Restores a byte stream produced by [`Plugin::save_state`].
    ///
    /// **RT contract:** non-RT only — may allocate.
    ///
    /// # Errors
    ///
    /// [`HostError::Unsupported`] by default; [`HostError::PluginFailed`]
    /// when the bytes are rejected.
    fn restore_state(&mut self, _state: &[u8]) -> Result<(), HostError> {
        Err(HostError::Unsupported)
    }

    /// Opens the plugin GUI under an opaque parent window token.
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// [`HostError::Unsupported`] by default — GUI-less plugins
    /// decline the command.
    fn open_gui(&mut self, _parent: GuiParentToken) -> Result<GuiHandle, HostError> {
        Err(HostError::Unsupported)
    }

    /// Closes the GUI opened by [`Plugin::open_gui`].
    ///
    /// **RT contract:** non-RT only.
    ///
    /// # Errors
    ///
    /// [`HostError::Unsupported`] by default.
    fn close_gui(&mut self) -> Result<(), HostError> {
        Err(HostError::Unsupported)
    }
}

/// One registered plugin type: descriptor plus instance construction.
///
/// **RT contract:** non-RT only — registration and instantiation are
/// host-side setup operations.
pub trait PluginFactory {
    /// The descriptor a scan (e.g. the host's `Scan` command or the
    /// [`crate::wire::Command::Scan`] wire verb) reports and
    /// instantiate requests match `plugin_id` against.
    ///
    /// **RT contract:** non-RT only.
    fn descriptor(&self) -> PluginDescriptor;

    /// Creates one fresh instance in `Created` state.
    ///
    /// **RT contract:** non-RT only — may allocate and block.
    fn create(&self) -> Box<dyn ErasedPlugin>;

    /// Opt-in `Send` capability: creates one fresh instance boxed as
    /// a `Send` erased plugin — the instance a host may hand to a
    /// cross-thread RT endpoint share (e.g. a graph node).
    ///
    /// The default returns `None`: factories that cannot produce a
    /// `Send` instance (e.g. the wrapped plugin object holds
    /// thread-affine state) leave the capability unimplemented, and
    /// their instances keep driving host-side surfaces only —
    /// endpoint-style consumers reject them with the defined
    /// [`HostError::EndpointUnavailable`].
    ///
    /// A factory returning `Some` MUST construct an instance of the
    /// same plugin [`PluginFactory::create`] would — descriptor- and
    /// behavior-equivalent; hosts may call it in place of `create()`
    /// when the instance may attach an RT endpoint.
    ///
    /// **RT contract:** non-RT only — may allocate and block.
    fn create_sendable(&self) -> Option<Box<dyn ErasedPlugin + Send>> {
        None
    }
}

/// The dyn-safe mirror of [`Plugin`]: the storage type
/// `Box<dyn ErasedPlugin>` the host holds per instance.
///
/// Adapters do not implement this trait directly — [`erase()`] wraps
/// any `P: Plugin` into it. Kept public because [`PluginFactory`]
/// signatures name it.
///
/// **RT contract:** [`ErasedPlugin::process_host`] is the RT entry
/// point; every other method is **non-RT only**.
pub trait ErasedPlugin {
    /// [`Plugin::descriptor`].
    fn descriptor(&self) -> PluginDescriptor;
    /// [`Plugin::params`].
    fn params(&self) -> &[ParamInfo];
    /// [`Plugin::buses`].
    fn buses(&self) -> &[BusDescriptor];
    /// [`PluginProcessor::prepare`] — the activation hook.
    fn prepare(
        &mut self,
        config: &ProcessConfig,
        context: &mut ProcessContext,
    ) -> Result<(), PrepareError>;
    /// [`Plugin::process_host`].
    fn process_host(
        &mut self,
        inputs: &[HostInput<'_>],
        outputs: &mut [HostOutput<'_>],
        events: EventList<'_>,
        transport: Option<TransportInfo>,
        context: &mut ProcessContext,
    ) -> ProcessResult;
    /// [`Plugin::init`].
    fn init(&mut self) -> Result<(), HostError>;
    /// [`Plugin::start_processing`].
    fn start_processing(&mut self) -> Result<(), HostError>;
    /// [`Plugin::stop_processing`].
    fn stop_processing(&mut self) -> Result<(), HostError>;
    /// [`Plugin::save_state`].
    fn save_state(&mut self) -> Result<Vec<u8>, HostError>;
    /// [`Plugin::restore_state`].
    fn restore_state(&mut self, state: &[u8]) -> Result<(), HostError>;
    /// [`Plugin::open_gui`].
    fn open_gui(&mut self, parent: GuiParentToken) -> Result<GuiHandle, HostError>;
    /// [`Plugin::close_gui`].
    fn close_gui(&mut self) -> Result<(), HostError>;
    /// [`PluginProcessor::drain_output_reports`].
    ///
    /// **RT contract:** non-RT only.
    fn drain_output_reports(&mut self, out: &mut Vec<Event>) {
        let _ = out;
    }
}

/// Type-erasing wrapper produced by [`erase()`].
struct Erased<P>(P);

impl<P: Plugin> ErasedPlugin for Erased<P> {
    fn prepare(
        &mut self,
        config: &ProcessConfig,
        context: &mut ProcessContext,
    ) -> Result<(), PrepareError> {
        self.0.prepare(config, context)
    }
    fn descriptor(&self) -> PluginDescriptor {
        self.0.descriptor()
    }
    fn params(&self) -> &[ParamInfo] {
        self.0.params()
    }
    fn buses(&self) -> &[BusDescriptor] {
        self.0.buses()
    }
    fn process_host(
        &mut self,
        inputs: &[HostInput<'_>],
        outputs: &mut [HostOutput<'_>],
        events: EventList<'_>,
        transport: Option<TransportInfo>,
        context: &mut ProcessContext,
    ) -> ProcessResult {
        self.0
            .process_host(inputs, outputs, events, transport, context)
    }
    fn init(&mut self) -> Result<(), HostError> {
        self.0.init()
    }
    fn start_processing(&mut self) -> Result<(), HostError> {
        self.0.start_processing()
    }
    fn stop_processing(&mut self) -> Result<(), HostError> {
        self.0.stop_processing()
    }
    fn save_state(&mut self) -> Result<Vec<u8>, HostError> {
        self.0.save_state()
    }
    fn restore_state(&mut self, state: &[u8]) -> Result<(), HostError> {
        self.0.restore_state(state)
    }
    fn open_gui(&mut self, parent: GuiParentToken) -> Result<GuiHandle, HostError> {
        self.0.open_gui(parent)
    }
    fn close_gui(&mut self) -> Result<(), HostError> {
        self.0.close_gui()
    }
    fn drain_output_reports(&mut self, out: &mut Vec<Event>) {
        self.0.drain_output_reports(out);
    }
}

/// Erases a concrete [`Plugin`] into the boxed [`ErasedPlugin`] the
/// host stores — the one-line adapter-side companion of
/// [`PluginFactory::create`].
///
/// **RT contract:** non-RT only — allocates the box.
pub fn erase<P: Plugin + 'static>(plugin: P) -> Box<dyn ErasedPlugin> {
    Box::new(Erased(plugin))
}

/// Erases a concrete `Send` [`Plugin`] into a `Send`-capable boxed
/// [`ErasedPlugin`] — the [`erase()`] companion for factories that
/// implement [`PluginFactory::create_sendable`].
///
/// `Erased<P>` is `Send` whenever `P: Send`, so the capability stays
/// opt-in: no blanket `Send` bound lands on [`Plugin`] or
/// [`ErasedPlugin`].
///
/// **RT contract:** non-RT only — allocates the box.
pub fn erase_sendable<P: Plugin + Send + 'static>(plugin: P) -> Box<dyn ErasedPlugin + Send> {
    Box::new(Erased(plugin))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProcessStatus;
    use ruviform_audio_core::SampleRate;

    /// Compile-time `Send` witness for the erased box.
    fn expect_send<T: Send>(_: &T) {}

    const DESC: PluginDescriptor = PluginDescriptor {
        id: "org.example.send-gain",
        name: "SendGain",
        vendor: "Ruviform",
        version: "1.0.0",
        features: &["audio-effect"],
    };

    /// Minimal `Send` plugin — the endpoint-capability fixture.
    struct SendGain;

    impl PluginProcessor for SendGain {
        fn prepare(
            &mut self,
            _config: &ProcessConfig,
            _context: &mut ProcessContext,
        ) -> Result<(), PrepareError> {
            Ok(())
        }

        fn process<I, O>(
            &mut self,
            _inputs: &[I],
            _outputs: &mut [O],
            _events: EventList<'_>,
            _transport: Option<TransportInfo>,
            _context: &mut ProcessContext,
        ) -> ProcessResult
        where
            I: BufferView,
            O: BufferViewMut,
        {
            ProcessResult::CONTINUE
        }
    }

    impl Plugin for SendGain {
        fn descriptor(&self) -> PluginDescriptor {
            DESC
        }
        fn params(&self) -> &[ParamInfo] {
            &[]
        }
    }

    /// Factory without the opt-in: `create_sendable` keeps its
    /// `None` default.
    struct PlainFactory;

    impl PluginFactory for PlainFactory {
        fn descriptor(&self) -> PluginDescriptor {
            DESC
        }
        fn create(&self) -> Box<dyn ErasedPlugin> {
            erase(SendGain)
        }
    }

    /// Factory that opts into the `Send` capability.
    struct SendFactory;

    impl PluginFactory for SendFactory {
        fn descriptor(&self) -> PluginDescriptor {
            DESC
        }
        fn create(&self) -> Box<dyn ErasedPlugin> {
            erase(SendGain)
        }
        fn create_sendable(&self) -> Option<Box<dyn ErasedPlugin + Send>> {
            Some(erase_sendable(SendGain))
        }
    }

    #[test]
    fn create_sendable_defaults_to_none() {
        // The capability is opt-in: a factory that does not implement
        // it reports `None`, never a silently non-`Send` box.
        assert!(PlainFactory.create_sendable().is_none());
    }

    #[test]
    fn create_sendable_returns_send_erased_plugin() {
        let plugin = SendFactory
            .create_sendable()
            .expect("send-capable factory yields an instance");
        expect_send(&plugin);
        assert_eq!(plugin.descriptor().id, DESC.id);
    }

    #[test]
    fn erase_sendable_boxes_a_send_erased_plugin() {
        let plugin: Box<dyn ErasedPlugin + Send> = erase_sendable(SendGain);
        expect_send(&plugin);
        assert_eq!(plugin.descriptor().id, DESC.id);
        // `erase()`'s non-`Send` contract is unchanged — the plain
        // `Box<dyn ErasedPlugin>` carries no `Send` bound.
        let plain: Box<dyn ErasedPlugin> = erase(SendGain);
        assert_eq!(plain.descriptor().id, DESC.id);
    }

    #[test]
    fn send_erased_plugin_drives_the_erased_surface() {
        let config = ProcessConfig::new(SampleRate::new(48_000).unwrap(), 1, 512).unwrap();
        let mut context = ProcessContext::new();
        let mut plugin = erase_sendable(SendGain);
        plugin.prepare(&config, &mut context).unwrap();
        let mut outputs: [HostOutput<'_>; 0] = [];
        let result =
            plugin.process_host(&[], &mut outputs, EventList::empty(512), None, &mut context);
        assert_eq!(result.status, ProcessStatus::Continue);
    }
}
