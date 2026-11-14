//! Plugin-dock scenario declaration + plugin artifact evidence
//! (Issue #631 first slice — RF-VTEST-04).
//!
//! Driver-neutral pieces every plugin-dock consumer shares:
//!
//! - [`plugin_dock_view_spec`] — a [`ViewSpec`] whose tree reserves a
//!   [`NativeSlotSpec`] region for the plugin editor surface. The slot
//!   is declarative only: drivers bind `key → SurfaceSlotId` after
//!   window creation (`GpuiDriver::bind_native_slot` precedent).
//! - [`plugin_dock_spec`] — the [`ScenarioSpec`] for dock cases:
//!   `required = {NativePluginDock, VisualCapture}` — the two evidence
//!   levels a real docked editor with pixel verification needs. An
//!   adapter that cannot provide them yields `Unsupported` at
//!   pre-flight, never a partially executed scenario.
//! - [`PluginArtifactEvidence`] — the plugin binary identity a case
//!   ran against (path, SHA-256 digest, descriptor id/name/vendor/
//!   version), serialized into `ScenarioSpec.environment` /
//!   `ScenarioReport.environment` and referenced from
//!   `ScenarioReport.evidence`.
//!
//! Nothing here claims a Dock PASS: these are declarations and
//! evidence vocabulary. Verdict honesty lives in the runner
//! (`run_scenario`) and in each adapter's capability report.
//!
//! **RT contract:** non-RT only.

use std::collections::BTreeSet;

use ruviform_ui::view::{
    A11ySpec, ComponentKey, ComponentSpec, ComponentVisualState, NativeSlotSpec, PanelSpec,
    ViewSpec,
};
use serde::Serialize;
use serde_json::{Value, json};

use super::runner::ScenarioSpec;
use super::{CaseId, RunId, ScenarioCapability, ScenarioId};

/// Scenario id shared by every plugin-dock case.
pub const PLUGIN_DOCK_SCENARIO: &str = "rf631.plugin-dock";

/// [`ComponentKey`] of the native-slot region inside
/// [`plugin_dock_view_spec`] — also the `bind_native_slot` lookup key
/// on drivers that support it.
pub const PLUGIN_DOCK_SLOT_KEY: &str = "plugin.editor";

/// Identity of the plugin binary artifact a scenario ran against.
///
/// Recorded into scenario `environment`/`evidence` so a `Pass` is
/// attributable to exact bytes — a rebuilt fixture with a different
/// digest is a different evidence subject.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PluginArtifactEvidence {
    /// Path of the plugin binary that was loaded.
    pub path: String,
    /// Lowercase hex SHA-256 of the artifact bytes.
    pub sha256: String,
    /// Descriptor id the module reported through the host scan.
    pub plugin_id: String,
    /// Human-readable plugin name.
    pub name: String,
    /// Vendor string the module reported.
    pub vendor: String,
    /// Plugin-reported version string.
    pub version: String,
}

impl PluginArtifactEvidence {
    /// The `environment` JSON fragment recorded on specs/reports.
    #[must_use]
    pub fn environment_value(&self) -> Value {
        json!({ "plugin_artifact": self })
    }

    /// A compact evidence reference for `ScenarioReport.evidence`.
    #[must_use]
    pub fn evidence_ref(&self) -> String {
        let short = self.sha256.get(..12).unwrap_or(&self.sha256);
        format!("plugin-artifact:{}@{short}", self.path)
    }
}

/// The plugin-dock view: a panel reserving one [`NativeSlotSpec`]
/// region for the plugin editor surface.
///
/// The spec is declarative — mounting it proves layout/semantics
/// only. Binding the slot to a real `SurfaceSlotId` and attaching a
/// plugin editor are driver/consumer responsibilities outside the
/// view contract.
#[must_use]
pub fn plugin_dock_view_spec() -> ViewSpec {
    ViewSpec {
        root: ComponentSpec::Panel(PanelSpec {
            key: ComponentKey("plugin.dock".to_string()),
            title: Some("Plugin Dock".to_string()),
            children: vec![ComponentSpec::NativeSlot(NativeSlotSpec {
                key: ComponentKey(PLUGIN_DOCK_SLOT_KEY.to_string()),
                min_size: Some([64.0, 64.0]),
                visual: ComponentVisualState::default(),
                a11y: A11ySpec::default(),
            })],
            visual: ComponentVisualState::default(),
            a11y: A11ySpec::default(),
        }),
    }
}

/// The plugin-dock [`ScenarioSpec`] for `case` under `run_id`.
///
/// Requires `NativePluginDock` + `VisualCapture` — the evidence
/// levels a real docked editor with pixel checks needs. When
/// `artifact` is known its identity is recorded under
/// `environment.plugin_artifact`; callers merge additional
/// environment facts (OS, scale, repo SHA, `environment_kind`)
/// into `spec.environment` themselves.
#[must_use]
pub fn plugin_dock_spec(
    run_id: RunId,
    case: CaseId,
    artifact: Option<&PluginArtifactEvidence>,
) -> ScenarioSpec {
    let mut spec = ScenarioSpec::new(run_id, ScenarioId::new(PLUGIN_DOCK_SCENARIO))
        .case(case)
        .required(BTreeSet::from([
            ScenarioCapability::NativePluginDock,
            ScenarioCapability::VisualCapture,
        ]));
    if let Some(artifact) = artifact {
        spec.environment = Some(artifact.environment_value());
    }
    spec
}
