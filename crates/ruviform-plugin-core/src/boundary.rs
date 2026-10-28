//! The shared host-boundary vocabulary: the opaque [`PluginHandle`],
//! the GUI tokens [`GuiHandle`]/[`GuiParentToken`] and the
//! command-surface error [`HostError`].
//!
//! Everything here is plain data — no trait objects, no pointers — so
//! both the host's in-process dispatch and the out-of-process (L1)
//! wire codec ([`crate::wire`]) can name and serialize the same types.
//! The instance command surface itself (`HostCommand`/`HostReply`)
//! stays in `ruviform-plugin-host`; this module holds only the shared
//! vocabulary that the command surface, the wire protocol and the
//! plugin-side contracts all reference.

use core::fmt;

use crate::{LifecycleError, PrepareError};

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
    /// Creates a handle for `slot`/`generation`.
    ///
    /// `#[doc(hidden)]` public because minting is a host-internal
    /// operation: the slot table lives in `ruviform-plugin-host`
    /// while the type is shared boundary vocabulary.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[doc(hidden)]
    #[must_use]
    pub const fn new(slot: u32, generation: u16) -> Self {
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

/// Why a host command or RT accessor failed.
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
    /// The instance sits in the terminal
    /// [`LifecycleState::Failed`](crate::LifecycleState::Failed) state
    /// — every command except `Destroy` is rejected. Distinct from
    /// [`HostError::PluginFailed`]: no plugin hook declined — the
    /// execution context itself (e.g. an L1 child process) is gone.
    Failed,
    /// Inter-process (L1) transport failure: spawn, handshake,
    /// version mismatch, transport I/O or a command-reply timeout.
    Ipc,
    /// A `plugin_id` was registered twice — e.g. the same id claimed
    /// by both an in-process factory and an out-of-process spec.
    DuplicatePluginId,
    /// The L1 child's instance registry is full —
    /// [`crate::wire::WIRE_MAX_INSTANCES`] live instances already
    /// occupy every handle the wire envelope can name.
    InstanceLimit,
    /// The instance cannot currently provide the requested RT
    /// endpoint — e.g. its factory did not opt into
    /// [`PluginFactory::create_sendable`](crate::PluginFactory::create_sendable),
    /// an endpoint is already attached, or the L1 processing leg is
    /// checked out.
    ///
    /// **Local-only** — a host-local topology fact that never carries
    /// a wire encoding: [`crate::wire::encode`] /
    /// [`crate::wire::encode_host_error`] reject it with
    /// `Err(WireError::UnsupportedVariant)` instead of collapsing it
    /// into `Ipc`/`Unsupported` (it is neither a transport failure
    /// nor a plugin-declined operation). Returning it from a
    /// [`Plugin`](crate::Plugin) hook inside an L1 child runner is a
    /// contract violation — the defined outcome there is the
    /// `UnsupportedVariant` rejection above plus the runner
    /// terminating the child, so the host observes `Ipc`/`Failed`.
    EndpointUnavailable,
    /// A caller-supplied value fell outside its defined domain — a
    /// defined rejection distinct from [`HostError::PluginFailed`]
    /// (no plugin hook declined) and from [`HostError::Prepare`]
    /// (not an activation-config question). Used by the video
    /// surface (Issue #118): a `VideoSetParamValue` carrying a
    /// non-finite `Double`, a `parameter_value`/`set_parameter`
    /// against an unknown `ParamId`, or a `Render{input}` whose
    /// dims/format disagree with the instance's
    /// [`crate::VideoProcessConfig`]. Wire tag `10` under
    /// [`crate::wire::WIRE_VERSION`] `3`.
    InvalidValue,
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
            Self::Failed => f.write_str("instance is in the terminal failed state"),
            Self::Ipc => f.write_str("inter-process transport failed"),
            Self::DuplicatePluginId => f.write_str("plugin id already registered"),
            Self::InstanceLimit => f.write_str("child instance registry is full"),
            Self::EndpointUnavailable => {
                f.write_str("RT endpoint is unavailable for this instance")
            }
            Self::InvalidValue => {
                f.write_str("caller-supplied value is outside its defined domain")
            }
        }
    }
}

impl core::error::Error for HostError {}

impl From<LifecycleError> for HostError {
    fn from(error: LifecycleError) -> Self {
        Self::Lifecycle(error)
    }
}
