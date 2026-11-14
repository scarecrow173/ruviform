//! `PluginHostProbe` — the [`ScenarioDriver::PluginHostProbe`] adapter.
//!
//! This is *not* a GUI adapter: it owns no windows, mounts no views,
//! and performs no semantic input. It exists so host-side cases
//! (artifact load, VST3 lifecycle, editor-attach attempt) flow through
//! the same `run_scenario` → `ScenarioReport` assembly as GUI cases,
//! with `driver` honestly naming the evidence level: real plugin-ABI
//! execution through the production in-process host — never real-OS
//! window, input, or pixel proof.
//!
//! [`capabilities`](SemanticGuiTest::capabilities) is therefore the
//! empty set: the probe provides no `SemanticGuiTest` operation at
//! all. Every GUI-facing method is a defined refusal.
//!
//! **RT contract:** non-RT only.

use std::collections::BTreeSet;

use ruviform_ui::WindowId;
use ruviform_ui::behavior::ControlInput;
use ruviform_ui::ipc::Event;
use ruviform_ui::view::{ComponentKey, ComponentSnapshot};
use ruviform_ui_test::SemanticGuiTest;
use ruviform_ui_test::scenario::{
    MintedIntent, ScenarioCapability, ScenarioDriver, ScenarioError, ScenarioTarget, TargetQuery,
};

/// The in-process plugin-host probe adapter (Issue #631 plugin-dock
/// lane). See the module docs — ABI/host evidence only.
#[derive(Debug, Default)]
pub struct PluginHostProbe;

impl PluginHostProbe {
    /// Creates the probe. It holds no state — all evidence lives in
    /// the case bodies and the returned [`ScenarioReport`]s.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

/// The defined refusal every GUI-facing probe method returns.
fn no_gui_surface() -> ScenarioError {
    ScenarioError::Driver(
        "plugin-host probe owns no GUI surface; it drives the plugin \
         host command path only"
            .to_string(),
    )
}

impl SemanticGuiTest for PluginHostProbe {
    fn driver(&self) -> ScenarioDriver {
        ScenarioDriver::PluginHostProbe
    }

    /// Empty — the probe claims no scenario capability. Host-side
    /// cases declare `required = {}` and gate on artifact presence
    /// inside the body instead; a real dock driver would declare
    /// `NativePluginDock`/`VisualCapture` and get `Unsupported` at
    /// pre-flight here, which is the honest outcome for this adapter.
    fn capabilities(&self) -> BTreeSet<ScenarioCapability> {
        BTreeSet::new()
    }

    fn windows(&self) -> Vec<WindowId> {
        Vec::new()
    }

    fn resolve_target(&mut self, _query: &TargetQuery) -> Result<ScenarioTarget, ScenarioError> {
        Err(no_gui_surface())
    }

    fn dispatch_input(
        &mut self,
        _target: &ScenarioTarget,
        _key: &ComponentKey,
        _input: ControlInput,
    ) -> Result<(), ScenarioError> {
        Err(no_gui_surface())
    }

    fn select_row(
        &mut self,
        _target: &ScenarioTarget,
        _key: &ComponentKey,
        _row: usize,
    ) -> Result<(), ScenarioError> {
        Err(no_gui_surface())
    }

    fn select_tab(
        &mut self,
        _target: &ScenarioTarget,
        _key: &ComponentKey,
        _index: usize,
    ) -> Result<(), ScenarioError> {
        Err(no_gui_surface())
    }

    fn invoke(
        &mut self,
        _target: &ScenarioTarget,
        _key: &ComponentKey,
    ) -> Result<(), ScenarioError> {
        Err(no_gui_surface())
    }

    fn observe(
        &self,
        _target: &ScenarioTarget,
        _key: &ComponentKey,
    ) -> Result<ComponentSnapshot, ScenarioError> {
        Err(no_gui_surface())
    }

    /// No queued work exists — settle is a defined no-op.
    fn settle(&mut self) -> Result<(), ScenarioError> {
        Ok(())
    }

    fn drain_intents(&mut self) -> Vec<MintedIntent> {
        Vec::new()
    }

    fn post_event(
        &mut self,
        _target: &ScenarioTarget,
        _event: &Event,
    ) -> Result<(), ScenarioError> {
        Err(no_gui_surface())
    }
}
