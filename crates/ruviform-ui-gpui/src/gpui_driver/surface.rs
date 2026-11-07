//! Issue #388 — the GPUI [`NativeSurfaceSlot`] port: real native
//! surface slots on real GPUI windows.
//!
//! The adapter owns a [`SurfaceSlotRegistry`] ledger plus the OS-side
//! *surface containers*: one native child window per live lease,
//! parented under the slot's GPUI window and driven by the registry's
//! geometry/visibility/clip/z-order state. A plugin's native editor
//! attaches to the container, so clip, tab-hide, overlap z-order and
//! resize land on it the same way the dock layout drives the slot —
//! this is the embedding mechanism a real Dock PASS is measured
//! against, not a fake or floating surface.
//!
//! Layering: this module speaks only `ruviform-ui` vocabulary. The
//! plugin-side `LeaseRedeemer` glue — `ParentSlotLease` →
//! `GuiParentToken` — lives at the consumer composition root
//! (`plugin`/`ui` crates never depend on each other).
//!
//! **RT contract:** non-RT only — UI-thread only, same as the driver.

use std::cell::{Ref, RefCell, RefMut};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use ruviform_ui::{
    LeaseInfo, LogicalRect, NativeSurfaceSlot, SlotCapabilities, SlotError, SlotEvent,
    SlotGeometry, SlotLease, SlotSpec, SlotState, SlotVisibility, Support, SurfaceSlotId,
    SurfaceSlotRegistry, WindowId,
};

use super::{GpuiDriver, Inner};

/// Whether a native container-surface backend exists on this OS.
///
/// Only Windows has the per-slot child-HWND backend today; other OSes
/// honestly report [`ruviform_ui::Capability::Unsupported`] on the
/// driver's `native_surface_slot` flag and every slot op here refuses
/// with [`SlotError::Unsupported`].
pub(super) const BACKEND_AVAILABLE: bool = cfg!(target_os = "windows");

/// The capability set this backend truly implements — intersected
/// with the consumer-declared `SlotSpec::capabilities` so a spec can
/// never report support no code path provides (Windows: attach,
/// hide/show, focus, clip, single-top z-order).
const BACKEND_CAPS: SlotCapabilities = SlotCapabilities {
    attach_parent: Support::Supported,
    reparent: Support::Unsupported,
    cross_window_move: Support::Unsupported,
    resize_negotiate: Support::Unsupported,
    hide_show: Support::Supported,
    focus: Support::Supported,
    keyboard_input: Support::Unsupported,
    ime: Support::Unsupported,
    popup: Support::Unsupported,
    clip: Support::Supported,
    z_order: Support::Supported,
    out_of_process_parent: Support::Unsupported,
};

/// Element-wise `declared ∧ implemented` — a capability reports
/// [`Support::Supported`] only when the consumer asked for it *and*
/// this backend has a code path for it.
fn intersect_caps(declared: SlotCapabilities) -> SlotCapabilities {
    const fn meet(a: Support, b: Support) -> Support {
        if a.supported() && b.supported() {
            Support::Supported
        } else {
            Support::Unsupported
        }
    }
    SlotCapabilities {
        attach_parent: meet(declared.attach_parent, BACKEND_CAPS.attach_parent),
        reparent: meet(declared.reparent, BACKEND_CAPS.reparent),
        cross_window_move: meet(declared.cross_window_move, BACKEND_CAPS.cross_window_move),
        resize_negotiate: meet(declared.resize_negotiate, BACKEND_CAPS.resize_negotiate),
        hide_show: meet(declared.hide_show, BACKEND_CAPS.hide_show),
        focus: meet(declared.focus, BACKEND_CAPS.focus),
        keyboard_input: meet(declared.keyboard_input, BACKEND_CAPS.keyboard_input),
        ime: meet(declared.ime, BACKEND_CAPS.ime),
        popup: meet(declared.popup, BACKEND_CAPS.popup),
        clip: meet(declared.clip, BACKEND_CAPS.clip),
        z_order: meet(declared.z_order, BACKEND_CAPS.z_order),
        out_of_process_parent: meet(
            declared.out_of_process_parent,
            BACKEND_CAPS.out_of_process_parent,
        ),
    }
}

/// Window-coords clip → container-local physical region (the clip
/// shifted by the geometry origin and scaled).
fn clip_local(clip: Option<LogicalRect>, geometry: SlotGeometry) -> Option<PhysRect> {
    clip.map(|c| {
        to_phys(
            LogicalRect {
                x: c.x - geometry.rect.x,
                y: c.y - geometry.rect.y,
                width: c.width,
                height: c.height,
            },
            geometry.scale_factor,
        )
    })
}

/// Captures `window`'s OS parent handle as an opaque `usize` — the
/// redemption seam's raw material. Called once at window open while
/// `&mut Window` is in scope.
pub(super) fn raw_parent_of(window: &gpui::Window) -> Option<usize> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    // `Window::window_handle` is shadowed by gpui's inherent
    // `AnyWindowHandle` getter — the raw-window-handle trait method
    // needs a fully qualified call.
    let handle = HasWindowHandle::window_handle(window).ok()?;
    match handle.as_raw() {
        RawWindowHandle::Win32(w) => Some(w.hwnd.get().cast_unsigned()),
        RawWindowHandle::AppKit(w) => Some(w.ns_view.as_ptr() as usize),
        RawWindowHandle::Xlib(w) => Some(w.window as usize),
        RawWindowHandle::Xcb(w) => Some(w.window.get() as usize),
        _ => None,
    }
}

/// Physical-pixel rect the OS backend positions a container with.
#[derive(Copy, Clone, Debug)]
pub(super) struct PhysRect {
    /// Left edge, parent client coordinates (physical px).
    pub x: i32,
    /// Top edge, parent client coordinates (physical px).
    pub y: i32,
    /// Width in physical px.
    pub w: i32,
    /// Height in physical px.
    pub h: i32,
}

/// Maps a logical rect to physical px at `scale`, rounding to the
/// nearest device pixel.
#[expect(
    clippy::cast_possible_truncation,
    reason = "logical extents are bounded to window sizes far below i32 range"
)]
fn to_phys(rect: LogicalRect, scale: f64) -> PhysRect {
    PhysRect {
        x: (rect.x * scale).round() as i32,
        y: (rect.y * scale).round() as i32,
        w: (rect.width * scale).round() as i32,
        h: (rect.height * scale).round() as i32,
    }
}

/// Which OS fact behind a [`Binding`] died — drives how far
/// [`GpuiSurfaceAdapter::sweep_dead`] reconciles.
enum DeadKind {
    /// The owner window is gone (driver `destroyed` ledger or dead
    /// captured OS parent) — the slot itself is finished.
    Owner,
    /// Only the lease's OS container is gone while the owner lives —
    /// the lease ends; the slot returns to `Declared` and may
    /// re-lease onto a fresh container.
    Container,
}

/// Per-slot adapter bookkeeping. `container` exists exactly while a
/// lease is live.
struct Binding {
    /// The slot's full generation-checked id — needed to auto-release
    /// the registry record when the owner window dies.
    id: SurfaceSlotId,
    /// The slot's owner window (`SlotSpec::window`).
    window: WindowId,
    /// The lease token this binding minted — the adapter's copy, used
    /// to revoke the lease when the container dies (the consumer may
    /// no longer be able to present it). `Some` exactly while
    /// `container` is `Some`.
    lease: Option<SlotLease>,
    /// OS container for the live lease (Windows: a child HWND).
    container: Option<backend::Container>,
    /// Explicit clip in window coordinates (`None` = unconstrained).
    clip: Option<LogicalRect>,
    /// Explicit z-order (`set_zorder`).
    z: i32,
}

/// The GPUI [`NativeSurfaceSlot`] implementation — slot ledger plus
/// real OS containers.
///
/// Construct it alongside [`GpuiDriver`]; calls must run on the UI
/// thread (inside the driver's event handler or between ticks).
///
/// # Lease redemption
///
/// `lease_parent` mints an opaque [`SlotLease`]; the consumer wraps
/// it in `ParentSlotLease::new(lease.raw())` for the session. At
/// `open_gui` time the consumer calls [`Self::redeem`] with that
/// payload: the adapter validates the lease against its ledger
/// (stale/revoked/forged leases are refused) and yields the raw OS
/// parent token for the container — the value a `GuiParentToken`
/// wraps. The raw token never crosses into shared vocabulary.
///
/// **RT contract:** non-RT only — UI-thread only.
pub struct GpuiSurfaceAdapter {
    inner: Rc<RefCell<Inner>>,
    registry: SurfaceSlotRegistry,
    bindings: BTreeMap<u32, Binding>,
    /// Indices whose registry record is currently a released
    /// tombstone — lets [`GpuiSurfaceAdapter::dead_slot_error`]
    /// mirror the registry's `index_of` distinction (a stale id on an
    /// existing record is `StaleGeneration`, not `UnknownSlot`).
    released: BTreeSet<u32>,
}

impl GpuiSurfaceAdapter {
    /// Creates a surface adapter bound to `driver`'s shared state.
    ///
    /// Each adapter owns its own [`SurfaceSlotRegistry`] ledger —
    /// slots declared on one adapter are unknown to another. Views
    /// that want renderer-projected geometry/clip/tab-hide must use
    /// the driver's canonical adapter via
    /// [`GpuiDriver::surface`](crate::GpuiDriver::surface); `new` is
    /// for standalone use (fixtures, the native-dock proof-of-concept).
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    #[must_use]
    pub fn new(driver: &GpuiDriver) -> Self {
        Self::bound(&driver.inner)
    }

    /// Creates an adapter bound to `inner` — used by
    /// [`GpuiDriver::new`](crate::GpuiDriver::new) to install the
    /// canonical shared adapter before `self` exists.
    pub(super) fn bound(inner: &Rc<RefCell<Inner>>) -> Self {
        Self {
            inner: inner.clone(),
            registry: SurfaceSlotRegistry::new(),
            bindings: BTreeMap::new(),
            released: BTreeSet::new(),
        }
    }

    /// Redeems an opaque lease payload (the value a session's
    /// `ParentSlotLease` carries) into the raw OS parent token — the
    /// container surface's handle (`u64`-narrowed).
    ///
    /// The lease is validated against the ledger first: stale or
    /// revoked payloads never yield a live parent. Validation is
    /// ledger lookup, not authentication — a payload is an enumerable
    /// `index | seq` bit-pattern that can only alias a *currently
    /// live* lease, so this seam must not cross a trust boundary
    /// (same-process use only today). The owner window and the OS
    /// container are additionally required to still be alive.
    ///
    /// # Errors
    /// [`SlotError::InvalidLease`]/[`SlotError::LeaseRevoked`] on a
    /// non-live payload (including a lease whose owner window is
    /// dead — the next [`Self::sweep_dead`] auto-releases it),
    /// [`SlotError::Backend`] when the owner window or container no
    /// longer exists, [`SlotError::Unsupported`] on OSes without a
    /// backend.
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    pub fn redeem(&self, payload: u64) -> Result<u64, SlotError> {
        let info = self.registry.validate_lease(SlotLease::new(payload))?;
        let Some(binding) = self.bindings.get(&info.slot.index()) else {
            return Err(self.dead_slot_error(info.slot));
        };
        if self.binding_death(binding).is_some() {
            // Owner/container already dead — the lease is effectively
            // revoked; `sweep_dead` records the same answer.
            return Err(SlotError::LeaseRevoked);
        }
        if self.parent_of(binding.window).is_none() {
            // Owner window retired: the OS already destroyed the
            // child container — never hand out a dead or recycled
            // handle.
            return Err(SlotError::Backend);
        }
        let container = binding
            .container
            .as_ref()
            .filter(|c| c.is_alive())
            .ok_or(SlotError::Backend)?;
        Ok(container.token())
    }

    /// The registry's drained event queue — forwards [`SlotEvent`]s
    /// for the consumer's evidence/diagnostics channel. Also exposed
    /// through the [`NativeSurfaceSlot`] port.
    ///
    /// Runs [`Self::sweep_dead`] first, so owner-window teardown
    /// surfaces here as `LeaseRevoked`/`Destroyed` at the consumer's
    /// normal polling cadence.
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    pub fn drain_events(&mut self) -> Vec<SlotEvent> {
        self.sweep_dead();
        self.registry.drain_events()
    }

    /// Marks `slot` busy (dock two-phase move) — refuses new leases
    /// until [`Self::clear_busy`].
    ///
    /// # Errors
    /// As [`SurfaceSlotRegistry::mark_busy`].
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    pub fn mark_busy(&mut self, slot: SurfaceSlotId) -> Result<(), SlotError> {
        self.sweep_dead();
        self.registry.mark_busy(slot)
    }

    /// Clears `slot`'s busy mark.
    ///
    /// # Errors
    /// As [`SurfaceSlotRegistry::clear_busy`].
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    pub fn clear_busy(&mut self, slot: SurfaceSlotId) -> Result<(), SlotError> {
        self.sweep_dead();
        self.registry.clear_busy(slot)
    }

    /// The current state of `slot`, if the id is live. A slot whose
    /// owner window already died reports [`SlotState::Released`], and
    /// one whose container died reports [`SlotState::Declared`],
    /// before [`Self::sweep_dead`] has run — the same states the
    /// sweep records, so the answer never jumps backwards.
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    #[must_use]
    pub fn state(&self, slot: SurfaceSlotId) -> Option<SlotState> {
        let state = self.registry.state(slot)?;
        match self
            .bindings
            .get(&slot.index())
            .filter(|b| b.id == slot)
            .and_then(|b| self.binding_death(b))
        {
            Some(DeadKind::Owner) => Some(SlotState::Released),
            Some(DeadKind::Container) if state == SlotState::Leased => Some(SlotState::Declared),
            _ => Some(state),
        }
    }

    /// The declared [`SlotSpec`] of `slot`, if the id is live — the
    /// owner window/view check for binding-time validation.
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    #[must_use]
    pub fn spec(&self, slot: SurfaceSlotId) -> Option<SlotSpec> {
        self.registry.spec(slot).copied()
    }

    /// The explicit clip currently recorded for `slot`, if bound —
    /// `None` when unclipped or unknown. Renderer/test visibility
    /// into port-side clip state (the registry ledger does not carry
    /// clip, so `spec()` alone cannot prove it).
    #[cfg(all(test, feature = "gpui-test"))]
    pub(crate) fn clip_of(&self, slot: SurfaceSlotId) -> Option<LogicalRect> {
        self.bindings.get(&slot.index()).and_then(|b| b.clip)
    }

    /// The container's raw OS token for `slot` while a lease is live —
    /// for the consumer's evidence channel (equivalent to `redeem`
    /// output without needing the payload at hand). `None` once the
    /// owner window or container is dead — never a dead handle.
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    #[must_use]
    pub fn container_token(&self, slot: SurfaceSlotId) -> Option<u64> {
        self.bindings
            .get(&slot.index())
            .filter(|b| b.id == slot)
            .filter(|b| self.binding_death(b).is_none())
            .and_then(|b| b.container.as_ref().map(backend::Container::token))
    }

    /// The typed error an id resolving to no live binding should
    /// surface — [`SlotError::Released`] when the registry still holds
    /// a released record (auto-release tombstone), the same answer
    /// the registry's own ops give; [`SlotError::StaleGeneration`]
    /// when the index now belongs to another generation;
    /// [`SlotError::UnknownSlot`] otherwise.
    fn dead_slot_error(&self, slot: SurfaceSlotId) -> SlotError {
        if self
            .bindings
            .get(&slot.index())
            .is_some_and(|b| b.id != slot)
        {
            return SlotError::StaleGeneration;
        }
        match self.registry.state(slot) {
            Some(SlotState::Released) => SlotError::Released,
            // A released index keeps a tombstone record — a stale id
            // on it is `StaleGeneration`, same as `index_of` answers
            // for the registry's own ops.
            None if self.released.contains(&slot.index()) => SlotError::StaleGeneration,
            _ => SlotError::UnknownSlot,
        }
    }

    /// The OS parent handle of `window`, if the backend captured it.
    fn parent_of(&self, window: WindowId) -> Option<usize> {
        if !BACKEND_AVAILABLE {
            return None;
        }
        self.inner
            .borrow()
            .windows
            .get(&window)
            .and_then(|slot| slot.raw_parent)
    }

    /// Which OS fact behind `binding` is already gone, if any —
    /// distinguishes a dead *owner window* (the slot itself is
    /// finished) from a dead *container* (only the lease's OS
    /// surface is gone; the owner still lives and the slot may
    /// re-lease).
    fn binding_death(&self, binding: &Binding) -> Option<DeadKind> {
        {
            let inner = self.inner.borrow();
            // `retire` is the single terminal path every recorded
            // teardown route reaches (`destroyed` is append-only —
            // adapter death-detection depends on it never dropping
            // entries for the driver's lifetime).
            if inner.destroyed.contains(&binding.window) {
                return Some(DeadKind::Owner);
            }
            // Pre-retire / unrecorded teardown (driver exit never
            // runs `retire`): the captured OS parent being dead is
            // proof the owner is gone — also the only signal for a
            // declared-only binding with no container to probe.
            // `IsWindow == false` proves death; `true` is never
            // trusted as liveness (HWND values recycle), so this
            // only ever detects.
            if inner
                .windows
                .get(&binding.window)
                .and_then(|slot| slot.raw_parent)
                .is_some_and(|parent| !backend::os_window_alive(parent))
            {
                return Some(DeadKind::Owner);
            }
        }
        binding
            .container
            .as_ref()
            .filter(|c| !c.is_alive())
            .map(|_| DeadKind::Container)
    }

    /// Reconciles bindings whose OS facts died: a dead *owner* ends
    /// the slot for good (it can never parent again) — the adapter
    /// records that OS fact as an auto-release (`LeaseRevoked` +
    /// terminal `Destroyed`, index recyclable) and drops the dead
    /// `Container`. A dead *container* on a live owner only ends the
    /// lease (`LeaseRevoked`, back to `Declared` — the binding keeps
    /// its clip/z for the next `lease_parent`). Consumer-owned
    /// declare/lease/release stays untouched for live windows; a
    /// consumer release after auto-release surfaces as
    /// `Released`/`LeaseRevoked` instead of a fake `Ok`.
    ///
    /// Borrow discipline: death detection borrows `inner` per call
    /// and never holds a guard across `release_slot`/`remove`/drop —
    /// `Container::drop` issues OS calls that must not run while a
    /// `RefCell` borrow is alive (a still-live container's
    /// `DestroyWindow` synchronously dispatches messages into the
    /// attached editor's wndproc — host code).
    fn sweep_dead(&mut self) {
        enum Kill {
            Owner(u32),
            Container(u32),
        }
        let dead: Vec<Kill> = self
            .bindings
            .iter()
            .filter_map(|(index, binding)| match self.binding_death(binding) {
                Some(DeadKind::Owner) => Some(Kill::Owner(*index)),
                Some(DeadKind::Container) => Some(Kill::Container(*index)),
                None => None,
            })
            .collect();
        for kill in dead {
            match kill {
                Kill::Owner(index) => {
                    // The binding is only removed when its record is
                    // live (declared/leased bindings always exist),
                    // so `release_slot` cannot fail here — its
                    // LeaseRevoked+Destroyed pair is what records
                    // the death. A known-dead container is `abandon`ed
                    // rather than `DestroyWindow`ed: a recycled handle
                    // value could name an innocent window.
                    if let Some(mut binding) = self.bindings.remove(&index) {
                        if let Some(container) = binding.container.take() {
                            container.abandon();
                        }
                        let _ = self.registry.release_slot(binding.id);
                        self.released.insert(index);
                    }
                }
                Kill::Container(index) => {
                    if let Some(binding) = self.bindings.get_mut(&index) {
                        if let Some(lease) = binding.lease.take() {
                            let _ = self.registry.release_lease(lease);
                        }
                        if let Some(container) = binding.container.take() {
                            container.abandon();
                        }
                    }
                }
            }
        }
    }
}

impl Drop for GpuiSurfaceAdapter {
    /// A container already dead at adapter-drop time is `abandon`ed
    /// rather than `DestroyWindow`ed — the handle value may have been
    /// recycled to an innocent window. `sweep_dead` is not used here:
    /// `inner` may be mid-borrow during teardown and a `RefCell`
    /// panic inside `drop` would abort. Owner death destroys child
    /// HWNDs at the OS level, so the container probe alone detects
    /// every handle `sweep_dead` would have abandoned.
    fn drop(&mut self) {
        for binding in self.bindings.values_mut() {
            if binding
                .container
                .as_ref()
                .is_some_and(|container| !container.is_alive())
                && let Some(container) = binding.container.take()
            {
                container.abandon();
            }
        }
    }
}

impl NativeSurfaceSlot for GpuiSurfaceAdapter {
    fn capabilities(&self, slot: SurfaceSlotId) -> Result<SlotCapabilities, SlotError> {
        // Same id/death resolution as the mutating ops — a released
        // or stale id gets its typed error instead of reported caps.
        let caps = match self.bindings.get(&slot.index()) {
            Some(binding) if binding.id == slot => {
                // Same answer `state()` gives: a dead owner ends the
                // slot — caps of a `Released` record are not
                // reported. Container-only death keeps the slot
                // `Declared`; its declared caps remain true.
                if matches!(self.binding_death(binding), Some(DeadKind::Owner)) {
                    return Err(SlotError::Released);
                }
                self.registry
                    .capabilities(slot)
                    .ok_or(SlotError::UnknownSlot)?
            }
            Some(_) => return Err(SlotError::StaleGeneration),
            None => return Err(self.dead_slot_error(slot)),
        };
        if !BACKEND_AVAILABLE {
            // Honest degradation: without a container backend no
            // surface-side capability is truly supported.
            return Ok(SlotCapabilities::default());
        }
        // Never report a capability the backend has no code path for,
        // even if the consumer's spec declared it.
        Ok(intersect_caps(caps))
    }

    fn declare_slot(&mut self, spec: SlotSpec) -> Result<SurfaceSlotId, SlotError> {
        self.sweep_dead();
        if !BACKEND_AVAILABLE {
            return Err(SlotError::Unsupported);
        }
        // The slot's owner window must be a live driver window — a
        // slot on an unknown or dead window could mint leases with no
        // real parent.
        if !self.inner.borrow().windows.contains_key(&spec.window) {
            return Err(SlotError::Backend);
        }
        let id = self.registry.declare_slot(spec)?;
        // Index reuse clears the tombstone marker.
        self.released.remove(&id.index());
        self.bindings.insert(
            id.index(),
            Binding {
                id,
                window: spec.window,
                lease: None,
                container: None,
                clip: None,
                z: 0,
            },
        );
        Ok(id)
    }

    fn release_slot(&mut self, slot: SurfaceSlotId) -> Result<(), SlotError> {
        self.sweep_dead();
        let index = slot.index();
        // `registry.release_slot` already revokes a live lease and
        // emits `LeaseRevoked`; the container dies with it so no OS
        // surface outlives its lease.
        self.registry.release_slot(slot)?;
        self.released.insert(index);
        if let Some(mut binding) = self.bindings.remove(&index) {
            binding.container = None; // Drop destroys the OS surface.
        }
        Ok(())
    }

    fn set_geometry(
        &mut self,
        slot: SurfaceSlotId,
        geometry: SlotGeometry,
    ) -> Result<(), SlotError> {
        self.sweep_dead();
        self.registry.set_geometry(slot, geometry)?;
        if let Some(binding) = self.bindings.get(&slot.index())
            && let Some(container) = &binding.container
        {
            container.set_rect(to_phys(geometry.rect, geometry.scale_factor))?;
            if binding.clip.is_some() {
                // The clip is window-coordinate state but the OS
                // region is container-local: a moved container needs
                // the stored rect re-projected against the new
                // origin, or the old region keeps clipping at the
                // old position.
                container.set_clip(clip_local(binding.clip, geometry))?;
            }
        }
        Ok(())
    }

    fn set_visibility(
        &mut self,
        slot: SurfaceSlotId,
        visibility: SlotVisibility,
    ) -> Result<(), SlotError> {
        self.sweep_dead();
        self.registry.set_visibility(slot, visibility)?;
        if let Some(binding) = self.bindings.get(&slot.index())
            && let Some(container) = &binding.container
        {
            container.set_visible(matches!(visibility, SlotVisibility::Visible));
        }
        Ok(())
    }

    fn set_clip(
        &mut self,
        slot: SurfaceSlotId,
        clip: Option<LogicalRect>,
    ) -> Result<(), SlotError> {
        self.sweep_dead();
        // Resolve the id against the live binding first — a stale
        // generation or auto-released slot must surface its typed
        // error, never slip through the capability gate.
        let binding = match self.bindings.get_mut(&slot.index()) {
            Some(binding) if binding.id == slot => binding,
            Some(_) => return Err(SlotError::StaleGeneration),
            None => return Err(self.dead_slot_error(slot)),
        };
        let caps = self
            .registry
            .capabilities(slot)
            .ok_or(SlotError::UnknownSlot)?;
        if !caps.clip.supported() {
            return Err(SlotError::Unsupported);
        }
        let geometry = self
            .registry
            .spec(slot)
            .map(|s| s.geometry)
            .ok_or(SlotError::UnknownSlot)?;
        if let Some(container) = &binding.container {
            // Record only what the OS actually took — a failed
            // `SetWindowRgn` must not leave the ledger claiming a
            // clip it never applied.
            container.set_clip(clip_local(clip, geometry))?;
        }
        binding.clip = clip;
        Ok(())
    }

    fn set_zorder(&mut self, slot: SurfaceSlotId, z: i32) -> Result<(), SlotError> {
        self.sweep_dead();
        let binding = match self.bindings.get_mut(&slot.index()) {
            Some(binding) if binding.id == slot => binding,
            Some(_) => return Err(SlotError::StaleGeneration),
            None => return Err(self.dead_slot_error(slot)),
        };
        let caps = self
            .registry
            .capabilities(slot)
            .ok_or(SlotError::UnknownSlot)?;
        if !caps.z_order.supported() {
            return Err(SlotError::Unsupported);
        }
        if let Some(container) = &binding.container {
            // Single-top approximation: multi-slot ordered chains land
            // with the real dock layout.
            container.raise()?;
        }
        binding.z = z;
        Ok(())
    }

    fn request_focus(&mut self, slot: SurfaceSlotId) -> Result<(), SlotError> {
        self.sweep_dead();
        let binding = match self.bindings.get(&slot.index()) {
            Some(binding) if binding.id == slot => binding,
            Some(_) => return Err(SlotError::StaleGeneration),
            None => return Err(self.dead_slot_error(slot)),
        };
        let caps = self
            .registry
            .capabilities(slot)
            .ok_or(SlotError::UnknownSlot)?;
        if !caps.focus.supported() {
            return Err(SlotError::Unsupported);
        }
        let container = binding.container.as_ref().ok_or(SlotError::Backend)?;
        container.focus()
    }

    fn lease_parent(&mut self, slot: SurfaceSlotId) -> Result<SlotLease, SlotError> {
        self.sweep_dead();
        if !BACKEND_AVAILABLE {
            return Err(SlotError::Unsupported);
        }
        let window = match self.bindings.get(&slot.index()) {
            Some(binding) if binding.id == slot => binding.window,
            Some(_) => return Err(SlotError::StaleGeneration),
            None => return Err(self.dead_slot_error(slot)),
        };
        let parent = self.parent_of(window).ok_or(SlotError::Backend)?;
        let (geometry, visibility) = self
            .registry
            .spec(slot)
            .map(|s| (s.geometry, s.visibility))
            .ok_or(SlotError::UnknownSlot)?;
        let rect = to_phys(geometry.rect, geometry.scale_factor);
        let lease = self.registry.lease_parent(slot)?;
        // A failed OS attach — create or state re-apply — never
        // leaves a live lease behind.
        let container = backend::Container::create(parent, rect).inspect_err(|_| {
            let _ = self.registry.release_lease(lease);
        })?;
        let Some(binding) = self.bindings.get_mut(&slot.index()) else {
            // Unreachable — the id check above proves the binding —
            // but a minted lease must never escape without a binding
            // to hold it (the created container drops → DestroyWindow).
            let _ = self.registry.release_lease(lease);
            return Err(SlotError::UnknownSlot);
        };
        // Re-apply the recorded slot state — clip and z-order set
        // before the lease must land on the new container, and a
        // `Hidden` spec must attach hidden (the OS default is
        // visible).
        if let Err(error) = apply_recorded_state(binding, &container, geometry, visibility) {
            let _ = self.registry.release_lease(lease);
            return Err(error);
        }
        binding.lease = Some(lease);
        binding.container = Some(container);
        Ok(lease)
    }

    fn release_lease(&mut self, lease: SlotLease) -> Result<(), SlotError> {
        self.sweep_dead();
        let info = self.registry.validate_lease(lease)?;
        self.registry.release_lease(lease)?;
        if let Some(binding) = self.bindings.get_mut(&info.slot.index()) {
            binding.lease = None;
            binding.container = None; // Drop destroys the OS surface.
        }
        Ok(())
    }

    fn validate_lease(&self, lease: SlotLease) -> Result<LeaseInfo, SlotError> {
        let info = self.registry.validate_lease(lease)?;
        if self
            .bindings
            .get(&info.slot.index())
            .is_some_and(|b| self.binding_death(b).is_some())
        {
            // The lease's owner/container is dead — `sweep_dead`
            // revokes it on the next mutation; report the same
            // answer now instead of validating a dead lease.
            return Err(SlotError::LeaseRevoked);
        }
        Ok(info)
    }

    fn drain_events(&mut self) -> Vec<SlotEvent> {
        GpuiSurfaceAdapter::drain_events(self)
    }
}

/// Cloneable, per-call-borrowed handle to a [`GpuiSurfaceAdapter`].
///
/// [`GpuiDriver::surface`] hands out these handles; every clone
/// points at the *same* canonical slot ledger the mounted view
/// projects onto. Each method borrows the adapter for exactly that
/// call — a consumer can never hold a `RefMut` across frames, so
/// renderer pushes can no longer be starved by a foreign guard (the
/// exposure class behind the stale-`Visible` residual). A
/// `try_borrow` refusal — reachable only through reentrancy inside
/// an adapter call — surfaces as [`SlotError::Backend`] (typed,
/// retryable) rather than a silent success or a panic.
///
/// **RT contract:** non-RT only — UI-thread only.
#[derive(Clone)]
pub struct SharedSurface {
    inner: Rc<RefCell<GpuiSurfaceAdapter>>,
}

impl SharedSurface {
    /// Wraps `adapter` as the shared handle. The driver owns the
    /// canonical construction — consumers receive clones via
    /// [`GpuiDriver::surface`], never mint their own.
    pub(crate) fn wrap(adapter: GpuiSurfaceAdapter) -> Self {
        Self {
            inner: Rc::new(RefCell::new(adapter)),
        }
    }

    fn ops(&self) -> Result<RefMut<'_, GpuiSurfaceAdapter>, SlotError> {
        self.inner.try_borrow_mut().map_err(|_| SlotError::Backend)
    }

    fn read(&self) -> Result<Ref<'_, GpuiSurfaceAdapter>, SlotError> {
        self.inner.try_borrow().map_err(|_| SlotError::Backend)
    }

    /// [`NativeSurfaceSlot::capabilities`]
    pub fn capabilities(&self, slot: SurfaceSlotId) -> Result<SlotCapabilities, SlotError> {
        self.read()?.capabilities(slot)
    }

    /// [`NativeSurfaceSlot::declare_slot`]
    pub fn declare_slot(&self, spec: SlotSpec) -> Result<SurfaceSlotId, SlotError> {
        self.ops()?.declare_slot(spec)
    }

    /// [`NativeSurfaceSlot::release_slot`]
    pub fn release_slot(&self, slot: SurfaceSlotId) -> Result<(), SlotError> {
        self.ops()?.release_slot(slot)
    }

    /// [`NativeSurfaceSlot::set_geometry`]
    pub fn set_geometry(
        &self,
        slot: SurfaceSlotId,
        geometry: SlotGeometry,
    ) -> Result<(), SlotError> {
        self.ops()?.set_geometry(slot, geometry)
    }

    /// [`NativeSurfaceSlot::set_visibility`]
    pub fn set_visibility(
        &self,
        slot: SurfaceSlotId,
        visibility: SlotVisibility,
    ) -> Result<(), SlotError> {
        self.ops()?.set_visibility(slot, visibility)
    }

    /// [`NativeSurfaceSlot::set_clip`]
    pub fn set_clip(
        &self,
        slot: SurfaceSlotId,
        clip: Option<LogicalRect>,
    ) -> Result<(), SlotError> {
        self.ops()?.set_clip(slot, clip)
    }

    /// [`NativeSurfaceSlot::set_zorder`]
    pub fn set_zorder(&self, slot: SurfaceSlotId, z: i32) -> Result<(), SlotError> {
        self.ops()?.set_zorder(slot, z)
    }

    /// [`NativeSurfaceSlot::request_focus`]
    pub fn request_focus(&self, slot: SurfaceSlotId) -> Result<(), SlotError> {
        self.ops()?.request_focus(slot)
    }

    /// [`NativeSurfaceSlot::lease_parent`]
    pub fn lease_parent(&self, slot: SurfaceSlotId) -> Result<SlotLease, SlotError> {
        self.ops()?.lease_parent(slot)
    }

    /// [`NativeSurfaceSlot::release_lease`]
    pub fn release_lease(&self, lease: SlotLease) -> Result<(), SlotError> {
        self.ops()?.release_lease(lease)
    }

    /// [`NativeSurfaceSlot::validate_lease`]
    pub fn validate_lease(&self, lease: SlotLease) -> Result<LeaseInfo, SlotError> {
        self.read()?.validate_lease(lease)
    }

    /// [`NativeSurfaceSlot::drain_events`]
    pub fn drain_events(&self) -> Result<Vec<SlotEvent>, SlotError> {
        Ok(self.ops()?.drain_events())
    }

    /// [`GpuiSurfaceAdapter::redeem`]
    pub fn redeem(&self, payload: u64) -> Result<u64, SlotError> {
        self.read()?.redeem(payload)
    }

    /// [`GpuiSurfaceAdapter::mark_busy`]
    pub fn mark_busy(&self, slot: SurfaceSlotId) -> Result<(), SlotError> {
        self.ops()?.mark_busy(slot)
    }

    /// [`GpuiSurfaceAdapter::clear_busy`]
    pub fn clear_busy(&self, slot: SurfaceSlotId) -> Result<(), SlotError> {
        self.ops()?.clear_busy(slot)
    }

    /// [`GpuiSurfaceAdapter::state`] — `Err` means the ledger could
    /// not be consulted this call, `Ok(None)` means unknown slot;
    /// the two are never conflated.
    pub fn state(&self, slot: SurfaceSlotId) -> Result<Option<SlotState>, SlotError> {
        Ok(self.read()?.state(slot))
    }

    /// [`GpuiSurfaceAdapter::spec`] — same `Err`/`Ok(None)`
    /// distinction as [`Self::state`].
    pub fn spec(&self, slot: SurfaceSlotId) -> Result<Option<SlotSpec>, SlotError> {
        Ok(self.read()?.spec(slot))
    }

    /// [`GpuiSurfaceAdapter::container_token`]
    pub fn container_token(&self, slot: SurfaceSlotId) -> Result<Option<u64>, SlotError> {
        Ok(self.read()?.container_token(slot))
    }

    /// The explicit clip recorded for `slot` — test visibility into
    /// port-side clip state.
    #[cfg(all(test, feature = "gpui-test"))]
    pub(crate) fn clip_of(&self, slot: SurfaceSlotId) -> Result<Option<LogicalRect>, SlotError> {
        Ok(self.read()?.clip_of(slot))
    }
}

impl NativeSurfaceSlot for SharedSurface {
    fn capabilities(&self, slot: SurfaceSlotId) -> Result<SlotCapabilities, SlotError> {
        SharedSurface::capabilities(self, slot)
    }

    fn declare_slot(&mut self, spec: SlotSpec) -> Result<SurfaceSlotId, SlotError> {
        SharedSurface::declare_slot(self, spec)
    }

    fn release_slot(&mut self, slot: SurfaceSlotId) -> Result<(), SlotError> {
        SharedSurface::release_slot(self, slot)
    }

    fn set_geometry(
        &mut self,
        slot: SurfaceSlotId,
        geometry: SlotGeometry,
    ) -> Result<(), SlotError> {
        SharedSurface::set_geometry(self, slot, geometry)
    }

    fn set_visibility(
        &mut self,
        slot: SurfaceSlotId,
        visibility: SlotVisibility,
    ) -> Result<(), SlotError> {
        SharedSurface::set_visibility(self, slot, visibility)
    }

    fn set_clip(
        &mut self,
        slot: SurfaceSlotId,
        clip: Option<LogicalRect>,
    ) -> Result<(), SlotError> {
        SharedSurface::set_clip(self, slot, clip)
    }

    fn set_zorder(&mut self, slot: SurfaceSlotId, z: i32) -> Result<(), SlotError> {
        SharedSurface::set_zorder(self, slot, z)
    }

    fn request_focus(&mut self, slot: SurfaceSlotId) -> Result<(), SlotError> {
        SharedSurface::request_focus(self, slot)
    }

    fn lease_parent(&mut self, slot: SurfaceSlotId) -> Result<SlotLease, SlotError> {
        SharedSurface::lease_parent(self, slot)
    }

    fn release_lease(&mut self, lease: SlotLease) -> Result<(), SlotError> {
        SharedSurface::release_lease(self, lease)
    }

    fn validate_lease(&self, lease: SlotLease) -> Result<LeaseInfo, SlotError> {
        SharedSurface::validate_lease(self, lease)
    }

    fn drain_events(&mut self) -> Vec<SlotEvent> {
        // A refused borrow means the drain never ran — the queue is
        // untouched and events surface on the next successful poll.
        // Returning empty is honest; panicking would unwind through
        // whichever OS wndproc re-entered on the borrowed call.
        self.ops()
            .map_or_else(|_| Vec::new(), |mut g| g.drain_events())
    }
}

/// Re-applies a binding's recorded state (clip, z-order, visibility)
/// onto a fresh container — `lease_parent` runs this after
/// `Container::create` so state declared before the lease lands on
/// the new OS surface.
fn apply_recorded_state(
    binding: &Binding,
    container: &backend::Container,
    geometry: SlotGeometry,
    visibility: SlotVisibility,
) -> Result<(), SlotError> {
    if binding.clip.is_some() {
        container.set_clip(clip_local(binding.clip, geometry))?;
    }
    if binding.z != 0 {
        container.raise()?;
    }
    container.set_visible(matches!(visibility, SlotVisibility::Visible));
    Ok(())
}

// ---- OS container backend --------------------------------------------

/// The OS container surface for this platform. Windows gets the real
/// child-HWND backend; every other OS gets a never-constructed stub
/// whose `create` reports [`SlotError::Unsupported`].
#[cfg(target_os = "windows")]
#[expect(unsafe_code)]
mod backend {
    use std::ptr;

    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::Graphics::Gdi::{CreateRectRgn, DeleteObject, HRGN, SetWindowRgn};
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetFocus, SetFocus};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DestroyWindow, HWND_TOP, IsWindow, MoveWindow, SW_HIDE, SW_SHOW,
        SWP_NOMOVE, SWP_NOSIZE, SetWindowPos, ShowWindow, WS_CHILD, WS_CLIPCHILDREN,
        WS_CLIPSIBLINGS, WS_VISIBLE,
    };

    use ruviform_ui::SlotError;

    use super::PhysRect;

    /// The pre-registered Win32 `STATIC` class — a plain no-paint
    /// child surface. The embedded editor covers the client area.
    fn static_class() -> Vec<u16> {
        "STATIC".encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// `true` while `raw` names a live OS window. Only `false` is
    /// trusted — handle values recycle, so `true` is never taken as
    /// liveness proof, only "not known dead".
    pub(super) fn os_window_alive(raw: usize) -> bool {
        // SAFETY: `IsWindow` accepts any HWND value — a stale handle
        // simply reports false.
        unsafe { IsWindow(raw as HWND) != 0 }
    }

    /// One native container surface (a child `HWND` of the slot's
    /// GPUI window). `Drop` destroys the OS window.
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    pub(super) struct Container {
        hwnd: HWND,
    }

    impl Container {
        /// Creates the container at `rect` under `parent` (an HWND).
        ///
        /// # Errors
        /// [`SlotError::Backend`] when `parent` is not a live window
        /// or `CreateWindowExW` fails.
        pub(super) fn create(parent: usize, rect: PhysRect) -> Result<Self, SlotError> {
            // Never parent under a dead or never-created HWND — the
            // OS would fail the create anyway, and the explicit
            // check keeps a stale handle from silently attaching
            // under nothing.
            if !os_window_alive(parent) {
                return Err(SlotError::Backend);
            }
            // SAFETY: `parent` is a live HWND captured from gpui's own
            // window via `HasWindowHandle`; the class name is a valid
            // NUL-terminated system class. The returned HWND is owned
            // by this container until `DestroyWindow` in `Drop`.
            let hwnd = unsafe {
                CreateWindowExW(
                    0,
                    static_class().as_ptr(),
                    ptr::null(),
                    WS_CHILD | WS_VISIBLE | WS_CLIPCHILDREN | WS_CLIPSIBLINGS,
                    rect.x,
                    rect.y,
                    rect.w,
                    rect.h,
                    parent as *mut core::ffi::c_void,
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null(),
                )
            };
            if hwnd.is_null() {
                return Err(SlotError::Backend);
            }
            Ok(Self { hwnd })
        }

        /// The raw parent token the plugin's editor parents into —
        /// `u64`-narrowed HWND.
        pub(super) fn token(&self) -> u64 {
            self.hwnd as usize as u64
        }

        /// `true` while the OS window still exists.
        pub(super) fn is_alive(&self) -> bool {
            // SAFETY: `IsWindow` accepts any HWND value — a stale
            // handle simply reports false.
            unsafe { IsWindow(self.hwnd) != 0 }
        }

        /// Consumes the container *without* destroying its HWND — for
        /// handles already known dead, whose value the OS may have
        /// recycled to an innocent foreign window (`DestroyWindow`
        /// would kill that window instead).
        pub(super) fn abandon(self) {
            std::mem::forget(self);
        }

        /// Repositions/resizes the container in parent client coords.
        ///
        /// # Errors
        /// [`SlotError::Backend`] when `MoveWindow` fails.
        pub(super) fn set_rect(&self, rect: PhysRect) -> Result<(), SlotError> {
            // SAFETY: `hwnd` is our live child HWND; `MoveWindow` on a
            // child moves it in parent client coordinates.
            if unsafe { MoveWindow(self.hwnd, rect.x, rect.y, rect.w, rect.h, 1) } == 0 {
                return Err(SlotError::Backend);
            }
            Ok(())
        }

        /// Shows or hides the container (and the attached editor).
        /// `ShowWindow`'s return reports the previous visibility, not
        /// success — there is no failure signal to surface.
        pub(super) fn set_visible(&self, visible: bool) {
            // SAFETY: `hwnd` is our live child HWND.
            unsafe {
                ShowWindow(self.hwnd, if visible { SW_SHOW } else { SW_HIDE });
            }
        }

        /// Applies a container-local clip region, or clears it.
        /// `local` is already in the container's client coordinates;
        /// `None` issues `SetWindowRgn(hwnd, NULL)` — the documented
        /// region-clear request. Without the call an old region keeps
        /// clipping forever (Issue #404).
        ///
        /// # Errors
        /// [`SlotError::Backend`] when `SetWindowRgn` fails; a created
        /// region is freed on failure rather than leaked.
        pub(super) fn set_clip(&self, local: Option<PhysRect>) -> Result<(), SlotError> {
            // SAFETY: `CreateRectRgn` yields an owned HRGN; a
            // successful `SetWindowRgn` transfers ownership to the OS
            // (it must not be `DeleteObject`ed). On failure the region
            // is still ours — free it explicitly instead of leaking.
            // `NULL` is the documented clear request and owns nothing.
            unsafe {
                let rgn: HRGN = match local {
                    Some(c) => {
                        let rgn = CreateRectRgn(c.x, c.y, c.x + c.w, c.y + c.h);
                        if rgn.is_null() {
                            // A failed create must not fall through to
                            // `SetWindowRgn(NULL)` — that would clear
                            // the clip and report success.
                            return Err(SlotError::Backend);
                        }
                        rgn
                    }
                    None => ptr::null_mut(),
                };
                if SetWindowRgn(self.hwnd, rgn, 1) == 0 {
                    if !rgn.is_null() {
                        DeleteObject(rgn);
                    }
                    return Err(SlotError::Backend);
                }
            }
            Ok(())
        }

        /// Raises the container to the top of the sibling z-order
        /// (single-top approximation until multi-slot ordering lands).
        ///
        /// # Errors
        /// [`SlotError::Backend`] when `SetWindowPos` fails.
        pub(super) fn raise(&self) -> Result<(), SlotError> {
            // SAFETY: `hwnd` is our live child HWND; SWP_NOMOVE|NOSIZE
            // keep current geometry, HWND_TOP raises in z-order.
            if unsafe { SetWindowPos(self.hwnd, HWND_TOP, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE) }
                == 0
            {
                return Err(SlotError::Backend);
            }
            Ok(())
        }

        /// Moves keyboard focus to the container.
        ///
        /// # Errors
        /// [`SlotError::Backend`] when focus did not land — `SetFocus`'s
        /// return is the *previously* focused HWND (NULL is a
        /// legitimate success), so the post-call `GetFocus` state is
        /// the honest check.
        pub(super) fn focus(&self) -> Result<(), SlotError> {
            // SAFETY: `hwnd` is our live child HWND; `SetFocus`/
            // `GetFocus` operate on the calling thread's message queue.
            unsafe {
                SetFocus(self.hwnd);
                if GetFocus() != self.hwnd {
                    return Err(SlotError::Backend);
                }
            }
            Ok(())
        }
    }

    impl Drop for Container {
        fn drop(&mut self) {
            // SAFETY: `hwnd` is our child HWND; `DestroyWindow` on an
            // already-destroyed handle is a harmless error.
            unsafe {
                DestroyWindow(self.hwnd);
            }
        }
    }
}

/// Stub backend for OSes without a container surface — `create` never
/// succeeds (`lease_parent` returns [`SlotError::Unsupported`] before
/// reaching it anyway), so a live `Container` cannot exist.
#[cfg(not(target_os = "windows"))]
mod backend {
    use ruviform_ui::SlotError;

    use super::PhysRect;

    /// Never reports death — there is no OS window to assess (also
    /// unreachable: no backend means no bindings can exist).
    pub(super) fn os_window_alive(_raw: usize) -> bool {
        true
    }

    /// Unconstructable container — no backend means no container.
    pub(super) struct Container;

    impl Container {
        /// Never reached — no backend, no container.
        pub(super) fn create(_parent: usize, _rect: PhysRect) -> Result<Self, SlotError> {
            Err(SlotError::Unsupported)
        }

        /// Never reached — no backend, no container.
        pub(super) fn token(&self) -> u64 {
            0
        }

        /// Never reached — no backend, no container.
        pub(super) fn is_alive(&self) -> bool {
            false
        }

        /// Never reached — no backend, no container.
        pub(super) fn abandon(self) {}

        /// Never reached — no backend, no container.
        pub(super) fn set_rect(&self, _rect: PhysRect) -> Result<(), SlotError> {
            Ok(())
        }

        /// Never reached — no backend, no container.
        pub(super) fn set_visible(&self, _visible: bool) {}

        /// Never reached — no backend, no container.
        pub(super) fn set_clip(&self, _local: Option<PhysRect>) -> Result<(), SlotError> {
            Ok(())
        }

        /// Never reached — no backend, no container.
        pub(super) fn raise(&self) -> Result<(), SlotError> {
            Ok(())
        }

        /// Never reached — no backend, no container.
        pub(super) fn focus(&self) -> Result<(), SlotError> {
            Ok(())
        }
    }
}

#[cfg(all(test, target_os = "windows"))]
#[expect(unsafe_code)]
mod tests {
    use std::ptr;

    use windows_sys::Win32::Foundation::{HWND, RECT};
    use windows_sys::Win32::Graphics::Gdi::{CreateRectRgn, DeleteObject, GetRgnBox, GetWindowRgn};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DestroyWindow, WS_DISABLED, WS_POPUP,
    };

    use ruviform_ui::{
        LogicalRect, NativeSurfaceSlot, SlotCapabilities, SlotError, SlotEvent, SlotGeometry,
        SlotSpec, SlotState, SlotVisibility, Support, ViewInstanceId, WindowDriver, WindowId,
        WindowSpec,
    };

    use super::backend::{self, Container};
    use super::{GpuiSurfaceAdapter, PhysRect};
    use crate::gpui_driver::{GpuiDriver, retire};

    /// An invisible, disabled, off-taskbar top-level window standing
    /// in for the GPUI window's OS parent — never shown or focused,
    /// so tests can't disturb the user's desktop.
    struct TestParent(HWND);

    impl TestParent {
        fn new() -> Self {
            let class: Vec<u16> = "STATIC".encode_utf16().chain(std::iter::once(0)).collect();
            // SAFETY: system class name is NUL-terminated; the window
            // is WS_POPUP|WS_DISABLED without WS_VISIBLE — created
            // invisible, never shown, destroyed in `Drop`.
            let hwnd = unsafe {
                CreateWindowExW(
                    0,
                    class.as_ptr(),
                    ptr::null(),
                    WS_POPUP | WS_DISABLED,
                    0,
                    0,
                    1,
                    1,
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null(),
                )
            };
            assert!(!hwnd.is_null(), "test parent CreateWindowExW failed");
            Self(hwnd)
        }

        fn raw(&self) -> usize {
            self.0 as usize
        }
    }

    impl Drop for TestParent {
        fn drop(&mut self) {
            // SAFETY: `self.0` is the window this guard created.
            unsafe {
                DestroyWindow(self.0);
            }
        }
    }

    /// A driver with one fabricated "open" window: `create_window`
    /// reserves the id, then the test injects the OS parent handle
    /// the `open_window` path would normally capture inside `App`
    /// context (no `Application`/message loop needed).
    fn driver_window(parent: &TestParent) -> (GpuiDriver, WindowId) {
        let mut driver = GpuiDriver::new();
        let window = driver
            .create_window(&WindowSpec::default())
            .expect("create window");
        driver
            .inner
            .borrow_mut()
            .windows
            .get_mut(&window)
            .expect("slot")
            .raw_parent = Some(parent.raw());
        (driver, window)
    }

    fn rect(x: f64, y: f64, w: f64, h: f64) -> LogicalRect {
        LogicalRect {
            x,
            y,
            width: w,
            height: h,
        }
    }

    fn spec(window: WindowId, geometry: LogicalRect) -> SlotSpec {
        SlotSpec {
            window,
            view: ViewInstanceId::new(1).expect("nonzero"),
            geometry: SlotGeometry {
                rect: geometry,
                scale_factor: 1.0,
            },
            visibility: SlotVisibility::Visible,
            capabilities: SlotCapabilities {
                attach_parent: Support::Supported,
                hide_show: Support::Supported,
                clip: Support::Supported,
                z_order: Support::Supported,
                focus: Support::Supported,
                ..SlotCapabilities::default()
            },
        }
    }

    /// `os_window_alive` on a `u64`-narrowed container token.
    fn alive(token: u64) -> bool {
        backend::os_window_alive(usize::try_from(token).expect("HWND token fits usize"))
    }

    /// The clip-region bounding box currently attached to `token`
    /// (container-local coords), or `None` when no region is set.
    /// `GetWindowRgn` copies the window's region into `probe`, so the
    /// OS-owned region is never touched.
    fn region_box(token: u64) -> Option<(i32, i32, i32, i32)> {
        unsafe {
            let probe = CreateRectRgn(0, 0, 0, 0);
            assert!(!probe.is_null());
            // SAFETY: `probe` is an owned HRGN; `GetWindowRgn` only
            // copies into it. ERROR (0) means no region is attached.
            let kind = GetWindowRgn(token as HWND, probe);
            if kind == 0 {
                DeleteObject(probe);
                return None;
            }
            let mut r = RECT::default();
            GetRgnBox(probe, &raw mut r);
            DeleteObject(probe);
            Some((r.left, r.top, r.right, r.bottom))
        }
    }

    // Issue #404-1: `set_clip(None)` must issue `SetWindowRgn(NULL)`
    // — the region previously applied must actually be removed.
    #[test]
    fn clip_clear_removes_os_region() {
        let parent = TestParent::new();
        let container = Container::create(
            parent.raw(),
            PhysRect {
                x: 0,
                y: 0,
                w: 64,
                h: 64,
            },
        )
        .unwrap();
        container
            .set_clip(Some(PhysRect {
                x: 0,
                y: 0,
                w: 10,
                h: 10,
            }))
            .unwrap();
        assert!(region_box(container.token()).is_some(), "region applied");
        container.set_clip(None).unwrap();
        assert_eq!(
            region_box(container.token()),
            None,
            "NULL region request must clear the attached region"
        );
    }

    // Issue #404-2: a stored window-coordinate clip must re-project
    // against the moved container origin — the old region must not
    // keep clipping at the old position.
    #[test]
    fn geometry_move_reprojects_stored_clip() {
        let parent = TestParent::new();
        let (driver, window) = driver_window(&parent);
        let mut adapter = GpuiSurfaceAdapter::new(&driver);
        let slot = adapter
            .declare_slot(spec(window, rect(10.0, 10.0, 100.0, 100.0)))
            .unwrap();
        let _lease = adapter.lease_parent(slot).unwrap();
        let token = adapter.container_token(slot).unwrap();
        adapter
            .set_clip(slot, Some(rect(30.0, 20.0, 40.0, 40.0)))
            .unwrap();
        // Window rect (30,20)-(70,60) minus origin (10,10) → local.
        assert_eq!(region_box(token), Some((20, 10, 60, 50)));
        adapter
            .set_geometry(
                slot,
                SlotGeometry {
                    rect: rect(60.0, 10.0, 100.0, 100.0),
                    scale_factor: 1.0,
                },
            )
            .unwrap();
        // Same window-coordinate clip against the new origin (60,10)
        // → local (−30,10)-(10,50): the region must follow the move.
        assert_eq!(region_box(token), Some((-30, 10, 10, 50)));

        // Adapter-level clear reaches the OS too (AC-404-1).
        adapter.set_clip(slot, None).unwrap();
        assert_eq!(region_box(token), None);
    }

    // Clip unset + geometry move: no region may appear (no
    // re-projection side effect on an empty clip).
    #[test]
    fn geometry_move_without_clip_leaves_no_region() {
        let parent = TestParent::new();
        let (driver, window) = driver_window(&parent);
        let mut adapter = GpuiSurfaceAdapter::new(&driver);
        let slot = adapter
            .declare_slot(spec(window, rect(10.0, 10.0, 100.0, 100.0)))
            .unwrap();
        let _lease = adapter.lease_parent(slot).unwrap();
        let token = adapter.container_token(slot).unwrap();
        adapter
            .set_geometry(
                slot,
                SlotGeometry {
                    rect: rect(60.0, 10.0, 100.0, 100.0),
                    scale_factor: 1.0,
                },
            )
            .unwrap();
        assert_eq!(region_box(token), None);
    }

    // `Container::create` refuses a dead parent HWND up front —
    // `lease_parent` then surfaces `Backend` with no lease left.
    #[test]
    fn create_under_dead_parent_is_rejected() {
        let parent = TestParent::new();
        let raw = parent.raw();
        drop(parent);
        assert_eq!(
            Container::create(
                raw,
                PhysRect {
                    x: 0,
                    y: 0,
                    w: 8,
                    h: 8
                }
            )
            .map(|_| ()),
            Err(SlotError::Backend)
        );
    }

    // Slots on other windows are untouched by an owner teardown —
    // death detection is per-binding, not global.
    #[test]
    fn teardown_of_other_window_leaves_slot_live() {
        let parent = TestParent::new();
        let (mut driver, window1) = driver_window(&parent);
        let window2 = driver.create_window(&WindowSpec::default()).unwrap();
        driver
            .inner
            .borrow_mut()
            .windows
            .get_mut(&window2)
            .unwrap()
            .raw_parent = Some(parent.raw());
        let mut adapter = GpuiSurfaceAdapter::new(&driver);
        let slot = adapter
            .declare_slot(spec(window1, rect(0.0, 0.0, 64.0, 64.0)))
            .unwrap();
        adapter.lease_parent(slot).unwrap();
        adapter.drain_events();

        retire(&driver.inner, window2); // a different owner dies

        assert_eq!(adapter.state(slot), Some(SlotState::Leased));
        assert!(adapter.container_token(slot).is_some());
        assert_eq!(adapter.drain_events(), Vec::<SlotEvent>::new());
    }

    // Two adapters on one driver reconcile independently — each owns
    // its registry/bindings and both observe the same owner death.
    #[test]
    fn two_adapters_reconcile_independently() {
        let parent = TestParent::new();
        let (driver, window) = driver_window(&parent);
        let mut a = GpuiSurfaceAdapter::new(&driver);
        let mut b = GpuiSurfaceAdapter::new(&driver);
        let slot_a = a
            .declare_slot(spec(window, rect(0.0, 0.0, 64.0, 64.0)))
            .unwrap();
        let slot_b = b
            .declare_slot(spec(window, rect(0.0, 0.0, 32.0, 32.0)))
            .unwrap();
        a.lease_parent(slot_a).unwrap();
        a.drain_events();
        b.drain_events();

        retire(&driver.inner, window);

        // Each ledger converges on its own sweep; ids are
        // per-registry and stay independent.
        assert_eq!(a.state(slot_a), Some(SlotState::Released));
        assert_eq!(b.state(slot_b), Some(SlotState::Released));
        assert!(
            b.drain_events()
                .contains(&SlotEvent::Destroyed { slot: slot_b })
        );
    }

    // Issue #404-3 (recorded-state side): clip declared before the
    // lease must land on the new container at `lease_parent`.
    #[test]
    fn clip_declared_before_lease_applies_on_attach() {
        let parent = TestParent::new();
        let (driver, window) = driver_window(&parent);
        let mut adapter = GpuiSurfaceAdapter::new(&driver);
        let slot = adapter
            .declare_slot(spec(window, rect(10.0, 10.0, 100.0, 100.0)))
            .unwrap();
        adapter
            .set_clip(slot, Some(rect(30.0, 20.0, 40.0, 40.0)))
            .unwrap();
        let _lease = adapter.lease_parent(slot).unwrap();
        let token = adapter.container_token(slot).unwrap();
        assert_eq!(region_box(token), Some((20, 10, 60, 50)));
    }

    // Issue #405: owner-window teardown (`retire` — the driver's
    // single terminal removal path) must reconcile the ledger: the
    // slot auto-releases, terminal events fire, and no later op
    // returns a fake Ok.
    #[test]
    fn owner_teardown_auto_releases_slot() {
        let parent = TestParent::new();
        let (driver, window) = driver_window(&parent);
        let mut adapter = GpuiSurfaceAdapter::new(&driver);
        let slot = adapter
            .declare_slot(spec(window, rect(0.0, 0.0, 64.0, 64.0)))
            .unwrap();
        let lease = adapter.lease_parent(slot).unwrap();
        adapter.drain_events(); // consume Declared + Leased

        retire(&driver.inner, window);

        // `&self` queries answer honestly even before the sweep runs.
        assert_eq!(adapter.state(slot), Some(SlotState::Released));
        assert_eq!(adapter.container_token(slot), None);
        assert_eq!(adapter.validate_lease(lease), Err(SlotError::LeaseRevoked));
        assert_eq!(adapter.redeem(lease.raw()), Err(SlotError::LeaseRevoked));

        let events = adapter.drain_events();
        assert!(events.contains(&SlotEvent::LeaseRevoked { slot }));
        assert!(events.contains(&SlotEvent::Destroyed { slot }));
        assert_eq!(adapter.state(slot), Some(SlotState::Released));

        // No fake Ok on a dead slot — every op surfaces a typed error.
        let geometry = SlotGeometry {
            rect: rect(0.0, 0.0, 8.0, 8.0),
            scale_factor: 1.0,
        };
        assert_eq!(
            adapter.set_geometry(slot, geometry),
            Err(SlotError::Released)
        );
        assert_eq!(
            adapter.set_visibility(slot, SlotVisibility::Hidden),
            Err(SlotError::Released)
        );
        assert_eq!(adapter.set_clip(slot, None), Err(SlotError::Released));
        assert_eq!(adapter.set_zorder(slot, 1), Err(SlotError::Released));
        assert_eq!(adapter.request_focus(slot), Err(SlotError::Released));
        assert_eq!(adapter.lease_parent(slot), Err(SlotError::Released));
        assert_eq!(adapter.release_slot(slot), Err(SlotError::Released));
        assert_eq!(adapter.release_lease(lease), Err(SlotError::LeaseRevoked));
    }

    // Issue #405 — teardown route that never reaches `retire`
    // (external destroy / app-exit): the dead captured OS parent is
    // itself the death signal, for leased and declared-only slots.
    #[test]
    fn unretired_owner_death_detected() {
        let parent = TestParent::new();
        let (driver, window) = driver_window(&parent);
        let mut adapter = GpuiSurfaceAdapter::new(&driver);
        let leased = adapter
            .declare_slot(spec(window, rect(0.0, 0.0, 64.0, 64.0)))
            .unwrap();
        let lease = adapter.lease_parent(leased).unwrap();
        let declared = adapter
            .declare_slot(spec(window, rect(0.0, 0.0, 32.0, 32.0)))
            .unwrap();
        adapter.drain_events();

        drop(parent); // owner HWND gone; `retire` never ran

        // Pre-sweep honest answers: released, regardless of the
        // recorded ledger state.
        assert_eq!(adapter.state(leased), Some(SlotState::Released));
        assert_eq!(adapter.state(declared), Some(SlotState::Released));

        let events = adapter.drain_events();
        assert!(events.contains(&SlotEvent::LeaseRevoked { slot: leased }));
        assert!(events.contains(&SlotEvent::Destroyed { slot: leased }));
        assert!(events.contains(&SlotEvent::Destroyed { slot: declared }));
        assert_eq!(adapter.validate_lease(lease), Err(SlotError::LeaseRevoked));
    }

    // A dead container on a *live* owner ends only the lease — the
    // slot returns to Declared and may re-lease (Issue #405
    // proportional scope: container death ≠ owner death).
    #[test]
    fn container_death_on_live_owner_revokes_lease_only() {
        let parent = TestParent::new();
        let (driver, window) = driver_window(&parent);
        let mut adapter = GpuiSurfaceAdapter::new(&driver);
        let slot = adapter
            .declare_slot(spec(window, rect(0.0, 0.0, 64.0, 64.0)))
            .unwrap();
        let lease = adapter.lease_parent(slot).unwrap();
        let token = adapter.container_token(slot).unwrap();
        adapter.drain_events();

        // A misbehaving plugin destroying the container surface.
        unsafe {
            DestroyWindow(token as HWND);
        }
        // The OS fact itself: the container HWND is dead.
        assert!(!alive(token));

        // The lease is dead; the slot is not.
        assert_eq!(adapter.state(slot), Some(SlotState::Declared));
        assert_eq!(adapter.container_token(slot), None);
        assert_eq!(adapter.redeem(lease.raw()), Err(SlotError::LeaseRevoked));
        let events = adapter.drain_events();
        assert!(events.contains(&SlotEvent::LeaseRevoked { slot }));
        assert!(!events.contains(&SlotEvent::Destroyed { slot }));
        assert_eq!(adapter.state(slot), Some(SlotState::Declared));

        // Re-lease onto a fresh container — recorded state survives.
        let lease2 = adapter.lease_parent(slot).unwrap();
        let info = adapter.validate_lease(lease2).unwrap();
        assert_eq!(info.slot, slot);
    }

    // Repeated close / double sweep must be harmless — no duplicate
    // terminal events, no panic.
    #[test]
    fn repeated_teardown_is_idempotent() {
        let parent = TestParent::new();
        let (driver, window) = driver_window(&parent);
        let mut adapter = GpuiSurfaceAdapter::new(&driver);
        let slot = adapter
            .declare_slot(spec(window, rect(0.0, 0.0, 64.0, 64.0)))
            .unwrap();
        adapter.lease_parent(slot).unwrap();
        adapter.drain_events();

        retire(&driver.inner, window);
        retire(&driver.inner, window); // second retire is a no-op
        let events = adapter.drain_events();
        assert_eq!(
            events
                .iter()
                .filter(|e| **e == SlotEvent::Destroyed { slot })
                .count(),
            1
        );
        assert_eq!(adapter.drain_events(), Vec::<SlotEvent>::new());
        assert_eq!(adapter.release_slot(slot), Err(SlotError::Released));
    }

    // Releasing a slot the owner-death sweep already released is a
    // typed error, not a second Destroyed event.
    #[test]
    fn consumer_release_after_auto_release_errors() {
        let parent = TestParent::new();
        let (driver, window) = driver_window(&parent);
        let mut adapter = GpuiSurfaceAdapter::new(&driver);
        let slot = adapter
            .declare_slot(spec(window, rect(0.0, 0.0, 64.0, 64.0)))
            .unwrap();
        adapter.drain_events();
        retire(&driver.inner, window);
        adapter.drain_events(); // sweep records the auto-release
        assert_eq!(adapter.release_slot(slot), Err(SlotError::Released));
        assert_eq!(adapter.drain_events(), Vec::<SlotEvent>::new());
    }

    // An index freed by auto-release recycles under a bumped
    // generation — the dead id stays safely stale, never aliases
    // the new slot.
    #[test]
    fn auto_released_index_recycles_generation() {
        let parent = TestParent::new();
        let (mut driver, window) = driver_window(&parent);
        let mut adapter = GpuiSurfaceAdapter::new(&driver);
        let stale = adapter
            .declare_slot(spec(window, rect(0.0, 0.0, 64.0, 64.0)))
            .unwrap();
        retire(&driver.inner, window);
        adapter.drain_events(); // sweep records the auto-release

        let window2 = driver
            .create_window(&WindowSpec::default())
            .expect("create window2");
        let fresh = adapter
            .declare_slot(spec(window2, rect(0.0, 0.0, 64.0, 64.0)))
            .unwrap();
        assert_eq!(fresh.index(), stale.index(), "index recycled");
        assert_ne!(fresh, stale, "generation bumped");

        // The stale id is a typed error, never the new slot's state.
        assert_eq!(
            adapter.set_geometry(
                stale,
                SlotGeometry {
                    rect: rect(0.0, 0.0, 8.0, 8.0),
                    scale_factor: 1.0,
                }
            ),
            Err(SlotError::StaleGeneration)
        );
        assert_eq!(adapter.state(stale), None);
        assert_eq!(adapter.state(fresh), Some(SlotState::Declared));
    }

    // `lease_parent` on a slot whose owner is already dead must not
    // attach under a dead HWND or mint a fake lease.
    #[test]
    fn lease_under_dead_owner_is_rejected() {
        let parent = TestParent::new();
        let (driver, window) = driver_window(&parent);
        let mut adapter = GpuiSurfaceAdapter::new(&driver);
        let slot = adapter
            .declare_slot(spec(window, rect(0.0, 0.0, 64.0, 64.0)))
            .unwrap();
        drop(parent); // owner HWND dead; `retire` never ran
        assert_eq!(adapter.lease_parent(slot), Err(SlotError::Released));
        assert_eq!(adapter.state(slot), Some(SlotState::Released));
    }

    // The adapter owns the container's lifetime — dropping it
    // destroys the OS surface.
    #[test]
    fn adapter_drop_destroys_container() {
        let parent = TestParent::new();
        let (driver, window) = driver_window(&parent);
        let token;
        {
            let mut adapter = GpuiSurfaceAdapter::new(&driver);
            let slot = adapter
                .declare_slot(spec(window, rect(0.0, 0.0, 64.0, 64.0)))
                .unwrap();
            adapter.lease_parent(slot).unwrap();
            token = adapter.container_token(slot).unwrap();
            assert!(alive(token));
        }
        assert!(!alive(token));
    }

    // Dropping the adapter *after* the owner window died (no sweep in
    // between) abandons the dead container instead of issuing
    // `DestroyWindow` on a handle whose value may already be recycled.
    // The observable half: `drop` itself must not panic, and the dead
    // handle is left to the OS rather than double-destroyed.
    #[test]
    fn adapter_drop_after_owner_death_abandons_dead_container() {
        let parent = TestParent::new();
        let (driver, window) = driver_window(&parent);
        let mut adapter = GpuiSurfaceAdapter::new(&driver);
        let slot = adapter
            .declare_slot(spec(window, rect(0.0, 0.0, 64.0, 64.0)))
            .unwrap();
        adapter.lease_parent(slot).unwrap();
        let token = adapter.container_token(slot).unwrap();

        drop(parent); // OS kills the container with its owner.
        assert!(!alive(token));
        drop(adapter); // No sweep ran — `Drop` must abandon, not destroy.
    }

    // Issue #387 canonical-path integration: the *public* surface
    // handle (`GpuiDriver::surface()` → `SharedSurface`, the same
    // canonical ledger the mounted renderer pushes onto) must drive
    // the whole real-HWND lifecycle — declare → lease → clip
    // Some/None → re-geometry → owner destroy → revoked/destroyed
    // events — through per-call borrows only. Standalone-adapter
    // coverage does not substitute: this is the API surface every
    // consumer actually holds.
    #[test]
    fn shared_surface_drives_real_hwnd_lifecycle() {
        let parent = TestParent::new();
        let (driver, window) = driver_window(&parent);
        let surface = driver.surface();

        let slot = surface
            .declare_slot(spec(window, rect(10.0, 10.0, 100.0, 100.0)))
            .expect("declare");

        // Clip recorded pre-lease lands on the fresh container.
        surface
            .set_clip(slot, Some(rect(30.0, 20.0, 40.0, 40.0)))
            .expect("record clip");
        let lease = surface.lease_parent(slot).expect("lease");
        let token = surface
            .container_token(slot)
            .expect("borrow")
            .expect("container token");
        assert!(alive(token), "leased container is a live HWND");
        assert_eq!(region_box(token), Some((20, 10, 60, 50)));

        // Clip clear → OS region removed; re-clip → region returns.
        surface.set_clip(slot, None).expect("clear clip");
        assert_eq!(region_box(token), None);
        surface
            .set_clip(slot, Some(rect(30.0, 20.0, 40.0, 40.0)))
            .expect("re-clip");
        assert_eq!(region_box(token), Some((20, 10, 60, 50)));

        // Re-geometry re-projects the stored clip into new local
        // coords: window clip (30,20)-(70,60) − origin (60,10).
        surface
            .set_geometry(
                slot,
                SlotGeometry {
                    rect: rect(60.0, 10.0, 100.0, 100.0),
                    scale_factor: 1.0,
                },
            )
            .expect("re-geometry");
        assert_eq!(region_box(token), Some((-30, 10, 10, 50)));

        surface.drain_events().expect("borrow"); // consume so far

        // Owner teardown → the bound adapter's sweep auto-releases:
        // honest reads before the sweep, terminal events on drain,
        // typed errors — never fake Ok — afterwards.
        retire(&driver.inner, window);
        assert_eq!(
            surface.state(slot).expect("borrow"),
            Some(SlotState::Released)
        );
        assert_eq!(surface.container_token(slot).expect("borrow"), None);
        assert_eq!(surface.validate_lease(lease), Err(SlotError::LeaseRevoked));
        assert_eq!(surface.redeem(lease.raw()), Err(SlotError::LeaseRevoked));

        let events = surface.drain_events().expect("borrow");
        assert!(events.contains(&SlotEvent::LeaseRevoked { slot }));
        assert!(events.contains(&SlotEvent::Destroyed { slot }));
        assert_eq!(
            surface.set_visibility(slot, SlotVisibility::Hidden),
            Err(SlotError::Released)
        );
    }
}
