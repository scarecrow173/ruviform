//! Issue #387 — GPUI Controls/Viz renderer.
//!
//! Projects the driver-neutral [`ViewSpec`] tree owned by a
//! [`ViewHost`] into a GPUI element tree, normalizes pointer/key
//! input into [`ControlInput`], and routes minted
//! [`CommandIntent`]s back to the consumer as `rvf.ui.command.v1`
//! [`Intent`]s.
//!
//! All GPUI types live inside this module — it is crate-private so
//! no `gpui` path reaches the public surface. Consumers mount a view
//! through [`crate::ViewMount`] + `create_view_window`; the driver
//! builds `RvfView` internally.
//!
//! **RT contract:** non-RT only.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::ops::Range;
use std::rc::Rc;

use gpui::prelude::FluentBuilder;
use gpui::{
    Bounds, Context, Hsla, InteractiveElement, IntoElement, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, ParentElement, Pixels, Point, Render, Rgba, ScrollDelta,
    ScrollWheelEvent, Styled, Subscription, Window, canvas, div, fill, point, px, relative, rgb,
    size,
};
use ruviform_ui::behavior::{
    CommandReceipt, ControlInput, GestureId, InputModifiers, InputSource, ReceiptStatus, Revision,
    StateProjection, ValueInput,
};
use ruviform_ui::view::{
    ComponentKey, ComponentSpec, ControlSpec, MeterSpec, NativeSlotSpec, Orientation,
    SpectrogramSpec, TimeAxisSpec, ViewError, ViewEvent, ViewHost, ViewSpec, WaveformBin,
    WaveformSpec,
};
use ruviform_ui::{
    Event, Intent, LogicalRect, SlotError, SlotGeometry, SlotVisibility, SurfaceSlotId,
};
use ruviform_ui_theme::ResolvedTheme;

use crate::IntentSink;
use crate::gpui_driver::surface::SharedSurface;

/// `op` carried on the [`Intent`] envelope for every projected
/// [`ruviform_ui::behavior::CommandIntent`] — the Action-channel
/// contract name consumers route on.
pub const COMMAND_INTENT_OP: &str = "rvf.ui.command.v1";

/// Palette resolved one-way from a [`ResolvedTheme`]. Unknown tokens
/// fall back to literal defaults — the theme interpreter stays the
/// single source of truth; this struct never re-derives token logic.
#[derive(Debug, Clone)]
pub struct ViewPalette {
    /// Panel/window background.
    pub surface_bg: Hsla,
    /// Raised surface (tab header, card).
    pub surface_raised: Hsla,
    /// Hairline borders.
    pub border: Hsla,
    /// Primary label text.
    pub text_primary: Hsla,
    /// Secondary/dim text.
    pub text_dim: Hsla,
    /// Accent (active tab underline, focus ring).
    pub accent: Hsla,
    /// Error state.
    pub error: Hsla,
    /// Disabled chrome.
    pub disabled: Hsla,
    /// Control track (unfilled portion).
    pub control_track: Hsla,
    /// Control fill (value portion).
    pub control_fill: Hsla,
    /// Control thumb/handle.
    pub control_thumb: Hsla,
    /// Waveform peak color.
    pub viz_waveform: Hsla,
    /// Waveform RMS overlay.
    pub viz_waveform_rms: Hsla,
    /// Waveform baseline.
    pub viz_baseline: Hsla,
    /// Playhead/cursor.
    pub viz_cursor: Hsla,
    /// Small spacing quantum (px).
    pub space_sm: f32,
    /// Medium spacing quantum (px).
    pub space_md: f32,
}

/// Token components and alpha are normalized `[0, 1]` — f64→f32
/// truncation is intentional for GPU colors.
#[allow(clippy::cast_possible_truncation)]
fn color_or(theme: &ResolvedTheme, key: &str, fallback: u32) -> Hsla {
    theme.color(key).map_or_else(
        || rgb(fallback).into(),
        |c| {
            let [r, g, b] = c.components;
            Rgba {
                r: r as f32,
                g: g as f32,
                b: b as f32,
                a: c.alpha as f32,
            }
            .into()
        },
    )
}

/// Theme lengths are pixel-scale values — f64→f32 truncation is
/// intentional.
#[allow(clippy::cast_possible_truncation)]
fn length_or(theme: &ResolvedTheme, key: &str, fallback: f32) -> f32 {
    theme.length(key).map_or(fallback, |l| l.value as f32)
}

impl ViewPalette {
    /// Resolves the canonical semantic token vocabulary into GPUI
    /// colors/lengths.
    #[must_use]
    pub fn from_theme(theme: &ResolvedTheme) -> Self {
        Self {
            surface_bg: color_or(theme, "surface.bg", 0x14_181d),
            surface_raised: color_or(theme, "surface.raised", 0x1d_232a),
            border: color_or(theme, "surface.border", 0x2e_3640),
            text_primary: color_or(theme, "text.primary", 0xd7_dde4),
            text_dim: color_or(theme, "text.dim", 0x8a_949f),
            accent: color_or(theme, "accent.primary", 0x3d_7be0),
            error: color_or(theme, "state.error", 0xe0_5555),
            disabled: color_or(theme, "state.disabled", 0x5a_636d),
            control_track: color_or(theme, "control.track", 0x26_2d36),
            control_fill: color_or(theme, "control.fill", 0x3d_7be0),
            control_thumb: color_or(theme, "control.thumb", 0xe8_edf2),
            viz_waveform: color_or(theme, "viz.waveform", 0x4a_90d9),
            viz_waveform_rms: color_or(theme, "viz.waveform_rms", 0x2b_5f9e),
            viz_baseline: color_or(theme, "viz.baseline", 0x39_424d),
            viz_cursor: color_or(theme, "viz.cursor", 0xe8_edf2),
            space_sm: length_or(theme, "space.sm", 4.0),
            space_md: length_or(theme, "space.md", 8.0),
        }
    }

    /// Literal default palette (no theme resolved yet).
    #[must_use]
    pub fn fallback() -> Self {
        Self {
            surface_bg: rgb(0x14_181d).into(),
            surface_raised: rgb(0x1d_232a).into(),
            border: rgb(0x2e_3640).into(),
            text_primary: rgb(0xd7_dde4).into(),
            text_dim: rgb(0x8a_949f).into(),
            accent: rgb(0x3d_7be0).into(),
            error: rgb(0xe0_5555).into(),
            disabled: rgb(0x5a_636d).into(),
            control_track: rgb(0x26_2d36).into(),
            control_fill: rgb(0x3d_7be0).into(),
            control_thumb: rgb(0xe8_edf2).into(),
            viz_waveform: rgb(0x4a_90d9).into(),
            viz_waveform_rms: rgb(0x2b_5f9e).into(),
            viz_baseline: rgb(0x39_424d).into(),
            viz_cursor: rgb(0xe8_edf2).into(),
            space_sm: 4.0,
            space_md: 8.0,
        }
    }
}

/// Track/fill/thumb quads for a slider-style control at fill ratio
/// `fill_frac` (`[0, 1]`). `thumb_frac` is the thumb's axis fraction.
fn slider_geometry(
    bounds: Bounds<Pixels>,
    fill_frac: f32,
    thumb_frac: f32,
    horizontal: bool,
) -> (Bounds<Pixels>, Bounds<Pixels>, Bounds<Pixels>) {
    let extent = bounds.size;
    let track = Bounds::new(bounds.origin, extent);
    if horizontal {
        let w = extent.width * fill_frac;
        let fill_b = Bounds::new(bounds.origin, size(w, extent.height));
        let tw = extent.width * thumb_frac;
        let tx = bounds.origin.x + w - tw / 2.0;
        let thumb = Bounds::new(
            point(tx, bounds.origin.y),
            size(tw.max(px(2.0)), extent.height),
        );
        (track, fill_b, thumb)
    } else {
        let h = extent.height * fill_frac;
        let fy = bounds.origin.y + (extent.height - h);
        let fill_b = Bounds::new(point(bounds.origin.x, fy), size(extent.width, h));
        let th = extent.height * thumb_frac;
        let ty = fy - th / 2.0;
        let thumb = Bounds::new(
            point(bounds.origin.x, ty),
            size(extent.width, th.max(px(2.0))),
        );
        (track, fill_b, thumb)
    }
}

/// Pointer→normalized mapping for one axis.
/// `pos`/`origin`/`extent` are pixel coordinates; `clamped` to `[0,1]`.
fn normalized_at(pos: f32, origin: f32, extent: f32, invert: bool) -> f64 {
    if extent <= 0.0 {
        return 0.0;
    }
    let n = ((pos - origin) / extent).clamp(0.0, 1.0);
    f64::from(if invert { 1.0 - n } else { n })
}

/// Drag bookkeeping for one control.
#[derive(Debug)]
struct DragState {
    gesture_id: GestureId,
}

/// GPUI `Render` entity projecting one [`ViewHost`].
///
/// Owns pointer→input normalization and intent egress. The host owns
/// all semantics — this entity never mutates control state directly.
/// Driver-side wiring the mounted view needs for native-surface
/// projection (Issue #387 × #388 glue): the canonical shared adapter
/// plus this window's `key → slot` map.
pub(crate) struct SlotMount {
    /// The driver's canonical surface handle — the same ledger
    /// consumers declare/lease/release on through
    /// `GpuiDriver::surface`. Per-call borrows only, so no foreign
    /// guard can starve the renderer's pushes.
    pub adapter: SharedSurface,
    /// `bind_native_slot` associations for this window — mutated
    /// live by the consumer, read each frame by the renderer.
    pub bindings: Rc<RefCell<BTreeMap<ComponentKey, SurfaceSlotId>>>,
}

/// What the renderer last pushed onto a slot — the dedupe ledger.
/// `None` on an aspect means "never pushed" (not `Some(default)`);
/// for `clip` that coincides with the port's initial state, so an
/// unpushed `None` legitimately reads as "no clip applied".
#[derive(Default)]
struct AppliedSlot {
    geometry: Option<SlotGeometry>,
    clip: Option<LogicalRect>,
    visibility: Option<SlotVisibility>,
    /// Aspects that failed with a permanent error — retried only when
    /// `Backend`/`SlotBusy` (transient) so a dead/unsupported slot
    /// does not spam the port every frame.
    failed: HashSet<&'static str>,
}

impl AppliedSlot {
    /// `true` while the port state recorded here satisfies what the
    /// current frame requires — geometry equal to the request and a
    /// clip equal to the request. `clip == None` is satisfied by a
    /// never-pushed ledger (the port's initial state is unclipped),
    /// which is the contract-level distinction that keeps a
    /// clip-incapable driver usable when the layout needs no clip.
    fn holds(&self, geometry: SlotGeometry, clip: Option<LogicalRect>) -> bool {
        self.geometry == Some(geometry) && self.clip == clip
    }
}

/// Per-view slot projection state — Panel/Tab/clip →
/// [`NativeSurfaceSlot`] port glue. `placeable` is rebuilt each
/// render pass: a bound slot not painted this frame (inactive tab,
/// removed component, unbound key) is suppressed `Hidden`, matching
/// the dock contract that invisible regions hide their native child.
struct SlotProjection {
    mount: SlotMount,
    applied: HashMap<SurfaceSlotId, AppliedSlot>,
    placeable: HashSet<SurfaceSlotId>,
    /// Last port error — surfaced for diagnostics; paint cannot
    /// propagate failures, so they are recorded instead.
    last_error: Option<SlotError>,
}

/// `Bounds<Pixels>` → window-coords [`LogicalRect`] for the slot port.
fn logical_rect_of(bounds: Bounds<Pixels>) -> LogicalRect {
    LogicalRect {
        x: f64::from(f32::from(bounds.origin.x)),
        y: f64::from(f32::from(bounds.origin.y)),
        width: f64::from(f32::from(bounds.size.width)),
        height: f64::from(f32::from(bounds.size.height)),
    }
}

/// Whether a port error is permanent for this slot — permanent
/// failures stop retrying that aspect (the ledger/adapter already
/// reported them; hammering every frame adds nothing). `Backend`
/// (OS/driver transient), `SlotBusy` (dock two-phase move — clears
/// on `BusyCleared`), and `InvalidValue` (request-specific — the
/// next frame's request may be valid) stay retryable.
fn permanent_slot_error(error: SlotError) -> bool {
    !matches!(
        error,
        SlotError::Backend | SlotError::SlotBusy | SlotError::InvalidValue
    )
}

/// Applies a single port call through the dedupe ledger — shared by
/// the painted path (`project_slot_frame`) and the suppression pass.
fn push_slot_op(
    proj: &mut SlotProjection,
    slot: SurfaceSlotId,
    aspect: &'static str,
    op: impl FnOnce(&SharedSurface) -> Result<(), SlotError>,
    mark: impl FnOnce(&mut AppliedSlot),
) {
    if proj
        .applied
        .get(&slot)
        .is_some_and(|a| a.failed.contains(aspect))
    {
        return;
    }
    match op(&proj.mount.adapter) {
        Ok(()) => mark(proj.applied.entry(slot).or_default()),
        Err(error) => {
            if permanent_slot_error(error) {
                proj.applied.entry(slot).or_default().failed.insert(aspect);
            }
            proj.last_error = Some(error);
        }
    }
}

/// Painted-frame projection for one bound slot: the element's
/// laid-out bounds → `set_geometry`, the ancestor clip mask →
/// `set_clip` (`None` when the mask is the whole viewport — the
/// layout applied no real clip). Visibility is fail-closed: the
/// native child is shown only while the port state actually holds
/// the geometry and clip this frame requests — a refused borrow, a
/// transient port error, or a permanent refusal on a required op
/// keeps (or returns) the slot `Hidden`, so an unclipped or
/// misplaced child is never exposed outside its ancestor region.
/// A `None` clip request needs nothing applied — a driver without
/// clip support still satisfies it; the `set_clip(None)` refusal
/// case is a contract-level necessary condition, not a silent
/// workaround for the upstream clip-clear defect (Issue #404).
fn project_slot_frame(
    proj: &Rc<RefCell<SlotProjection>>,
    slot: SurfaceSlotId,
    bounds: Bounds<Pixels>,
    window: &mut Window,
) {
    let mut proj = proj.borrow_mut();
    let geometry = SlotGeometry {
        rect: logical_rect_of(bounds),
        scale_factor: f64::from(window.scale_factor()),
    };
    let mask = window.content_mask().bounds;
    let clip = (mask != window.bounds()).then(|| logical_rect_of(mask));
    if proj.applied.get(&slot).and_then(|a| a.geometry) != Some(geometry) {
        push_slot_op(
            &mut proj,
            slot,
            "geometry",
            |adapter| adapter.set_geometry(slot, geometry),
            |a| a.geometry = Some(geometry),
        );
    }
    if proj.applied.get(&slot).and_then(|a| a.clip) != clip {
        push_slot_op(
            &mut proj,
            slot,
            "clip",
            |adapter| adapter.set_clip(slot, clip),
            |a| a.clip = clip,
        );
    }
    // Readiness is judged on the post-push ledger, not the op
    // results — dedupe means "not pushed this frame" can still be
    // "already holding the requested state".
    let ready = proj
        .applied
        .get(&slot)
        .is_some_and(|a| a.holds(geometry, clip));
    let wanted = if ready {
        SlotVisibility::Visible
    } else {
        SlotVisibility::Hidden
    };
    if proj.applied.get(&slot).and_then(|a| a.visibility) != Some(wanted) {
        push_slot_op(
            &mut proj,
            slot,
            "visibility",
            |adapter| adapter.set_visibility(slot, wanted),
            |a| a.visibility = Some(wanted),
        );
    }
}

pub struct RvfView {
    host: ViewHost,
    palette: ViewPalette,
    /// Intent egress — `IntentSink` or an arbitrary consumer callback.
    intent_handler: Box<dyn FnMut(Intent)>,
    /// Component bounds recorded during the last paint — maps pointer
    /// position to normalized values.
    bounds: Rc<RefCell<BTreeMap<ComponentKey, Bounds<Pixels>>>>,
    drags: BTreeMap<ComponentKey, DragState>,
    next_gesture: u64,
    /// `post_event` channel drained into the host each render —
    /// `Some` only for driver-mounted views (Issue #387 seam).
    inbox: Option<Rc<RefCell<Vec<Event>>>>,
    /// Panel/Tab/clip → [`NativeSurfaceSlot`] projection — `Some`
    /// only for driver-mounted views.
    slots: Option<Rc<RefCell<SlotProjection>>>,
    _subscriptions: Vec<Subscription>,
}

impl RvfView {
    /// Replaces the mounted spec at the current generation and repaints —
    /// the driver's [`crate::GpuiDriver::update_view`] apply step. A
    /// rejected spec leaves the mounted tree untouched.
    pub(crate) fn update_spec(
        &mut self,
        spec: ViewSpec,
        cx: &mut Context<Self>,
    ) -> Result<(), ViewError> {
        let generation = self.host.generation();
        let result = self.host.update(spec, generation).map(|_| ());
        self.flush_events(cx);
        result
    }

    /// Creates a view projecting `host` with a callback receiving
    /// minted command intents — test construction; production views
    /// come through [`Self::mounted`].
    #[cfg(all(test, feature = "gpui-test"))]
    pub fn new(
        host: ViewHost,
        palette: ViewPalette,
        intent_handler: impl FnMut(Intent) + 'static,
    ) -> Self {
        Self {
            host,
            palette,
            intent_handler: Box::new(intent_handler),
            bounds: Rc::new(RefCell::new(BTreeMap::new())),
            drags: BTreeMap::new(),
            next_gesture: 0,
            inbox: None,
            slots: None,
            _subscriptions: Vec::new(),
        }
    }

    /// Mounts a consumer view inside a driver window — called by
    /// `open_window` for [`crate::GpuiDriver::create_view_window`].
    /// `sink` carries minted intents onto the window's `Action`
    /// channel; `inbox` receives `post_event` payloads drained into
    /// the host at render; `slots` wires `NativeSlot` components to
    /// the driver's surface adapter. Window deactivation cancels
    /// in-flight drags through the canonical `FocusLost` lifecycle
    /// input.
    pub(crate) fn mounted(
        host: ViewHost,
        theme: Option<&ResolvedTheme>,
        sink: &IntentSink,
        inbox: Rc<RefCell<Vec<Event>>>,
        slots: SlotMount,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let palette = theme.map_or_else(ViewPalette::fallback, ViewPalette::from_theme);
        let sink = sink.clone();
        Self {
            host,
            palette,
            intent_handler: Box::new(move |intent| sink.send(intent)),
            bounds: Rc::new(RefCell::new(BTreeMap::new())),
            drags: BTreeMap::new(),
            next_gesture: 0,
            inbox: Some(inbox),
            slots: Some(Rc::new(RefCell::new(SlotProjection {
                mount: slots,
                applied: HashMap::new(),
                placeable: HashSet::new(),
                last_error: None,
            }))),
            _subscriptions: vec![cx.observe_window_activation(window, |this, window, cx| {
                if !window.is_window_active() {
                    this.cancel_active_drags(cx);
                }
            })],
        }
    }

    fn mint_gesture(&mut self) -> GestureId {
        self.next_gesture += 1;
        format!("gpui-{}", self.next_gesture)
    }

    #[cfg(feature = "gpui-test")]
    pub(crate) fn host(&self) -> &ViewHost {
        &self.host
    }

    /// Dispatch an input to `key` and flush emitted events.
    pub(crate) fn input(
        &mut self,
        key: &ComponentKey,
        input: ControlInput,
        cx: &mut Context<Self>,
    ) {
        let generation = self.host.generation();
        let _ = self.host.dispatch_input(generation, key, input);
        self.flush_events(cx);
    }

    /// Cancels every in-flight drag on window deactivation — the
    /// canonical `FocusLost` lifecycle input (exempt from seq drops).
    fn cancel_active_drags(&mut self, cx: &mut Context<Self>) {
        let active: Vec<(ComponentKey, GestureId)> = self
            .drags
            .iter()
            .map(|(k, d)| (k.clone(), d.gesture_id.clone()))
            .collect();
        for (key, gesture_id) in active {
            self.input(
                &key,
                ControlInput::FocusLost {
                    gesture_id: Some(gesture_id),
                    seq: None,
                },
                cx,
            );
        }
        self.drags.clear();
    }

    /// Projects `post_event` inbox payloads into the host — the
    /// driver→view half of the mount seam. `Result` replies map to
    /// [`CommandReceipt`]s: `ok` is only the transport ack — the
    /// semantic status is the typed `data.status` field decoded
    /// against the canonical receipt vocabulary (`accepted`/
    /// `queued`/`applied`/`rejected`/`unsupported`). A bare `ok`
    /// with no status records `Accepted` — the gateway took the
    /// command, not that it landed — so the head keeps its ack
    /// bound instead of being faked `Applied`. An unrecognized
    /// status is an honest terminal `Rejected`, never a default
    /// success. `ok == false` is `Rejected` with the error message
    /// (`data.revision` → receipt revision either way). `Notify`
    /// payloads decode as `{control_id, revision, value,
    /// provenance?}` → [`StateProjection`]. `Job`/`Lifecycle`/
    /// `Error` events have no view route — driver-level concerns only.
    fn drain_inbox(&mut self) {
        let Some(inbox) = &self.inbox else { return };
        let events: Vec<Event> = inbox.borrow_mut().drain(..).collect();
        for event in events {
            match event {
                Event::Result {
                    id,
                    ok,
                    data,
                    error,
                } => {
                    let revision = data
                        .as_ref()
                        .and_then(|d| d.get("revision"))
                        .and_then(|v| serde_json::from_value::<Revision>(v.clone()).ok());
                    let mut detail = error.map(|e| e.message).or_else(|| {
                        data.as_ref()
                            .and_then(|d| d.get("detail"))
                            .and_then(serde_json::Value::as_str)
                            .map(str::to_string)
                    });
                    let status = if ok {
                        match data.as_ref().and_then(|d| d.get("status")) {
                            None => ReceiptStatus::Accepted,
                            Some(raw) => {
                                if let Ok(status) =
                                    serde_json::from_value::<ReceiptStatus>(raw.clone())
                                {
                                    status
                                } else {
                                    detail = Some(format!("unrecognized receipt status: {raw}"));
                                    ReceiptStatus::Rejected
                                }
                            }
                        }
                    } else {
                        ReceiptStatus::Rejected
                    };
                    self.host.apply_receipt(&CommandReceipt {
                        command_id: id,
                        status,
                        revision,
                        detail,
                    });
                }
                Event::Notify { data, .. } => {
                    let Some(control_id) =
                        data.get("control_id").and_then(serde_json::Value::as_str)
                    else {
                        continue;
                    };
                    let Some(revision) = data
                        .get("revision")
                        .and_then(|v| serde_json::from_value::<Revision>(v.clone()).ok())
                    else {
                        continue;
                    };
                    let provenance = data
                        .get("provenance")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_string);
                    self.host.apply_projection(
                        control_id,
                        &StateProjection {
                            revision,
                            value: data
                                .get("value")
                                .cloned()
                                .unwrap_or(serde_json::Value::Null),
                            provenance,
                        },
                    );
                }
                _ => {}
            }
        }
    }

    /// Drain host events: intents → sink, others are recorded by the
    /// host. Repaint after semantic changes.
    fn flush_events(&mut self, cx: &mut Context<Self>) {
        for event in self.host.drain_events() {
            if let ViewEvent::Intent {
                intent,
                command_id,
                expect_receipt,
            } = event
            {
                let params = serde_json::json!({
                    "command_id": command_id,
                    "expect_receipt": expect_receipt,
                    "intent": intent,
                });
                (self.intent_handler)(Intent::new(command_id, COMMAND_INTENT_OP, params));
            }
        }
        cx.notify();
    }

    /// Normalized position for `key`'s control at window `position`,
    /// using bounds recorded by the last paint.
    fn normalized_for(
        &self,
        key: &ComponentKey,
        orientation: Orientation,
        position: Point<Pixels>,
    ) -> f64 {
        let b = self.bounds.borrow();
        let Some(bounds) = b.get(key) else {
            return 0.0;
        };
        match orientation {
            // Vertical faders: bottom = min, top = max.
            Orientation::Vertical => normalized_at(
                f32::from(position.y),
                f32::from(bounds.origin.y),
                f32::from(bounds.size.height),
                true,
            ),
            // Horizontal and unknown future orientations map onto the
            // horizontal axis.
            _ => normalized_at(
                f32::from(position.x),
                f32::from(bounds.origin.x),
                f32::from(bounds.size.width),
                false,
            ),
        }
    }

    fn dispatch(
        &mut self,
        key: &ComponentKey,
        orientation: Orientation,
        make: impl FnOnce(f64) -> ControlInput,
        position: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        let n = self.normalized_for(key, orientation, position);
        self.input(key, make(n), cx);
    }

    // -- element builders --------------------------------------------------

    /// The displayed normalized value is already `[0, 1]` — the
    /// f64→f32 narrowing for quad geometry is intentional.
    #[allow(clippy::cast_possible_truncation)]
    fn control_element(
        &mut self,
        spec: &ControlSpec,
        is_seek: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let key = spec.key.clone();
        let orientation = spec.orientation;
        let palette = self.palette.clone();
        let bounds_store = self.bounds.clone();
        let snap = self.host.observe(&spec.key);
        let n = snap.and_then(|s| s.display).map_or(0.0, |v| v.normalized) as f32;
        let horizontal = matches!(orientation, Orientation::Horizontal);
        let thumb_frac = 0.06_f32;
        // Visual state → fill/thumb color: the descriptor is the
        // availability authority (disabled/read_only), `visual.error`
        // marks the control, and a Seek's thumb is the viz cursor.
        let (fill_color, thumb_color) = if spec.descriptor.disabled || spec.descriptor.read_only {
            (palette.disabled, palette.disabled)
        } else if spec.visual.error.is_some() {
            (palette.control_fill, palette.error)
        } else if is_seek {
            (palette.control_fill, palette.viz_cursor)
        } else {
            (palette.control_fill, palette.control_thumb)
        };

        // The control paints itself: track + fill + thumb via quads,
        // and records its bounds for pointer mapping.
        let bounds_key = key.clone();
        let paint = canvas(
            move |bounds, _window, _cx| {
                bounds_store.borrow_mut().insert(bounds_key.clone(), bounds);
            },
            move |bounds, (), window, _cx| {
                let (track_b, fill_b, thumb_b) =
                    slider_geometry(bounds, n.clamp(0.0, 1.0), thumb_frac, horizontal);
                window.paint_quad(fill(track_b, palette.control_track));
                window.paint_quad(fill(fill_b, fill_color));
                window.paint_quad(fill(thumb_b, thumb_color));
            },
        );

        let down_key = key.clone();
        let move_key = key.clone();
        let up_key = key.clone();
        let scroll_key = key.clone();
        div()
            .size_full()
            .flex()
            .when(is_seek, |d| d.h(px(24.0)))
            .when(!is_seek, |d| d.w(px(32.0)).h_full())
            .child(paint)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, ev: &MouseDownEvent, _w, cx| {
                    let gid = this.mint_gesture();
                    this.drags.insert(
                        down_key.clone(),
                        DragState {
                            gesture_id: gid.clone(),
                        },
                    );
                    this.dispatch(
                        &down_key,
                        orientation,
                        move |n| ControlInput::Begin {
                            gesture_id: gid,
                            source: InputSource::from(InputSource::POINTER),
                            origin_view: None,
                            target: None,
                            at: Some(ValueInput::Normalized(n)),
                            modifiers: None,
                            seq: None,
                        },
                        ev.position,
                        cx,
                    );
                }),
            )
            .on_mouse_move(cx.listener(move |this, ev: &MouseMoveEvent, _w, cx| {
                if !ev.dragging() || !this.drags.contains_key(&move_key) {
                    return;
                }
                let gid = this.drags[&move_key].gesture_id.clone();
                this.dispatch(
                    &move_key,
                    orientation,
                    move |n| ControlInput::Preview {
                        gesture_id: gid,
                        value: ValueInput::Normalized(n),
                        modifiers: None,
                        seq: None,
                    },
                    ev.position,
                    cx,
                );
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(move |this, ev: &MouseUpEvent, _w, cx| {
                    this.end_drag(&up_key, orientation, ev.position, cx);
                }),
            )
            .on_scroll_wheel(cx.listener(move |this, ev: &ScrollWheelEvent, _w, cx| {
                this.scroll_input(&scroll_key, ev, cx);
            }))
    }

    /// Toggle: one click is one atomic `Set` to the flipped binary
    /// value — no drag, so there is no preview phase to coalesce.
    fn toggle_element(&mut self, spec: &ControlSpec, cx: &mut Context<Self>) -> impl IntoElement {
        let key = spec.key.clone();
        let on = self.toggle_on(&key);
        let color = if spec.descriptor.disabled || spec.descriptor.read_only {
            self.palette.disabled
        } else if on {
            self.palette.accent
        } else {
            self.palette.control_track
        };
        div().w(px(32.0)).h(px(24.0)).bg(color).on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _ev: &MouseDownEvent, _w, cx| {
                let next = !this.toggle_on(&key);
                this.input(
                    &key,
                    ControlInput::Set {
                        value: ValueInput::Normalized(if next { 1.0 } else { 0.0 }),
                        gesture_id: None,
                        source: Some(InputSource::from(InputSource::POINTER)),
                        modifiers: None,
                        seq: None,
                    },
                    cx,
                );
            }),
        )
    }

    /// Current binary state of a toggle, read from the display value.
    fn toggle_on(&self, key: &ComponentKey) -> bool {
        self.host
            .observe(key)
            .and_then(|s| s.display)
            .is_some_and(|v| v.normalized >= 0.5)
    }

    /// Wheel input → `Adjust` (Shift = fine step).
    fn scroll_input(&mut self, key: &ComponentKey, ev: &ScrollWheelEvent, cx: &mut Context<Self>) {
        let steps = match ev.delta {
            ScrollDelta::Lines(p) => f64::from(p.y),
            ScrollDelta::Pixels(p) => f64::from(p.y) / 20.0,
        };
        if steps == 0.0 {
            return;
        }
        self.input(
            key,
            ControlInput::Adjust {
                steps,
                gesture_id: None,
                source: Some(InputSource::from(InputSource::WHEEL)),
                modifiers: Some(InputModifiers {
                    fine: ev.modifiers.shift,
                }),
                seq: None,
            },
            cx,
        );
    }

    fn end_drag(
        &mut self,
        key: &ComponentKey,
        orientation: Orientation,
        position: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        let Some(drag) = self.drags.remove(key) else {
            return;
        };
        let gid = drag.gesture_id.clone();
        self.dispatch(
            key,
            orientation,
            move |n| ControlInput::Commit {
                gesture_id: gid,
                value: Some(ValueInput::Normalized(n)),
                modifiers: None,
                seq: None,
            },
            position,
            cx,
        );
    }

    /// Frame indices (`u64`/`usize`) and sample amplitudes (`f64`)
    /// narrow to `f32` pixel coordinates — sub-pixel error at extreme
    /// zoom-out is intentional; bin semantics are never consumed here.
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    fn waveform_element(&mut self, spec: &WaveformSpec) -> impl IntoElement {
        let palette = self.palette.clone();
        let channels = spec.channel_bins.clone();
        let viewport: Range<u64> = spec.viewport.clone();
        let channel_count = channels.len().max(1);
        let paint = canvas(
            move |_bounds, _window, _cx| (),
            move |bounds, (), window, _cx| {
                let span = (viewport.end - viewport.start).max(1) as f32;
                let origin = viewport.start as f32;
                let ch_h = bounds.size.height / channel_count as f32;
                window.paint_quad(fill(bounds, palette.surface_bg));
                for (ci, bins) in channels.iter().enumerate() {
                    let top = bounds.origin.y + ch_h * ci as f32;
                    // baseline
                    let base_y = top + ch_h * 0.5;
                    window.paint_quad(fill(
                        Bounds::new(
                            point(bounds.origin.x, base_y),
                            size(bounds.size.width, px(1.0)),
                        ),
                        palette.viz_baseline,
                    ));
                    for bin in bins {
                        if !waveform_bin_drawn(bin) {
                            continue; // gap — draw nothing
                        }
                        let x0 = bounds.origin.x
                            + ((bin.start as f32 - origin) / span) * bounds.size.width;
                        let x1 = bounds.origin.x
                            + ((bin.end as f32 - origin) / span) * bounds.size.width;
                        let w = (x1 - x0).max(px(1.0));
                        // min..max mapped [-1,1] → channel band.
                        let y_min = top + (1.0 - (bin.max as f32 * 0.5 + 0.5)) * ch_h;
                        let y_max = top + (1.0 - (bin.min as f32 * 0.5 + 0.5)) * ch_h;
                        window.paint_quad(fill(
                            Bounds::new(point(x0, y_min), size(w, (y_max - y_min).max(px(1.0)))),
                            palette.viz_waveform,
                        ));
                        if bin.rms.is_finite() {
                            let half = (bin.rms as f32 * 0.5) * ch_h;
                            window.paint_quad(fill(
                                Bounds::new(
                                    point(x0, base_y - half),
                                    size(w, (half * 2.0).max(px(1.0))),
                                ),
                                palette.viz_waveform_rms,
                            ));
                        }
                    }
                }
            },
        );
        div().flex_grow().min_h(px(120.0)).child(paint)
    }

    /// Level bars per channel band. Peaks grow from the band floor and
    /// the RMS overlay sits inside them; gap bins draw nothing.
    #[allow(clippy::cast_precision_loss)]
    fn meter_element(&self, spec: &MeterSpec) -> impl IntoElement {
        let palette = self.palette.clone();
        let channel_count = spec.channel_levels.len().max(1);
        let bars = meter_bar_plan(spec);
        let paint = canvas(
            move |_bounds, _window, _cx| (),
            move |bounds, (), window, _cx| {
                window.paint_quad(fill(bounds, palette.surface_bg));
                let ch_h = bounds.size.height / channel_count as f32;
                for bar in &bars {
                    let floor = bounds.origin.y + ch_h * (bar.channel + 1) as f32;
                    let x0 = bounds.origin.x + bounds.size.width * bar.x0;
                    let w = bounds.size.width * (bar.x1 - bar.x0);
                    let peak_h = ch_h * bar.peak;
                    window.paint_quad(fill(
                        Bounds::new(point(x0, floor - peak_h), size(w, peak_h)),
                        palette.viz_waveform,
                    ));
                    if let Some(rms) = bar.rms {
                        let rms_h = ch_h * rms;
                        window.paint_quad(fill(
                            Bounds::new(point(x0, floor - rms_h), size(w, rms_h)),
                            palette.viz_waveform_rms,
                        ));
                    }
                }
            },
        );
        div().flex_grow().min_h(px(120.0)).child(paint)
    }

    /// Tick marks along a baseline with labels above each mark. Major
    /// ticks span the full height; minor ticks span half of it.
    fn time_axis_element(&self, spec: &TimeAxisSpec) -> impl IntoElement {
        let palette = self.palette.clone();
        let marks = axis_mark_plan(spec);
        let labels: Vec<_> = marks
            .iter()
            .map(|m| {
                div()
                    .absolute()
                    .left(relative(m.x))
                    .top(px(palette.space_sm))
                    .text_color(palette.text_dim)
                    .child(m.label.clone())
            })
            .collect();
        let paint = canvas(
            move |_bounds, _window, _cx| (),
            move |bounds, (), window, _cx| {
                for m in &marks {
                    let h = if m.major {
                        bounds.size.height
                    } else {
                        bounds.size.height * 0.5
                    };
                    let x = bounds.origin.x + bounds.size.width * m.x;
                    let top = bounds.origin.y + bounds.size.height - h;
                    window.paint_quad(fill(
                        Bounds::new(point(x, top), size(px(1.0), h)),
                        palette.viz_baseline,
                    ));
                }
            },
        )
        .size_full();
        div()
            .relative()
            .w_full()
            .h(px(24.0))
            .child(paint)
            .children(labels)
    }

    /// Intensity cells per time column and frequency row. Gap cells
    /// draw nothing; intensity is drawn as alpha of the viz colour
    /// over the surface.
    fn spectrogram_element(&self, spec: &SpectrogramSpec) -> impl IntoElement {
        let palette = self.palette.clone();
        let cells = spectrogram_cell_plan(spec);
        let paint = canvas(
            move |_bounds, _window, _cx| (),
            move |bounds, (), window, _cx| {
                window.paint_quad(fill(bounds, palette.surface_bg));
                for cell in &cells {
                    let x0 = bounds.origin.x + bounds.size.width * cell.x0;
                    let y0 = bounds.origin.y + bounds.size.height * cell.y0;
                    let w = bounds.size.width * (cell.x1 - cell.x0);
                    let h = bounds.size.height * (cell.y1 - cell.y0);
                    window.paint_quad(fill(
                        Bounds::new(point(x0, y0), size(w.max(px(1.0)), h.max(px(1.0)))),
                        palette.viz_waveform.opacity(cell.intensity),
                    ));
                }
            },
        );
        div().flex_grow().min_h(px(120.0)).child(paint)
    }

    fn element_for(&mut self, spec: &ComponentSpec, cx: &mut Context<Self>) -> gpui::AnyElement {
        match spec {
            ComponentSpec::Panel(p) => {
                let mut el = div()
                    .size_full()
                    .flex()
                    .flex_col()
                    .bg(self.palette.surface_bg)
                    .text_color(self.palette.text_primary);
                for child in &p.children {
                    el = el.child(self.element_for(child, cx));
                }
                el.into_any_element()
            }
            ComponentSpec::Tabs(t) => {
                let active = t.active.min(t.tabs.len().saturating_sub(1));
                let mut header = div()
                    .flex()
                    .flex_row()
                    .bg(self.palette.surface_raised)
                    .border_b_1()
                    .border_color(self.palette.border);
                for (i, tab) in t.tabs.iter().enumerate() {
                    let mut tab_el = div()
                        .px(px(self.palette.space_md))
                        .py(px(self.palette.space_sm))
                        .text_color(if i == active {
                            self.palette.text_primary
                        } else {
                            self.palette.text_dim
                        })
                        .child(tab.title.clone());
                    if i == active {
                        tab_el = tab_el.border_b_2().border_color(self.palette.accent);
                    }
                    header = header.child(tab_el);
                }
                let mut body = div().size_full().flex().flex_col().child(header);
                if let Some(tab) = t.tabs.get(active) {
                    body = body.child(self.element_for(&tab.child, cx));
                }
                body.into_any_element()
            }
            ComponentSpec::Fader(c) | ComponentSpec::Knob(c) => div()
                .flex()
                .h_full()
                .child(self.control_element(c, false, cx))
                .into_any_element(),
            ComponentSpec::Toggle(c) => self.toggle_element(c, cx).into_any_element(),
            ComponentSpec::Seek(c) => div()
                .flex()
                .w_full()
                .child(self.control_element(c, true, cx))
                .into_any_element(),
            ComponentSpec::Waveform(w) => div()
                .flex_grow()
                .child(self.waveform_element(w))
                .into_any_element(),
            ComponentSpec::Meter(m) => self.meter_element(m).into_any_element(),
            ComponentSpec::TimeAxis(t) => self.time_axis_element(t).into_any_element(),
            ComponentSpec::Spectrogram(s) => self.spectrogram_element(s).into_any_element(),
            ComponentSpec::NativeSlot(n) => self.native_slot_element(n),
            other => div()
                .child(format!("unsupported component: {}", other.kind()))
                .into_any_element(),
        }
    }

    /// A region a native child surface draws into. When `key` is
    /// bound (`GpuiDriver::bind_native_slot`) the canvas prepaint
    /// hook projects the laid-out bounds + content-mask clip onto
    /// the slot and registers it `placeable` for the frame;
    /// unbound components render the placeholder only.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "min_size logical extents stay far below f32 range"
    )]
    fn native_slot_element(&mut self, spec: &NativeSlotSpec) -> gpui::AnyElement {
        let bound = self
            .slots
            .as_ref()
            .and_then(|p| p.borrow().mount.bindings.borrow().get(&spec.key).copied());
        let mut el = div()
            .flex_grow()
            .border_1()
            .border_color(self.palette.border)
            .bg(self.palette.surface_raised);
        if let Some([w, h]) = spec.min_size {
            el = el.min_w(px(w as f32)).min_h(px(h as f32));
        }
        let Some(proj) = self.slots.clone() else {
            return el.into_any_element();
        };
        let Some(slot) = bound else {
            return el
                .child(format!("native slot {}", spec.key.0))
                .into_any_element();
        };
        proj.borrow_mut().placeable.insert(slot);
        let fill_color = self.palette.control_track;
        let projection = canvas(
            move |bounds, window, _cx| {
                project_slot_frame(&proj, slot, bounds, window);
            },
            move |bounds, (), window, _cx| {
                // Placeholder under the native surface — visible only
                // where the native child does not cover the region.
                window.paint_quad(fill(bounds, fill_color));
            },
        )
        .size_full();
        el.child(projection).into_any_element()
    }

    /// Suppresses bound slots that were not painted this frame —
    /// inactive-tab children, removed components, and dropped
    /// bindings all count as "not painted". Bound slots stay in the
    /// dedupe ledger (re-activation re-projects them); unbound ones
    /// are hidden once then dropped from it.
    fn suppress_unpainted_slots(&mut self) {
        let Some(proj) = &self.slots else { return };
        let mut proj = proj.borrow_mut();
        let bound: HashSet<SurfaceSlotId> =
            proj.mount.bindings.borrow().values().copied().collect();
        let suppressed: Vec<SurfaceSlotId> = bound
            .iter()
            .copied()
            .chain(proj.applied.keys().copied())
            .collect::<HashSet<_>>()
            .into_iter()
            .filter(|s| !proj.placeable.contains(s))
            .collect();
        for slot in suppressed {
            let hidden = proj
                .applied
                .get(&slot)
                .is_some_and(|a| a.visibility == Some(SlotVisibility::Hidden));
            if !hidden {
                push_slot_op(
                    &mut proj,
                    slot,
                    "visibility",
                    |adapter| adapter.set_visibility(slot, SlotVisibility::Hidden),
                    |a| a.visibility = Some(SlotVisibility::Hidden),
                );
            }
            if bound.contains(&slot) {
                continue;
            }
            // Forgetting an unbound slot must leave the port at the
            // fresh-ledger invariant — hidden AND unclipped — or a
            // rebound slot's `clip: None` ledger would vacuously
            // satisfy a `None` request while the port still clips to
            // a stale rect.
            if proj.applied.get(&slot).is_some_and(|a| a.clip.is_some()) {
                push_slot_op(
                    &mut proj,
                    slot,
                    "clip",
                    |adapter| adapter.set_clip(slot, None),
                    |a| a.clip = None,
                );
            }
            let forgotten = proj
                .applied
                .get(&slot)
                .is_some_and(|a| a.visibility == Some(SlotVisibility::Hidden) && a.clip.is_none());
            if forgotten {
                proj.applied.remove(&slot);
            }
        }
    }
}

impl Render for RvfView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Explicit consumption point: `post_event` payloads posted
        // since the last paint project into the host before the
        // element tree is built from it.
        self.drain_inbox();
        // Slot projection frame boundary: `placeable` is repopulated
        // as `element_for` builds bound `NativeSlot` elements; slots
        // absent from the set after the tree is built are suppressed.
        if let Some(proj) = &self.slots {
            proj.borrow_mut().placeable.clear();
        }
        // Commit any drag still in flight when the pointer is released
        // anywhere in the window (window-wide mouse-up continuation).
        let root = div().size_full().on_mouse_up(
            MouseButton::Left,
            cx.listener(|this, ev: &MouseUpEvent, _w, cx| {
                let keys: Vec<ComponentKey> = this.drags.keys().cloned().collect();
                for key in keys {
                    let orientation = this
                        .host
                        .spec()
                        .and_then(|s| find_control(&s.root, &key))
                        .map_or(Orientation::Horizontal, |c| c.orientation);
                    this.end_drag(&key, orientation, ev.position, cx);
                }
            }),
        );
        let tree = match self.host.spec().map(|s| s.root.clone()) {
            Some(spec) => root.child(self.element_for(&spec, cx)),
            None => root,
        };
        self.suppress_unpainted_slots();
        tree
    }
}

fn find_control<'a>(spec: &'a ComponentSpec, key: &ComponentKey) -> Option<&'a ControlSpec> {
    match spec {
        ComponentSpec::Panel(p) => p.children.iter().find_map(|c| find_control(c, key)),
        ComponentSpec::Tabs(t) => t.tabs.iter().find_map(|tab| find_control(&tab.child, key)),
        ComponentSpec::Fader(c) | ComponentSpec::Seek(c) | ComponentSpec::Knob(c)
            if c.key == *key =>
        {
            Some(c)
        }
        _ => None,
    }
}

/// Fraction of `viewport` at which `at` falls (`0.0` = start, `1.0` =
/// end). The common layer validates `end >= start`; a zero-length
/// viewport counts as one frame wide so the division stays finite.
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn viewport_fraction(at: f64, viewport: &Range<u64>) -> f32 {
    let span = (viewport.end - viewport.start).max(1) as f64;
    ((at - viewport.start as f64) / span) as f32
}

/// One drawn Meter bar: a normalized x span inside the viewport, its
/// channel band, and `[0, 1]` peak/RMS heights (amplitudes clamped).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct MeterBarPlan {
    pub(crate) channel: usize,
    pub(crate) x0: f32,
    pub(crate) x1: f32,
    pub(crate) peak: f32,
    pub(crate) rms: Option<f32>,
}

/// Bars to draw for a Meter spec. A gap bin (`count == 0` or a
/// non-finite peak), an empty or inverted bin, and a bin fully outside
/// the viewport draw nothing; a non-finite RMS draws the peak alone.
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn meter_bar_plan(spec: &MeterSpec) -> Vec<MeterBarPlan> {
    spec.channel_levels
        .iter()
        .enumerate()
        .flat_map(|(channel, bins)| {
            bins.iter().filter_map(move |bin| {
                if bin.count == 0 || !bin.peak.is_finite() {
                    return None;
                }
                let f0 = viewport_fraction(bin.start as f64, &spec.viewport);
                let f1 = viewport_fraction(bin.end as f64, &spec.viewport);
                if f1 <= f0 || f1 <= 0.0 || f0 >= 1.0 {
                    return None;
                }
                Some(MeterBarPlan {
                    channel,
                    x0: f0.max(0.0),
                    x1: f1.min(1.0),
                    peak: bin.peak.clamp(0.0, 1.0) as f32,
                    rms: bin.rms.is_finite().then(|| bin.rms.clamp(0.0, 1.0) as f32),
                })
            })
        })
        .collect()
}

/// One drawn time-axis tick: normalized x inside the viewport and its
/// weight and label.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AxisMarkPlan {
    pub(crate) x: f32,
    pub(crate) major: bool,
    pub(crate) label: String,
}

/// Ticks to draw for a `TimeAxis` spec. Ticks outside the viewport are
/// culled — the payload keeps them, the renderer does not draw them.
fn axis_mark_plan(spec: &TimeAxisSpec) -> Vec<AxisMarkPlan> {
    spec.ticks
        .iter()
        .filter_map(|tick| {
            let x = viewport_fraction(tick.at, &spec.viewport);
            (0.0..=1.0).contains(&x).then(|| AxisMarkPlan {
                x,
                major: tick.major,
                label: tick.label.clone(),
            })
        })
        .collect()
}

/// One drawn spectrogram cell: normalized x span inside the viewport,
/// top-down normalized y span (row 0 = lowest Hz sits at the bottom),
/// and intensity in `[0, 1]`.
#[derive(Debug, Clone, PartialEq)]
struct SpectrogramCellPlan {
    x0: f32,
    x1: f32,
    y0: f32,
    y1: f32,
    intensity: f32,
}

/// Cells to draw for a `Spectrogram` spec. A gap column (`count == 0`)
/// and a non-finite magnitude draw nothing; a column fully outside the
/// viewport is culled; magnitudes map linearly onto the declared
/// `magnitude_range` and clamp to `[0, 1]`.
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn spectrogram_cell_plan(spec: &SpectrogramSpec) -> Vec<SpectrogramCellPlan> {
    let rows = spec.rows.max(1) as f32;
    let lo = spec.magnitude_range.start;
    let span = spec.magnitude_range.end - lo;
    let mut cells = Vec::new();
    for column in &spec.columns {
        let f0 = viewport_fraction(column.start as f64, &spec.viewport);
        let f1 = viewport_fraction(column.end as f64, &spec.viewport);
        if column.count == 0 || f1 <= f0 || f1 <= 0.0 || f0 >= 1.0 {
            continue;
        }
        for (row, &magnitude) in column.magnitudes.iter().enumerate() {
            if !magnitude.is_finite() {
                continue;
            }
            cells.push(SpectrogramCellPlan {
                x0: f0.max(0.0),
                x1: f1.min(1.0),
                y0: 1.0 - (row + 1) as f32 / rows,
                y1: 1.0 - row as f32 / rows,
                intensity: ((magnitude - lo) / span).clamp(0.0, 1.0) as f32,
            });
        }
    }
    cells
}

/// A Waveform bin that paints. A gap bin (`count == 0` or a non-finite
/// min/max) draws nothing; the paint closure and the drawn plan share
/// this rule.
fn waveform_bin_drawn(bin: &WaveformBin) -> bool {
    bin.count != 0 && bin.min.is_finite() && bin.max.is_finite()
}

/// Plan of one drawing component, the crate-internal form of the
/// consumer observation (`observe::DrawnPlan`).
#[cfg(feature = "gpui-test")]
pub(crate) enum DrawPlan {
    Meter(Vec<MeterBarPlan>),
    TimeAxis(Vec<AxisMarkPlan>),
    /// Per channel, the frame ranges of the bins that paint.
    Waveform(Vec<Vec<Range<u64>>>),
}

/// Drawing plans of the Meter, `TimeAxis`, and Waveform components the
/// renderer paints under `spec`, in paint order. Follows `element_for`:
/// a Panel paints every child, and a Tabs strip paints only its active
/// child.
#[cfg(feature = "gpui-test")]
pub(crate) fn drawn_plans(spec: &ComponentSpec) -> Vec<(ComponentKey, DrawPlan)> {
    let mut out = Vec::new();
    collect_drawn(spec, &mut out);
    out
}

#[cfg(feature = "gpui-test")]
fn collect_drawn(spec: &ComponentSpec, out: &mut Vec<(ComponentKey, DrawPlan)>) {
    match spec {
        ComponentSpec::Panel(p) => p.children.iter().for_each(|c| collect_drawn(c, out)),
        ComponentSpec::Tabs(t) => {
            let active = t.active.min(t.tabs.len().saturating_sub(1));
            if let Some(tab) = t.tabs.get(active) {
                collect_drawn(&tab.child, out);
            }
        }
        ComponentSpec::Meter(m) => out.push((m.key.clone(), DrawPlan::Meter(meter_bar_plan(m)))),
        ComponentSpec::TimeAxis(t) => {
            out.push((t.key.clone(), DrawPlan::TimeAxis(axis_mark_plan(t))));
        }
        ComponentSpec::Waveform(w) => {
            let channels = w
                .channel_bins
                .iter()
                .map(|bins| {
                    bins.iter()
                        .filter(|bin| waveform_bin_drawn(bin))
                        .map(|bin| bin.start..bin.end)
                        .collect()
                })
                .collect();
            out.push((w.key.clone(), DrawPlan::Waveform(channels)));
        }
        _ => {}
    }
}

#[cfg(all(test, feature = "gpui-test"))]
mod tests {
    use super::*;
    use gpui::{Modifiers, MouseButton, TestAppContext, point, px};
    use ruviform_ui::behavior::{DetachPolicy, ValueDescriptor};
    use ruviform_ui::dock::ViewInstanceId;
    use ruviform_ui::view::{
        A11ySpec, AttachEpoch, AxisTick, ComponentVisualState, MeterBin, PanelSpec, ProjectionOp,
        SpectrogramColumn, TabSpec, TabsSpec, ViewSpec,
    };
    use ruviform_ui_theme::{ThemeResolver, TokenDocument};

    fn fader_spec(key_name: &str) -> ComponentSpec {
        ComponentSpec::Fader(ControlSpec {
            key: ComponentKey(key_name.to_string()),
            control_id: None,
            target: Some("mixer.fader".to_string()),
            descriptor: ValueDescriptor {
                default_value: Some(0.5),
                step: Some(0.05),
                ..ValueDescriptor::ranged(0.0, 1.0)
            },
            orientation: Orientation::Vertical,
            detach_policy: DetachPolicy::Cancel,
            unsupported: false,
            visual: ComponentVisualState::default(),
            a11y: A11ySpec::default(),
        })
    }

    fn toggle_spec(key_name: &str) -> ComponentSpec {
        ComponentSpec::Toggle(ControlSpec {
            key: ComponentKey(key_name.to_string()),
            control_id: None,
            target: Some("mixer.mute".to_string()),
            descriptor: ValueDescriptor {
                default_value: Some(0.0),
                step: Some(1.0),
                ..ValueDescriptor::ranged(0.0, 1.0)
            },
            orientation: Orientation::Horizontal,
            detach_policy: DetachPolicy::Cancel,
            unsupported: false,
            visual: ComponentVisualState::default(),
            a11y: A11ySpec::default(),
        })
    }

    fn host_with_fader() -> ViewHost {
        let mut host = ViewHost::new(
            ViewInstanceId::new(1).expect("nonzero"),
            AttachEpoch::new(1),
        );
        host.mount(ViewSpec {
            root: ComponentSpec::Panel(PanelSpec {
                key: ComponentKey("panel".to_string()),
                title: None,
                children: vec![fader_spec("fader")],
                visual: ComponentVisualState::default(),
                a11y: A11ySpec::default(),
            }),
        })
        .expect("mount");
        host.drain_events();
        host
    }

    /// The fail-closed readiness truth table: `Visible` is allowed
    /// only while the recorded port state equals the frame's exact
    /// request — a stale geometry (layout moved, re-push refused),
    /// a stale clip (clear denied), or a never-pushed ledger all
    /// hold the slot `Hidden`; a never-pushed clip satisfies a
    /// `None` request since the port starts unclipped.
    #[test]
    fn applied_slot_holds_only_matching_state() {
        let geo = |x: f64| SlotGeometry {
            rect: LogicalRect {
                x,
                y: 0.0,
                width: 10.0,
                height: 10.0,
            },
            scale_factor: 1.0,
        };
        let clip_r = |x: f64| {
            Some(LogicalRect {
                x,
                y: 0.0,
                width: 5.0,
                height: 5.0,
            })
        };

        // Never pushed anything → not ready (geometry is required).
        assert!(!AppliedSlot::default().holds(geo(0.0), None));

        // Geometry + clip both applied as requested → ready.
        let applied = AppliedSlot {
            geometry: Some(geo(0.0)),
            clip: clip_r(0.0),
            ..AppliedSlot::default()
        };
        assert!(applied.holds(geo(0.0), clip_r(0.0)));

        // Geometry applied but clip required and never landed → Hidden.
        let applied = AppliedSlot {
            geometry: Some(geo(0.0)),
            clip: None,
            ..AppliedSlot::default()
        };
        assert!(!applied.holds(geo(0.0), clip_r(0.0)));
        // …while the same ledger satisfies a clip-free frame.
        assert!(applied.holds(geo(0.0), None));

        // Layout moved but the new geometry push was refused → the
        // stale ledger must not justify visibility.
        let applied = AppliedSlot {
            geometry: Some(geo(0.0)),
            clip: None,
            visibility: Some(SlotVisibility::Visible),
            ..AppliedSlot::default()
        };
        assert!(!applied.holds(geo(2.0), None));

        // Clip was applied, then a clear (`None`) was refused → the
        // stale clip keeps the slot hidden rather than leaking.
        let applied = AppliedSlot {
            geometry: Some(geo(0.0)),
            clip: clip_r(0.0),
            ..AppliedSlot::default()
        };
        assert!(!applied.holds(geo(0.0), None));
    }

    #[test]
    fn normalized_at_math() {
        // Horizontal: left edge = 0, right edge = 1, clamped outside.
        assert_eq!(normalized_at(0.0, 0.0, 100.0, false), 0.0);
        assert_eq!(normalized_at(50.0, 0.0, 100.0, false), 0.5);
        assert_eq!(normalized_at(200.0, 0.0, 100.0, false), 1.0);
        assert_eq!(normalized_at(-10.0, 0.0, 100.0, false), 0.0);
        // Vertical fader: bottom = 0 (inverted axis).
        assert_eq!(normalized_at(100.0, 0.0, 100.0, true), 0.0);
        assert_eq!(normalized_at(0.0, 0.0, 100.0, true), 1.0);
        // Degenerate extent.
        assert_eq!(normalized_at(10.0, 0.0, 0.0, false), 0.0);
    }

    #[test]
    fn palettes_differ_per_product_theme() {
        let dir = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../ruviform-ui-test/fixtures/themes"
        );
        let resolve = |name: &str| {
            let json =
                std::fs::read_to_string(format!("{dir}/{name}.dtcg.json")).expect("read fixture");
            let doc = TokenDocument::from_json(&json).expect("parse");
            ThemeResolver::resolve(&[doc], "dark").expect("resolve")
        };
        let sv = ViewPalette::from_theme(&resolve("synvelis-dark"));
        let al = ViewPalette::from_theme(&resolve("audiolens-dark"));
        // The two product themes must project to different palettes.
        assert_ne!(sv.accent.l, al.accent.l);
        assert_ne!(sv.viz_waveform.h, al.viz_waveform.h);
        assert_ne!(sv.control_fill.h, al.control_fill.h);
    }

    /// UI-DVR-04 GPUI lane: the palette the view paints follows the
    /// resolved theme's consumer override (the highest-precedence product
    /// document) and leaves untouched tokens alone. A mode the theme does
    /// not declare never reaches a palette — the resolver rejects it.
    #[test]
    fn ui_dvr_04_gpui_mode_override_palette() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../ruviform-ui-test/fixtures/themes/synvelis-dark.dtcg.json"
        );
        let base_json = std::fs::read_to_string(path).expect("read fixture");
        let mut override_value: serde_json::Value =
            serde_json::from_str(&base_json).expect("fixture json");
        override_value["accent"]["primary"]["$value"] = serde_json::json!("#ff0000");
        override_value["$extensions"]["dev.ruviform.dtcg-profile"]["id"] =
            serde_json::json!("synvelis-dark-consumer-override");
        let parse = |json: &str| TokenDocument::from_json(json).expect("parse");

        let base = ThemeResolver::resolve(&[parse(&base_json)], "dark").expect("resolve");
        let overridden = ThemeResolver::resolve(
            &[parse(&base_json), parse(&override_value.to_string())],
            "dark",
        )
        .expect("resolve override");
        let base_p = ViewPalette::from_theme(&base);
        let over_p = ViewPalette::from_theme(&overridden);
        let hsla = |c: Hsla| (c.h, c.s, c.l, c.a);
        assert_ne!(hsla(base_p.accent), hsla(over_p.accent));
        assert_eq!(hsla(base_p.surface_bg), hsla(over_p.surface_bg));
        assert!(ThemeResolver::resolve(&[parse(&base_json)], "light").is_err());
    }

    /// UI-DVR-03 GPUI lane (cancel + delayed arrival): window deactivation
    /// mid-drag cancels through the canonical `FocusLost` input. The drag
    /// is released, the committed value keeps its pre-gesture value (no
    /// commit applied), and a late preview for the cancelled gesture is
    /// rejected by the mounted host.
    #[gpui::test]
    fn ui_dvr_03_gpui_cancel_on_deactivate_keeps_committed(cx: &mut TestAppContext) {
        use ruviform_ui::WindowDriver;
        use ruviform_ui::behavior::{ControlInput, InteractionEvent, RejectReason, ValueInput};
        let mut driver = crate::GpuiDriver::new();
        let window = WindowDriver::create_window(&mut driver, &ruviform_ui::WindowSpec::default())
            .expect("reserve window");
        let bindings = driver.slot_binding_map(window).expect("binding map");
        let sink = driver.intent_sink(window);
        let adapter = driver.surface();
        let (view, cx) = cx.add_window_view(move |window, cx| {
            RvfView::mounted(
                host_with_fader(),
                None,
                &sink,
                Rc::new(RefCell::new(Vec::new())),
                SlotMount { adapter, bindings },
                window,
                cx,
            )
        });
        cx.run_until_parked();
        view.update_in(cx, |_v, window, _cx| window.activate_window());
        cx.run_until_parked();
        cx.simulate_mouse_down(
            point(px(16.0), px(300.0)),
            MouseButton::Left,
            Modifiers::default(),
        );
        cx.run_until_parked();
        cx.simulate_mouse_move(
            point(px(16.0), px(200.0)),
            MouseButton::Left,
            Modifiers::default(),
        );
        cx.run_until_parked();
        let key = ComponentKey("fader".to_string());
        let gesture_id = view.update(cx, |v, _cx| {
            v.drags.get(&key).expect("active drag").gesture_id.clone()
        });
        cx.deactivate_window();
        cx.run_until_parked();

        view.update(cx, |v, _cx| {
            assert!(v.drags.is_empty(), "deactivation releases the drag");
            let snap = v.host.observe(&key).expect("fader snapshot");
            assert_eq!(snap.committed.map(|c| c.normalized), Some(0.5));
            let late = ControlInput::Preview {
                gesture_id: gesture_id.clone(),
                value: ValueInput::Plain(0.9),
                modifiers: None,
                seq: None,
            };
            let generation = v.host.generation();
            let ev = v
                .host
                .dispatch_input(generation, &key, late)
                .expect("dispatch late");
            assert!(ev.iter().any(|e| matches!(
                e,
                InteractionEvent::Rejected {
                    reason: RejectReason::StaleGesture | RejectReason::AlreadyTerminated,
                    ..
                }
            )));
        });
    }

    /// UI-DVR-03 GPUI lane (out-of-order): a preview whose `seq` is older
    /// than the last accepted input is rejected by the mounted view's
    /// host, and the newer accepted input is not displaced.
    #[gpui::test]
    fn ui_dvr_03_gpui_out_of_order_preview_rejected(cx: &mut TestAppContext) {
        use ruviform_ui::behavior::{ControlInput, InteractionEvent, RejectReason, ValueInput};
        let (view, cx) = cx.add_window_view(|_window, _cx| {
            RvfView::new(host_with_fader(), ViewPalette::fallback(), |_| {})
        });
        cx.run_until_parked();
        cx.simulate_mouse_down(
            point(px(16.0), px(300.0)),
            MouseButton::Left,
            Modifiers::default(),
        );
        cx.run_until_parked();

        view.update(cx, |v, _cx| {
            let key = ComponentKey("fader".to_string());
            let gesture_id = v.drags.get(&key).expect("active drag").gesture_id.clone();
            let generation = v.host.generation();
            let preview = |seq: u64| ControlInput::Preview {
                gesture_id: gesture_id.clone(),
                value: ValueInput::Plain(0.9),
                modifiers: None,
                seq: Some(seq),
            };
            let newer = v
                .host
                .dispatch_input(generation, &key, preview(10))
                .expect("dispatch newer");
            assert!(
                !newer
                    .iter()
                    .any(|e| matches!(e, InteractionEvent::Rejected { .. }))
            );
            let older = v
                .host
                .dispatch_input(generation, &key, preview(4))
                .expect("dispatch older");
            assert!(older.iter().any(|e| matches!(
                e,
                InteractionEvent::Rejected {
                    reason: RejectReason::OutOfOrder,
                    ..
                }
            )));
        });
    }

    /// Real GPUI element path on the headless test platform: mount a
    /// fader, simulate drag, expect begin → preview → commit intents.
    #[gpui::test]
    fn rvf_view_fader_gesture(cx: &mut TestAppContext) {
        let collected = Rc::new(RefCell::new(Vec::<Intent>::new()));
        let sink = collected.clone();
        let (view, cx) = cx.add_window_view(|_window, _cx| {
            RvfView::new(host_with_fader(), ViewPalette::fallback(), move |i| {
                sink.borrow_mut().push(i);
            })
        });
        cx.run_until_parked();

        // The fader occupies the window's left 32px column.
        let start = point(px(16.0), px(300.0));
        let mid = point(px(16.0), px(200.0));
        let end = point(px(16.0), px(100.0));
        cx.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();
        cx.simulate_mouse_move(mid, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();
        cx.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        let kinds: Vec<String> = collected
            .borrow()
            .iter()
            .filter(|i| i.op == COMMAND_INTENT_OP)
            .filter_map(|i| {
                i.params
                    .get("intent")?
                    .get("kind")?
                    .as_str()
                    .map(str::to_string)
            })
            .collect();
        assert_eq!(kinds, ["begin", "preview", "commit"]);

        // Committed value lands between the mid/end normalized values.
        view.update(cx, |v, _cx| {
            let snap = v
                .host
                .observe(&ComponentKey("fader".to_string()))
                .expect("fader snapshot");
            let committed = snap.committed.expect("committed").normalized;
            assert!((0.0..=1.0).contains(&committed), "committed {committed}");
        });

        // Reconcile path: replace the spec with an empty panel — the
        // fader unmounts and the view keeps rendering without panic.
        view.update(cx, |v, _cx| {
            let generation = v.host.generation();
            v.host
                .update(
                    ViewSpec {
                        root: ComponentSpec::Panel(PanelSpec {
                            key: ComponentKey("panel".to_string()),
                            title: None,
                            children: vec![],
                            visual: ComponentVisualState::default(),
                            a11y: A11ySpec::default(),
                        }),
                    },
                    generation,
                )
                .expect("reconcile to empty panel");
            assert!(v.host.observe(&ComponentKey("fader".to_string())).is_none());
        });
        cx.run_until_parked();
    }

    /// Toggle click → atomic set: each click flips the binary value
    /// through the headless contract, and the re-rendered control
    /// reflects the new state on the next click.
    #[gpui::test]
    fn rvf_view_toggle_click_flips_binary(cx: &mut TestAppContext) {
        let mut host = ViewHost::new(
            ViewInstanceId::new(1).expect("nonzero"),
            AttachEpoch::new(1),
        );
        host.mount(ViewSpec {
            root: ComponentSpec::Panel(PanelSpec {
                key: ComponentKey("panel".to_string()),
                title: None,
                children: vec![toggle_spec("mute")],
                visual: ComponentVisualState::default(),
                a11y: A11ySpec::default(),
            }),
        })
        .expect("mount");
        host.drain_events();
        let (view, cx) =
            cx.add_window_view(|_window, _cx| RvfView::new(host, ViewPalette::fallback(), |_| {}));
        cx.run_until_parked();

        // The toggle is the 32x24 cell at the window origin.
        let at = point(px(16.0), px(12.0));
        let committed = |cx: &mut gpui::VisualTestContext| {
            view.update(cx, |v, _cx| {
                v.host
                    .observe(&ComponentKey("mute".to_string()))
                    .and_then(|s| s.committed)
                    .map(|c| c.normalized)
            })
        };
        cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();
        assert_eq!(committed(cx), Some(1.0));
        cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();
        assert_eq!(committed(cx), Some(0.0));
    }

    /// Issue #421 Fake/GPUI parity — the shared fixture the Fake side
    /// (`ruviform-ui-test/tests/driver_parity.rs`) also reads.
    fn parity_fixture() -> serde_json::Value {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../ruviform-ui-test/fixtures/driver-lab/parity/new-kinds.json"
        );
        let text = std::fs::read_to_string(path).expect("read parity fixture");
        serde_json::from_str(&text).expect("parse parity fixture")
    }

    fn parity_view(fx: &serde_json::Value) -> ViewSpec {
        serde_json::from_value(fx["view"].clone()).expect("ViewSpec from parity fixture")
    }

    fn parity_host(fx: &serde_json::Value) -> ViewHost {
        let mut host = ViewHost::new(
            ViewInstanceId::new(421).expect("nonzero"),
            AttachEpoch::new(1),
        );
        host.mount(parity_view(fx)).expect("mount parity fixture");
        host.drain_events();
        host
    }

    fn fixture_strings(v: &serde_json::Value) -> Vec<String> {
        v.as_array()
            .expect("array")
            .iter()
            .map(|s| s.as_str().expect("string").to_string())
            .collect()
    }

    fn command_kinds(collected: &RefCell<Vec<Intent>>) -> Vec<String> {
        collected
            .borrow()
            .iter()
            .filter(|i| i.op == COMMAND_INTENT_OP)
            .filter_map(|i| {
                i.params
                    .get("intent")?
                    .get("kind")?
                    .as_str()
                    .map(str::to_string)
            })
            .collect()
    }

    fn assert_near(actual: f32, expected: Option<f64>) {
        let e = expected.expect("fixture number");
        assert!((f64::from(actual) - e).abs() < 1e-3, "{actual} != {e}");
    }

    /// Drawn Meter bars and TimeAxis marks equal the shared fixture's
    /// `expect` plan — the GPUI half of the Fake/GPUI parity contract.
    #[test]
    fn parity_new_kinds_plans_match_shared_fixture() {
        let fx = parity_fixture();
        let ComponentSpec::Panel(panel) = &parity_view(&fx).root else {
            panic!("parity fixture root must be a panel");
        };
        let meter = panel
            .children
            .iter()
            .find_map(|c| match c {
                ComponentSpec::Meter(m) => Some(m),
                _ => None,
            })
            .expect("meter");
        let axis = panel
            .children
            .iter()
            .find_map(|c| match c {
                ComponentSpec::TimeAxis(t) => Some(t),
                _ => None,
            })
            .expect("time_axis");

        let bars = meter_bar_plan(meter);
        let want_bars = fx["expect"]["meter"]["drawnBars"].as_array().expect("bars");
        assert_eq!(bars.len(), want_bars.len(), "drawn bar count");
        for (bar, want) in bars.iter().zip(want_bars) {
            assert_eq!(u64::try_from(bar.channel).ok(), want["channel"].as_u64());
            assert_near(bar.x0, want["x0"].as_f64());
            assert_near(bar.x1, want["x1"].as_f64());
            assert_near(bar.peak, want["peak"].as_f64());
            assert_near(bar.rms.expect("rms"), want["rms"].as_f64());
        }

        let marks = axis_mark_plan(axis);
        let want_marks = fx["expect"]["timeAxis"]["drawnMarks"]
            .as_array()
            .expect("marks");
        assert_eq!(marks.len(), want_marks.len(), "drawn mark count");
        for (mark, want) in marks.iter().zip(want_marks) {
            assert_near(mark.x, want["x"].as_f64());
            assert_eq!(Some(mark.major), want["major"].as_bool(), "major");
            assert_eq!(Some(mark.label.as_str()), want["label"].as_str(), "label");
        }

        let ComponentSpec::Spectrogram(spectrogram) = panel
            .children
            .iter()
            .find(|c| matches!(c, ComponentSpec::Spectrogram(_)))
            .expect("spectrogram")
        else {
            unreachable!("found by matches!")
        };
        let cells = spectrogram_cell_plan(spectrogram);
        let want_cells = fx["expect"]["spectrogram"]["drawnCells"]
            .as_array()
            .expect("cells");
        assert_eq!(cells.len(), want_cells.len(), "drawn cell count");
        for (cell, want) in cells.iter().zip(want_cells) {
            assert_near(cell.x0, want["x0"].as_f64());
            assert_near(cell.x1, want["x1"].as_f64());
            assert_near(cell.y0, want["y0"].as_f64());
            assert_near(cell.y1, want["y1"].as_f64());
            assert_near(cell.intensity, want["intensity"].as_f64());
        }
    }

    /// Spectrogram gap columns (`count == 0`), non-finite magnitudes and
    /// viewport-culled columns draw nothing; magnitudes clamp to the
    /// declared range; row 0 (lowest Hz) sits at the bottom.
    #[test]
    fn spectrogram_plan_skips_gaps_and_clamps() {
        let column = |start, end, count, magnitudes: Vec<f64>| SpectrogramColumn {
            start,
            end,
            count,
            magnitudes,
        };
        let spectrogram = SpectrogramSpec {
            key: ComponentKey("s".to_string()),
            viewport: 0..300,
            rows: 2,
            magnitude_range: 0.0..1.0,
            columns: vec![
                column(0, 100, 100, vec![f64::NAN, 0.5]),
                column(100, 200, 0, vec![1.0, 1.0]),
                column(200, 300, 100, vec![3.0, -1.0]),
                column(300, 400, 100, vec![1.0, 1.0]),
            ],
            visual: ComponentVisualState::default(),
            a11y: A11ySpec::default(),
        };
        let cells = spectrogram_cell_plan(&spectrogram);
        assert_eq!(
            cells.len(),
            3,
            "NaN, count==0 and culled cells draw nothing"
        );
        assert!((cells[0].intensity - 0.5).abs() < f32::EPSILON);
        assert!(
            (cells[0].y0 - 0.0).abs() < f32::EPSILON,
            "row 1 sits at the top"
        );
        assert!((cells[0].y1 - 0.5).abs() < f32::EPSILON);
        assert!(
            (cells[1].intensity - 1.0).abs() < f32::EPSILON,
            "over-range clamps"
        );
        assert!(
            (cells[2].intensity - 0.0).abs() < f32::EPSILON,
            "under-range clamps"
        );
        assert!(
            (cells[1].y0 - 0.5).abs() < f32::EPSILON,
            "row 0 sits at the bottom"
        );
    }

    /// Gap bins (`count == 0`) and non-finite peaks draw nothing; a
    /// non-finite RMS keeps the peak bar; over-range levels clamp to the
    /// band; an out-of-range tick is culled.
    #[test]
    fn meter_and_axis_plans_skip_gaps_and_clamp() {
        let bin = |start, end, peak: f64, rms: f64, count| MeterBin {
            start,
            end,
            peak,
            rms,
            count,
        };
        let meter = MeterSpec {
            key: ComponentKey("m".to_string()),
            channels: 1,
            channel_levels: vec![vec![
                bin(0, 100, f64::NAN, f64::NAN, 100),
                bin(100, 200, 0.5, f64::NAN, 100),
                bin(200, 300, 2.0, 0.1, 0),
                bin(300, 400, 3.0, 0.2, 100),
                bin(350, 300, 0.5, 0.2, 10),
            ]],
            viewport: 0..400,
            visual: ComponentVisualState::default(),
            a11y: A11ySpec::default(),
        };
        let bars = meter_bar_plan(&meter);
        assert_eq!(bars.len(), 2, "NaN peak and count==0 bins draw nothing");
        assert_eq!(bars[0].rms, None, "non-finite RMS keeps the peak only");
        assert!(
            (bars[1].peak - 1.0).abs() < f32::EPSILON,
            "over-range peak clamps"
        );

        let axis = TimeAxisSpec {
            key: ComponentKey("t".to_string()),
            viewport: 100..200,
            ticks: vec![
                AxisTick {
                    at: 99.0,
                    label: "before".into(),
                    major: true,
                },
                AxisTick {
                    at: 150.0,
                    label: "mid".into(),
                    major: false,
                },
                AxisTick {
                    at: 200.0,
                    label: "end".into(),
                    major: true,
                },
            ],
            visual: ComponentVisualState::default(),
            a11y: A11ySpec::default(),
        };
        let marks = axis_mark_plan(&axis);
        assert_eq!(
            marks.iter().map(|m| m.label.as_str()).collect::<Vec<_>>(),
            ["mid", "end"]
        );
        assert!(
            (marks[0].x - 0.5).abs() < f32::EPSILON,
            "culled ticks keep fractions"
        );
    }

    /// `drawn_plans` walks the tree the way `element_for` paints it: a
    /// Panel paints every child, a Tabs strip only its active child.
    #[test]
    fn drawn_plans_follow_paint_tree() {
        let meter = |key_name: &str| {
            ComponentSpec::Meter(MeterSpec {
                key: ComponentKey(key_name.to_string()),
                channels: 1,
                channel_levels: vec![vec![MeterBin {
                    start: 0,
                    end: 100,
                    peak: 0.5,
                    rms: 0.2,
                    count: 100,
                }]],
                viewport: 0..100,
                visual: ComponentVisualState::default(),
                a11y: A11ySpec::default(),
            })
        };
        let tab = |key_name: &str| TabSpec {
            key: ComponentKey(key_name.to_string()),
            title: key_name.to_string(),
            child: Box::new(meter(key_name)),
        };
        let tabs = ComponentSpec::Tabs(TabsSpec {
            key: ComponentKey("tabs".to_string()),
            active: 1,
            tabs: vec![tab("hidden"), tab("shown")],
            visual: ComponentVisualState::default(),
            a11y: A11ySpec::default(),
        });
        let root = ComponentSpec::Panel(PanelSpec {
            key: ComponentKey("root".to_string()),
            title: None,
            children: vec![meter("first"), tabs],
            visual: ComponentVisualState::default(),
            a11y: A11ySpec::default(),
        });
        let keys: Vec<String> = drawn_plans(&root)
            .into_iter()
            .map(|(key, _)| key.0)
            .collect();
        assert_eq!(keys, ["first", "shown"]);
    }

    /// Gap bins (`count == 0` or a non-finite min/max) are absent from the
    /// Waveform plan; the painted bins keep their frame ranges per channel.
    #[test]
    fn waveform_plan_keeps_only_painted_bins() {
        let bin = |start, end, min: f64, max: f64, count| WaveformBin {
            start,
            end,
            min,
            max,
            rms: 0.1,
            count,
        };
        let waveform = ComponentSpec::Waveform(WaveformSpec {
            key: ComponentKey("wave".to_string()),
            sample_rate: 48_000,
            channels: 2,
            frames: 300,
            viewport: 0..300,
            channel_bins: vec![
                vec![
                    bin(0, 100, -0.5, 0.5, 100),
                    bin(100, 200, 0.0, 0.0, 0),
                    bin(200, 300, f64::NAN, f64::NAN, 100),
                ],
                vec![bin(0, 300, -1.0, 1.0, 300)],
            ],
            visual: ComponentVisualState::default(),
            a11y: A11ySpec::default(),
        });
        let plans = drawn_plans(&waveform);
        let [(key, DrawPlan::Waveform(channels))] = plans.as_slice() else {
            panic!("one waveform plan");
        };
        assert_eq!(key.0, "wave");
        assert_eq!(channels, &vec![vec![0..100], vec![0..300]]);
    }

    /// Knob and Toggle gestures through the real element path mint the
    /// same intent kinds as the fixture, and the new kinds lay out and
    /// paint in the same frame.
    #[gpui::test]
    fn rvf_view_parity_new_kinds_gesture_and_render(cx: &mut TestAppContext) {
        let fx = parity_fixture();
        let collected = Rc::new(RefCell::new(Vec::<Intent>::new()));
        let sink = collected.clone();
        let (view, cx) = cx.add_window_view(|_window, _cx| {
            RvfView::new(parity_host(&fx), ViewPalette::fallback(), move |i| {
                sink.borrow_mut().push(i);
            })
        });
        cx.run_until_parked();

        // The toggle is the first child — the 32x24 cell at the origin.
        cx.simulate_mouse_down(
            point(px(16.0), px(12.0)),
            MouseButton::Left,
            Modifiers::default(),
        );
        cx.run_until_parked();
        assert_eq!(
            command_kinds(&collected),
            fixture_strings(&fx["expect"]["toggle"]["intents"])
        );
        let toggled = view.update(cx, |v, _cx| {
            v.host
                .observe(&ComponentKey("toggle".to_string()))
                .and_then(|s| s.committed)
                .map(|c| c.normalized)
        });
        assert_eq!(
            toggled,
            fx["expect"]["toggle"]["committedAfterClick"].as_f64()
        );
        collected.borrow_mut().clear();

        // The knob's bounds are recorded at prepaint; drag up through its center.
        let knob = view.update(cx, |v, _cx| {
            *v.bounds
                .borrow()
                .get(&ComponentKey("knob".to_string()))
                .expect("knob bounds")
        });
        let c = knob.center();
        let mid = point(c.x, c.y - px(20.0));
        let end = point(c.x, c.y - px(40.0));
        cx.simulate_mouse_down(c, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();
        cx.simulate_mouse_move(mid, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();
        cx.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();
        assert_eq!(
            command_kinds(&collected),
            fixture_strings(&fx["expect"]["knob"]["intents"])
        );
    }

    /// `ui_dvr_02` GPUI lane: the view reconciles the new kinds with
    /// stable keys — a dropped key is removed, a surviving key keeps its
    /// identity — and the reconciled view still renders.
    #[gpui::test]
    fn rvf_view_parity_new_kinds_mount_reconcile(cx: &mut TestAppContext) {
        let fx = parity_fixture();
        let (view, cx) = cx.add_window_view(|_window, _cx| {
            RvfView::new(parity_host(&fx), ViewPalette::fallback(), |_| {})
        });
        cx.run_until_parked();
        view.update(cx, |v, _cx| {
            let mut next = parity_view(&fx);
            if let ComponentSpec::Panel(p) = &mut next.root {
                p.children.retain(|c| c.key().0 != "toggle");
            }
            let generation = v.host.generation();
            let ops = v.host.update(next, generation).expect("reconcile");
            assert!(ops.contains(&ProjectionOp::Removed {
                key: ComponentKey("toggle".to_string()),
            }));
            assert!(v.host.observe(&ComponentKey("knob".to_string())).is_some());
        });
        cx.run_until_parked();
    }

    /// Window-wide mouse-up recovers a control's orientation through
    /// `find_control`; a vertical Knob must not fall back to horizontal.
    #[test]
    fn find_control_resolves_knob_orientation() {
        let knob = ComponentSpec::Knob(ControlSpec {
            key: ComponentKey("knob".to_string()),
            control_id: None,
            target: None,
            descriptor: ValueDescriptor::ranged(0.0, 1.0),
            orientation: Orientation::Vertical,
            detach_policy: DetachPolicy::Cancel,
            unsupported: false,
            visual: ComponentVisualState::default(),
            a11y: A11ySpec::default(),
        });
        let found = find_control(&knob, &ComponentKey("knob".to_string())).expect("knob");
        assert_eq!(found.orientation, Orientation::Vertical);
    }

    /// `Tabs` whose first tab holds a `NativeSlot` for `slot_key` —
    /// the docked-editor layout the slot glue is measured against.
    fn tabs_with_native_slot(slot_key: &ComponentKey) -> ComponentSpec {
        use ruviform_ui::view::{NativeSlotSpec, TabSpec, TabsSpec};
        let slot_component = ComponentSpec::NativeSlot(NativeSlotSpec {
            key: slot_key.clone(),
            min_size: None,
            visual: ComponentVisualState::default(),
            a11y: A11ySpec::default(),
        });
        ComponentSpec::Tabs(TabsSpec {
            key: ComponentKey("tabs".to_string()),
            active: 0,
            tabs: vec![
                TabSpec {
                    key: ComponentKey("tab.editor".to_string()),
                    title: "Editor".to_string(),
                    child: Box::new(slot_component),
                },
                TabSpec {
                    key: ComponentKey("tab.mixer".to_string()),
                    title: "Mixer".to_string(),
                    child: Box::new(ComponentSpec::Panel(PanelSpec {
                        key: ComponentKey("mixer".to_string()),
                        title: None,
                        children: vec![],
                        visual: ComponentVisualState::default(),
                        a11y: A11ySpec::default(),
                    })),
                },
            ],
            visual: ComponentVisualState::default(),
            a11y: A11ySpec::default(),
        })
    }

    /// Declares a Hidden slot on `window`'s canonical adapter and
    /// binds it to `key` — the consumer-side mount flow.
    fn declare_and_bind(
        driver: &mut crate::GpuiDriver,
        window: ruviform_ui::WindowId,
        view_id: ViewInstanceId,
        key: &ComponentKey,
    ) -> SurfaceSlotId {
        use ruviform_ui::{
            LogicalRect, SlotCapabilities, SlotGeometry, SlotSpec, SlotVisibility, Support,
        };
        let caps = SlotCapabilities {
            attach_parent: Support::Supported,
            hide_show: Support::Supported,
            clip: Support::Supported,
            ..SlotCapabilities::default()
        };
        let slot = driver
            .surface()
            .declare_slot(SlotSpec {
                window,
                view: view_id,
                geometry: SlotGeometry {
                    rect: LogicalRect {
                        x: 0.0,
                        y: 0.0,
                        width: 0.0,
                        height: 0.0,
                    },
                    scale_factor: 1.0,
                },
                visibility: SlotVisibility::Hidden,
                capabilities: caps,
            })
            .expect("declare slot");
        driver
            .bind_native_slot(window, key.clone(), slot)
            .expect("bind slot");
        slot
    }

    /// Panel/Tab → slot-port glue: a bound `NativeSlot` painted in
    /// the active tab gets geometry/clip/visible; switching to a tab
    /// where it is absent suppresses it `Hidden` — all asserted on
    /// the canonical adapter's event ledger (Issue #387 × #388).
    #[gpui::test]
    fn rvf_view_native_slot_projects_layout(cx: &mut TestAppContext) {
        use ruviform_ui::{SlotEvent, WindowDriver};

        let key = ComponentKey("dock.editor".to_string());
        let mut driver = crate::GpuiDriver::new();
        let window = WindowDriver::create_window(&mut driver, &ruviform_ui::WindowSpec::default())
            .expect("reserve window");
        let view_id = ViewInstanceId::new(1).expect("nonzero");
        let slot = declare_and_bind(&mut driver, window, view_id, &key);
        let bindings = driver.slot_binding_map(window).expect("binding map");
        let sink = driver.intent_sink(window);
        let adapter = driver.surface();

        let mut host = ViewHost::new(view_id, AttachEpoch::new(1));
        host.mount(ViewSpec {
            root: ComponentSpec::Panel(PanelSpec {
                key: ComponentKey("root".to_string()),
                title: None,
                children: vec![tabs_with_native_slot(&key)],
                visual: ComponentVisualState::default(),
                a11y: A11ySpec::default(),
            }),
        })
        .expect("mount");
        host.drain_events();

        let inbox = Rc::new(RefCell::new(Vec::new()));
        let mount_adapter = adapter.clone();
        let (view, cx) = cx.add_window_view(move |window, cx| {
            RvfView::mounted(
                host,
                None,
                &sink,
                inbox,
                SlotMount {
                    adapter: mount_adapter,
                    bindings,
                },
                window,
                cx,
            )
        });
        cx.run_until_parked();

        // Painted + bound → geometry, clip, Visible all projected.
        {
            let spec = adapter
                .spec(slot)
                .expect("surface borrow")
                .expect("slot spec on canonical adapter");
            assert!(
                spec.geometry.rect.width > 0.0 && spec.geometry.rect.height > 0.0,
                "painted bounds projected: {:?}",
                spec.geometry.rect
            );
            assert_eq!(spec.visibility, SlotVisibility::Visible);
        }

        // Switch to the tab without the slot → suppressed Hidden.
        view.update(cx, |v, cx| {
            let generation = v.host.generation();
            let mut spec = v.host.spec().expect("spec").clone();
            if let ComponentSpec::Panel(p) = &mut spec.root
                && let ComponentSpec::Tabs(t) = &mut p.children[0]
            {
                t.active = 1;
            }
            v.host.update(spec, generation).expect("switch tab");
            cx.notify();
        });
        cx.run_until_parked();
        {
            let spec = adapter
                .spec(slot)
                .expect("surface borrow")
                .expect("slot spec");
            assert_eq!(
                spec.visibility,
                SlotVisibility::Hidden,
                "unpainted bound slot suppressed"
            );
        }
        let events = adapter.drain_events().expect("surface borrow");
        assert!(
            events
                .iter()
                .any(|e| matches!(e, SlotEvent::GeometryChanged { slot: s } if *s == slot)),
            "geometry event recorded: {events:?}"
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, SlotEvent::VisibilityChanged { slot: s } if *s == slot)),
            "visibility event recorded: {events:?}"
        );
    }

    /// A mounted fader view with a driver-wired inbox — the seam a
    /// consumer's `post_event` replies travel through.
    fn mount_view_with_inbox(
        cx: &mut TestAppContext,
    ) -> (gpui::Entity<RvfView>, Rc<RefCell<Vec<Event>>>) {
        use ruviform_ui::WindowDriver;
        let mut driver = crate::GpuiDriver::new();
        let window = WindowDriver::create_window(&mut driver, &ruviform_ui::WindowSpec::default())
            .expect("reserve window");
        let bindings = driver.slot_binding_map(window).expect("binding map");
        let sink = driver.intent_sink(window);
        let adapter = driver.surface();
        let inbox = Rc::new(RefCell::new(Vec::new()));
        let inbox_view = inbox.clone();
        let (view, _cx) = cx.add_window_view(move |window, cx| {
            RvfView::mounted(
                host_with_fader(),
                None,
                &sink,
                inbox_view,
                SlotMount { adapter, bindings },
                window,
                cx,
            )
        });
        (view, inbox)
    }

    /// Begins + commits a gesture on `key` inside the view and
    /// returns the minted commit `command_id`.
    fn commit_in_view(
        view: &gpui::Entity<RvfView>,
        cx: &mut TestAppContext,
        gesture: &str,
    ) -> String {
        view.update(cx, |v, _cx| commit_on_host(&mut v.host, gesture))
    }

    /// Begins + commits a gesture on the `fader` control of `host`
    /// and returns the minted commit `command_id`. Works on a bare
    /// host, so a driver test can mint a pending command before the
    /// view is mounted.
    fn commit_on_host(host: &mut ViewHost, gesture: &str) -> String {
        let generation = host.generation();
        let key = ComponentKey("fader".to_string());
        host.dispatch_input(
            generation,
            &key,
            ControlInput::Begin {
                gesture_id: gesture.to_string(),
                source: InputSource::from(InputSource::POINTER),
                origin_view: None,
                target: None,
                at: None,
                modifiers: None,
                seq: None,
            },
        )
        .expect("begin");
        host.dispatch_input(
            generation,
            &key,
            ControlInput::Commit {
                gesture_id: gesture.to_string(),
                value: None,
                modifiers: None,
                seq: None,
            },
        )
        .expect("commit");
        host.drain_events()
            .into_iter()
            .filter_map(|e| match e {
                ViewEvent::Intent {
                    intent: ruviform_ui::behavior::CommandIntent::Commit { .. },
                    command_id,
                    ..
                } => Some(command_id),
                _ => None,
            })
            .next_back()
            .expect("commit minted")
    }

    fn view_phase(
        view: &gpui::Entity<RvfView>,
        cx: &mut TestAppContext,
    ) -> ruviform_ui::behavior::CommandPhase {
        use ruviform_ui::behavior::CommandPhase;
        view.update(cx, |v, _cx| {
            v.host.binding("fader").map_or(
                CommandPhase::Idle,
                ruviform_ui::behavior::CommandBinding::phase,
            )
        })
    }

    /// `Result.ok` is a transport ack, not a semantic receipt: a
    /// typed `queued` keeps the head pending (still ack-bound, never
    /// faked `Applied`), and a later projection cannot retire a head
    /// that never reached `applied` rank.
    #[gpui::test]
    fn rvf_view_inbox_queued_receipt_keeps_head(cx: &mut TestAppContext) {
        use ruviform_ui::behavior::CommandPhase;
        let (view, inbox) = mount_view_with_inbox(cx);
        cx.run_until_parked();
        let cmd = commit_in_view(&view, cx, "g1");
        inbox.borrow_mut().push(Event::result_ok(
            cmd.clone(),
            serde_json::json!({"status": "queued", "revision": 1}),
        ));
        view.update(cx, |_v, cx| cx.notify());
        cx.run_until_parked();
        assert_eq!(view_phase(&view, cx), CommandPhase::Accepted);
        // An advancing projection proves domain state — but the head
        // still answers "did my command land": only an applied/terminal
        // receipt retires it.
        inbox.borrow_mut().push(Event::Notify {
            seq: 1,
            data: serde_json::json!({
                "control_id": "fader",
                "revision": 2,
                "value": 0.8
            }),
        });
        view.update(cx, |_v, cx| cx.notify());
        cx.run_until_parked();
        assert_eq!(view_phase(&view, cx), CommandPhase::Accepted);
        let pending = view.update(cx, |v, _cx| {
            v.host
                .binding("fader")
                .and_then(|b| b.state().pending_command_id)
        });
        assert_eq!(pending.as_deref(), Some(cmd.as_str()));
    }

    /// `applied` then an advancing projection — the canonical
    /// settle path: head retires on the observed truth.
    #[gpui::test]
    fn rvf_view_inbox_applied_then_projection_observed(cx: &mut TestAppContext) {
        use ruviform_ui::behavior::CommandPhase;
        let (view, inbox) = mount_view_with_inbox(cx);
        cx.run_until_parked();
        let cmd = commit_in_view(&view, cx, "g1");
        inbox.borrow_mut().push(Event::result_ok(
            cmd,
            serde_json::json!({"status": "applied", "revision": 3}),
        ));
        view.update(cx, |_v, cx| cx.notify());
        cx.run_until_parked();
        assert_eq!(view_phase(&view, cx), CommandPhase::Applied);
        inbox.borrow_mut().push(Event::Notify {
            seq: 2,
            data: serde_json::json!({
                "control_id": "fader",
                "revision": 4,
                "value": 0.9
            }),
        });
        view.update(cx, |_v, cx| cx.notify());
        cx.run_until_parked();
        assert_eq!(view_phase(&view, cx), CommandPhase::Observed);
    }

    /// Issue #423: a receipt posted through the driver before the window
    /// is `Ready` is held in the window inbox, then projected into the
    /// mounted view once the window opens — the head retires to
    /// `Applied` in the rendered binding. The headless lifecycle fixture
    /// only counts the inbox; this checks the projection itself.
    #[gpui::test]
    fn rvf_driver_late_receipt_projects_into_view_after_ready(cx: &mut TestAppContext) {
        use ruviform_ui::WindowDriver;
        use ruviform_ui::behavior::CommandPhase;
        let mut host = host_with_fader();
        let cmd = commit_on_host(&mut host, "late-g1");
        let mut driver = crate::GpuiDriver::new();
        let window = driver
            .create_view_window(
                &ruviform_ui::WindowSpec::default(),
                crate::ViewMount { host, theme: None },
            )
            .expect("create view window");
        driver.post_event(
            window,
            &Event::result_ok(cmd, serde_json::json!({"status": "applied", "revision": 3})),
        );
        assert_eq!(driver.inbox_len_for_test(window), 1);
        driver.settle_for_test(cx, |_, _| {});
        let view = driver.view_for_test(cx, window).expect("mounted view");
        view.update(cx, |_v, cx| cx.notify());
        cx.run_until_parked();
        assert_eq!(driver.inbox_len_for_test(window), 0);
        assert_eq!(view_phase(&view, cx), CommandPhase::Applied);
    }

    /// Issue #414: an authoritative projection must reach the value
    /// the renderer actually paints. `control_element` reads
    /// `snapshot.display.normalized` — this test asserts that exact
    /// field through the real inbox → `apply_projection` path, not
    /// merely that `observed` was recorded.
    #[gpui::test]
    fn rvf_view_inbox_projection_updates_controlled_display(cx: &mut TestAppContext) {
        use ruviform_ui::behavior::CommandPhase;
        let (view, inbox) = mount_view_with_inbox(cx);
        cx.run_until_parked();
        // Baseline: the descriptor default is what the renderer reads.
        let before = view.update(cx, |v, _cx| {
            v.host
                .observe(&ComponentKey("fader".to_string()))
                .and_then(|s| s.display)
                .map(|d| d.normalized)
        });
        assert_eq!(before, Some(0.5));

        // Idle external nonzero update through the wire envelope.
        inbox.borrow_mut().push(Event::Notify {
            seq: 1,
            data: serde_json::json!({
                "control_id": "fader",
                "revision": 1,
                "value": {"normalized": 0.8}
            }),
        });
        view.update(cx, |_v, cx| cx.notify());
        cx.run_until_parked();
        let after = view.update(cx, |v, _cx| {
            let snap = v
                .host
                .observe(&ComponentKey("fader".to_string()))
                .expect("snapshot");
            (snap.display.map(|d| d.normalized), snap.phase)
        });
        let (Some(after_n), CommandPhase::Observed) = after else {
            panic!("projection must land as observed display: {after:?}");
        };
        assert!((after_n - 0.8).abs() < 1e-9, "display={after_n}");
    }

    /// Issue #414: mid-gesture the authoritative write must not
    /// overwrite the local preview; once the gesture ends the staged
    /// truth becomes the displayed value.
    #[gpui::test]
    fn rvf_view_inbox_projection_defers_to_active_gesture(cx: &mut TestAppContext) {
        use ruviform_ui::behavior::CommandPhase;
        let (view, inbox) = mount_view_with_inbox(cx);
        cx.run_until_parked();
        // Begin + preview a local drag to 0.7.
        view.update(cx, |v, _cx| {
            let generation = v.host.generation();
            let key = ComponentKey("fader".to_string());
            v.host
                .dispatch_input(
                    generation,
                    &key,
                    ControlInput::Begin {
                        gesture_id: "g1".to_string(),
                        source: InputSource::from(InputSource::POINTER),
                        origin_view: None,
                        target: None,
                        at: None,
                        modifiers: None,
                        seq: None,
                    },
                )
                .expect("begin");
            v.host
                .dispatch_input(
                    generation,
                    &key,
                    ControlInput::Preview {
                        gesture_id: "g1".to_string(),
                        value: ValueInput::Normalized(0.7),
                        modifiers: None,
                        seq: None,
                    },
                )
                .expect("preview");
        });
        // The projection lands while the gesture is active: observed
        // records it, but the display keeps the preview.
        inbox.borrow_mut().push(Event::Notify {
            seq: 1,
            data: serde_json::json!({
                "control_id": "fader",
                "revision": 1,
                "value": {"normalized": 0.3}
            }),
        });
        view.update(cx, |_v, cx| cx.notify());
        cx.run_until_parked();
        let during = view.update(cx, |v, _cx| {
            let snap = v
                .host
                .observe(&ComponentKey("fader".to_string()))
                .expect("snapshot");
            (snap.display.map(|d| d.normalized), snap.phase)
        });
        let (Some(during_n), CommandPhase::Preview) = during else {
            panic!("preview must win mid-gesture: {during:?}");
        };
        assert!((during_n - 0.7).abs() < 1e-9, "display={during_n}");
        // Commit: the staged authoritative value takes the display.
        view.update(cx, |v, _cx| {
            let generation = v.host.generation();
            v.host
                .dispatch_input(
                    generation,
                    &ComponentKey("fader".to_string()),
                    ControlInput::Commit {
                        gesture_id: "g1".to_string(),
                        value: None,
                        modifiers: None,
                        seq: None,
                    },
                )
                .expect("commit");
        });
        let after = view.update(cx, |v, _cx| {
            v.host
                .observe(&ComponentKey("fader".to_string()))
                .and_then(|s| s.display)
                .map(|d| d.normalized)
        });
        let Some(after_n) = after else {
            panic!("display missing after commit");
        };
        assert!((after_n - 0.3).abs() < 1e-9, "display={after_n}");
    }

    /// Terminal and malformed replies stay honest: `ok:false` and an
    /// unrecognized typed status both read `Rejected` (the latter
    /// preserving the raw status in the detail), while a bare `ok`
    /// with no status is `Accepted` — never a default `Applied`.
    #[gpui::test]
    fn rvf_view_inbox_terminal_and_unknown_status(cx: &mut TestAppContext) {
        use ruviform_ui::behavior::CommandPhase;
        use ruviform_ui::ipc::ErrorBody;
        let (view, inbox) = mount_view_with_inbox(cx);
        cx.run_until_parked();

        let cmd = commit_in_view(&view, cx, "g1");
        inbox.borrow_mut().push(Event::result_err(
            cmd,
            ErrorBody::new("service_error", "domain refused"),
        ));
        view.update(cx, |_v, cx| cx.notify());
        cx.run_until_parked();
        assert_eq!(view_phase(&view, cx), CommandPhase::Rejected);

        let cmd = commit_in_view(&view, cx, "g2");
        inbox.borrow_mut().push(Event::result_ok(
            cmd,
            serde_json::json!({"status": "bogus"}),
        ));
        view.update(cx, |_v, cx| cx.notify());
        cx.run_until_parked();
        assert_eq!(view_phase(&view, cx), CommandPhase::Rejected);
        let detail = view.update(cx, |v, _cx| {
            v.host
                .binding("fader")
                .and_then(|b| b.state().last_receipt)
                .and_then(|r| r.detail)
        });
        assert!(
            detail.as_deref().is_some_and(|d| d.contains("bogus")),
            "unrecognized status surfaced in detail: {detail:?}"
        );

        let cmd = commit_in_view(&view, cx, "g3");
        inbox
            .borrow_mut()
            .push(Event::result_ok(cmd, serde_json::json!({"revision": 9})));
        view.update(cx, |_v, cx| cx.notify());
        cx.run_until_parked();
        assert_eq!(view_phase(&view, cx), CommandPhase::Accepted);
    }

    /// Fail-closed visibility: a painted slot whose port ops are
    /// refused (released slot — permanent `Released` on every
    /// aspect) must never be shown, and nothing is recorded on the
    /// ledger as if it had been.
    #[gpui::test]
    fn rvf_view_native_slot_fail_closed_on_refusal(cx: &mut TestAppContext) {
        use ruviform_ui::{SlotEvent, WindowDriver};

        let key = ComponentKey("dock.editor".to_string());
        let mut driver = crate::GpuiDriver::new();
        let window = WindowDriver::create_window(&mut driver, &ruviform_ui::WindowSpec::default())
            .expect("reserve window");
        let view_id = ViewInstanceId::new(1).expect("nonzero");
        let slot = declare_and_bind(&mut driver, window, view_id, &key);
        let bindings = driver.slot_binding_map(window).expect("binding map");
        let sink = driver.intent_sink(window);
        let adapter = driver.surface();

        let mut host = ViewHost::new(view_id, AttachEpoch::new(1));
        host.mount(ViewSpec {
            root: ComponentSpec::Panel(PanelSpec {
                key: ComponentKey("root".to_string()),
                title: None,
                children: vec![tabs_with_native_slot(&key)],
                visual: ComponentVisualState::default(),
                a11y: A11ySpec::default(),
            }),
        })
        .expect("mount");
        host.drain_events();

        // Release after binding: every port op now refuses with a
        // permanent `Released` — the painted frame must stay Hidden.
        adapter.release_slot(slot).expect("release slot");
        adapter.drain_events().expect("surface borrow");

        let inbox = Rc::new(RefCell::new(Vec::new()));
        let mount_adapter = adapter.clone();
        let (view, cx) = cx.add_window_view(move |window, cx| {
            RvfView::mounted(
                host,
                None,
                &sink,
                inbox,
                SlotMount {
                    adapter: mount_adapter,
                    bindings,
                },
                window,
                cx,
            )
        });
        cx.run_until_parked();
        cx.run_until_parked();

        let spec = adapter
            .spec(slot)
            .expect("surface borrow")
            .expect("slot spec");
        assert_eq!(
            spec.visibility,
            SlotVisibility::Hidden,
            "a refused slot is never shown"
        );
        let events = adapter.drain_events().expect("surface borrow");
        assert!(
            !events.iter().any(|e| matches!(
                e,
                SlotEvent::VisibilityChanged { slot: s } if *s == slot
            )),
            "no visibility op could have landed: {events:?}"
        );
        // The refusals are recorded honestly — the ops ran and were
        // denied, not silently skipped.
        let last_error = view.update(cx, |v, _cx| {
            v.slots.as_ref().and_then(|p| p.borrow().last_error)
        });
        assert!(
            matches!(last_error, Some(SlotError::Released)),
            "refusal recorded: {last_error:?}"
        );
    }

    /// Public-API starvation regression: a consumer holding only a
    /// `SharedSurface` clone (per-call borrows — it cannot hold a
    /// `RefMut`) can never block the renderer's pushes, so every
    /// `Visible` → `Hidden` → `Visible` transition lands honestly.
    #[gpui::test]
    fn rvf_view_native_slot_public_api_cannot_starve_pushes(cx: &mut TestAppContext) {
        use ruviform_ui::WindowDriver;

        let key = ComponentKey("dock.editor".to_string());
        let mut driver = crate::GpuiDriver::new();
        let window = WindowDriver::create_window(&mut driver, &ruviform_ui::WindowSpec::default())
            .expect("reserve window");
        let view_id = ViewInstanceId::new(1).expect("nonzero");
        let slot = declare_and_bind(&mut driver, window, view_id, &key);
        let bindings = driver.slot_binding_map(window).expect("binding map");
        let sink = driver.intent_sink(window);
        let adapter = driver.surface();

        let mut host = ViewHost::new(view_id, AttachEpoch::new(1));
        host.mount(ViewSpec {
            root: ComponentSpec::Panel(PanelSpec {
                key: ComponentKey("root".to_string()),
                title: None,
                children: vec![tabs_with_native_slot(&key)],
                visual: ComponentVisualState::default(),
                a11y: A11ySpec::default(),
            }),
        })
        .expect("mount");
        host.drain_events();

        let inbox = Rc::new(RefCell::new(Vec::new()));
        let mount_adapter = adapter.clone();
        let (view, cx) = cx.add_window_view(move |window, cx| {
            RvfView::mounted(
                host,
                None,
                &sink,
                inbox,
                SlotMount {
                    adapter: mount_adapter,
                    bindings,
                },
                window,
                cx,
            )
        });
        cx.run_until_parked();

        // The stale-`Visible` counterexample is unconstructible
        // through the public API: the consumer keeps its
        // `SharedSurface` clone across every frame below — the only
        // thing it *can* hold — and the renderer's pushes still land.
        let spec = adapter
            .spec(slot)
            .expect("surface borrow")
            .expect("slot spec");
        assert_eq!(spec.visibility, SlotVisibility::Visible);

        // Visible → inactive tab: `Hidden` must land — a starved
        // push would leave the port stuck at `Visible`.
        view.update(cx, |v, cx| {
            let generation = v.host.generation();
            let mut spec = v.host.spec().expect("spec").clone();
            if let ComponentSpec::Panel(p) = &mut spec.root
                && let ComponentSpec::Tabs(t) = &mut p.children[0]
            {
                t.active = 1;
            }
            v.host.update(spec, generation).expect("switch tab");
            cx.notify();
        });
        cx.run_until_parked();
        let spec = adapter
            .spec(slot)
            .expect("surface borrow")
            .expect("slot spec");
        assert_eq!(
            spec.visibility,
            SlotVisibility::Hidden,
            "inactive tab must hide an already-Visible slot"
        );

        // Back to the slot's tab: geometry + Visible project again.
        view.update(cx, |v, cx| {
            let generation = v.host.generation();
            let mut spec = v.host.spec().expect("spec").clone();
            if let ComponentSpec::Panel(p) = &mut spec.root
                && let ComponentSpec::Tabs(t) = &mut p.children[0]
            {
                t.active = 0;
            }
            v.host.update(spec, generation).expect("switch back");
            cx.notify();
        });
        cx.run_until_parked();
        let spec = adapter
            .spec(slot)
            .expect("surface borrow")
            .expect("slot spec");
        assert_eq!(
            spec.visibility,
            SlotVisibility::Visible,
            "re-painting an already-Visible slot re-projects it"
        );

        // Unbind → suppressed + forgotten; rebind → Visible again.
        driver
            .unbind_native_slot(window, &key)
            .expect("unbind slot");
        view.update(cx, |_v, cx| cx.notify());
        cx.run_until_parked();
        cx.run_until_parked();
        let spec = adapter
            .spec(slot)
            .expect("surface borrow")
            .expect("slot spec");
        assert_eq!(spec.visibility, SlotVisibility::Hidden);
        driver
            .bind_native_slot(window, key.clone(), slot)
            .expect("rebind slot");
        view.update(cx, |_v, cx| cx.notify());
        cx.run_until_parked();
        let spec = adapter
            .spec(slot)
            .expect("surface borrow")
            .expect("slot spec");
        assert_eq!(
            spec.visibility,
            SlotVisibility::Visible,
            "rebound slot is re-projected, not left at a stale state"
        );
    }

    /// Necessary-condition distinction: a driver without clip
    /// support still satisfies a frame that needs no clip (mask ==
    /// viewport → `None`), so the slot shows — while `set_clip` is
    /// never pushed at all.
    #[gpui::test]
    fn rvf_view_native_slot_unclipped_shows_without_clip_support(cx: &mut TestAppContext) {
        use ruviform_ui::{
            LogicalRect, SlotCapabilities, SlotGeometry, SlotSpec, SlotVisibility, Support,
            WindowDriver,
        };

        let key = ComponentKey("dock.editor".to_string());
        let mut driver = crate::GpuiDriver::new();
        let window = WindowDriver::create_window(&mut driver, &ruviform_ui::WindowSpec::default())
            .expect("reserve window");
        let view_id = ViewInstanceId::new(1).expect("nonzero");
        let slot = driver
            .surface()
            .declare_slot(SlotSpec {
                window,
                view: view_id,
                geometry: SlotGeometry {
                    rect: LogicalRect {
                        x: 0.0,
                        y: 0.0,
                        width: 0.0,
                        height: 0.0,
                    },
                    scale_factor: 1.0,
                },
                visibility: SlotVisibility::Hidden,
                capabilities: SlotCapabilities {
                    attach_parent: Support::Supported,
                    hide_show: Support::Supported,
                    clip: Support::Unsupported,
                    ..SlotCapabilities::default()
                },
            })
            .expect("declare slot");
        driver
            .bind_native_slot(window, key.clone(), slot)
            .expect("bind slot");
        let bindings = driver.slot_binding_map(window).expect("binding map");
        let sink = driver.intent_sink(window);
        let adapter = driver.surface();
        adapter.drain_events().expect("surface borrow");

        let mut host = ViewHost::new(view_id, AttachEpoch::new(1));
        host.mount(ViewSpec {
            root: ComponentSpec::Panel(PanelSpec {
                key: ComponentKey("root".to_string()),
                title: None,
                children: vec![tabs_with_native_slot(&key)],
                visual: ComponentVisualState::default(),
                a11y: A11ySpec::default(),
            }),
        })
        .expect("mount");
        host.drain_events();

        let inbox = Rc::new(RefCell::new(Vec::new()));
        let mount_adapter = adapter.clone();
        let (view, cx) = cx.add_window_view(move |window, cx| {
            RvfView::mounted(
                host,
                None,
                &sink,
                inbox,
                SlotMount {
                    adapter: mount_adapter,
                    bindings,
                },
                window,
                cx,
            )
        });
        cx.run_until_parked();

        let spec = adapter
            .spec(slot)
            .expect("surface borrow")
            .expect("slot spec");
        assert_eq!(
            spec.visibility,
            SlotVisibility::Visible,
            "no clip requested → clip capability irrelevant"
        );
        // A vacuous `None` clip request is satisfied without pushing
        // `set_clip` at all — the `Unsupported` refusal the port
        // would have returned is never even attempted.
        let last_error = view.update(cx, |v, _cx| {
            v.slots.as_ref().and_then(|p| p.borrow().last_error)
        });
        assert!(
            last_error.is_none(),
            "no clip op attempted for a clip-free frame: {last_error:?}"
        );
    }

    /// Unbind → ledger drop must restore the port to the
    /// fresh-ledger invariant — hidden AND unclipped. Without the
    /// clip reset, a rebound slot's fresh `clip: None` ledger
    /// vacuously satisfies a `None` request while the port still
    /// clips to a stale rect.
    #[gpui::test]
    fn rvf_view_native_slot_forget_clears_clip(cx: &mut TestAppContext) {
        use ruviform_ui::WindowDriver;

        let key = ComponentKey("dock.editor".to_string());
        let mut driver = crate::GpuiDriver::new();
        let window = WindowDriver::create_window(&mut driver, &ruviform_ui::WindowSpec::default())
            .expect("reserve window");
        let view_id = ViewInstanceId::new(1).expect("nonzero");
        let slot = declare_and_bind(&mut driver, window, view_id, &key);
        let bindings = driver.slot_binding_map(window).expect("binding map");
        let sink = driver.intent_sink(window);
        let adapter = driver.surface();

        let mut host = ViewHost::new(view_id, AttachEpoch::new(1));
        host.mount(ViewSpec {
            root: ComponentSpec::Panel(PanelSpec {
                key: ComponentKey("root".to_string()),
                title: None,
                children: vec![tabs_with_native_slot(&key)],
                visual: ComponentVisualState::default(),
                a11y: A11ySpec::default(),
            }),
        })
        .expect("mount");
        host.drain_events();

        let inbox = Rc::new(RefCell::new(Vec::new()));
        let mount_adapter = adapter.clone();
        let (view, cx) = cx.add_window_view(move |window, cx| {
            RvfView::mounted(
                host,
                None,
                &sink,
                inbox,
                SlotMount {
                    adapter: mount_adapter,
                    bindings,
                },
                window,
                cx,
            )
        });
        cx.run_until_parked();
        assert_eq!(
            adapter
                .spec(slot)
                .expect("surface borrow")
                .expect("spec")
                .visibility,
            SlotVisibility::Visible
        );

        // Stage a previously-applied clip on both sides — the
        // port's recorded clip and the renderer's dedupe ledger —
        // as if an earlier frame had required a real clip mask.
        let stale_clip = LogicalRect {
            x: 4.0,
            y: 4.0,
            width: 40.0,
            height: 40.0,
        };
        adapter
            .set_clip(slot, Some(stale_clip))
            .expect("stage port clip");
        view.update(cx, |v, _cx| {
            if let Some(proj) = &v.slots {
                proj.borrow_mut().applied.entry(slot).or_default().clip = Some(stale_clip);
            }
        });

        // Unbind → suppressed → forgotten only once port state is
        // back at hidden + unclipped.
        driver
            .unbind_native_slot(window, &key)
            .expect("unbind slot");
        view.update(cx, |_v, cx| cx.notify());
        cx.run_until_parked();
        cx.run_until_parked();

        assert_eq!(
            adapter.clip_of(slot).expect("surface borrow"),
            None,
            "port clip restored to unclipped before forgetting"
        );
        let retained = view.update(cx, |v, _cx| {
            v.slots
                .as_ref()
                .is_some_and(|p| p.borrow().applied.contains_key(&slot))
        });
        assert!(!retained, "ledger entry dropped after hidden+clear");
    }
}
