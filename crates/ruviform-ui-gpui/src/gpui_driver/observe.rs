//! Consumer observation seam (Issue #429, drawing plans Issue #436).
//!
//! Lets a consumer crate's test drive the GPUI driver and observe the
//! projection, intents, and drawing plans it produces without naming a
//! `gpui` type: [`GpuiObserver`] owns the gpui test context internally,
//! and every public item here is built from `ruviform-ui` contract types
//! or std types.
//!
//! Test-only: gated by `gpui-test`, which enables gpui's test-support.
//! The platform loop is turned one step at a time, so real OS window
//! behaviour is not covered here. The crate-internal `*_for_test` hooks
//! at the bottom share the same turn/lookup implementation.
//!
//! **RT contract:** non-RT only.

use std::cell::RefCell;
use std::ops::Range;
use std::rc::Rc;

use gpui::{App, Entity, TestAppContext};
use ruviform_ui::behavior::ControlInput;
use ruviform_ui::view::{ComponentKey, ComponentSnapshot};
use ruviform_ui::{DriverEvent, Intent, UiError, WindowDriver, WindowId, WindowLifecycle};

use super::{GpuiDriver, drain, flush_events};
use crate::render::{self, DrawPlan, RvfView};

/// Observed state of one mounted component.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ComponentObservation {
    /// Key of the component in the mounted view.
    pub key: ComponentKey,
    /// Display, committed, and observed-projection state.
    pub snapshot: ComponentSnapshot,
}

/// Observed state of the view mounted in one window.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ViewObservation {
    /// Window hosting the view.
    pub window: WindowId,
    /// Host generation — advances on mount, accepted update, and detach.
    pub generation: u64,
    /// Mounted components, sorted by key.
    pub components: Vec<ComponentObservation>,
    /// Events queued in the window inbox and not yet consumed by the view.
    pub inbox_len: usize,
}

/// Drawing plan of one drawing component: the values the renderer paints,
/// with GPUI types left out (Issues #436, #442).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum DrawnPlan {
    /// Meter bars in paint order.
    Meter {
        /// Bars of every channel, each a span of the viewport.
        bars: Vec<DrawnBar>,
    },
    /// `TimeAxis` marks inside the viewport, in tick order.
    TimeAxis {
        /// Marks the renderer draws; ticks outside the viewport are culled.
        marks: Vec<DrawnMark>,
    },
    /// Waveform bins that paint.
    Waveform {
        /// One entry per channel, including channels with no painted bin.
        channels: Vec<DrawnWaveformChannel>,
    },
    /// Spectrogram cells that paint. Gap columns and non-finite magnitudes
    /// draw nothing and are absent.
    Spectrogram {
        /// Cells in paint order: by column, then by row.
        cells: Vec<DrawnCell>,
    },
    /// Label text as painted.
    Label {
        /// Display text.
        text: String,
    },
    /// Table grid as painted: the header and the data rows.
    Table {
        /// Column titles in declared order.
        header: Vec<String>,
        /// Data rows in declared order. `None` is a missing cell: it draws
        /// empty and keeps its column position.
        rows: Vec<Vec<Option<String>>>,
    },
    /// Button label as painted.
    Button {
        /// Display text.
        label: String,
    },
}

/// One drawn Meter bar. `x0` and `x1` are fractions of the viewport
/// width in `[0, 1]`; `peak` and `rms` are amplitudes clamped to `[0, 1]`.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct DrawnBar {
    /// Channel band the bar is drawn in.
    pub channel: usize,
    /// Left edge, as a fraction of the viewport width.
    pub x0: f64,
    /// Right edge, as a fraction of the viewport width.
    pub x1: f64,
    /// Peak height, as a fraction of the channel band.
    pub peak: f64,
    /// RMS overlay height; `None` when the bin's RMS is not finite.
    pub rms: Option<f64>,
}

/// One drawn `TimeAxis` mark.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct DrawnMark {
    /// Position, as a fraction of the viewport width in `[0, 1]`.
    pub x: f64,
    /// Whether the mark is a major tick (full height).
    pub major: bool,
    /// Label drawn above the mark.
    pub label: String,
}

/// One drawn Spectrogram cell. `x0`/`x1` are fractions of the viewport
/// width and `y0`/`y1` are fractions of the spectrogram height, both in
/// `[0, 1]`; `y` is measured top-down, so row 0 (lowest frequency) sits at
/// the bottom. `intensity` is clamped to `[0, 1]`.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct DrawnCell {
    /// Left edge, as a fraction of the viewport width.
    pub x0: f64,
    /// Right edge, as a fraction of the viewport width.
    pub x1: f64,
    /// Top edge, as a fraction of the spectrogram height.
    pub y0: f64,
    /// Bottom edge, as a fraction of the spectrogram height.
    pub y1: f64,
    /// Intensity in `[0, 1]`, the alpha of the cell's fill.
    pub intensity: f64,
}

/// The painted Waveform bins of one channel.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct DrawnWaveformChannel {
    /// Channel index.
    pub channel: usize,
    /// Frame range (`start..end`) of each painted bin, in bin order.
    /// Gap bins are absent.
    pub bins: Vec<Range<u64>>,
}

/// Drawing plans of the components a window's view paints.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ViewDrawn {
    /// Window hosting the view.
    pub window: WindowId,
    /// Drawing plans in paint order. Only drawing components appear (Meter,
    /// `TimeAxis`, Waveform, Spectrogram, Label, Table); a Tabs strip
    /// contributes its active child only.
    pub components: Vec<ComponentDrawn>,
}

/// Drawing plan of one component.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ComponentDrawn {
    /// Key of the component in the mounted view.
    pub key: ComponentKey,
    /// What the renderer paints for it.
    pub plan: DrawnPlan,
}

/// Consumer-side handle onto a [`GpuiDriver`] for test observation.
///
/// Each [`settle`](Self::settle) is one harness turn: queued window
/// ops are applied, deferred work parks (the first-frame `Ready`), and
/// the emitted [`DriverEvent`]s reach the consumer's handler. `Action`
/// intents are also recorded and readable through [`intents`](Self::intents).
pub struct GpuiObserver {
    cx: TestAppContext,
    driver: GpuiDriver,
    intents: Rc<RefCell<Vec<Intent>>>,
}

impl Default for GpuiObserver {
    fn default() -> Self {
        Self::new()
    }
}

impl GpuiObserver {
    /// Creates an observer over a fresh driver and test context.
    #[must_use]
    pub fn new() -> Self {
        Self {
            cx: TestAppContext::single(),
            driver: GpuiDriver::new(),
            intents: Rc::default(),
        }
    }

    /// The driver, for read-only queries such as [`GpuiDriver::exited`].
    #[must_use]
    pub fn driver(&self) -> &GpuiDriver {
        &self.driver
    }

    /// The driver, for `WindowDriver` calls and
    /// [`GpuiDriver::create_view_window`].
    pub fn driver_mut(&mut self) -> &mut GpuiDriver {
        &mut self.driver
    }

    /// One harness turn. Delivers the emitted events to `on_event`,
    /// which may call back into the driver. The handler stays installed
    /// until the next `settle`: events flushed by later observer calls
    /// (for example [`dispatch_input`](Self::dispatch_input)) reach the
    /// most recent handler, and intents are recorded either way.
    pub fn settle(
        &mut self,
        mut on_event: impl FnMut(DriverEvent, &mut dyn WindowDriver) + 'static,
    ) {
        let intents = self.intents.clone();
        self.driver.settle_turn(&mut self.cx, move |event, drv| {
            if let DriverEvent::Windowed {
                event: WindowLifecycle::Action(intent),
                ..
            } = &event
            {
                intents.borrow_mut().push(intent.clone());
            }
            on_event(event, drv);
        });
    }

    /// Dispatches a control input to `key` in `window`, as the pointer
    /// path does. Inputs the host rejects are dropped; their effect shows
    /// in [`observe`](Self::observe) and [`intents`](Self::intents) after
    /// the next [`settle`](Self::settle).
    ///
    /// # Errors
    /// [`UiError::Host`] when `window` hosts no view.
    pub fn dispatch_input(
        &self,
        window: WindowId,
        key: &ComponentKey,
        input: ControlInput,
    ) -> Result<(), UiError> {
        let view = self.view(window)?;
        self.cx
            .update(|app| view.update(app, |v, cx| v.input(key, input, cx)));
        Ok(())
    }

    /// Selects `row` of the table `key` in `window`, as a row press does.
    /// Like [`dispatch_input`](Self::dispatch_input), the emitted intent
    /// shows in [`intents`](Self::intents) after [`settle`](Self::settle).
    ///
    /// # Errors
    /// [`UiError::Host`] when `window` hosts no view.
    pub fn select_row(
        &self,
        window: WindowId,
        key: &ComponentKey,
        row: usize,
    ) -> Result<(), UiError> {
        let view = self.view(window)?;
        self.cx
            .update(|app| view.update(app, |v, cx| v.select_row(key, row, cx)));
        Ok(())
    }

    /// Presses the button `key` in `window`, as a click does. The emitted
    /// intent shows in [`intents`](Self::intents) after
    /// [`settle`](Self::settle).
    ///
    /// # Errors
    /// [`UiError::Host`] when `window` hosts no view.
    pub fn invoke(&self, window: WindowId, key: &ComponentKey) -> Result<(), UiError> {
        let view = self.view(window)?;
        self.cx
            .update(|app| view.update(app, |v, cx| v.invoke(key, cx)));
        Ok(())
    }

    /// Projection and window state of the view mounted in `window`;
    /// `None` when the window hosts no view.
    #[must_use]
    pub fn observe(&self, window: WindowId) -> Option<ViewObservation> {
        let view = self.view(window).ok()?;
        let (generation, components) = self.cx.update(|app| {
            let host = view.read(app).host();
            let components: Vec<ComponentObservation> = host
                .mounted_keys()
                .into_iter()
                .filter_map(|key| {
                    let snapshot = host.observe(&key)?;
                    Some(ComponentObservation { key, snapshot })
                })
                .collect();
            (host.generation(), components)
        });
        Some(ViewObservation {
            window,
            generation,
            components,
            inbox_len: self.driver.inbox_len(window),
        })
    }

    /// Drawing plans of the drawing components the view mounted in `window`
    /// paints; `None` when the window hosts no view. Components are in paint
    /// order, which differs from the key order of [`observe`](Self::observe).
    #[must_use]
    pub fn drawn(&self, window: WindowId) -> Option<ViewDrawn> {
        let view = self.view(window).ok()?;
        let plans = self.cx.update(|app| {
            let spec = view.read(app).host().spec()?;
            Some(render::drawn_plans(&spec.root))
        })?;
        Some(ViewDrawn {
            window,
            components: plans
                .into_iter()
                .map(|(key, plan)| ComponentDrawn {
                    key,
                    plan: to_drawn_plan(plan),
                })
                .collect(),
        })
    }

    /// `Action` intents delivered by [`settle`](Self::settle) so far,
    /// in delivery order.
    #[must_use]
    pub fn intents(&self) -> Vec<Intent> {
        self.intents.borrow().clone()
    }

    /// Events queued in `window`'s inbox and not yet consumed by its root
    /// view; `0` for unknown windows.
    #[must_use]
    pub fn inbox_len(&self, window: WindowId) -> usize {
        self.driver.inbox_len(window)
    }

    fn view(&self, window: WindowId) -> Result<Entity<RvfView>, UiError> {
        self.cx
            .update(|app| self.driver.view_of(app, window))
            .ok_or_else(|| UiError::host(format!("no view mounted in window {window}")))
    }
}

impl GpuiDriver {
    /// One harness turn over `cx`: applies queued window ops inside an App
    /// update, lets deferred work park (the first-frame `Ready`), applies
    /// follow-up ops, then delivers the emitted events to `on_event`.
    pub(crate) fn settle_turn(
        &self,
        cx: &mut TestAppContext,
        on_event: impl FnMut(DriverEvent, &mut dyn WindowDriver) + 'static,
    ) {
        let this = self.clone();
        this.inner.borrow_mut().handler = Some(Box::new(on_event));
        cx.update(|app| drain(&this.inner, &this, app));
        cx.run_until_parked();
        cx.update(|app| drain(&this.inner, &this, app));
        flush_events(&this.inner, &this);
    }

    /// Events queued in `window`'s inbox and not yet consumed by its root
    /// view; `0` for unknown windows.
    pub(crate) fn inbox_len(&self, window: WindowId) -> usize {
        self.inner
            .borrow()
            .windows
            .get(&window)
            .map_or(0, |slot| slot.inbox.borrow().len())
    }

    /// The view mounted by [`GpuiDriver::create_view_window`], looked up
    /// through the window's root; `None` for fixture windows.
    pub(crate) fn view_of(&self, app: &mut App, window: WindowId) -> Option<Entity<RvfView>> {
        let root = self.inner.borrow().windows.get(&window)?.root.clone()?;
        root.update(app, |root, _cx| {
            root.content.clone().downcast::<RvfView>().ok()
        })
    }
}

/// Widens a crate-internal plan to the public DTO. The `f32` values are
/// exact in `f64`, so the observed numbers equal the painted ones.
fn to_drawn_plan(plan: DrawPlan) -> DrawnPlan {
    match plan {
        DrawPlan::Meter(bars) => DrawnPlan::Meter {
            bars: bars
                .into_iter()
                .map(|b| DrawnBar {
                    channel: b.channel,
                    x0: f64::from(b.x0),
                    x1: f64::from(b.x1),
                    peak: f64::from(b.peak),
                    rms: b.rms.map(f64::from),
                })
                .collect(),
        },
        DrawPlan::TimeAxis(marks) => DrawnPlan::TimeAxis {
            marks: marks
                .into_iter()
                .map(|m| DrawnMark {
                    x: f64::from(m.x),
                    major: m.major,
                    label: m.label,
                })
                .collect(),
        },
        DrawPlan::Waveform(channels) => DrawnPlan::Waveform {
            channels: channels
                .into_iter()
                .enumerate()
                .map(|(channel, bins)| DrawnWaveformChannel { channel, bins })
                .collect(),
        },
        DrawPlan::Spectrogram(cells) => DrawnPlan::Spectrogram {
            cells: cells
                .into_iter()
                .map(|c| DrawnCell {
                    x0: f64::from(c.x0),
                    x1: f64::from(c.x1),
                    y0: f64::from(c.y0),
                    y1: f64::from(c.y1),
                    intensity: f64::from(c.intensity),
                })
                .collect(),
        },
        DrawPlan::Label(text) => DrawnPlan::Label { text },
        DrawPlan::Table(table) => DrawnPlan::Table {
            header: table.header,
            rows: table.rows,
        },
        DrawPlan::Button(label) => DrawnPlan::Button { label },
    }
}

/// Crate-internal hooks for the gpui-test fixtures in `render.rs` and
/// `lifecycle_fixture.rs`. They are thin views onto the same turn and
/// lookup used by [`GpuiObserver`].
#[cfg(all(test, feature = "gpui-test"))]
impl GpuiDriver {
    pub(crate) fn settle_for_test(
        &mut self,
        cx: &mut TestAppContext,
        on_event: impl FnMut(DriverEvent, &mut dyn WindowDriver) + 'static,
    ) {
        self.settle_turn(cx, on_event);
    }

    pub(crate) fn inbox_len_for_test(&self, window: WindowId) -> usize {
        self.inbox_len(window)
    }

    pub(crate) fn view_for_test(
        &self,
        cx: &mut TestAppContext,
        window: WindowId,
    ) -> Option<Entity<RvfView>> {
        cx.update(|app| self.view_of(app, window))
    }
}
