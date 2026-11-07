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

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use ruviform_ui::{
    LeaseInfo, LogicalRect, NativeSurfaceSlot, SlotCapabilities, SlotError, SlotEvent,
    SlotGeometry, SlotLease, SlotSpec, SlotState, SlotVisibility, SurfaceSlotId,
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
    /// Explicit clip in slot coordinates (`None` = unconstrained).
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
    /// **RT contract:** non-RT only — UI-thread only.
    #[must_use]
    pub fn new(driver: &GpuiDriver) -> Self {
        Self {
            inner: driver.inner.clone(),
            registry: SurfaceSlotRegistry::new(),
            bindings: BTreeMap::new(),
        }
    }

    /// Redeems an opaque lease payload (the value a session's
    /// `ParentSlotLease` carries) into the raw OS parent token — the
    /// container surface's handle (`u64`-narrowed).
    ///
    /// The lease is validated against the ledger first: stale,
    /// revoked or forged payloads never yield a live parent.
    ///
    /// # Errors
    /// [`SlotError::InvalidLease`]/[`SlotError::LeaseRevoked`] on a
    /// non-live payload, [`SlotError::Backend`] when the container is
    /// missing, [`SlotError::Unsupported`] on OSes without a backend.
    ///
    /// **RT contract:** non-RT only — UI-thread only.
    pub fn redeem(&self, payload: u64) -> Result<u64, SlotError> {
        let info = self.registry.validate_lease(SlotLease::new(payload))?;
        let binding = self
            .bindings
            .get(&info.slot.index())
            .ok_or(SlotError::UnknownSlot)?;
        let container = binding.container.as_ref().ok_or(SlotError::Backend)?;
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
        Ok(caps)
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
        let scale = self
            .registry
            .spec(slot)
            .map_or(1.0, |s| s.geometry.scale_factor);
        let geometry = self
            .registry
            .spec(slot)
            .map(|s| s.geometry.rect)
            .ok_or(SlotError::UnknownSlot)?;
        let binding = self
            .bindings
            .get_mut(&slot.index())
            .ok_or(SlotError::UnknownSlot)?;
        binding.clip = clip;
        if let Some(container) = &binding.container {
            // Slot coords → container-local region: the region rect
            // is the clip shifted by the geometry origin.
            let local = clip.map(|c| {
                to_phys(
                    LogicalRect {
                        x: c.x - geometry.x,
                        y: c.y - geometry.y,
                        width: c.width,
                        height: c.height,
                    },
                    scale,
                )
            });
            container.set_clip(local);
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
        let spec = self.registry.spec(slot).ok_or(SlotError::UnknownSlot)?;
        let rect = to_phys(spec.geometry.rect, spec.geometry.scale_factor);
        let lease = self.registry.lease_parent(slot)?;
        match backend::Container::create(parent, rect) {
            Ok(container) => {
                if let Some(binding) = self.bindings.get_mut(&slot.index()) {
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

// ---- OS container backend --------------------------------------------

/// The OS container surface for this platform. Windows gets the real
/// child-HWND backend; every other OS gets a never-constructed stub
/// whose `create` reports [`SlotError::Unsupported`].
#[cfg(target_os = "windows")]
#[expect(unsafe_code)]
mod backend {
    use std::ptr;

    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::Graphics::Gdi::{CreateRectRgn, HRGN, SetWindowRgn};
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::SetFocus;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DestroyWindow, HWND_TOP, MoveWindow, SW_HIDE, SW_SHOW, SWP_NOMOVE,
        SWP_NOSIZE, SetWindowPos, ShowWindow, WS_CHILD, WS_CLIPCHILDREN, WS_CLIPSIBLINGS,
        WS_VISIBLE,
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
            // SAFETY: `CreateRectRgn` yields an owned HRGN; passing it
            // to `SetWindowRgn` transfers ownership to the OS — a
            // successful call must not `DeleteObject` it. On failure
            // the rgn is dropped via DeleteObject-free NULL path (an
            // HRGN leak on SetWindowRgn failure is bounded and rare;
            // the OS frees it with the window).
            unsafe {
                let rgn: HRGN = local.map_or(ptr::null_mut(), |c| {
                    CreateRectRgn(c.x, c.y, c.x + c.w, c.y + c.h)
                });
                SetWindowRgn(self.hwnd, rgn, 1);
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
