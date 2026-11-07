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
use std::collections::BTreeMap;
use std::ops::Range;
use std::rc::Rc;

use gpui::prelude::FluentBuilder;
use gpui::{
    Bounds, Context, Hsla, InteractiveElement, IntoElement, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, ParentElement, Pixels, Point, Render, Rgba, ScrollDelta,
    ScrollWheelEvent, Styled, Subscription, Window, canvas, div, fill, point, px, rgb, size,
};
use ruviform_ui::behavior::{
    CommandReceipt, ControlInput, GestureId, InputModifiers, InputSource, ReceiptStatus, Revision,
    StateProjection, ValueInput,
};
use ruviform_ui::view::{
    ComponentKey, ComponentSpec, ControlSpec, Orientation, ViewEvent, ViewHost, WaveformSpec,
};
use ruviform_ui::{Event, Intent};
use ruviform_ui_theme::ResolvedTheme;

use crate::IntentSink;

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
    _subscriptions: Vec<Subscription>,
}

impl RvfView {
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
            _subscriptions: Vec::new(),
        }
    }

    /// Mounts a consumer view inside a driver window — called by
    /// `open_window` for [`crate::GpuiDriver::create_view_window`].
    /// `sink` carries minted intents onto the window's `Action`
    /// channel; `inbox` receives `post_event` payloads drained into
    /// the host at render. Window deactivation cancels in-flight
    /// drags through the canonical `FocusLost` lifecycle input.
    pub(crate) fn mounted(
        host: ViewHost,
        theme: Option<&ResolvedTheme>,
        sink: &IntentSink,
        inbox: Rc<RefCell<Vec<Event>>>,
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

    /// Dispatch an input to `key` and flush emitted events.
    fn input(&mut self, key: &ComponentKey, input: ControlInput, cx: &mut Context<Self>) {
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
    /// [`CommandReceipt`]s (`ok` → `Applied`, failure → `Rejected`,
    /// `data.revision` → receipt revision); `Notify` payloads decode
    /// as `{control_id, revision, value, provenance?}` →
    /// [`StateProjection`]. `Job`/`Lifecycle`/`Error` events have no
    /// view route — driver-level concerns only.
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
                    let status = if ok {
                        ReceiptStatus::Applied
                    } else {
                        ReceiptStatus::Rejected
                    };
                    let revision = data
                        .as_ref()
                        .and_then(|d| d.get("revision"))
                        .and_then(|v| serde_json::from_value::<Revision>(v.clone()).ok());
                    let detail = error.map(|e| e.message).or_else(|| {
                        data.as_ref()
                            .and_then(|d| d.get("detail"))
                            .and_then(serde_json::Value::as_str)
                            .map(str::to_string)
                    });
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
                        if bin.count == 0 || !bin.min.is_finite() || !bin.max.is_finite() {
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
            ComponentSpec::Fader(c) => div()
                .flex()
                .h_full()
                .child(self.control_element(c, false, cx))
                .into_any_element(),
            ComponentSpec::Seek(c) => div()
                .flex()
                .w_full()
                .child(self.control_element(c, true, cx))
                .into_any_element(),
            ComponentSpec::Waveform(w) => div()
                .flex_grow()
                .child(self.waveform_element(w))
                .into_any_element(),
            other => {
                let _ = other;
                div().child("unsupported component").into_any_element()
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
        match self.host.spec().map(|s| s.root.clone()) {
            Some(spec) => root.child(self.element_for(&spec, cx)),
            None => root,
        }
    }
}

fn find_control<'a>(spec: &'a ComponentSpec, key: &ComponentKey) -> Option<&'a ControlSpec> {
    match spec {
        ComponentSpec::Panel(p) => p.children.iter().find_map(|c| find_control(c, key)),
        ComponentSpec::Tabs(t) => t.tabs.iter().find_map(|tab| find_control(&tab.child, key)),
        ComponentSpec::Fader(c) | ComponentSpec::Seek(c) if c.key == *key => Some(c),
        _ => None,
    }
}

#[cfg(all(test, feature = "gpui-test"))]
mod tests {
    use super::*;
    use gpui::{Modifiers, MouseButton, TestAppContext, point, px};
    use ruviform_ui::behavior::{DetachPolicy, ValueDescriptor};
    use ruviform_ui::dock::ViewInstanceId;
    use ruviform_ui::view::{A11ySpec, ComponentVisualState, PanelSpec, ViewSpec};
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

    fn host_with_fader() -> ViewHost {
        let mut host = ViewHost::new(ViewInstanceId::new(1).expect("nonzero"));
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
}
