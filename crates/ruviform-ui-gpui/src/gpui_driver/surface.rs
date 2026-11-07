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
use std::collections::BTreeMap;
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

/// Per-slot adapter bookkeeping. `container` exists exactly while a
/// lease is live.
struct Binding {
    /// The slot's owner window (`SlotSpec::window`).
    window: WindowId,
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
    /// non-live payload, [`SlotError::Backend`] when the owner window
    /// or container no longer exists, [`SlotError::Unsupported`] on
    /// OSes without a backend.
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    pub fn redeem(&self, payload: u64) -> Result<u64, SlotError> {
        let info = self.registry.validate_lease(SlotLease::new(payload))?;
        let binding = self
            .bindings
            .get(&info.slot.index())
            .ok_or(SlotError::UnknownSlot)?;
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
    /// **RT contract:** non-RT only — UI-thread only.
    pub fn drain_events(&mut self) -> Vec<SlotEvent> {
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
        self.registry.mark_busy(slot)
    }

    /// Clears `slot`'s busy mark.
    ///
    /// # Errors
    /// As [`SurfaceSlotRegistry::clear_busy`].
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    pub fn clear_busy(&mut self, slot: SurfaceSlotId) -> Result<(), SlotError> {
        self.registry.clear_busy(slot)
    }

    /// The current state of `slot`, if the id is live.
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    #[must_use]
    pub fn state(&self, slot: SurfaceSlotId) -> Option<SlotState> {
        self.registry.state(slot)
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
    /// output without needing the payload at hand).
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    #[must_use]
    pub fn container_token(&self, slot: SurfaceSlotId) -> Option<u64> {
        self.bindings
            .get(&slot.index())
            .and_then(|b| b.container.as_ref().map(backend::Container::token))
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
}

impl NativeSurfaceSlot for GpuiSurfaceAdapter {
    fn capabilities(&self, slot: SurfaceSlotId) -> Result<SlotCapabilities, SlotError> {
        let caps = self
            .registry
            .capabilities(slot)
            .ok_or(SlotError::UnknownSlot)?;
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
        self.bindings.insert(
            id.index(),
            Binding {
                window: spec.window,
                container: None,
                clip: None,
                z: 0,
            },
        );
        Ok(id)
    }

    fn release_slot(&mut self, slot: SurfaceSlotId) -> Result<(), SlotError> {
        let index = slot.index();
        // `registry.release_slot` already revokes a live lease and
        // emits `LeaseRevoked`; the container dies with it so no OS
        // surface outlives its lease.
        self.registry.release_slot(slot)?;
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
        self.registry.set_geometry(slot, geometry)?;
        if let Some(binding) = self.bindings.get(&slot.index())
            && let Some(container) = &binding.container
        {
            container.set_rect(to_phys(geometry.rect, geometry.scale_factor));
        }
        Ok(())
    }

    fn set_visibility(
        &mut self,
        slot: SurfaceSlotId,
        visibility: SlotVisibility,
    ) -> Result<(), SlotError> {
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
        let binding = self
            .bindings
            .get_mut(&slot.index())
            .ok_or(SlotError::UnknownSlot)?;
        binding.clip = clip;
        if let Some(container) = &binding.container {
            container.set_clip(clip_local(clip, geometry));
        }
        Ok(())
    }

    fn set_zorder(&mut self, slot: SurfaceSlotId, z: i32) -> Result<(), SlotError> {
        let caps = self
            .registry
            .capabilities(slot)
            .ok_or(SlotError::UnknownSlot)?;
        if !caps.z_order.supported() {
            return Err(SlotError::Unsupported);
        }
        let binding = self
            .bindings
            .get_mut(&slot.index())
            .ok_or(SlotError::UnknownSlot)?;
        binding.z = z;
        if let Some(container) = &binding.container {
            // Single-top approximation: multi-slot ordered chains land
            // with the real dock layout.
            container.raise();
        }
        Ok(())
    }

    fn request_focus(&mut self, slot: SurfaceSlotId) -> Result<(), SlotError> {
        let caps = self
            .registry
            .capabilities(slot)
            .ok_or(SlotError::UnknownSlot)?;
        if !caps.focus.supported() {
            return Err(SlotError::Unsupported);
        }
        let container = self
            .bindings
            .get(&slot.index())
            .and_then(|b| b.container.as_ref())
            .ok_or(SlotError::Backend)?;
        container.focus();
        Ok(())
    }

    fn lease_parent(&mut self, slot: SurfaceSlotId) -> Result<SlotLease, SlotError> {
        if !BACKEND_AVAILABLE {
            return Err(SlotError::Unsupported);
        }
        let window = self
            .bindings
            .get(&slot.index())
            .map(|b| b.window)
            .ok_or(SlotError::UnknownSlot)?;
        let parent = self.parent_of(window).ok_or(SlotError::Backend)?;
        let (geometry, visibility) = self
            .registry
            .spec(slot)
            .map(|s| (s.geometry, s.visibility))
            .ok_or(SlotError::UnknownSlot)?;
        let rect = to_phys(geometry.rect, geometry.scale_factor);
        let lease = self.registry.lease_parent(slot)?;
        match backend::Container::create(parent, rect) {
            Ok(container) => {
                if let Some(binding) = self.bindings.get_mut(&slot.index()) {
                    // Re-apply the recorded slot state — clip and
                    // z-order set before the lease must land on the
                    // new container, and a `Hidden` spec must attach
                    // hidden (the OS default is visible).
                    if binding.clip.is_some() {
                        container.set_clip(clip_local(binding.clip, geometry));
                    }
                    if binding.z != 0 {
                        container.raise();
                    }
                    container.set_visible(matches!(visibility, SlotVisibility::Visible));
                    binding.container = Some(container);
                }
                Ok(lease)
            }
            Err(error) => {
                // Keep the ledger honest: a failed OS attach never
                // leaves a live lease behind.
                let _ = self.registry.release_lease(lease);
                Err(error)
            }
        }
    }

    fn release_lease(&mut self, lease: SlotLease) -> Result<(), SlotError> {
        let info = self.registry.validate_lease(lease)?;
        self.registry.release_lease(lease)?;
        if let Some(binding) = self.bindings.get_mut(&info.slot.index()) {
            binding.container = None; // Drop destroys the OS surface.
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
        self.ops()
            .expect("shared surface borrow conflicted during drain_events")
            .drain_events()
    }
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
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::SetFocus;
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
        /// [`SlotError::Backend`] when `CreateWindowExW` fails.
        pub(super) fn create(parent: usize, rect: PhysRect) -> Result<Self, SlotError> {
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

        /// Repositions/resizes the container in parent client coords.
        pub(super) fn set_rect(&self, rect: PhysRect) {
            // SAFETY: `hwnd` is our live child HWND; `MoveWindow` on a
            // child moves it in parent client coordinates.
            unsafe {
                MoveWindow(self.hwnd, rect.x, rect.y, rect.w, rect.h, 1);
            }
        }

        /// Shows or hides the container (and the attached editor).
        pub(super) fn set_visible(&self, visible: bool) {
            // SAFETY: `hwnd` is our live child HWND.
            unsafe {
                ShowWindow(self.hwnd, if visible { SW_SHOW } else { SW_HIDE });
            }
        }

        /// Applies a container-local clip region, or clears it.
        /// `local` is already in the container's client coordinates;
        /// `None` removes the region.
        pub(super) fn set_clip(&self, local: Option<PhysRect>) {
            // SAFETY: `CreateRectRgn` yields an owned HRGN; a
            // successful `SetWindowRgn` transfers ownership to the OS
            // (it must not be `DeleteObject`ed). On failure the region
            // is still ours — free it explicitly instead of leaking.
            unsafe {
                let rgn: HRGN = local.map_or(ptr::null_mut(), |c| {
                    CreateRectRgn(c.x, c.y, c.x + c.w, c.y + c.h)
                });
                if !rgn.is_null() && SetWindowRgn(self.hwnd, rgn, 1) == 0 {
                    DeleteObject(rgn);
                }
            }
        }

        /// Raises the container to the top of the sibling z-order
        /// (single-top approximation until multi-slot ordering lands).
        pub(super) fn raise(&self) {
            // SAFETY: `hwnd` is our live child HWND; SWP_NOMOVE|NOSIZE
            // keep current geometry, HWND_TOP raises in z-order.
            unsafe {
                SetWindowPos(self.hwnd, HWND_TOP, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE);
            }
        }

        /// Moves keyboard focus to the container.
        pub(super) fn focus(&self) {
            // SAFETY: `hwnd` is our live child HWND.
            unsafe {
                SetFocus(self.hwnd);
            }
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
        pub(super) fn set_rect(&self, _rect: PhysRect) {}

        /// Never reached — no backend, no container.
        pub(super) fn set_visible(&self, _visible: bool) {}

        /// Never reached — no backend, no container.
        pub(super) fn set_clip(&self, _local: Option<PhysRect>) {}

        /// Never reached — no backend, no container.
        pub(super) fn raise(&self) {}

        /// Never reached — no backend, no container.
        pub(super) fn focus(&self) {}
    }
}
