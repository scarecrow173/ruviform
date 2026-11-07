//! Consumer observation seam (Issue #429).
//!
//! Lets a consumer crate's test drive the GPUI driver and observe the
//! projection and intents it produces without naming a `gpui` type:
//! [`GpuiObserver`] owns the gpui test context internally, and every
//! public item here is built from `ruviform-ui` contract types.
//!
//! Test-only: gated by `gpui-test`, which enables gpui's test-support.
//! The platform loop is turned one step at a time, so real OS window
//! behaviour is not covered here. The crate-internal `*_for_test` hooks
//! at the bottom share the same turn/lookup implementation.
//!
//! **RT contract:** non-RT only.

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{App, Entity, TestAppContext};
use ruviform_ui::behavior::ControlInput;
use ruviform_ui::view::{ComponentKey, ComponentSnapshot};
use ruviform_ui::{DriverEvent, Intent, UiError, WindowDriver, WindowId, WindowLifecycle};

use super::{GpuiDriver, drain, flush_events};
use crate::render::RvfView;

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
    /// which may call back into the driver.
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
