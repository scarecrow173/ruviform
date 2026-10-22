//! The host boundary vocabulary: [`PluginHandle`], [`HostCommand`],
//! [`HostReply`] and [`HostError`].
//!
//! Everything here is plain data — no trait objects, no pointers — so
//! an out-of-process (L1) backend can serialize the same types onto a
//! wire later. Per the plugin-isolation design
//! (`docs/designs/plugin-isolation.md`) `process` is deliberately **not**
//! a command: L0 calls [`PluginProcessor`] directly and L1 uses a
//! shared-memory path; the command surface carries non-RT operations
//! only.

use core::fmt;

use ruviform_plugin_core::{LifecycleError, PluginDescriptor, PrepareError, ProcessConfig};

/// Opaque reference to one hosted plugin instance.
///
/// The `{slot, generation}` pair lets the host detect a stale handle
/// whose slot has been destroyed and reused — callers treat the value
/// as fully opaque.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PluginHandle {
    slot: u32,
    generation: u16,
}

impl PluginHandle {
    /// Creates a handle for `slot`/`generation` (host-internal).
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub(crate) const fn new(slot: u32, generation: u16) -> Self {
        Self { slot, generation }
    }

    /// The slot index this handle refers to.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn slot(self) -> u32 {
        self.slot
    }

    /// The generation stamped on the slot when this handle was minted.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn generation(self) -> u16 {
        self.generation
    }
}

/// Opaque host-side identifier for one open plugin GUI surface.
///
/// The platform/native window token itself stays behind the standard
/// adapters; the host only tracks that a surface exists.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct GuiHandle(u32);

impl GuiHandle {
    /// Creates a GUI handle from a host-issued token.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    /// The raw token.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn raw(self) -> u32 {
        self.0
    }
}

/// Opaque parent-window token for GUI embedding (e.g. a HWND or
/// `NSView` pointer narrowed to an integer by the caller).
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct GuiParentToken(u64);

impl GuiParentToken {
    /// Wraps a platform parent identifier as an opaque token.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    /// The raw platform identifier.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }
}

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

/// Why a [`HostCommand`] or RT accessor failed.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy — new variants may be added without breaking downstream
/// `match` arms.
///
/// **RT contract:** RT-suitable — `Copy`, allocation-free.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum HostError {
    /// `Instantiate` referenced an unregistered `plugin_id`.
    UnknownPluginId,
    /// The handle does not name a live instance (unknown slot or a
    /// stale generation).
    InvalidHandle,
    /// The command maps to a lifecycle transition that is illegal
    /// from the instance's current state.
    Lifecycle(LifecycleError),
    /// `Activate` ran `prepare` and the plugin rejected the config.
    Prepare(PrepareError),
    /// A plugin hook (`init`/processing hooks/state/gui) declined the
    /// operation.
    PluginFailed,
    /// The instance does not implement the requested operation
    /// (default state/GUI hooks land here).
    Unsupported,
}

impl fmt::Display for HostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownPluginId => f.write_str("unknown plugin id"),
            Self::InvalidHandle => f.write_str("invalid plugin handle"),
            Self::Lifecycle(error) => write!(f, "lifecycle rejected command: {error}"),
            Self::Prepare(error) => write!(f, "activation rejected: {error}"),
            Self::PluginFailed => f.write_str("plugin hook declined the operation"),
            Self::Unsupported => f.write_str("operation not supported by this plugin"),
        }
    }
}

impl core::error::Error for HostError {}
