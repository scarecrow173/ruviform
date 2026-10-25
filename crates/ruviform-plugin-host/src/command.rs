//! The host boundary vocabulary: [`HostCommand`] and [`HostReply`].
//!
//! The shared vocabulary the command surface names —
//! [`PluginHandle`], [`GuiHandle`], [`GuiParentToken`] and
//! [`HostError`] — is owned by `ruviform-plugin-core` (so the L1 wire
//! codec and the plugin runner can share it) and re-exported here
//! unchanged: `ruviform_plugin_host::PluginHandle` and friends keep
//! resolving on their existing paths.
//!
//! Everything here is plain data — no trait objects, no pointers — so
//! an out-of-process (L1) backend can serialize the same types onto a
//! wire later. Per the plugin-isolation design
//! (`docs/designs/plugin-isolation.md`) `process` is deliberately **not**
//! a command: L0 calls [`PluginProcessor`] directly and L1 uses a
//! shared-memory path; the command surface carries non-RT operations
//! only.
//!
//! [`PluginProcessor`]: ruviform_plugin_core::PluginProcessor

use ruviform_plugin_core::{PluginDescriptor, ProcessConfig};

pub use ruviform_plugin_core::{GuiHandle, GuiParentToken, HostError, PluginHandle};

/// A non-RT host operation expressed as serializable plain data.
///
/// Lifecycle verbs map one-to-one onto [`LifecycleEvent`]s the host
/// feeds into the plugin-core state machine; see
/// `docs/designs/plugin-host-l0.md` for the transition wiring.
///
/// **RT contract:** commands are **non-RT only** — the dispatch may
/// allocate and call non-RT plugin hooks.
///
/// [`LifecycleEvent`]: ruviform_plugin_core::LifecycleEvent
#[non_exhaustive]
#[derive(Clone, Debug)]
pub enum HostCommand {
    /// Enumerate the descriptors of every registered factory.
    Scan,
    /// Create an instance of the plugin whose
    /// [`PluginDescriptor::id`] equals `plugin_id`.
    Instantiate {
        /// Standard-neutral plugin identifier.
        plugin_id: String,
    },
    /// `Created → Initialized` — run the plugin's init hook.
    Initialize {
        /// Target instance.
        handle: PluginHandle,
    },
    /// `Initialized → Activated` — run `prepare` under `config`.
    Activate {
        /// Target instance.
        handle: PluginHandle,
        /// Processing configuration for activation.
        config: ProcessConfig,
    },
    /// `Activated → Processing` — begin the RT `process` stream.
    StartProcessing {
        /// Target instance.
        handle: PluginHandle,
    },
    /// `Processing → Activated` — stop the RT `process` stream.
    StopProcessing {
        /// Target instance.
        handle: PluginHandle,
    },
    /// `Activated → Initialized` — release activation resources.
    Deactivate {
        /// Target instance.
        handle: PluginHandle,
    },
    /// Terminal transition; frees the instance slot.
    Destroy {
        /// Target instance.
        handle: PluginHandle,
    },
    /// Serialize the instance's state (plugin-defined byte stream).
    SaveState {
        /// Target instance.
        handle: PluginHandle,
    },
    /// Restore a previously saved state byte stream.
    RestoreState {
        /// Target instance.
        handle: PluginHandle,
        /// Bytes produced by an earlier `SaveState`.
        bytes: Vec<u8>,
    },
    /// Open the plugin GUI under an opaque parent window.
    OpenGui {
        /// Target instance.
        handle: PluginHandle,
        /// Platform parent window token.
        parent: GuiParentToken,
    },
    /// Close the plugin GUI.
    CloseGui {
        /// Target instance.
        handle: PluginHandle,
    },
}

/// The reply payload of a successful [`HostCommand`].
///
/// **RT contract:** replies are **non-RT only**.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum HostReply {
    /// The command completed with no payload.
    Ok,
    /// `Scan` result: descriptors in registration order.
    Scanned(Vec<PluginDescriptor>),
    /// `Instantiate` result: the new instance handle.
    Instantiated {
        /// Handle of the created instance.
        handle: PluginHandle,
    },
    /// `SaveState` result: plugin-defined state bytes.
    State {
        /// Serialized state.
        bytes: Vec<u8>,
    },
    /// `OpenGui` result: token for the opened surface.
    Opened {
        /// Handle of the opened GUI surface.
        gui: GuiHandle,
    },
}
