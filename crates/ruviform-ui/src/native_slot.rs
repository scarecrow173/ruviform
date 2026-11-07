//! Driver-neutral native-surface slot contract (`NativeSurfaceSlot`).
//!
//! A *surface slot* is a driver-owned, reserved embedding region inside a
//! host window's dock layout — the place a plugin's own native editor is
//! attached. This module is the port definition point: it holds the pure,
//! headless-verifiable model ([`SurfaceSlotRegistry`]) plus the trait a
//! GUI driver implements ([`NativeSurfaceSlot`]). No `gpui`, `wry`,
//! `tao`, `HWND`, `NSView`, X11 or GPU handle types appear here — those
//! live inside the driver's OS adapter.
//!
//! The companion to this module is the session side in
//! `ruviform-plugin-host::editor_session`, which consumes an opaque
//! [`SlotLease`] (encoded into a `ParentSlotLease`) rather than a raw
//! [`crate::render::SurfaceId`]. The older [`crate::render::NativeSurfaceHost`]
//! port predates this contract and targets adjacent `WebView` surfaces; it
//! is kept for that path and is not the native-dock embedding vocabulary.
//!
//! **RT contract:** non-RT only — everything here is UI-thread only.

use crate::dock::ViewInstanceId;
use crate::window::WindowId;

/// Opaque, generation-checked identifier of one surface slot.
///
/// The `{index, generation}` pair lets the registry detect a stale id
/// whose slot was released and its index recycled — callers treat the
/// value as fully opaque and only [`SurfaceSlotRegistry`] mints ids.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SurfaceSlotId {
    index: u32,
    generation: u16,
}

impl SurfaceSlotId {
    /// Creates an id for `index`/`generation`.
    ///
    /// `#[doc(hidden)]` public because minting is a registry-internal
    /// operation.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[doc(hidden)]
    #[must_use]
    pub const fn new(index: u32, generation: u16) -> Self {
        Self { index, generation }
    }

    /// The slot index this id refers to.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn index(self) -> u32 {
        self.index
    }

    /// The generation stamped on the slot when this id was minted.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn generation(self) -> u16 {
        self.generation
    }
}

/// Opaque, generation-bound, lifetime-checked parent-surface lease.
///
/// A lease proves "this session may parent onto that slot" without
/// exposing an OS handle. The issuer ([`SurfaceSlotRegistry`]) binds the
/// lease to a registry-global unique sequence, and is the only authority
/// that can validate or revoke it — a released slot, an unknown or
/// superseded sequence, or a released lease all fail
/// [`SurfaceSlotRegistry::validate_lease`]. At most one lease is live
/// per slot at a time (one slot parents at most one attach).
///
/// `encode()` produces the opaque `u64` the session side carries inside
/// `ParentSlotLease`; `decode` is used only by the issuer-side adapter
/// when redeeming a lease back into an OS parent handle.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SlotLease(u64);

impl SlotLease {
    /// Creates a lease from a packed `u64` payload.
    ///
    /// `#[doc(hidden)]` public because minting is a registry-internal
    /// operation — the session surface only ever receives a lease.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[doc(hidden)]
    #[must_use]
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    /// The raw `u64` payload (carried opaquely by `ParentSlotLease`).
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }

    /// Encodes `{index, seq}` into the packed payload.
    ///
    /// Layout: `index << 32 | seq`. `seq` is a registry-global unique
    /// sequence, so a lease is unforgeable without the issuer's ledger
    /// and cannot alias a lease minted for another slot or generation.
    ///
    /// `#[doc(hidden)]` public because minting is a registry-internal
    /// operation — the session surface only ever receives a lease.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[doc(hidden)]
    #[must_use]
    pub const fn encode_parts(index: u32, seq: u32) -> Self {
        Self(((index as u64) << 32) | (seq as u64))
    }

    /// The slot index packed into this lease.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn index(self) -> u32 {
        (self.0 >> 32) as u32
    }

    /// The issuer sequence packed into this lease.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn seq(self) -> u32 {
        (self.0 & 0xffff_ffff) as u32
    }
}

/// A rectangle in logical (scale-independent) coordinates.
///
/// Logical units keep the contract DPI-independent; `scale_factor` on
/// [`SlotGeometry`] maps them to physical pixels at the driver.
///
/// **RT contract:** non-RT only — UI-thread only.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct LogicalRect {
    /// Left edge in logical units.
    pub x: f64,
    /// Top edge in logical units.
    pub y: f64,
    /// Width in logical units.
    pub width: f64,
    /// Height in logical units.
    pub height: f64,
}

/// Placement of a slot: its logical rect plus the scale factor that maps
/// it to physical pixels.
///
/// **RT contract:** non-RT only — UI-thread only.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct SlotGeometry {
    /// The slot's rect in logical coordinates within its window.
    pub rect: LogicalRect,
    /// Logical-to-physical scale factor (DPI) of the slot's monitor.
    pub scale_factor: f64,
}

/// Whether a slot's native surface is shown or hidden (e.g. its dock tab
/// is inactive).
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy.
///
/// **RT contract:** non-RT only — UI-thread only.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum SlotVisibility {
    /// The surface is shown and receives normal clipping/input.
    Visible,
    /// The surface is hidden (tab inactive, collapsed) — not clipped
    /// out by overlap but logically suppressed.
    Hidden,
}

/// Declarative description of one surface slot's logical placement.
///
/// `window`/`view` name the *logical* owner (a dock view inside a
/// window), not OS handles — the driver resolves them to real native
/// parents when it redeems a lease.
///
/// **RT contract:** non-RT only — UI-thread only.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct SlotSpec {
    /// The logical window this slot lives in.
    pub window: WindowId,
    /// The dock view this slot is reserved for.
    pub view: ViewInstanceId,
    /// Initial geometry.
    pub geometry: SlotGeometry,
    /// Initial visibility.
    pub visibility: SlotVisibility,
    /// Capabilities this slot declares support for.
    pub capabilities: SlotCapabilities,
}

/// Whether a capability is advertised as supported or explicitly
/// unsupported.
///
/// `Unsupported` is a defined refusal — an operation requested against
/// it must return an error, never fake a success that could be
/// miscounted as a real Dock PASS. `Default` is `Unsupported` so a
/// capability set starts honest and opts in explicitly.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum Support {
    /// The driver can perform this operation.
    Supported,
    /// Explicitly unsupported — a defined refusal, not a silent no-op.
    #[default]
    Unsupported,
}

impl Support {
    /// `true` when [`Support::Supported`].
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn supported(self) -> bool {
        matches!(self, Self::Supported)
    }
}

/// Capability flags a slot/driver advertises.
///
/// An operation requested against a flag that is
/// [`Support::Unsupported`] must return [`SlotError::Unsupported`] — an
/// explicit refusal, never a fake success. Drivers report only what
/// they truly implement.
///
/// **RT contract:** non-RT only — UI-thread only.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct SlotCapabilities {
    /// Can attach a native child surface (basic embed).
    pub attach_parent: Support,
    /// Can re-parent an already-attached child to another slot.
    pub reparent: Support,
    /// Can move a child across different host windows.
    pub cross_window_move: Support,
    /// Can negotiate a resize with the child (plugin asks, host grants).
    pub resize_negotiate: Support,
    /// Can hide/show the child (tab switching).
    pub hide_show: Support,
    /// Can route keyboard focus to/from the child.
    pub focus: Support,
    /// Can route keyboard input to the child.
    pub keyboard_input: Support,
    /// Supports IME inside the child.
    pub ime: Support,
    /// Supports the child's own popup/menu windows.
    pub popup: Support,
    /// Supports clipping the child to the dock layout rect.
    pub clip: Support,
    /// Supports explicit z-ordering of the child.
    pub z_order: Support,
    /// Can parent a child that lives in another process (L1 embed).
    pub out_of_process_parent: Support,
}

/// The validated lease a [`SurfaceSlotRegistry::validate_lease`] returns.
///
/// **RT contract:** non-RT only — UI-thread only.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct LeaseInfo {
    /// The slot the lease is bound to.
    pub slot: SurfaceSlotId,
    /// The lease sequence number.
    pub seq: u32,
}

/// Lifecycle state of a surface slot in the registry.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy.
///
/// **RT contract:** non-RT only — UI-thread only.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SlotState {
    /// Declared and leasable, no live lease.
    Declared,
    /// A live lease is held (the slot parents an attach).
    Leased,
    /// Released — index may be recycled under a bumped generation.
    Released,
}

/// Errors returned by [`SurfaceSlotRegistry`] and [`NativeSurfaceSlot`].
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy.
///
/// **RT contract:** non-RT only — surfaced on the UI thread.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SlotError {
    /// No slot exists at the lease/id's index.
    UnknownSlot,
    /// The id/lease generation does not match the slot's current
    /// generation (slot was released and recycled).
    StaleGeneration,
    /// The lease payload is malformed or names an unknown sequence.
    InvalidLease,
    /// A live lease already exists on the slot (one slot parents at
    /// most one attach).
    LeaseBusy,
    /// The lease was already released or superseded.
    LeaseRevoked,
    /// The slot is busy (e.g. its dock view is mid two-phase move) —
    /// new leases are refused until `clear_busy`.
    SlotBusy,
    /// The slot has been released.
    Released,
    /// The requested operation/capability is not supported by this
    /// slot or driver — an explicit refusal, not a silent no-op.
    Unsupported,
    /// A caller-supplied value fell outside its defined domain (e.g. a
    /// non-finite or negative geometry).
    InvalidValue,
}

impl std::fmt::Display for SlotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::UnknownSlot => "unknown surface slot",
            Self::StaleGeneration => "stale slot generation",
            Self::InvalidLease => "invalid or forged lease token",
            Self::LeaseBusy => "slot already holds a live lease",
            Self::LeaseRevoked => "lease was released or superseded",
            Self::SlotBusy => "slot is busy (dock move in progress)",
            Self::Released => "slot has been released",
            Self::Unsupported => "operation is unsupported by this slot/driver",
            Self::InvalidValue => "value outside its defined domain",
        };
        f.write_str(message)
    }
}

impl std::error::Error for SlotError {}

/// Events emitted by [`SurfaceSlotRegistry`].
///
/// `Destroyed` is terminal for its slot — no further events for that
/// slot arrive.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy.
///
/// **RT contract:** non-RT only — UI-thread only.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SlotEvent {
    /// A slot was declared (fresh or recycled index under a bumped
    /// generation).
    Declared {
        /// The new slot id.
        slot: SurfaceSlotId,
    },
    /// The slot's single live lease was minted.
    Leased {
        /// The slot that was leased.
        slot: SurfaceSlotId,
    },
    /// A lease on the slot was revoked (released or superseded).
    LeaseRevoked {
        /// The slot whose lease ended.
        slot: SurfaceSlotId,
    },
    /// The slot was marked busy (dock two-phase move in progress) —
    /// new leases are refused until `BusyCleared`.
    BusySet {
        /// The affected slot.
        slot: SurfaceSlotId,
    },
    /// The slot's busy mark was cleared — leases are allowed again.
    BusyCleared {
        /// The affected slot.
        slot: SurfaceSlotId,
    },
    /// The slot's geometry changed.
    GeometryChanged {
        /// The affected slot.
        slot: SurfaceSlotId,
    },
    /// The slot's visibility changed.
    VisibilityChanged {
        /// The affected slot.
        slot: SurfaceSlotId,
    },
    /// The slot was released/destroyed — terminal for this slot.
    Destroyed {
        /// The destroyed slot.
        slot: SurfaceSlotId,
    },
}

#[derive(Copy, Clone, Debug)]
struct SlotRecord {
    id: SurfaceSlotId,
    state: SlotState,
    spec: SlotSpec,
    /// `Some(seq)` while a lease is live; the lease's `seq` must match.
    live_lease_seq: Option<u32>,
    /// Busy flag (dock two-phase move in progress).
    busy: bool,
}

impl SlotRecord {
    fn matches(&self, index: u32, generation: u16) -> bool {
        self.id.index() == index && self.id.generation() == generation
    }
}

/// Pure, headless model owning surface-slot lifetimes and leases.
///
/// This is the single source of truth for which slots exist and which
/// leases are valid — it performs no OS/driver calls itself (a real
/// driver applies the recorded geometry/visibility/clip to native
/// windows). It mirrors [`crate::dock::DockWorkspace`]'s "pure model +
/// drained events" shape so the whole contract is testable headless.
///
/// Invariants enforced:
/// - A [`SurfaceSlotId`] is valid only for its exact `{index,generation}`.
///   A slot whose generation reaches `u16::MAX` is never recycled — its
///   index stays a tombstone so a stale id can never alias a new slot.
/// - A [`SlotLease`] is valid only while it is the slot's live lease.
///   Sequences are minted from one registry-global `u32` counter, so a
///   lease can never collide with an earlier or later lease (the
///   counter would need 4 billion mints to wrap — unreachable on a UI
///   lifetime).
/// - At most one live lease per slot; released/stale leases are rejected.
/// - A released slot's index may be recycled with a bumped generation.
///
/// **RT contract:** non-RT only — UI-thread only.
#[derive(Debug, Default)]
pub struct SurfaceSlotRegistry {
    slots: Vec<SlotRecord>,
    events: Vec<SlotEvent>,
    /// Registry-global lease sequence — bumped every `lease_parent`, so
    /// a minted seq never repeats for the registry's lifetime.
    next_lease_seq: u32,
}

impl SurfaceSlotRegistry {
    /// Creates an empty registry.
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn record(&self, id: SurfaceSlotId) -> Option<&SlotRecord> {
        self.slots
            .iter()
            .find(|r| r.matches(id.index(), id.generation()))
    }

    /// Resolves `id` to a live slot position, mapping a stale id to
    /// `StaleGeneration` and an unknown index to `UnknownSlot`.
    fn index_of(&self, id: SurfaceSlotId) -> Result<usize, SlotError> {
        match self
            .slots
            .iter()
            .position(|r| r.matches(id.index(), id.generation()))
        {
            Some(position) => Ok(position),
            None if self.slots.iter().any(|r| r.id.index() == id.index()) => {
                Err(SlotError::StaleGeneration)
            }
            None => Err(SlotError::UnknownSlot),
        }
    }

    /// Declares a new slot from `spec` and returns its id.
    ///
    /// Reuses a released index when available (bumping its generation);
    /// otherwise appends a new index.
    ///
    /// # Errors
    /// Returns [`SlotError::InvalidValue`] on non-finite or negative
    /// geometry.
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    pub fn declare_slot(&mut self, spec: SlotSpec) -> Result<SurfaceSlotId, SlotError> {
        let r = spec.geometry.rect;
        if !r.x.is_finite()
            || !r.y.is_finite()
            || !r.width.is_finite()
            || !r.height.is_finite()
            || r.width < 0.0
            || r.height < 0.0
            || !spec.geometry.scale_factor.is_finite()
            || spec.geometry.scale_factor <= 0.0
        {
            return Err(SlotError::InvalidValue);
        }
        // Recycle a released index only while its generation can still
        // be bumped — a slot at `u16::MAX` stays a tombstone so a stale
        // `{index,generation}` id can never alias a new slot.
        if let Some(pos) = self
            .slots
            .iter()
            .position(|r| r.state == SlotState::Released && r.id.generation() < u16::MAX)
        {
            let next_generation = self.slots[pos].id.generation() + 1;
            let id = SurfaceSlotId::new(self.slots[pos].id.index(), next_generation);
            self.slots[pos] = SlotRecord {
                id,
                state: SlotState::Declared,
                spec,
                live_lease_seq: None,
                busy: false,
            };
            self.events.push(SlotEvent::Declared { slot: id });
            return Ok(id);
        }
        let id = SurfaceSlotId::new(
            u32::try_from(self.slots.len()).map_err(|_| SlotError::InvalidValue)?,
            0,
        );
        self.slots.push(SlotRecord {
            id,
            state: SlotState::Declared,
            spec,
            live_lease_seq: None,
            busy: false,
        });
        self.events.push(SlotEvent::Declared { slot: id });
        Ok(id)
    }

    /// Releases a slot: revokes any live lease, emits `LeaseRevoked` then
    /// terminal `Destroyed`, and marks the index recyclable.
    ///
    /// # Errors
    /// Returns [`SlotError::UnknownSlot`]/[`SlotError::StaleGeneration`]
    /// on a bad id; [`SlotError::Released`] if already released.
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    pub fn release_slot(&mut self, slot: SurfaceSlotId) -> Result<(), SlotError> {
        let pos = self.index_of(slot)?;
        if self.slots[pos].state == SlotState::Released {
            return Err(SlotError::Released);
        }
        if self.slots[pos].live_lease_seq.is_some() {
            self.slots[pos].live_lease_seq = None;
            self.events.push(SlotEvent::LeaseRevoked { slot });
        }
        self.slots[pos].state = SlotState::Released;
        self.slots[pos].busy = false;
        self.events.push(SlotEvent::Destroyed { slot });
        Ok(())
    }

    /// Mints the single live lease for `slot`.
    ///
    /// # Errors
    /// Returns [`SlotError::Released`]/[`SlotError::StaleGeneration`]/
    /// [`SlotError::UnknownSlot`] on a bad slot; [`SlotError::LeaseBusy`]
    /// if a lease is already live; [`SlotError::SlotBusy`] while the slot
    /// is mid two-phase move.
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    pub fn lease_parent(&mut self, slot: SurfaceSlotId) -> Result<SlotLease, SlotError> {
        let pos = self.index_of(slot)?;
        let rec = &mut self.slots[pos];
        match rec.state {
            SlotState::Released => return Err(SlotError::Released),
            SlotState::Leased => return Err(SlotError::LeaseBusy),
            SlotState::Declared => {}
        }
        if rec.busy {
            return Err(SlotError::SlotBusy);
        }
        // The slot itself declared `attach_parent` — minting a lease for
        // an unattachable slot would be a fake success.
        if !rec.spec.capabilities.attach_parent.supported() {
            return Err(SlotError::Unsupported);
        }
        let seq = self.next_lease_seq;
        self.next_lease_seq = self.next_lease_seq.wrapping_add(1);
        let rec = &mut self.slots[pos];
        rec.live_lease_seq = Some(seq);
        rec.state = SlotState::Leased;
        self.events.push(SlotEvent::Leased { slot });
        Ok(SlotLease::encode_parts(slot.index(), seq))
    }

    /// Releases a lease back to `Declared` and emits `LeaseRevoked`.
    ///
    /// # Errors
    /// Propagates [`Self::validate_lease`] errors for a stale/invalid/
    /// revoked lease.
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    pub fn release_lease(&mut self, lease: SlotLease) -> Result<(), SlotError> {
        let info = self.validate_lease(lease)?;
        let slot = info.slot;
        let pos = self.index_of(slot)?;
        self.slots[pos].live_lease_seq = None;
        self.slots[pos].state = SlotState::Declared;
        self.events.push(SlotEvent::LeaseRevoked { slot });
        Ok(())
    }

    /// Validates a lease against the registry's ledger.
    ///
    /// # Errors
    /// [`SlotError::UnknownSlot`] — index names no slot.
    /// [`SlotError::InvalidLease`] — the sequence was never minted by
    ///   this registry (a forged or malformed token).
    /// [`SlotError::LeaseRevoked`] — the sequence was minted but is no
    ///   longer live (released, superseded, or its slot was destroyed).
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    pub fn validate_lease(&self, lease: SlotLease) -> Result<LeaseInfo, SlotError> {
        let index = lease.index();
        let seq = lease.seq();
        match self.slots.iter().find(|r| r.id.index() == index) {
            None => Err(SlotError::UnknownSlot),
            Some(rec) if rec.live_lease_seq == Some(seq) => Ok(LeaseInfo { slot: rec.id, seq }),
            // `seq` below the mint frontier was once issued — it is an
            // expired token (`LeaseRevoked`), never `InvalidLease`.
            Some(_) if seq < self.next_lease_seq => Err(SlotError::LeaseRevoked),
            Some(_) => Err(SlotError::InvalidLease),
        }
    }

    /// Records new geometry for `slot` and emits `GeometryChanged`.
    ///
    /// # Errors
    /// [`SlotError::InvalidValue`] on non-finite/negative geometry;
    /// propagates a bad id.
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    pub fn set_geometry(
        &mut self,
        slot: SurfaceSlotId,
        geometry: SlotGeometry,
    ) -> Result<(), SlotError> {
        let r = geometry.rect;
        if !r.x.is_finite()
            || !r.y.is_finite()
            || !r.width.is_finite()
            || !r.height.is_finite()
            || r.width < 0.0
            || r.height < 0.0
            || !geometry.scale_factor.is_finite()
            || geometry.scale_factor <= 0.0
        {
            return Err(SlotError::InvalidValue);
        }
        let pos = self.index_of(slot)?;
        if self.slots[pos].state == SlotState::Released {
            return Err(SlotError::Released);
        }
        self.slots[pos].spec.geometry = geometry;
        self.events.push(SlotEvent::GeometryChanged { slot });
        Ok(())
    }

    /// Records new visibility for `slot` and emits `VisibilityChanged`.
    ///
    /// # Errors
    /// Propagates a bad id / released slot.
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    pub fn set_visibility(
        &mut self,
        slot: SurfaceSlotId,
        visibility: SlotVisibility,
    ) -> Result<(), SlotError> {
        let pos = self.index_of(slot)?;
        if self.slots[pos].state == SlotState::Released {
            return Err(SlotError::Released);
        }
        // Recording a hide/show the slot declared it cannot perform
        // would be a fake success.
        if !self.slots[pos].spec.capabilities.hide_show.supported() {
            return Err(SlotError::Unsupported);
        }
        self.slots[pos].spec.visibility = visibility;
        self.events.push(SlotEvent::VisibilityChanged { slot });
        Ok(())
    }

    /// Marks the slot busy (its dock view is mid two-phase move) — new
    /// `lease_parent` calls are refused with [`SlotError::SlotBusy`]
    /// until [`Self::clear_busy`].
    ///
    /// # Errors
    /// Propagates a bad id / released slot.
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    pub fn mark_busy(&mut self, slot: SurfaceSlotId) -> Result<(), SlotError> {
        let pos = self.index_of(slot)?;
        if self.slots[pos].state == SlotState::Released {
            return Err(SlotError::Released);
        }
        if !self.slots[pos].busy {
            self.slots[pos].busy = true;
            self.events.push(SlotEvent::BusySet { slot });
        }
        Ok(())
    }

    /// Clears the busy mark set by [`Self::mark_busy`].
    ///
    /// # Errors
    /// Propagates a bad id / released slot.
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    pub fn clear_busy(&mut self, slot: SurfaceSlotId) -> Result<(), SlotError> {
        let pos = self.index_of(slot)?;
        if self.slots[pos].state == SlotState::Released {
            return Err(SlotError::Released);
        }
        if self.slots[pos].busy {
            self.slots[pos].busy = false;
            self.events.push(SlotEvent::BusyCleared { slot });
        }
        Ok(())
    }

    /// The current state of `slot`, if the id is live.
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    #[must_use]
    pub fn state(&self, slot: SurfaceSlotId) -> Option<SlotState> {
        self.record(slot).map(|r| r.state)
    }

    /// The declared spec of `slot`, if the id is live.
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    #[must_use]
    pub fn spec(&self, slot: SurfaceSlotId) -> Option<&SlotSpec> {
        self.record(slot).map(|r| &r.spec)
    }

    /// The declared capabilities of `slot`, if the id is live.
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    #[must_use]
    pub fn capabilities(&self, slot: SurfaceSlotId) -> Option<SlotCapabilities> {
        self.record(slot).map(|r| r.spec.capabilities)
    }

    /// Drains pending events.
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    pub fn drain_events(&mut self) -> Vec<SlotEvent> {
        std::mem::take(&mut self.events)
    }
}

/// Port a GUI driver implements to attach native plugin surfaces into
/// reserved dock slots.
///
/// The driver owns real native windows and the [`SurfaceSlotRegistry`]
/// ledger behind this port; consumers speak only this vocabulary plus
/// opaque [`SlotLease`]s. Every operation may return
/// [`SlotError::Unsupported`] for a capability the driver does not
/// implement — an explicit refusal, never a fake success that could be
/// miscounted as a real Dock PASS.
///
/// **RT contract:** non-RT only — UI-thread only.
pub trait NativeSurfaceSlot {
    /// Reports `slot`'s capabilities.
    ///
    /// # Errors
    /// Returns [`SlotError`] on a bad id.
    fn capabilities(&self, slot: SurfaceSlotId) -> Result<SlotCapabilities, SlotError>;

    /// Declares a slot from `spec` and returns its id.
    ///
    /// # Errors
    /// Returns [`SlotError`] on invalid spec or backend failure.
    fn declare_slot(&mut self, spec: SlotSpec) -> Result<SurfaceSlotId, SlotError>;

    /// Releases `slot` and any live lease.
    ///
    /// # Errors
    /// Returns [`SlotError`] on a bad id.
    fn release_slot(&mut self, slot: SurfaceSlotId) -> Result<(), SlotError>;

    /// Applies new geometry (layout/resize/DPI move).
    ///
    /// # Errors
    /// Returns [`SlotError`] on a bad id or rejected geometry.
    fn set_geometry(
        &mut self,
        slot: SurfaceSlotId,
        geometry: SlotGeometry,
    ) -> Result<(), SlotError>;

    /// Applies new visibility (tab switching, collapse).
    ///
    /// # Errors
    /// Returns [`SlotError`] on a bad id.
    fn set_visibility(
        &mut self,
        slot: SurfaceSlotId,
        visibility: SlotVisibility,
    ) -> Result<(), SlotError>;

    /// Applies an explicit clip rect (overlap/dock clipping), `None` to
    /// clear.
    ///
    /// # Errors
    /// Returns [`SlotError`] on a bad id or `Unsupported` if the driver
    /// cannot clip.
    fn set_clip(&mut self, slot: SurfaceSlotId, clip: Option<LogicalRect>)
    -> Result<(), SlotError>;

    /// Applies explicit z-ordering.
    ///
    /// # Errors
    /// Returns [`SlotError`] on a bad id or `Unsupported`.
    fn set_zorder(&mut self, slot: SurfaceSlotId, z: i32) -> Result<(), SlotError>;

    /// Requests keyboard focus onto the slot's native surface.
    ///
    /// # Errors
    /// Returns [`SlotError`] on a bad id or `Unsupported`/`focus`
    /// refusal.
    fn request_focus(&mut self, slot: SurfaceSlotId) -> Result<(), SlotError>;

    /// Mints the single live parent lease for `slot`.
    ///
    /// # Errors
    /// Returns [`SlotError`] on a bad id, busy slot, or lease-busy.
    fn lease_parent(&mut self, slot: SurfaceSlotId) -> Result<SlotLease, SlotError>;

    /// Ends a live lease (detach without destroying the slot).
    ///
    /// # Errors
    /// Returns [`SlotError`] on a stale, revoked, or forged lease.
    fn release_lease(&mut self, lease: SlotLease) -> Result<(), SlotError>;

    /// Checks a lease against the driver's ledger — the redemption
    /// boundary calls this before turning a [`SlotLease`] into an OS
    /// parent handle.
    ///
    /// # Errors
    /// Returns [`SlotError`] on a stale, revoked, or forged lease.
    fn validate_lease(&self, lease: SlotLease) -> Result<LeaseInfo, SlotError>;

    /// Drains pending events.
    fn drain_events(&mut self) -> Vec<SlotEvent>;
}
