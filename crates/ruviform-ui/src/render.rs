//! Rendering responsibility split — type-level contract only.
//!
//! The accepted hybrid-UI ADR separates three tiers; this module names
//! them and sketches the native-surface port. Implementation is
//! intentionally **not** in this crate:
//!
//! - [`RenderTier::Dom`]/[`RenderTier::Canvas2d`]/[`RenderTier::Webgl2`]
//!   run inside the webview and are frontend concerns (the ADR assigns
//!   `WebGL2` as the practical floor for high-rate visualization —
//!   `poc/ui` measured `Canvas2D` collapsing at ~32k columns while
//!   `WebGL2` held ~58fps).
//! - [`RenderTier::NativeSurface`] is the heavy-duty `wgpu` path. Its
//!   adapter is the `ruviform-gpu` candidate; only the port contract
//!   ([`SurfaceRegion`], [`NativeSurfaceHost`]) lives here so host and
//!   services can describe regions without backend types.
//!
//! Known backend constraint (measured in `poc/ui`, AC3): a `WebView2`
//! container paints topmost by default and DOM-inside compositing needs
//! the pending `wry` PR #1762 / `ICoreWebView2CompositionController`.
//! Native surfaces are therefore *adjacent* regions; DOM overlays on
//! top of a native surface are not guaranteed.

use crate::error::UiError;

/// Rendering tier a surface/component lives on.
///
/// **RT contract:** non-RT only — UI-thread concept.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderTier {
    /// DOM/controls (forms, lists, dialogs) — frontend framework.
    Dom,
    /// In-webview `Canvas2D` — compatibility path for small/medium
    /// visualizations.
    Canvas2d,
    /// In-webview WebGL2 — the ADR-designated practical floor for
    /// high-rate visualization across the three engines.
    Webgl2,
    /// Native wgpu surface adjacent to the webview — heavy-duty path
    /// owned by a future `ruviform-gpu` adapter.
    NativeSurface,
}

/// Physical-pixel rectangle describing an adjacent native-surface
/// region relative to the window content area.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfaceRegion {
    /// Left edge in physical pixels.
    pub x: i32,
    /// Top edge in physical pixels.
    pub y: i32,
    /// Width in physical pixels.
    pub width: u32,
    /// Height in physical pixels.
    pub height: u32,
}

/// Opaque identifier of a live native surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SurfaceId(pub u64);

/// Port for managing adjacent native surfaces on the host window.
///
/// Implemented by a native-surface adapter (`ruviform-gpu`
/// candidate). The contract captures the `poc/ui` finding that native
/// surfaces are siblings of the webview, not children composited
/// inside the DOM.
///
/// **RT contract:** non-RT only — UI-thread only.
pub trait NativeSurfaceHost {
    /// Creates a native surface covering `region`.
    ///
    /// # Errors
    /// Returns [`UiError`] when the backend cannot create a surface.
    fn create_surface(&self, region: SurfaceRegion) -> Result<SurfaceId, UiError>;

    /// Moves/resizes an existing surface (window resize, layout
    /// changes, DPI transitions).
    ///
    /// # Errors
    /// Returns [`UiError`] on unknown ids or backend failures.
    fn move_surface(&self, id: SurfaceId, region: SurfaceRegion) -> Result<(), UiError>;

    /// Destroys a surface.
    ///
    /// # Errors
    /// Returns [`UiError`] on unknown ids or backend failures.
    fn destroy_surface(&self, id: SurfaceId) -> Result<(), UiError>;
}
