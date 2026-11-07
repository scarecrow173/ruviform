//! Deterministic fake implementing the driver-neutral
//! [`NativeSurfaceSlot`] port (Issue #388) — no OS window, no GPUI, no
//! plugin. It wraps the pure [`SurfaceSlotRegistry`] ledger and records
//! what a real driver would apply to a native child surface, so the port
//! contract (attach / hide / resize / clip / z-order / focus / error /
//! stale / teardown / busy rollback) is CI-verifiable.
//!
//! The fake follows the GPUI adapter's semantics on the same port:
//!
//! - ids are resolved first: a released id reports `Released`, a
//!   superseded generation reports `StaleGeneration`, an index that was
//!   never declared reports `UnknownSlot`;
//! - clip / z-order / focus are capability-gated and refuse with
//!   `Unsupported` instead of a silent no-op;
//! - clip and z recorded before a lease are re-applied when the child
//!   attaches; a failed attach releases the minted lease so no live
//!   lease outlives its child.
//! - `capabilities` reports the declared spec as-is. The GPUI adapter
//!   additionally intersects it with what the backend implements; a fake
//!   has no backend to model, so that degradation is not reproduced here.
//!
//! **A fake child is a record, not a window.** Fake success is contract
//! conformance only — it is never evidence of a real Dock PASS. Real-OS
//! outcomes stay `NotRun` in the OS/format capability matrix.
//!
//! **RT contract:** non-RT only. Test-only — crate is a dev/test
//! dependency.

use std::collections::BTreeMap;

use ruviform_ui::native_slot::{
    LeaseInfo, LogicalRect, NativeSurfaceSlot, SlotCapabilities, SlotError, SlotEvent,
    SlotGeometry, SlotLease, SlotSpec, SlotState, SlotVisibility, SurfaceSlotId,
    SurfaceSlotRegistry,
};

/// What the fake records for one attached native child — a snapshot of
/// the slot's ledger plus the child-side clip / z / focus state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FakeChild {
    /// The live lease the child was attached under.
    pub lease: SlotLease,
    /// Geometry the child currently occupies.
    pub geometry: SlotGeometry,
    /// Visibility the child currently has.
    pub visibility: SlotVisibility,
    /// Clip applied to the child, in window coordinates.
    pub clip: Option<LogicalRect>,
    /// Recorded z-order value.
    pub z: i32,
    /// Whether keyboard focus is routed to this child.
    pub focused: bool,
}

/// Per-index state the fake keeps across lease changes; reset when the
/// slot is released.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Applied {
    clip: Option<LogicalRect>,
    z: i32,
    focused: bool,
    lease: Option<SlotLease>,
}

/// Deterministic fake [`NativeSurfaceSlot`] over a [`SurfaceSlotRegistry`].
#[derive(Debug, Default)]
pub struct FakeNativeSlot {
    registry: SurfaceSlotRegistry,
    /// Latest id minted per index — separates a stale id (index reused)
    /// from an id that was never declared.
    latest: BTreeMap<u32, SurfaceSlotId>,
    applied: BTreeMap<u32, Applied>,
    attach_fault: Option<SlotError>,
}

impl FakeNativeSlot {
    /// Creates an empty fake.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Arms a fault: the next successful lease mint fails with `error`
    /// and releases that lease — the attach never lands. Refused
    /// `lease_parent` calls (stale, busy, unsupported) do not consume it.
    pub fn arm_attach_fault(&mut self, error: SlotError) {
        self.attach_fault = Some(error);
    }

    /// The attached child for `slot`, or `None` when the id is not live
    /// or the slot holds no lease.
    #[must_use]
    pub fn child(&self, slot: SurfaceSlotId) -> Option<FakeChild> {
        let applied = self.applied.get(&slot.index())?;
        let lease = applied.lease?;
        let spec = self.registry.spec(slot)?;
        Some(FakeChild {
            lease,
            geometry: spec.geometry,
            visibility: spec.visibility,
            clip: applied.clip,
            z: applied.z,
            focused: applied.focused,
        })
    }

    /// Number of slots currently holding an attached child.
    #[must_use]
    pub fn attached_count(&self) -> usize {
        self.applied.values().filter(|a| a.lease.is_some()).count()
    }

    /// The current registry state of `slot`, if the id has a record.
    #[must_use]
    pub fn state(&self, slot: SurfaceSlotId) -> Option<SlotState> {
        self.registry.state(slot)
    }

    /// Marks `slot` busy — the consumer-side mirror of
    /// `DockWorkspace::begin_move`. New leases are refused until
    /// [`Self::clear_busy`]; a live lease is left attached.
    ///
    /// # Errors
    /// Propagates [`SurfaceSlotRegistry::mark_busy`] errors.
    pub fn mark_busy(&mut self, slot: SurfaceSlotId) -> Result<(), SlotError> {
        self.registry.mark_busy(slot)
    }

    /// Clears the busy mark — the mirror of `abort_move`/`commit_move`
    /// completion.
    ///
    /// # Errors
    /// Propagates [`SurfaceSlotRegistry::clear_busy`] errors.
    pub fn clear_busy(&mut self, slot: SurfaceSlotId) -> Result<(), SlotError> {
        self.registry.clear_busy(slot)
    }

    fn missing(&self, slot: SurfaceSlotId) -> SlotError {
        if self.latest.contains_key(&slot.index()) {
            SlotError::StaleGeneration
        } else {
            SlotError::UnknownSlot
        }
    }

    /// Resolves `slot` to its capabilities, classifying bad ids first.
    fn live_capabilities(&self, slot: SurfaceSlotId) -> Result<SlotCapabilities, SlotError> {
        match self.registry.state(slot) {
            Some(SlotState::Released) => Err(SlotError::Released),
            Some(_) => self
                .registry
                .capabilities(slot)
                .ok_or(SlotError::UnknownSlot),
            None => Err(self.missing(slot)),
        }
    }
}

impl NativeSurfaceSlot for FakeNativeSlot {
    fn capabilities(&self, slot: SurfaceSlotId) -> Result<SlotCapabilities, SlotError> {
        self.live_capabilities(slot)
    }

    fn declare_slot(&mut self, spec: SlotSpec) -> Result<SurfaceSlotId, SlotError> {
        let slot = self.registry.declare_slot(spec)?;
        self.latest.insert(slot.index(), slot);
        self.applied.insert(slot.index(), Applied::default());
        Ok(slot)
    }

    fn release_slot(&mut self, slot: SurfaceSlotId) -> Result<(), SlotError> {
        self.registry.release_slot(slot)?;
        self.applied.remove(&slot.index());
        Ok(())
    }

    fn set_geometry(
        &mut self,
        slot: SurfaceSlotId,
        geometry: SlotGeometry,
    ) -> Result<(), SlotError> {
        self.registry.set_geometry(slot, geometry)
    }

    fn set_visibility(
        &mut self,
        slot: SurfaceSlotId,
        visibility: SlotVisibility,
    ) -> Result<(), SlotError> {
        self.registry.set_visibility(slot, visibility)
    }

    fn set_clip(
        &mut self,
        slot: SurfaceSlotId,
        clip: Option<LogicalRect>,
    ) -> Result<(), SlotError> {
        if !self.live_capabilities(slot)?.clip.supported() {
            return Err(SlotError::Unsupported);
        }
        self.applied.entry(slot.index()).or_default().clip = clip;
        Ok(())
    }

    fn set_zorder(&mut self, slot: SurfaceSlotId, z: i32) -> Result<(), SlotError> {
        if !self.live_capabilities(slot)?.z_order.supported() {
            return Err(SlotError::Unsupported);
        }
        self.applied.entry(slot.index()).or_default().z = z;
        Ok(())
    }

    fn request_focus(&mut self, slot: SurfaceSlotId) -> Result<(), SlotError> {
        if !self.live_capabilities(slot)?.focus.supported() {
            return Err(SlotError::Unsupported);
        }
        if self
            .applied
            .get(&slot.index())
            .and_then(|a| a.lease)
            .is_none()
        {
            return Err(SlotError::Backend);
        }
        for applied in self.applied.values_mut() {
            applied.focused = false;
        }
        if let Some(applied) = self.applied.get_mut(&slot.index()) {
            applied.focused = true;
        }
        Ok(())
    }

    fn lease_parent(&mut self, slot: SurfaceSlotId) -> Result<SlotLease, SlotError> {
        self.live_capabilities(slot)?;
        let lease = self.registry.lease_parent(slot)?;
        if let Some(error) = self.attach_fault.take() {
            self.registry.release_lease(lease)?;
            return Err(error);
        }
        self.applied.entry(slot.index()).or_default().lease = Some(lease);
        Ok(lease)
    }

    fn release_lease(&mut self, lease: SlotLease) -> Result<(), SlotError> {
        self.registry.release_lease(lease)?;
        if let Some(applied) = self.applied.get_mut(&lease.index()) {
            applied.lease = None;
            applied.focused = false;
        }
        Ok(())
    }

    fn validate_lease(&self, lease: SlotLease) -> Result<LeaseInfo, SlotError> {
        self.registry.validate_lease(lease)
    }

    fn drain_events(&mut self) -> Vec<SlotEvent> {
        self.registry.drain_events()
    }
}
