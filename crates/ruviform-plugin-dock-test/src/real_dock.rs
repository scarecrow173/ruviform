//! Real-window plugin-dock lane (Issue #631 real-window slice —
//! feature `real-dock`).
//!
//! [`run_all`] mounts [`plugin_dock_view_spec`] into a *real OS
//! window* through [`RealOsGuiTest`] and records the four cases of
//! the `rf631.plugin-dock-real` scenario:
//!
//! | case                  | driver      | honest outcome today                                  |
//! |-----------------------|-------------|--------------------------------------------------------|
//! | `real-window-mount`   | `RealOs`    | `Pass` — spec mounts; `NativeSlot` resolves/observes   |
//! | `real-native-slot`    | `RealOs`    | `Pass`/`Fail` — declare→bind→project→`lease_parent`→   |
//! |                       |             | validate→`container_token`→release + stale/forged      |
//! |                       |             | lease rejection, all real ops                          |
//! | `real-editor-attach`  | `RealOs`    | `Unsupported` — `createView` null behind a REAL        |
//! |                       |             | container-HWND lease (never `Pass` without an editor)  |
//! | `real-capture`        | `RealOs`    | `Pass`/`Unsupported`/`NotRun` — window/slot-region     |
//! |                       |             | pixels, explicitly NOT plugin editor pixels            |
//!
//! The chain under test is the production one: `NativeSlot` →
//! `GpuiSurfaceAdapter` (real container HWND on Windows) →
//! `lease_parent` → `ParentSlotLease` → `InProcessEditorOps`
//! redeem → `session.open(Docked)`. Every step verdict and the
//! observed `container_token` land in the case's `observed`/`evidence`
//! — a `Pass` is only claimed for steps that really executed.
//!
//! Environment honesty: a headless/non-interactive host cannot open
//! the window — [`RealOsGuiTest::run`] fails and every case reports
//! `NotRun` with `observed.environment_blocked` (fail-closed, the
//! `test-ui-real-scenario` precedent). `EnvironmentKind::RealWindow`
//! is claimed only by reports produced inside the real session.
//!
//! **RT contract:** non-RT only — real OS window, dlopen,
//! filesystem. **Test-only:** nothing here ships.

use ruviform_audio_test::EnvironmentKind;
use ruviform_plugin_host::{
    AttachMode, EditorCapabilities, EditorOpError, EditorSessionError, HostCommand,
    InProcessEditorOps, InProcessHost, ParentLease, ParentSlotLease, PluginEditorSessions,
    PluginHandle,
};
use ruviform_plugin_vst3::host::Vst3Library;
use ruviform_ui::dock::ViewInstanceId;
use ruviform_ui::native_slot::{
    LogicalRect, SlotCapabilities, SlotGeometry, SlotLease, SlotSpec, SlotVisibility, Support,
    SurfaceSlotId,
};
use ruviform_ui::view::{AttachEpochAllocator, ComponentKey, ViewHost};
use ruviform_ui::{WindowId, WindowSpec};
use ruviform_ui_gpui::{RealOsGuiTest, RealWindowMount, RealWindowOptions, ViewMount};
use ruviform_ui_test::scenario::plugin_dock::{
    PLUGIN_DOCK_SLOT_KEY, PluginArtifactEvidence, plugin_dock_view_spec,
};
use ruviform_ui_test::scenario::visual::{CaptureOutcome, VisualGuiTest};
use ruviform_ui_test::{
    CaseId, RunId, ScenarioCapability, ScenarioDriver, ScenarioEntry, ScenarioError, ScenarioId,
    ScenarioOutcome, ScenarioReport, ScenarioSpec, ScenarioSpecExtras, ScenarioVerdict,
    SemanticGuiTest, TargetQuery, run_entries, run_scenario, settle_until,
};
use serde_json::{Map, Value, json};

use crate::LaneEnvironment;
use crate::artifact;
use crate::scenario::{
    activate_test_gain, editor_size, fail, finalize, host_error_outcome, not_run,
    register_factories, rejecting_editor_ops, session_failed_unsupported, session_id_json,
};

/// The run id shared by every case of this lane.
pub const RUN_ID: &str = "rf631-plugin-dock-real";

/// Scenario id — distinct from the headless probe lane's
/// `rf631.plugin-dock` so evidence never conflates the two drivers.
const SCENARIO: &str = "rf631.plugin-dock-real";

/// The resolved plugin binary + its evidence identity — only
/// `real-editor-attach` consumes it.
type PluginPair = (Vst3Library, PluginArtifactEvidence);

/// Why the plugin binary could not be loaded — `prerequisite`
/// distinguishes an absent artifact (`NotRun`, fail-closed) from a
/// present-but-broken one (`Fail`).
#[derive(Clone)]
struct PluginUnavailable {
    /// Human-readable resolution/load failure.
    detail: String,
    /// `true` when the artifact simply is not there (resolution
    /// failed); `false` when it exists but hash/load/discovery
    /// failed — a real defect, not a missing prerequisite.
    prerequisite: bool,
}

/// Session-scoped lane context for the real-window catalog — the
/// live `RealOsGuiTest`, the mounted window/view, the lane
/// environment, and the plugin pair/error/evidence resolved up
/// front. Only `real-editor-attach` consumes the plugin fields; the
/// window-only cases ignore them.
struct RealDockContext<'a> {
    /// The live session driver — `run_scenario`'s `G`.
    gui: &'a mut RealOsGuiTest,
    /// The mounted window id.
    window: WindowId,
    /// The mounted view instance id.
    view: ViewInstanceId,
    /// Lane environment snapshot (borrowed — the session is
    /// short-lived).
    env: &'a LaneEnvironment,
    /// The resolved `(library, evidence)` pair, when the artifact
    /// loaded.
    plugin: Option<&'a PluginPair>,
    /// Why the plugin is unavailable, when it is.
    plugin_error: Option<&'a PluginUnavailable>,
    /// The artifact evidence identity, when the artifact loaded.
    artifact: Option<&'a PluginArtifactEvidence>,
}

/// Runs the four real-window cases inside one `RealOsGuiTest`
/// session and returns their reports in case order — a thin
/// projection over [`real_entries`], the session-scoped catalog the
/// blocked path also reads so case ids/required caps never drift.
///
/// The plugin artifact is resolved/hashed/loaded once up front — a
/// missing or unloadable binary degrades only `real-editor-attach`
/// (`NotRun`/`Fail` with the reason); the window-only cases still
/// run. A session that cannot start (headless host, no interactive
/// desktop) fails closed: every case reports `NotRun` with
/// `observed.environment_blocked`.
///
/// # Panics
/// Panics only if the fixed view/slot constants used to build the
/// mount are invalid — a programming error, never an environment
/// condition (environment failures are reported as `NotRun`).
#[must_use]
pub fn run_all(env: &LaneEnvironment) -> Vec<ScenarioReport> {
    let run_id = RunId::new(RUN_ID);
    let plugin = load_plugin(env);
    let plugin_evidence = plugin.as_ref().ok().map(|(_, evidence)| evidence.clone());
    let plugin_error = plugin.as_ref().err().cloned();

    let view = ViewInstanceId::new(631).expect("view instance id 631 is non-zero");
    let mut epochs = AttachEpochAllocator::new();
    let mut host = ViewHost::new(view, epochs.mint());
    host.mount(plugin_dock_view_spec())
        .expect("plugin dock view spec mounts");
    host.drain_events();

    let outcome = RealOsGuiTest::run(
        vec![RealWindowMount {
            window: WindowSpec::default(),
            view: ViewMount { host, theme: None },
        }],
        RealWindowOptions::default(),
        |gui| {
            let Some(window) = gui.windows().into_iter().next() else {
                return blocked_reports(
                    &run_id,
                    env,
                    plugin_evidence.as_ref(),
                    "real-window session reported no mounted window",
                );
            };
            // The session-scoped catalog — `real_entries` rows run
            // through `run_entries` against the live context.
            let entries = real_entries();
            let mut ctx = RealDockContext {
                gui,
                window,
                view,
                env,
                plugin: plugin.as_ref().ok(),
                plugin_error: plugin_error.as_ref(),
                artifact: plugin_evidence.as_ref(),
            };
            run_entries(&entries, &run_id, &mut ctx)
        },
    );

    match outcome {
        Ok(reports) => reports,
        Err(error) => blocked_reports(
            &run_id,
            env,
            plugin_evidence.as_ref(),
            &format!("real-window session could not start: {error}"),
        ),
    }
}

// ---------------------------------------------------------------------
// shared helpers
// ---------------------------------------------------------------------

/// The `ComponentKey` of the dock `NativeSlot` inside
/// [`plugin_dock_view_spec`] — the `bind_native_slot` lookup key.
fn slot_key() -> ComponentKey {
    ComponentKey(PLUGIN_DOCK_SLOT_KEY.to_string())
}

/// The `{"lane": "real-window-dock"}` spec extras shared by the
/// window-only real cases — spec-level declaration only;
/// `finalize` overwrites `report.environment` post-run.
fn real_window_extras(_: &RealDockContext<'_>) -> ScenarioSpecExtras {
    ScenarioSpecExtras {
        environment: Some(json!({ "lane": "real-window-dock" })),
        timeout_ms: None,
    }
}

/// `real-editor-attach` spec extras — the lane marker plus the
/// plugin artifact identity when the binary resolved (what
/// `real_spec` used to stamp under `environment.plugin_artifact`).
fn editor_attach_extras(ctx: &RealDockContext<'_>) -> ScenarioSpecExtras {
    let mut environment = Map::from_iter([("lane".to_string(), json!("real-window-dock"))]);
    if let Some(artifact) = ctx.artifact {
        environment.insert("plugin_artifact".to_string(), json!(artifact));
    }
    ScenarioSpecExtras {
        environment: Some(Value::Object(environment)),
        timeout_ms: None,
    }
}

/// The `rf631.plugin-dock-real` catalog rows — session-scoped: the
/// entries' `run` needs the live `RealOsGuiTest`, so the table is
/// produced per call (the entries themselves are `'static` fn items;
/// only the context type carries the session borrow).
/// `blocked_reports` reads the same rows for case ids and required
/// capabilities, so the blocked path can never drift from the live
/// catalog.
fn real_entries<'a>() -> [ScenarioEntry<RealDockContext<'a>>; 4] {
    [
        ScenarioEntry {
            case: "real-window-mount",
            scenario: SCENARIO,
            required: &[ScenarioCapability::SemanticAction],
            spec_extras: real_window_extras,
            run: real_window_mount_case,
        },
        ScenarioEntry {
            case: "real-native-slot",
            scenario: SCENARIO,
            required: &[ScenarioCapability::NativePluginDock],
            spec_extras: real_window_extras,
            run: real_native_slot_case,
        },
        ScenarioEntry {
            case: "real-editor-attach",
            scenario: SCENARIO,
            required: &[ScenarioCapability::NativePluginDock],
            spec_extras: editor_attach_extras,
            run: real_editor_attach_case,
        },
        ScenarioEntry {
            case: "real-capture",
            scenario: SCENARIO,
            required: &[ScenarioCapability::VisualCapture],
            spec_extras: real_window_extras,
            run: real_capture_case,
        },
    ]
}

/// Resolves + hashes + loads the `test_gain` artifact once for the
/// lane — only `real-editor-attach` consumes it. `Err` carries the
/// honest unavailable reason plus whether the artifact was simply
/// absent (`NotRun`) or present-but-broken (`Fail`).
fn load_plugin(env: &LaneEnvironment) -> Result<PluginPair, PluginUnavailable> {
    let unavailable = |detail: String, prerequisite: bool| PluginUnavailable {
        detail,
        prerequisite,
    };
    let path = artifact::resolve_artifact(env.plugin_path.as_deref())
        .map_err(|detail| unavailable(detail, true))?;
    let sha256 = artifact::sha256_hex(&path).map_err(|error| {
        unavailable(
            format!("artifact hash failed for {}: {error}", path.display()),
            false,
        )
    })?;
    let library = Vst3Library::load(&path).map_err(|error| {
        unavailable(
            format!("Vst3Library::load failed for {}: {error}", path.display()),
            false,
        )
    })?;
    let identity = {
        let Some(descriptor) = library
            .descriptors()
            .iter()
            .find(|d| d.name == "test_gain")
            .or_else(|| library.descriptors().first())
        else {
            return Err(unavailable(
                format!(
                    "loaded {} but no plugin descriptor was discovered",
                    path.display()
                ),
                false,
            ));
        };
        (
            descriptor.id.to_string(),
            descriptor.name.to_string(),
            descriptor.vendor.to_string(),
            descriptor.version.to_string(),
        )
    };
    let (plugin_id, name, vendor, version) = identity;
    Ok((
        library,
        PluginArtifactEvidence {
            path: path.display().to_string(),
            sha256,
            plugin_id,
            name,
            vendor,
            version,
        },
    ))
}

/// The declarative slot spec for the real window's dock region —
/// zeroed `Hidden` geometry like the `PoC` precedent; the renderer's
/// per-frame projection overwrites it with real layout bounds before
/// `lease_parent` reads it. Declared capabilities match the Windows
/// backend's implemented set exactly (`attach_parent`, `hide_show`,
/// `focus`, `clip`, `z_order`) — never claiming keyboard/IME/
/// reparent/resize negotiation the backend refuses.
fn real_slot_spec(window: WindowId, view: ViewInstanceId) -> SlotSpec {
    SlotSpec {
        window,
        view,
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
            focus: Support::Supported,
            clip: Support::Supported,
            z_order: Support::Supported,
            ..SlotCapabilities::default()
        },
    }
}

/// `true` once the renderer's per-frame projection has published a
/// `Visible` spec with non-empty geometry for `slot` — the
/// precondition `lease_parent` needs for a real container rect.
fn slot_projected(gui: &RealOsGuiTest, slot: SurfaceSlotId) -> bool {
    gui.native_slot_spec(slot)
        .ok()
        .flatten()
        .is_some_and(|spec| {
            matches!(spec.visibility, SlotVisibility::Visible)
                && spec.geometry.rect.width > 0.0
                && spec.geometry.rect.height > 0.0
        })
}

/// Runs `op`, pushes a `pass`/`fail` step row into `steps`, and
/// returns the value or the formatted halt detail. `extra` merges
/// observation fields (lease seq, token, ...) into the pass row.
fn step<T>(
    steps: &mut Vec<Value>,
    name: &str,
    result: Result<T, ScenarioError>,
    extra: impl FnOnce(&T) -> Value,
) -> Result<T, String> {
    match result {
        Ok(value) => {
            let mut row = json!({ "step": name, "verdict": "pass" });
            if let (Some(object), Value::Object(fields)) = (row.as_object_mut(), extra(&value)) {
                object.extend(fields);
            }
            steps.push(row);
            Ok(value)
        }
        Err(error) => {
            steps.push(json!({
                "step": name,
                "verdict": "fail",
                "error": error.to_string(),
            }));
            Err(format!("{name}: {error}"))
        }
    }
}

/// Records an expected-refusal check: `op` must return `Err`. A
/// `pass` row means the stale/forged lease was rejected; `fail`
/// means it resolved — the ledger-honesty violation this lane
/// exists to catch. Returns whether the refusal held.
fn expect_rejected<T: std::fmt::Debug>(
    steps: &mut Vec<Value>,
    name: &str,
    result: Result<T, ScenarioError>,
) -> bool {
    if let Ok(value) = result {
        steps.push(json!({
            "step": name,
            "verdict": "fail",
            "error": format!("lease resolved unexpectedly: {value:?}"),
        }));
        false
    } else {
        steps.push(json!({ "step": name, "verdict": "pass" }));
        true
    }
}

/// Best-effort teardown on the failure path — the session window
/// teardown destroys everything anyway; errors are ignored because
/// the halt detail is the evidence.
fn bail_slot(
    gui: &RealOsGuiTest,
    window: WindowId,
    slot_key: &ComponentKey,
    slot: SurfaceSlotId,
    lease: Option<SlotLease>,
) {
    let _ = gui.unbind_native_slot(window, slot_key.clone());
    if let Some(lease) = lease {
        let _ = gui.release_native_lease(lease);
    }
    let _ = gui.release_native_slot(slot);
}

/// Teardown of a real slot + lease on the success path: unbind →
/// release lease → release slot. Every failure is recorded in the
/// returned notes — teardown never masks the case verdict.
fn release_real_slot(
    gui: &RealOsGuiTest,
    window: WindowId,
    slot_key: &ComponentKey,
    slot: SurfaceSlotId,
    lease: Option<SlotLease>,
) -> Vec<String> {
    let mut notes = Vec::new();
    if let Err(error) = gui.unbind_native_slot(window, slot_key.clone()) {
        notes.push(format!("unbind_native_slot: {error}"));
    }
    if let Some(lease) = lease
        && let Err(error) = gui.release_native_lease(lease)
    {
        notes.push(format!("release_native_lease: {error}"));
    }
    if let Err(error) = gui.release_native_slot(slot) {
        notes.push(format!("release_native_slot: {error}"));
    }
    notes
}

/// declare → bind → settle-until-projected → `lease_parent` on the
/// real window's canonical surface. Pushes one row per executed op
/// into `steps`; on failure performs best-effort cleanup and returns
/// the halt detail. The caller owns the returned `(slot, lease)`.
fn lease_real_slot(
    gui: &mut RealOsGuiTest,
    window: WindowId,
    view: ViewInstanceId,
    steps: &mut Vec<Value>,
) -> Result<(SurfaceSlotId, SlotLease), String> {
    let key = slot_key();
    let slot = step(
        steps,
        "declare_slot",
        gui.declare_native_slot(real_slot_spec(window, view)),
        |_| Value::Null,
    )?;
    if let Err(detail) = step(
        steps,
        "bind_native_slot",
        gui.bind_native_slot(window, key.clone(), slot),
        |()| Value::Null,
    ) {
        bail_slot(gui, window, &key, slot, None);
        return Err(detail);
    }
    let projected = match settle_until(gui, |g| slot_projected(g, slot), 120) {
        Ok(projected) => projected,
        Err(error) => {
            steps.push(json!({
                "step": "project_geometry",
                "verdict": "fail",
                "error": error.to_string(),
            }));
            bail_slot(gui, window, &key, slot, None);
            return Err(format!("project_geometry: {error}"));
        }
    };
    if !projected {
        steps.push(json!({
            "step": "project_geometry",
            "verdict": "fail",
            "error": "slot spec never reached Visible with non-empty geometry",
        }));
        bail_slot(gui, window, &key, slot, None);
        return Err(
            "project_geometry: slot spec never reached Visible with non-empty geometry".to_string(),
        );
    }
    steps.push(json!({ "step": "project_geometry", "verdict": "pass" }));
    match step(
        steps,
        "lease_parent",
        gui.lease_native_slot(slot),
        |lease| json!({ "slot_index": lease.index(), "seq": lease.seq() }),
    ) {
        Ok(lease) => Ok((slot, lease)),
        Err(detail) => {
            bail_slot(gui, window, &key, slot, None);
            Err(detail)
        }
    }
}

/// Folds the accumulated step rows into the case outcome — `Err`
/// detail becomes `Fail` (the steps stay as observed evidence), `Ok`
/// extras merge into `observed` on `Pass`.
fn steps_outcome(
    steps: &[Value],
    attempt: Result<Value, String>,
    pass_detail: &str,
    evidence: &[&str],
) -> ScenarioOutcome {
    let mut observed = Map::from_iter([("steps".to_string(), json!(steps))]);
    match attempt {
        Ok(extras) => {
            if let Value::Object(fields) = extras {
                observed.extend(fields);
            }
            let mut outcome = ScenarioOutcome::new(ScenarioVerdict::Pass);
            outcome.detail = Some(pass_detail.to_string());
            outcome.observed = Some(Value::Object(observed));
            outcome.evidence = evidence.iter().map(|tag| (*tag).to_string()).collect();
            outcome
        }
        Err(detail) => {
            let mut outcome = fail(detail);
            outcome.observed = Some(Value::Object(observed));
            outcome
        }
    }
}

/// The four fail-closed reports for a session that never started —
/// `NotRun` + `observed.environment_blocked`, the
/// `test-ui-real-scenario` precedent. `environment_kind` stays the
/// honest "no real environment produced this" bucket; nothing
/// executed, so `RealWindow` is never claimed here. Case ids and
/// required capabilities come from [`real_entries`] — the same
/// catalog rows the live path runs.
fn blocked_reports(
    run_id: &RunId,
    env: &LaneEnvironment,
    artifact: Option<&PluginArtifactEvidence>,
    detail: &str,
) -> Vec<ScenarioReport> {
    // `'static` context: only entry metadata is read here — no
    // `RealDockContext` is ever constructed on this path.
    let entries: [ScenarioEntry<RealDockContext<'static>>; 4] = real_entries();
    entries
        .iter()
        .map(|entry| {
            let mut report = ScenarioReport::new(
                run_id.clone(),
                ScenarioId::new(entry.scenario),
                Some(CaseId::new(entry.case)),
                ScenarioDriver::RealOs,
            );
            report.required = entry.required.iter().copied().collect();
            report.verdict = ScenarioVerdict::NotRun;
            report.detail = Some(detail.to_string());
            report.observed = Some(json!({
                "observed": "environment_blocked",
                "detail": detail,
            }));
            finalize(
                report,
                env,
                EnvironmentKind::DeterministicBackend,
                artifact,
                Map::new(),
            )
        })
        .collect()
}

// ---------------------------------------------------------------------
// case: real-window-mount (semantic mount proof on a real OS window)
// ---------------------------------------------------------------------

/// `real-window-mount`: the dock `ViewSpec` mounts into a real OS
/// window and its `NativeSlot` component resolves + observes through
/// the semantic port — contract evidence that the real window hosts
/// the declared slot region (not yet dock evidence).
fn real_window_mount_case(ctx: &mut RealDockContext<'_>, spec: &ScenarioSpec) -> ScenarioReport {
    let window = ctx.window;
    let view = ctx.view;
    let report = run_scenario(&mut *ctx.gui, spec, |gui| {
        let key = slot_key();
        let target = gui.resolve_target(&TargetQuery::new(window, view).component(key.clone()))?;
        let _snapshot = gui.observe(&target, &key)?;
        let mut outcome = ScenarioOutcome::new(ScenarioVerdict::Pass);
        outcome.detail = Some(format!(
            "real OS window mounts plugin_dock_view_spec; NativeSlot \
             `{PLUGIN_DOCK_SLOT_KEY}` resolved and observed"
        ));
        outcome.observed = Some(json!({
            "window": window.to_string(),
            "component": PLUGIN_DOCK_SLOT_KEY,
        }));
        outcome
            .evidence
            .push("real-window:semantic-mount:plugin.editor".to_string());
        Ok(outcome)
    });
    finalize(
        report,
        ctx.env,
        EnvironmentKind::RealWindow,
        None,
        Map::new(),
    )
}

// ---------------------------------------------------------------------
// case: real-native-slot (declare→bind→project→lease→validate→token→release)
// ---------------------------------------------------------------------

/// The native-slot attempt after `lease_real_slot`: validate the
/// live lease, observe `container_token`, release everything, then
/// prove the released and forged payloads no longer resolve —
/// returning the observation extras on success.
fn native_slot_attempt(
    gui: &mut RealOsGuiTest,
    window: WindowId,
    view: ViewInstanceId,
    steps: &mut Vec<Value>,
) -> Result<Value, String> {
    let key = slot_key();
    let (slot, lease) = lease_real_slot(gui, window, view, steps)?;

    let info = match step(
        steps,
        "validate_lease",
        gui.validate_native_lease(lease),
        |info| {
            json!({
                "slot_index": info.slot.index(),
                "slot_generation": info.slot.generation(),
                "seq": info.seq,
            })
        },
    ) {
        Ok(info) => info,
        Err(detail) => {
            bail_slot(gui, window, &key, slot, Some(lease));
            return Err(detail);
        }
    };

    let token = match gui.native_container_token(slot) {
        Ok(Some(token)) => {
            steps.push(json!({
                "step": "container_token",
                "verdict": "pass",
                "token": token,
            }));
            token
        }
        outcome => {
            let detail = match outcome {
                Ok(None) => "no live container after lease_parent".to_string(),
                Err(error) => error.to_string(),
                Ok(Some(_)) => unreachable!("handled above"),
            };
            steps.push(json!({
                "step": "container_token",
                "verdict": "fail",
                "error": detail,
            }));
            bail_slot(gui, window, &key, slot, Some(lease));
            return Err(format!("container_token: {detail}"));
        }
    };

    // Success-path teardown with recorded notes.
    let notes = release_real_slot(gui, window, &key, slot, Some(lease));
    if notes.is_empty() {
        steps.push(json!({ "step": "release", "verdict": "pass" }));
    } else {
        steps.push(json!({
            "step": "release",
            "verdict": "note",
            "detail": notes.join("; "),
        }));
    }

    // Post-release honesty: the dead lease and a forged payload must
    // never resolve again — validate AND redeem both refuse.
    let mut honest = true;
    honest &= expect_rejected(
        steps,
        "released_lease_rejected",
        gui.validate_native_lease(lease),
    );
    honest &= expect_rejected(
        steps,
        "released_lease_redeem_rejected",
        gui.redeem_native_lease(lease.raw()),
    );
    honest &= expect_rejected(
        steps,
        "forged_lease_rejected",
        gui.validate_native_lease(SlotLease::new(u64::MAX)),
    );
    if !honest {
        return Err("stale/forged lease resolved against the live ledger — \
             a released or invented payload must never reach a real HWND"
            .to_string());
    }

    Ok(json!({
        "container_token": token,
        "lease_seq": lease.seq(),
        "slot_index": info.slot.index(),
    }))
}

/// `real-native-slot`: the full slot lifecycle on the real window's
/// canonical surface — declare → bind → renderer projection →
/// `lease_parent` (real container HWND) → `validate_lease` →
/// `container_token` observation → unbind/release → stale/forged
/// lease rejection. Every executed step is a `pass`/`fail` row in
/// `observed.steps`; the first failure halts the sequence honestly.
fn real_native_slot_case(ctx: &mut RealDockContext<'_>, spec: &ScenarioSpec) -> ScenarioReport {
    let window = ctx.window;
    let view = ctx.view;
    let report = run_scenario(&mut *ctx.gui, spec, |gui| {
        let mut steps = Vec::new();
        let attempt = native_slot_attempt(gui, window, view, &mut steps);
        Ok(steps_outcome(
            &steps,
            attempt,
            "real OS window: NativeSlot declare → bind → renderer \
             projection → lease_parent (real container HWND) → \
             validate_lease → container_token → release → stale/forged \
             lease rejection — every step executed on the live session",
            &[
                "native-slot:declare-slot",
                "native-slot:bind",
                "native-slot:project-geometry",
                "native-slot:lease-parent",
                "native-slot:validate-lease",
                "native-slot:container-token",
                "native-slot:release",
                "native-slot:lease-rejection",
            ],
        ))
    });
    finalize(
        report,
        ctx.env,
        EnvironmentKind::RealWindow,
        None,
        Map::new(),
    )
}

// ---------------------------------------------------------------------
// case: real-editor-attach (session.open(Docked) through a real lease)
// ---------------------------------------------------------------------

/// What one real editor-attach attempt produced.
enum RealAttachAttempt {
    /// `session.open(Docked)` returned this.
    Open(Result<EditorCapabilities, EditorSessionError>),
    /// Setup failed before `open` — the detail string.
    Setup(String),
}

/// The real attach attempt — session begin, real-slot
/// declare/bind/project/`lease_parent` (real container HWND),
/// `ParentSlotLease` → `InProcessEditorOps` redeem through
/// [`RealOsGuiTest::redeem_native_lease`], then `session.open(Docked)`.
/// Always tears down (session close/end, slot/lease release, instance
/// destroy). Returns the open outcome (or the setup failure) plus
/// cleanup notes.
fn attempt_real_attach(
    gui: &mut RealOsGuiTest,
    window: WindowId,
    view: ViewInstanceId,
    host: &mut InProcessHost,
    handle: PluginHandle,
    steps: &mut Vec<Value>,
) -> (RealAttachAttempt, Vec<String>) {
    let mut cleanup = Vec::new();
    let mut sessions = PluginEditorSessions::new();

    let (slot, lease) = match lease_real_slot(gui, window, view, steps) {
        Ok(pair) => pair,
        Err(detail) => {
            let _ = host.execute(HostCommand::Destroy { handle });
            cleanup.push("slot-setup-failed/instance-destroyed".to_string());
            return (RealAttachAttempt::Setup(detail), cleanup);
        }
    };
    let key = slot_key();

    let session_id = match sessions.begin_session(handle) {
        Ok(id) => {
            steps.push(json!({
                "step": "begin_session",
                "verdict": "pass",
                "session": session_id_json(id),
            }));
            id
        }
        Err(error) => {
            steps.push(json!({
                "step": "begin_session",
                "verdict": "fail",
                "error": error.to_string(),
            }));
            cleanup.extend(release_real_slot(gui, window, &key, slot, Some(lease)));
            let _ = host.execute(HostCommand::Destroy { handle });
            cleanup.push("instance-destroyed".to_string());
            return (
                RealAttachAttempt::Setup(format!("begin_session: {error}")),
                cleanup,
            );
        }
    };

    // The session-facing lease wraps the registry's opaque payload —
    // never an OS handle. The issuer-side redeem happens inside
    // `InProcessEditorOps`: `ParentSlotLease.raw()` →
    // `redeem_native_lease` (adapter validates against its live
    // ledger) → `ParentLease::new(seq, container_token)`.
    let parent_slot_lease = ParentSlotLease::new(lease.raw());
    let open_result = {
        let gui_ref: &RealOsGuiTest = gui;
        let mut ops = InProcessEditorOps::with_redeemer(
            host,
            handle,
            Box::new(move |session_lease| {
                gui_ref
                    .redeem_native_lease(session_lease.raw())
                    .map(|token| ParentLease::new(SlotLease::new(session_lease.raw()).seq(), token))
                    .map_err(|_| EditorOpError::LeaseRejected)
            }),
        );
        sessions.session_mut(session_id).map(|session| {
            session.open(
                &mut ops,
                &parent_slot_lease,
                editor_size(),
                AttachMode::Docked,
            )
        })
    };

    // Session state at this point is the honest landing — a refused
    // open sits `Failed{Unsupported}`.
    let failed_unsupported = session_failed_unsupported(&sessions, session_id);
    let (open_verdict, open_detail) = match &open_result {
        Some(Ok(_)) => ("pass", "editor capabilities returned".to_string()),
        Some(Err(error)) => {
            let verdict = if matches!(error, EditorSessionError::Ops(EditorOpError::Unsupported)) {
                "unsupported"
            } else {
                "fail"
            };
            (verdict, error.to_string())
        }
        None => ("fail", "editor session vanished before open".to_string()),
    };
    steps.push(json!({
        "step": "session.open(Docked)",
        "verdict": open_verdict,
        "detail": open_detail,
        "failed_unsupported": failed_unsupported,
    }));

    // Converge + teardown — `close` on a refused open is a no-op
    // adapter call (`editor_live` stays false), so the rejecting
    // redeemer is never consulted.
    if let Some(session) = sessions.session_mut(session_id) {
        let mut close_ops = rejecting_editor_ops(host, handle);
        let _ = session.close(&mut close_ops);
    }
    let _ = sessions.end_session(session_id);
    cleanup.extend(release_real_slot(gui, window, &key, slot, Some(lease)));
    let _ = host.execute(HostCommand::Destroy { handle });
    cleanup.push("session-ended/lease-released/slot-released/instance-destroyed".to_string());

    let attempt = match open_result {
        Some(result) => RealAttachAttempt::Open(result),
        None => RealAttachAttempt::Setup("editor session vanished before open".to_string()),
    };
    (attempt, cleanup)
}

/// `real-editor-attach`: drives the real attach path —
/// `PluginEditorSessions` → `InProcessEditorOps` → `OpenGui` —
/// against a *real* container-HWND lease redeemed through
/// [`RealOsGuiTest::redeem_native_lease`]. The current `test_gain`
/// fixture has no editor (`createView` → null →
/// `HostError::Unsupported` → `EditorOpError::Unsupported`), so the
/// honest verdict stays `Unsupported` — the slot/lease/redeem chain
/// itself is the real-window evidence this case records.
fn editor_attach_body(
    gui: &mut RealOsGuiTest,
    window: WindowId,
    view: ViewInstanceId,
    plugin: Option<&PluginPair>,
    plugin_error: Option<&PluginUnavailable>,
) -> ScenarioOutcome {
    let Some((library, artifact)) = plugin else {
        return match plugin_error {
            Some(unavailable) if unavailable.prerequisite => not_run(format!(
                "plugin artifact unavailable: {}",
                unavailable.detail
            )),
            Some(unavailable) => fail(format!(
                "plugin artifact failed to load: {}",
                unavailable.detail
            )),
            None => not_run("plugin artifact unavailable".to_string()),
        };
    };
    let plugin_id = artifact.plugin_id.clone();
    let mut host = InProcessHost::new();
    register_factories(&mut host, library);

    let handle = match activate_test_gain(&mut host, &plugin_id) {
        Ok(handle) => handle,
        Err(error) => return host_error_outcome("activate", error),
    };

    let mut steps = Vec::new();
    let (attempt, cleanup) = attempt_real_attach(gui, window, view, &mut host, handle, &mut steps);

    let mut outcome = match attempt {
        RealAttachAttempt::Open(Ok(_capabilities)) => {
            // The plugin accepted OpenGui through the REAL
            // container-HWND lease — a genuine dock attach on a real
            // OS window. `Pass` is honest here: the child surface
            // was parented into the leased container, not a fake.
            let mut outcome = ScenarioOutcome::new(ScenarioVerdict::Pass);
            outcome.detail = Some(
                "editor open accepted through a REAL container-HWND lease \
                 (NativeSlot → lease_parent → ParentSlotLease → \
                 InProcessEditorOps redeem → session.open(Docked))"
                    .to_string(),
            );
            outcome
                .evidence
                .push("editor-attach:real-container-parent".to_string());
            outcome
        }
        RealAttachAttempt::Open(Err(EditorSessionError::Ops(EditorOpError::Unsupported))) => {
            let mut outcome = ScenarioOutcome::new(ScenarioVerdict::Unsupported);
            outcome.detail = Some(
                "plugin declined OpenGui through the REAL container lease: \
                 createView returned null → HostError::Unsupported → \
                 EditorOpError::Unsupported (test_gain ships no editor \
                 surface); the slot/lease/redeem chain itself executed \
                 on a real OS window"
                    .to_string(),
            );
            outcome
                .evidence
                .push("editor-attach:createView-null-real-lease".to_string());
            outcome
        }
        RealAttachAttempt::Open(Err(error)) => fail(format!("session.open: {error}")),
        RealAttachAttempt::Setup(detail) => fail(detail),
    };
    outcome.observed = Some(json!({
        "steps": steps,
        "cleanup": cleanup,
    }));
    outcome
}

/// `real-editor-attach` case: pre-flight requires
/// `NativePluginDock` (the adapter claims it only where the surface
/// backend mints real container parents), then the body drives the
/// attach chain — or reports the honest artifact/session failure.
fn real_editor_attach_case(ctx: &mut RealDockContext<'_>, spec: &ScenarioSpec) -> ScenarioReport {
    let window = ctx.window;
    let view = ctx.view;
    let plugin = ctx.plugin;
    let plugin_error = ctx.plugin_error;
    let artifact = ctx.artifact;
    let report = run_scenario(&mut *ctx.gui, spec, |gui| {
        Ok(editor_attach_body(gui, window, view, plugin, plugin_error))
    });
    finalize(
        report,
        ctx.env,
        EnvironmentKind::RealWindow,
        artifact,
        Map::new(),
    )
}

// ---------------------------------------------------------------------
// case: real-capture (window/slot-region pixels — never editor pixels)
// ---------------------------------------------------------------------

/// `real-capture`: `capture_frame` on the real window — the pixels
/// are the window/slot region through the OS capture backend
/// (`PrintWindow`/`BitBlt`), explicitly NOT plugin editor pixels:
/// no editor surface exists (`createView` → null).
fn real_capture_case(ctx: &mut RealDockContext<'_>, spec: &ScenarioSpec) -> ScenarioReport {
    let window = ctx.window;
    let view = ctx.view;
    let report = run_scenario(&mut *ctx.gui, spec, |gui| {
        let target = gui.resolve_target(&TargetQuery::new(window, view))?;
        match gui.capture_frame(&target)? {
            CaptureOutcome::Captured(image) => {
                let backend = gui.last_capture_meta().map(|meta| meta.backend);
                let mut outcome = ScenarioOutcome::new(ScenarioVerdict::Pass);
                outcome.detail = Some(format!(
                    "real OS window pixels captured ({}x{}, backend {backend:?}) — \
                     window/slot-region pixels, NOT plugin editor pixels: no \
                     editor surface exists (test_gain createView returns null)",
                    image.width(),
                    image.height(),
                ));
                outcome.observed = Some(json!({
                    "width": image.width(),
                    "height": image.height(),
                    "backend": backend,
                }));
                outcome
                    .evidence
                    .push("real-capture:window-pixels".to_string());
                Ok(outcome)
            }
            CaptureOutcome::Unsupported { reason } => {
                let mut outcome = ScenarioOutcome::new(ScenarioVerdict::Unsupported);
                outcome.detail = Some(format!("capture_frame unsupported: {reason}"));
                Ok(outcome)
            }
            CaptureOutcome::NotRun { reason } => {
                let mut outcome = ScenarioOutcome::new(ScenarioVerdict::NotRun);
                outcome.detail = Some(format!("capture_frame not_run: {reason}"));
                Ok(outcome)
            }
            other => Err(ScenarioError::Driver(format!(
                "unexpected capture outcome {other:?}"
            ))),
        }
    });
    finalize(
        report,
        ctx.env,
        EnvironmentKind::RealWindow,
        None,
        Map::new(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The declared slot spec must never claim capabilities the
    /// Windows surface backend refuses — capability honesty is the
    /// lane's core contract.
    #[test]
    fn real_slot_spec_declares_only_backend_supported_caps() {
        let spec = real_slot_spec(
            WindowId::new(1).expect("window id 1 is non-zero"),
            ViewInstanceId::new(1).expect("view id 1 is non-zero"),
        );
        for capability in [
            spec.capabilities.attach_parent,
            spec.capabilities.hide_show,
            spec.capabilities.focus,
            spec.capabilities.clip,
            spec.capabilities.z_order,
        ] {
            assert!(capability.supported());
        }
        for capability in [
            spec.capabilities.keyboard_input,
            spec.capabilities.ime,
            spec.capabilities.reparent,
            spec.capabilities.resize_negotiate,
            spec.capabilities.cross_window_move,
            spec.capabilities.popup,
            spec.capabilities.out_of_process_parent,
        ] {
            assert!(!capability.supported());
        }
    }

    /// A session that never started must produce four `NotRun`
    /// reports tagged `environment_blocked` — fail-closed, never a
    /// fabricated pass and never a `RealWindow` claim. Case ids and
    /// required capabilities come from the same `real_entries`
    /// catalog the live path runs, so they are asserted against it
    /// directly — drift fails here.
    #[test]
    fn blocked_reports_fail_closed() {
        let env = LaneEnvironment::default();
        let run_id = RunId::new(RUN_ID);
        let reports = blocked_reports(&run_id, &env, None, "test: no session");
        assert_eq!(reports.len(), 4, "four real-dock cases");

        // The blocked path reads the same catalog rows — case ids
        // and required caps must match exactly.
        let entries: [ScenarioEntry<RealDockContext<'static>>; 4] = real_entries();
        assert_eq!(reports.len(), entries.len());
        for (entry, report) in entries.iter().zip(reports.iter()) {
            assert_eq!(report.case.as_ref().map(CaseId::as_str), Some(entry.case));
            assert_eq!(report.scenario.as_str(), entry.scenario);
            assert_eq!(
                report.required,
                entry.required.iter().copied().collect(),
                "required-capability drift for `{}`",
                entry.case
            );
        }

        for report in &reports {
            assert_eq!(report.verdict, ScenarioVerdict::NotRun);
            assert!(matches!(report.driver, ScenarioDriver::RealOs));
            let observed = report.observed.as_ref().expect("observed recorded");
            assert_eq!(observed["observed"], "environment_blocked");
            let kind = report
                .environment
                .as_ref()
                .and_then(|env| env["environment_kind"].as_str());
            assert_ne!(kind, Some("real-window"), "nothing ran — no real claim");
        }
    }
}
