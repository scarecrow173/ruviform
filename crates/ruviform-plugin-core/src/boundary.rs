//! The shared host-boundary vocabulary: the opaque [`PluginHandle`],
//! the GUI tokens [`GuiHandle`]/[`ParentLease`], the session-side
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

use std::collections::VecDeque;

use core::fmt;

use crate::{LifecycleError, ParamId, ParamValue, PrepareError};

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

/// Opaque, generation-bound parent-window lease presented to
/// [`Plugin::open_gui`](crate::Plugin::open_gui).
///
/// This is the *plugin-side* parent vocabulary: it replaces the raw
/// OS-handle token so that no caller can hand an unvalidated native
/// handle to a plugin. `generation` binds the lease to the issuer's
/// issuance epoch (e.g. a `SurfaceSlotRegistry` sequence) and
/// `payload` carries the redeemed raw OS parent token (`u64`-narrowed
/// HWND/`NSView`/etc.) that only the OS adapter interprets.
///
/// `ParentLease` is opaque vocabulary, not authentication: stale,
/// revoked, or forged *session-side* leases are rejected earlier, at
/// the issuer-side redeem seam, before a `ParentLease` can be minted.
/// Minting is an adapter-internal operation — [`ParentLease::new`] is
/// `#[doc(hidden)]` and session-facing surfaces only ever receive a
/// [`ParentSlotLease`], never this type.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ParentLease {
    generation: u32,
    payload: u64,
}

impl ParentLease {
    /// Creates a lease binding `generation` to the redeemed `payload`.
    ///
    /// `#[doc(hidden)]` public because minting is an adapter-internal
    /// operation — only a redeem seam that has validated the
    /// session-side [`ParentSlotLease`] may produce one.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[doc(hidden)]
    #[must_use]
    pub const fn new(generation: u32, payload: u64) -> Self {
        Self {
            generation,
            payload,
        }
    }

    /// The issuer-side lease generation this parent lease is bound to.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn generation(self) -> u32 {
        self.generation
    }

    /// The redeemed raw OS parent token.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn payload(self) -> u64 {
        self.payload
    }
}

/// A plugin GUI's logical size (scale-independent units).
///
/// The driver-neutral size the editor-session verbs carry — each
/// format adapter interprets it in its own units (VST3 `ViewRect`
/// pixels, CLAP `GuiSize` pixels or logical units per `GuiApiType`).
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct GuiSize {
    /// Width in logical units.
    pub width: f64,
    /// Height in logical units.
    pub height: f64,
}

/// Which direction a [`GuiKeyEvent`] travels — press or release.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy — new variants may be added without breaking downstream
/// `match` arms.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum GuiKeyDirection {
    /// Key press (`onKeyDown`-direction).
    Down,
    /// Key release (`onKeyUp`-direction).
    Up,
}

/// Modifier state held while a [`GuiKeyEvent`] fires.
///
/// A bitmask over the [`GuiModifiers::SHIFT`]/[`GuiModifiers::ALTERNATE`]/
/// [`GuiModifiers::COMMAND`]/[`GuiModifiers::CONTROL`] constants — the
/// same shape every plugin format's key verb carries (VST3's
/// `KeyModifier` `int16`, CLAP's modifier mask). `COMMAND` is the
/// platform's primary shortcut modifier (Ctrl on Windows, Cmd on
/// macOS) and `CONTROL` the secondary control modifier — the same
/// split VST3's `kCommandKey`/`kControlKey` encodes.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct GuiModifiers {
    /// The held-modifier bitmask — `0` for none.
    pub bits: u16,
}

impl GuiModifiers {
    /// No modifiers held.
    pub const NONE: Self = Self { bits: 0 };
    /// Shift.
    pub const SHIFT: Self = Self { bits: 1 << 0 };
    /// Alt/Option.
    pub const ALTERNATE: Self = Self { bits: 1 << 1 };
    /// Primary shortcut modifier (Ctrl on Windows, Cmd on macOS).
    pub const COMMAND: Self = Self { bits: 1 << 2 };
    /// Secondary control modifier.
    pub const CONTROL: Self = Self { bits: 1 << 3 };

    /// `true` when every `flag` bit is held.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn contains(self, flag: Self) -> bool {
        self.bits & flag.bits == flag.bits
    }

    /// The union of two masks.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self {
            bits: self.bits | other.bits,
        }
    }
}

/// One key event delivered to an open plugin GUI — press or release.
///
/// The driver-neutral triple every plugin format's key verb carries:
/// `key` is the character the key produces (`'\0'` when the key has
/// no text representation — arrows, function keys), `key_code` is
/// the virtual key code **in the adapter's key-code space** (opaque
/// to this vocabulary, like [`ParentLease::payload`]: the VST3
/// adapter carries the OS virtual-key code its `onKeyDown`/`onKeyUp`
/// verbs define; a format without a virtual-key concept declines the
/// verb), and `modifiers` is the held-modifier state.
///
/// `key` is limited to the basic multilingual plane: format verbs
/// carry the character as one UTF-16 code unit, so an adapter
/// rejects a supplementary-plane `key` with
/// [`HostError::InvalidValue`] rather than truncating it.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct GuiKeyEvent {
    /// Press or release.
    pub direction: GuiKeyDirection,
    /// The character the key produces, or `'\0'` when it has no
    /// text representation.
    pub key: char,
    /// The virtual key code in the adapter's key-code space.
    pub key_code: i16,
    /// Held modifiers.
    pub modifiers: GuiModifiers,
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
/// issuer-side adapter redeems it into a [`ParentLease`] through the
/// host's redeem seam; the redeemed lease stays behind the adapter
/// boundary and is *not* exposed on the session-facing surface.
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

/// One plugin→host GUI request — the neutral vocabulary every
/// format's spontaneous GUI verb maps onto (VST3
/// `IPlugFrame::resizeView`, CLAP `clap_host_gui` `request_*` verbs
/// and notifications).
///
/// Carried by the plugin-side request channel —
/// [`Plugin::set_gui_request_sink`](crate::Plugin::set_gui_request_sink)
/// for synchronous-verdict formats (VST3) and
/// [`Plugin::drain_gui_requests`](crate::Plugin::drain_gui_requests)
/// for async-ack formats that queue (CLAP `[thread-safe]` verbs).
/// Never a `HostCommand` — commands travel host→plugin; this enum
/// travels the other direction.
///
/// Notification variants (`ResizeHintsChanged`, `Closed`) carry no
/// decision — their [`GuiRequestVerdict::Accepted`] is an
/// acknowledge of receipt, not a grant.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy — new variants may be added without breaking downstream
/// `match` arms.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum GuiRequest {
    /// The plugin asks the container to resize its editor surface —
    /// VST3 `IPlugFrame::resizeView` / CLAP `request_resize`.
    Resize {
        /// The requested logical size.
        size: GuiSize,
    },
    /// The plugin asks the host to show its editor surface — CLAP
    /// `request_show`.
    Show,
    /// The plugin asks the host to hide its editor surface — CLAP
    /// `request_hide`.
    Hide,
    /// Notification: the plugin's resize constraints changed — CLAP
    /// `resize_hints_changed`; the host should re-query the format's
    /// constraint surface.
    ResizeHintsChanged,
    /// Notification: the plugin's editor surface closed or its GUI
    /// connection was lost — CLAP `closed`. When `was_destroyed` the
    /// host must run the format's GUI teardown.
    Closed {
        /// Whether the plugin's window was already destroyed.
        was_destroyed: bool,
    },
}

/// The adjudication verdict for one [`GuiRequest`] — what the
/// request path returns to the plugin.
///
/// `Accepted` means the container-side effect is confirmed — never
/// an acknowledgement of a future effect. For [`GuiRequest::Resize`],
/// `granted` carries the size the container actually applied:
/// `Some` when the arbiter corrected the request, `None` when the
/// requested size applied verbatim; the plugin-side notification
/// (VST3 `onSize`, CLAP `set_size`) then completes the roundtrip.
/// For variants carrying no size, `granted` is `None`. `Refused`
/// declines the request with the defined `reason`.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy — new variants may be added without breaking downstream
/// `match` arms.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum GuiRequestVerdict {
    /// Accepted — the container-side effect is confirmed.
    Accepted {
        /// The applied size for [`GuiRequest::Resize`] — `Some` when
        /// the arbiter corrected the request, `None` when the
        /// requested size applied verbatim or the request carries
        /// no size.
        granted: Option<GuiSize>,
    },
    /// Refused — declined with the defined cause.
    Refused {
        /// Why the request was declined.
        reason: GuiRequestRefusal,
    },
}

/// Why a [`GuiRequest`] was refused — the defined-refusal vocabulary
/// the request path maps onto each format's honest decline (VST3
/// `kResultFalse`, CLAP `false`).
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy — new variants may be added without breaking downstream
/// `match` arms.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum GuiRequestRefusal {
    /// No arbiter is configured or the request surface is not routed
    /// — the honest "nothing is listening" answer.
    Unrouted,
    /// The arbiter/container declined the request — a policy refusal
    /// (size constraints, hidden slot, …).
    Rejected,
    /// The adjudication channel is busy — a re-entrant or concurrent
    /// request met the in-flight adjudication. Retryable per the
    /// format's contract.
    Busy,
    /// The request's target is gone — the editor/session converged
    /// while the request was in flight.
    Gone,
}

/// Bounded capacity of one [`DirtyNoticeQueue`]'s `ParamEdited`
/// backlog — the flag latch can never overflow, so only the edit
/// backlog needs a bound. A full backlog drops the *oldest* edit:
/// the newest report is the freshest parameter state, so it is the
/// one worth keeping (the drop is counted in
/// [`DirtyNoticeQueue::dropped_edits`]).
pub const MAX_DIRTY_EDITS: usize = 64;

/// One plugin→host state-dirty notification — the neutral vocabulary
/// every format's "the plugin's state diverged from what the host
/// last saved" signal maps onto (CLAP `clap_host_state` `mark_dirty`,
/// VST3 `IComponentHandler::performEdit` /
/// `IComponentHandler2::setDirty` /
/// `restartComponent(kParamValuesChanged)`).
///
/// Carried by the pull drain
/// [`Plugin::drain_dirty_notices`](crate::Plugin::drain_dirty_notices)
/// — never a `HostCommand` (commands travel host→plugin; this enum
/// travels the other direction), and never an [`crate::Event`] (the
/// report stream is sample-accurate process-path data; a dirty
/// notification is a lifecycle-adjacent flag with no frame offset).
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy — new variants may be added without breaking downstream
/// `match` arms.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum DirtyNotice {
    /// The plugin's state diverged from the host's last saved state
    /// — the host should re-save it. CLAP `mark_dirty`, VST3
    /// `setDirty(true)`, VST3 `restartComponent(kParamValuesChanged)`
    /// and every `performEdit` all map here (an edit is itself a
    /// state divergence).
    StateDirty,
    /// The plugin reports its state is again in sync with the saved
    /// state — VST3 `setDirty(false)`; CLAP has no clean edge.
    StateClean,
    /// A plugin-originated parameter edit — VST3 `performEdit(id,
    /// value_normalized)`: the plugin changed a parameter outside
    /// host-driven writes (its own UI, internal state application)
    /// and reports the resulting normalized value. Non-finite
    /// reported values never reach the neutral surface — the adapter
    /// degrades them to [`Self::StateDirty`].
    ParamEdited {
        /// The edited parameter.
        param_id: ParamId,
        /// The plugin-reported normalized value.
        value: ParamValue,
    },
}

/// The canonical bounded pending store for [`DirtyNotice`]s — the
/// plugin→host counterpart of an adapter's `pending_reports`
/// `Vec<Event>`.
///
/// Storage is split by payload kind: the dirty flag is a *latch*
/// (`Option<bool>` — last declaration wins) while `ParamEdited`
/// payloads are a FIFO backlog bounded by [`MAX_DIRTY_EDITS`].
/// Coalescing is the defined delivery contract: a dirty→clean→dirty
/// toggle between drains delivers only the final flag state, because
/// "re-save me" is a state, not an occurrence count; `ParamEdited`
/// entries, which carry distinct payloads, are preserved one per
/// call.
///
/// [`Self::drain`] emits the edit backlog first (oldest first), then
/// the latched flag as the summarizing edge (`Some(true)` →
/// [`DirtyNotice::StateDirty`], `Some(false)` →
/// [`DirtyNotice::StateClean`]). `enqueue(DirtyNotice::ParamEdited)`
/// also latches the flag dirty — a bare `performEdit` still surfaces
/// the flag for consumers that only track dirty state.
///
/// **RT contract:** non-RT only — the `out` append may allocate.
#[derive(Debug, Default)]
pub struct DirtyNoticeQueue {
    /// The latched flag — `Some(true)` dirty, `Some(false)` clean;
    /// `None` while no flag-carrying call arrived. Last declaration
    /// wins.
    flag: Option<bool>,
    /// FIFO backlog of `ParamEdited` payloads, oldest first.
    edits: VecDeque<(ParamId, ParamValue)>,
    /// `ParamEdited` payloads dropped at the [`MAX_DIRTY_EDITS`]
    /// bound — cumulative back-pressure counter.
    dropped_edits: usize,
}

impl DirtyNoticeQueue {
    /// An empty queue.
    ///
    /// **RT contract:** non-RT only — allocates no storage yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Enqueues one notice per the defined coalescing contract (see
    /// the type docs): flag variants update the latch;
    /// `ParamEdited` appends to the bounded backlog (dropping the
    /// oldest past [`MAX_DIRTY_EDITS`]) and latches dirty.
    ///
    /// **RT contract:** non-RT only — bounded `VecDeque` push.
    pub fn enqueue(&mut self, notice: DirtyNotice) {
        match notice {
            DirtyNotice::StateDirty => self.flag = Some(true),
            DirtyNotice::StateClean => self.flag = Some(false),
            DirtyNotice::ParamEdited { param_id, value } => {
                if self.edits.len() >= MAX_DIRTY_EDITS {
                    self.edits.pop_front();
                    self.dropped_edits += 1;
                }
                self.edits.push_back((param_id, value));
                self.flag = Some(true);
            }
        }
    }

    /// Moves the pending notices into `out` — the edit backlog first
    /// (oldest first), then the latched flag edge when set. A second
    /// drain without an intervening `enqueue` yields nothing.
    ///
    /// **RT contract:** non-RT only — the `out` append may allocate.
    pub fn drain(&mut self, out: &mut Vec<DirtyNotice>) {
        out.extend(
            self.edits
                .drain(..)
                .map(|(param_id, value)| DirtyNotice::ParamEdited { param_id, value }),
        );
        if let Some(dirty) = self.flag.take() {
            out.push(if dirty {
                DirtyNotice::StateDirty
            } else {
                DirtyNotice::StateClean
            });
        }
    }

    /// `ParamEdited` payloads dropped at the capacity bound so far —
    /// the back-pressure counter consumers surface for observability.
    ///
    /// **RT contract:** RT-suitable — pure read.
    #[must_use]
    pub fn dropped_edits(&self) -> usize {
        self.dropped_edits
    }

    /// Whether nothing is pending — no latched flag and no backlog.
    ///
    /// **RT contract:** RT-suitable — pure read.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.flag.is_none() && self.edits.is_empty()
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The flag is a latch: between drains only the final declared
    /// edge is delivered, never the toggle history.
    #[test]
    fn flag_edges_coalesce_to_last_declaration() {
        let mut queue = DirtyNoticeQueue::new();
        queue.enqueue(DirtyNotice::StateDirty);
        queue.enqueue(DirtyNotice::StateClean);
        queue.enqueue(DirtyNotice::StateDirty);
        let mut out = Vec::new();
        queue.drain(&mut out);
        assert_eq!(out, [DirtyNotice::StateDirty]);
        // A second drain without an intervening enqueue yields
        // nothing — since-last-drain semantics.
        queue.drain(&mut out);
        assert_eq!(out, [DirtyNotice::StateDirty]);
        assert!(queue.is_empty());
    }

    /// The clean edge is a first-class notification — VST3
    /// `setDirty(false)` — delivered like the dirty edge.
    #[test]
    fn clean_edge_delivers_state_clean() {
        let mut queue = DirtyNoticeQueue::new();
        queue.enqueue(DirtyNotice::StateDirty);
        let mut out = Vec::new();
        queue.drain(&mut out);
        assert_eq!(out, [DirtyNotice::StateDirty]);

        queue.enqueue(DirtyNotice::StateClean);
        out.clear();
        queue.drain(&mut out);
        assert_eq!(out, [DirtyNotice::StateClean]);
    }

    /// `ParamEdited` payloads are preserved one per call, in order —
    /// and each also latches the dirty flag for flag-only consumers.
    #[test]
    fn edits_fifo_then_flag_edge_and_edit_implies_dirty() {
        let mut queue = DirtyNoticeQueue::new();
        let v1 = ParamValue::new(0.25).unwrap();
        let v2 = ParamValue::new(0.75).unwrap();
        queue.enqueue(DirtyNotice::ParamEdited {
            param_id: ParamId(1),
            value: v1,
        });
        queue.enqueue(DirtyNotice::ParamEdited {
            param_id: ParamId(2),
            value: v2,
        });
        // A dirty latch exists with no explicit flag call — the bare
        // `performEdit` still surfaces `StateDirty`.
        assert!(!queue.is_empty());
        let mut out = Vec::new();
        queue.drain(&mut out);
        assert_eq!(
            out,
            [
                DirtyNotice::ParamEdited {
                    param_id: ParamId(1),
                    value: v1,
                },
                DirtyNotice::ParamEdited {
                    param_id: ParamId(2),
                    value: v2,
                },
                DirtyNotice::StateDirty,
            ]
        );
    }

    /// An edit followed by a clean edge keeps the edit payload and
    /// reports the latest flag — the drain order is edits-then-flag.
    #[test]
    fn edit_then_clean_reports_backlog_then_clean_edge() {
        let mut queue = DirtyNoticeQueue::new();
        let value = ParamValue::new(0.5).unwrap();
        queue.enqueue(DirtyNotice::ParamEdited {
            param_id: ParamId(3),
            value,
        });
        queue.enqueue(DirtyNotice::StateClean);
        let mut out = Vec::new();
        queue.drain(&mut out);
        assert_eq!(
            out,
            [
                DirtyNotice::ParamEdited {
                    param_id: ParamId(3),
                    value,
                },
                DirtyNotice::StateClean,
            ]
        );
    }

    /// Past `MAX_DIRTY_EDITS` the oldest edit drops — the newest is
    /// the freshest parameter state — and the drop is counted.
    #[test]
    fn edit_overflow_drops_oldest_and_counts() {
        let mut queue = DirtyNoticeQueue::new();
        let value = ParamValue::new(0.5).unwrap();
        for id in 0..(MAX_DIRTY_EDITS + 2) {
            queue.enqueue(DirtyNotice::ParamEdited {
                param_id: ParamId(u32::try_from(id).unwrap()),
                value,
            });
        }
        assert_eq!(queue.dropped_edits(), 2);
        let mut out = Vec::new();
        queue.drain(&mut out);
        assert_eq!(out.len(), MAX_DIRTY_EDITS + 1); // edits + flag
        // Ids 0 and 1 dropped; the backlog starts at id 2.
        assert_eq!(
            out[0],
            DirtyNotice::ParamEdited {
                param_id: ParamId(2),
                value,
            }
        );
        assert_eq!(out[MAX_DIRTY_EDITS], DirtyNotice::StateDirty);
    }
}
