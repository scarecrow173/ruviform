//! The shared host-boundary vocabulary: the opaque [`PluginHandle`],
//! the GUI tokens [`GuiHandle`]/[`GuiParentToken`], the session-side
//! [`PluginEditorSessionId`]/[`ParentSlotLease`] vocabulary and the
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

/// Opaque reference to one open plugin editor session.
///
/// The `{slot, generation}` pair mirrors [`PluginHandle`]'s stale
/// detection: the session registry lives in `ruviform-plugin-host`
/// while the type is shared boundary vocabulary, so callers treat the
/// value as fully opaque and only the registry can mint a valid id.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PluginEditorSessionId {
    slot: u32,
    generation: u32,
}

impl PluginEditorSessionId {
    /// Creates an id for `slot`/`generation`.
    ///
    /// `#[doc(hidden)]` public because minting is a host-internal
    /// operation — the session registry owns the slot table.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[doc(hidden)]
    #[must_use]
    pub const fn new(slot: u32, generation: u32) -> Self {
        Self { slot, generation }
    }

    /// The slot index this session id refers to.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn slot(self) -> u32 {
        self.slot
    }

    /// The generation stamped on the slot when this id was minted.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn generation(self) -> u32 {
        self.generation
    }
}

/// Opaque, generation-bound parent-surface lease presented to a
/// plugin editor session.
///
/// This is the *session-side* parent vocabulary: it carries an opaque
/// `u64` payload minted by whoever owns the parent surface (e.g. a
/// `SurfaceSlotRegistry` in a GUI driver), never a raw OS handle. The
/// payload structure is the issuer's private implementation detail —
/// sessions and the wire carry it uninterpreted, and only the
/// issuer-side adapter redeems it into an OS parent token.
///
/// [`GuiParentToken`] remains the lower-level raw vocabulary used at
/// the OS-adapter boundary inside that redemption; it is *not*
/// replaced by this type and stays behind the adapter, never on the
/// session-facing surface.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ParentSlotLease(u64);

impl ParentSlotLease {
    /// Wraps an issuer-minted lease payload as an opaque session token.
    ///
    /// `#[doc(hidden)]` public because minting is an issuer-internal
    /// operation — the session surface only ever receives a lease.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[doc(hidden)]
    #[must_use]
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    /// The raw issuer payload.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }
}

/// Why an L1 child process or a spawn/create attempt failed — the
/// causal vocabulary carried by [`HostError::SpawnFailed`] and by the
/// host-side `HostNotification::InstanceFailed.cause` payload.
///
/// Classification is by **error meaning, not implementation
/// location** — the first observed failure wins: the cause is fixed
/// under the same lock that converges the failure and later
/// observations never overwrite it.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy — new variants may be added without breaking downstream
/// `match` arms.
///
/// **RT contract:** RT-suitable — `Copy`, allocation-free.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum FailureCause {
    /// The OS process spawn failed — `pre-mint` only; no child ever
    /// existed.
    Spawn,
    /// A defined handshake rejection: `Hello` version mismatch, a
    /// decodable non-`Hello` first frame, `Scan`/descriptor mismatch,
    /// or a `CreateInstance`/`CreateVideoInstance` error reply.
    Handshake,
    /// A host deadline decision: `Hello` wait, `CreateInstance` reply
    /// wait, command waiter timeout, or the watchdog kill — recorded
    /// *before* the kill is issued, so a subsequent EOF does not
    /// overwrite it with `Transport`.
    Timeout,
    /// A stream violation: an undecodable frame (any phase), a stream
    /// violation after `Hello` acceptance, a second `Hello`, an
    /// unknown or parent-to-child kind, a scope/instance violation, a
    /// waiter-less `Reply`, or an instance-attribution violation.
    Protocol,
    /// A pipe-level transport observation — child stdout EOF/read
    /// error or stdin write failure, regardless of phase (a
    /// handshake-time EOF is `Transport`, not `Handshake`).
    Transport,
    /// Child termination observed via `try_wait`/reap — the exit
    /// status itself travels separately (`ChildExit`).
    ExitObserved,
    /// Internal machinery failure — channel disconnect, unreachable
    /// state, or a `try_wait`/`wait` syscall error. A reap error
    /// observed first still lands here: the cause is always defined.
    Internal,
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
    /// Inter-process (L1) transport failure: transport I/O or a
    /// command-reply timeout on a live child — the post-mint
    /// surface. A *pre-mint* spawn/create-instance attempt failure
    /// carries its classified cause and diagnostics id in
    /// [`HostError::SpawnFailed`] instead; `Ipc` also remains the
    /// host's defined rejection for spec-validation failures (e.g.
    /// an illegal `lookahead_blocks`), which register no record.
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
    /// An out-of-process (L1) spawn or create-instance attempt failed
    /// before the handle was minted — carries the classified
    /// [`FailureCause`] and a `diagnostics_id` naming the retained
    /// diagnostics record in the host's registry (Issue #121).
    ///
    /// Defined rejections are *not* wrapped: `InstanceLimit` and
    /// spec-validation `Ipc` keep their own variants and register no
    /// record.
    ///
    /// **Local-only** — like [`HostError::EndpointUnavailable`], it
    /// carries no wire encoding: [`crate::wire::encode_host_error`]
    /// rejects it with `Err(WireError::UnsupportedVariant)`.
    SpawnFailed {
        /// Why the attempt failed — the first-observed cause.
        cause: FailureCause,
        /// The retained diagnostics record id — contiguous, never
        /// reused; resolvable to `Found`/`Evicted`/`NotFound` through
        /// the host's diagnostics query.
        diagnostics_id: u64,
    },
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
            Self::SpawnFailed {
                cause,
                diagnostics_id,
            } => write!(
                f,
                "out-of-process spawn or create-instance failed \
                 ({cause:?}, diagnostics {diagnostics_id})"
            ),
        }
    }
}

impl core::error::Error for HostError {}

impl From<LifecycleError> for HostError {
    fn from(error: LifecycleError) -> Self {
        Self::Lifecycle(error)
    }
}
