//! GPUI [`SemanticGuiTest`] adapter (Issue #464) — `gpui-test` only.
//!
//! Wraps [`GpuiObserver`]: every operation maps onto the public
//! observation seam (`try_*` variants surface host [`ViewError`]s the
//! pointer path deliberately drops), target resolution reads the view
//! identity exposed by [`ViewObservation`], and `settle` is one
//! harness turn. `post_event` queues through the driver's normal
//! `post_event`/`inbox`/`deliver_event` path — the same decode the
//! fake adapter uses.
//!
//! The adapter claims `SemanticAction` + `MultiWindow` only. GPUI
//! `dispatch_input` bypasses hit-testing: it is semantic dispatch,
//! never simulated-pointer, real-OS, draw-plan, or pixel evidence.
//!
//! `gpui::TestAppContext`, `Entity`, `Window`, and `Context` stay
//! inside `GpuiObserver`; this module names only `ruviform-ui` /
//! `ruviform-ui-test` contract types. The adapter is `!Send` like the
//! observer it wraps — keep it on the test thread.
//!
//! **RT contract:** non-RT only.

use std::collections::BTreeSet;

use ruviform_ui::behavior::{CommandIntent, ControlInput};
use ruviform_ui::dock::ViewInstanceId;
use ruviform_ui::ipc::Event;
use ruviform_ui::view::{ComponentKey, ComponentSnapshot};
use ruviform_ui::{Intent, WindowDriver, WindowId};
use ruviform_ui_test::scenario::visual::{CaptureOutcome, VisualGuiTest};
use ruviform_ui_test::scenario::{
    MintedIntent, ScenarioCapability, ScenarioDriver, ScenarioError, ScenarioTarget,
    SemanticGuiTest, TargetQuery,
};

use super::observe::{GpuiObserver, ViewObservation};
use crate::COMMAND_INTENT_OP;

/// `GpuiObserver`-backed [`SemanticGuiTest`] — the simulated-GPUI
/// evidence level (`driver()` reports [`ScenarioDriver::GpuiSimulated`]).
///
/// Mount views through [`observer_mut`](Self::observer_mut) →
/// [`super::GpuiDriver::create_view_window`], settle one turn, then
/// resolve [`TargetQuery`]s like any other adapter.
pub struct GpuiGuiTest {
    observer: GpuiObserver,
    /// Cursor into `observer.intents()` — the drain position for
    /// [`drain_intents`](SemanticGuiTest::drain_intents).
    intent_cursor: usize,
}

impl Default for GpuiGuiTest {
    fn default() -> Self {
        Self::new()
    }
}

impl GpuiGuiTest {
    /// An adapter over a fresh observer and driver.
    #[must_use]
    pub fn new() -> Self {
        Self {
            observer: GpuiObserver::new(),
            intent_cursor: 0,
        }
    }

    /// The wrapped observer, for mounting views and test inspection.
    #[must_use]
    pub fn observer(&self) -> &GpuiObserver {
        &self.observer
    }

    /// The wrapped observer, for `driver_mut()` window creation.
    pub fn observer_mut(&mut self) -> &mut GpuiObserver {
        &mut self.observer
    }

    /// Detaches the view in `window` — bumps the host generation so
    /// previously resolved targets go stale (dock-transfer /
    /// window-hide simulation). Returns the new generation.
    ///
    /// # Errors
    /// [`ScenarioError::Driver`] when `window` hosts no view.
    pub fn detach_view(&mut self, window: WindowId, reason: &str) -> Result<u64, ScenarioError> {
        Ok(self.observer.detach_view(window, reason)?)
    }

    /// The live [`ViewObservation`] for `window`/`view`, or the honest
    /// missing/stale-free error.
    fn observation(
        &self,
        window: WindowId,
        view: ViewInstanceId,
    ) -> Result<ViewObservation, ScenarioError> {
        if !WindowDriver::windows(self.observer.driver()).contains(&window) {
            return Err(ScenarioError::NoSuchWindow { window });
        }
        let obs = self
            .observer
            .observe(window)
            .ok_or(ScenarioError::NoSuchView { window, view })?;
        if obs.view != view {
            return Err(ScenarioError::NoSuchView { window, view });
        }
        Ok(obs)
    }

    /// Re-checks a resolved target's pins — the stale gate every
    /// operation passes through. Returns the fresh observation so
    /// callers can check component mounting without a second read.
    fn check_target(&self, target: &ScenarioTarget) -> Result<ViewObservation, ScenarioError> {
        let obs = self.observation(target.window, target.view)?;
        if obs.attach_epoch != target.attach_epoch {
            return Err(ScenarioError::StaleEpoch {
                expected: target.attach_epoch,
                got: obs.attach_epoch,
            });
        }
        if obs.generation != target.generation {
            return Err(ScenarioError::StaleGeneration {
                expected: target.generation,
                got: obs.generation,
            });
        }
        Ok(obs)
    }

    /// The component observation for `key`, or
    /// [`ScenarioError::UnknownComponent`].
    fn mounted<'a>(
        obs: &'a ViewObservation,
        key: &ComponentKey,
    ) -> Result<&'a super::observe::ComponentObservation, ScenarioError> {
        obs.components
            .iter()
            .find(|c| &c.key == key)
            .ok_or_else(|| ScenarioError::UnknownComponent { key: key.clone() })
    }
}

impl VisualGuiTest for GpuiGuiTest {
    /// The `gpui-test` platform has no pixel readback — `GpuiObserver`
    /// exposes draw plans and projections, never a framebuffer.
    /// `GpuiObserver::drawn()` is a semantic render plan, not a
    /// screenshot, so it must not be returned as `Captured`. Pins are
    /// still re-checked first. Slice 3 adds the real capture path;
    /// until then the honest outcome is `Unsupported` — and
    /// `capabilities()` keeps not claiming `VisualCapture`.
    fn capture_frame(&mut self, target: &ScenarioTarget) -> Result<CaptureOutcome, ScenarioError> {
        self.check_target(target)?;
        Ok(CaptureOutcome::Unsupported {
            reason: "gpui_test_platform_has_no_pixel_readback".to_string(),
        })
    }
}

/// Decodes one recorded [`Intent`] into a [`MintedIntent`] — the
/// `rvf.ui.command.v1` params carry `command_id`/`expect_receipt`/
/// `intent` (see `render::flush_events`).
fn minted_from_ipc(intent: &Intent) -> Option<MintedIntent> {
    if intent.op != COMMAND_INTENT_OP {
        return None;
    }
    let params = &intent.params;
    let command_id = params.get("command_id")?.as_str()?.to_string();
    let expect_receipt = params
        .get("expect_receipt")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let intent = serde_json::from_value::<CommandIntent>(params.get("intent")?.clone()).ok()?;
    Some(MintedIntent {
        command_id,
        expect_receipt,
        intent,
    })
}

impl SemanticGuiTest for GpuiGuiTest {
    fn driver(&self) -> ScenarioDriver {
        ScenarioDriver::GpuiSimulated
    }

    fn capabilities(&self) -> BTreeSet<ScenarioCapability> {
        BTreeSet::from([
            ScenarioCapability::SemanticAction,
            ScenarioCapability::MultiWindow,
        ])
    }

    fn windows(&self) -> Vec<WindowId> {
        WindowDriver::windows(self.observer.driver())
    }

    fn resolve_target(&mut self, query: &TargetQuery) -> Result<ScenarioTarget, ScenarioError> {
        let obs = self.observation(query.window, query.view)?;
        if let Some(expected) = query.attach_epoch
            && obs.attach_epoch != expected
        {
            return Err(ScenarioError::StaleEpoch {
                expected,
                got: obs.attach_epoch,
            });
        }
        if let Some(expected) = query.generation
            && obs.generation != expected
        {
            return Err(ScenarioError::StaleGeneration {
                expected,
                got: obs.generation,
            });
        }
        if let Some(key) = &query.component {
            Self::mounted(&obs, key)?;
        }
        Ok(ScenarioTarget {
            window: query.window,
            view: query.view,
            attach_epoch: obs.attach_epoch,
            generation: obs.generation,
            component: query.component.clone(),
        })
    }

    fn dispatch_input(
        &mut self,
        target: &ScenarioTarget,
        key: &ComponentKey,
        input: ControlInput,
    ) -> Result<(), ScenarioError> {
        let obs = self.check_target(target)?;
        Self::mounted(&obs, key)?;
        self.observer
            .try_dispatch_input(target.window, key, input)?;
        Ok(())
    }

    fn select_row(
        &mut self,
        target: &ScenarioTarget,
        key: &ComponentKey,
        row: usize,
    ) -> Result<(), ScenarioError> {
        let obs = self.check_target(target)?;
        Self::mounted(&obs, key)?;
        self.observer.try_select_row(target.window, key, row)?;
        Ok(())
    }

    fn select_tab(
        &mut self,
        target: &ScenarioTarget,
        key: &ComponentKey,
        index: usize,
    ) -> Result<(), ScenarioError> {
        let obs = self.check_target(target)?;
        Self::mounted(&obs, key)?;
        self.observer.try_select_tab(target.window, key, index)?;
        Ok(())
    }

    fn invoke(&mut self, target: &ScenarioTarget, key: &ComponentKey) -> Result<(), ScenarioError> {
        let obs = self.check_target(target)?;
        Self::mounted(&obs, key)?;
        self.observer.try_invoke(target.window, key)?;
        Ok(())
    }

    fn observe(
        &self,
        target: &ScenarioTarget,
        key: &ComponentKey,
    ) -> Result<ComponentSnapshot, ScenarioError> {
        let obs = self.check_target(target)?;
        Ok(Self::mounted(&obs, key)?.snapshot.clone())
    }

    fn settle(&mut self) -> Result<(), ScenarioError> {
        // One harness turn: queued window ops apply, deferred work
        // parks, emitted events flush — intents land in
        // `observer.intents()` for `drain_intents`.
        self.observer.settle(|_, _| {});
        Ok(())
    }

    fn drain_intents(&mut self) -> Vec<MintedIntent> {
        let all = self.observer.intents();
        let mut out = Vec::new();
        for intent in all.iter().skip(self.intent_cursor) {
            if let Some(minted) = minted_from_ipc(intent) {
                out.push(minted);
            }
        }
        self.intent_cursor = all.len();
        out
    }

    fn post_event(&mut self, target: &ScenarioTarget, event: &Event) -> Result<(), ScenarioError> {
        self.check_target(target)?;
        // The driver's `post_event` queues `Op::PostEvent`; `settle`
        // (or the next render) delivers it into the window inbox where
        // `drain_inbox` runs `ViewHost::deliver_event` — the same
        // decode the fake adapter applies synchronously.
        WindowDriver::post_event(self.observer.driver_mut(), target.window, event);
        Ok(())
    }
}
