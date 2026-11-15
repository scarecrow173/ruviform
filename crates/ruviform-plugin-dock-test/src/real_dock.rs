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
//! | `real-editor-attach`  | `RealOs`    | `Pass`/`Fail`/`Unsupported` — `test_editor` real   |
//! |                       |             | `IPlugView` child HWND under the real container     |
//! |                       |             | lease, OS-observed attach → detach (`IsWindow==0`)  |
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
    AttachMode, EditorCapabilities, EditorOpError, EditorSessionError, EditorSessionEvent,
    HostCommand, InProcessEditorOps, InProcessHost, ParentLease, ParentSlotLease,
    PluginEditorSession, PluginEditorSessionId, PluginEditorSessions, PluginHandle,
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
use crate::real_catalog::{LANE, SCENARIO};
// The run id is shared with the console's catalog metadata —
// re-exported so `real_dock::RUN_ID` keeps working for the lane.
pub use crate::real_catalog::RUN_ID;
use crate::scenario::{
    activate_test_gain, editor_size, events_debug, fail, finalize, host_error_outcome, not_run,
    register_factories, rejecting_editor_ops, session_closed_concluded, session_failed_unsupported,
    session_id_json,
};

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

/// The `{"lane": real_catalog::LANE}` spec extras shared by the
/// window-only real cases — spec-level declaration only;
/// `finalize` overwrites `report.environment` post-run.
fn real_window_extras(_: &RealDockContext<'_>) -> ScenarioSpecExtras {
    ScenarioSpecExtras {
        environment: Some(json!({ "lane": LANE })),
        timeout_ms: None,
    }
}

/// `real-editor-attach` spec extras — the lane marker plus the
/// plugin artifact identity when the binary resolved (what
/// `real_spec` used to stamp under `environment.plugin_artifact`).
fn editor_attach_extras(ctx: &RealDockContext<'_>) -> ScenarioSpecExtras {
    let mut environment = Map::from_iter([("lane".to_string(), json!(LANE))]);
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

/// Resolves + hashes + loads the test-plugin cdylib once for the
/// lane and selects its `test_editor` descriptor — only
/// `real-editor-attach` consumes it. `Err` carries the honest
/// unavailable reason plus whether the artifact was simply absent
/// (`NotRun`) or present-but-broken (`Fail`). There is deliberately
/// no fallback to another descriptor: an editor-less fixture
/// (`test_gain`) would degrade the case back to `Unsupported` and
/// fake the slice's intent — a missing `test_editor` is an honest
/// `Fail`, not a silent retreat.
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
            .find(|d| d.name == "test_editor")
        else {
            return Err(unavailable(
                format!(
                    "loaded {} but no `test_editor` descriptor was discovered — \
                     the real-editor-attach case requires the editor fixture \
                     (Issue #630); refusing silent fallback to an editor-less class",
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
// OS child-HWND observation (real-editor-attach — query-only Win32)
// ---------------------------------------------------------------------

/// The attach-phase contract checks for one observed child HWND.
#[derive(Debug)]
struct ChildAttachFacts {
    /// `GetParent(child) == container` — the plugin parented its
    /// editor into the leased container, not elsewhere.
    parent_matches: bool,
    /// `IsWindowVisible(child) != 0` — visible through the whole
    /// ancestor chain at observation time.
    visible: bool,
    /// `IsWindow(child) != 0` while attached.
    alive: bool,
}

/// Facts recorded for one observed child HWND — plain integers and
/// booleans so no Win32 type crosses into `observed`/evidence (the
/// same `u64`-narrowed-handle convention as `container_token`).
#[derive(Debug)]
struct ChildWindowFacts {
    /// The child HWND value as `u64`.
    hwnd: u64,
    /// `GetWindowThreadProcessId(child)` — the thread that created
    /// the child (the thread `attached` ran on).
    thread_id: u32,
    /// The three attach-phase checks, taken while the lease is live.
    attach: ChildAttachFacts,
    /// `IsWindow(child) == 0` after `session.close` — the plugin's
    /// `removed()` → `DestroyWindow` effect, observed post-detach.
    destroyed: bool,
}

/// The phase-1 OS observation of the editor child — `supported` is
/// `false` off-Windows where no HWND concept exists (the case then
/// records `Unsupported` for the observation half honestly).
#[derive(Debug)]
struct ChildWindowObservation {
    /// Every direct child `EnumChildWindows` found under the
    /// container, in enumeration order.
    children: Vec<ChildWindowFacts>,
    /// `GetCurrentThreadId` — the scenario caller thread that drove
    /// `session.open`/`attached` (the thread-affinity record).
    caller_thread: u32,
    /// `GetWindowThreadProcessId(container)` — the thread owning
    /// the container HWND's message pump (the GPUI app thread).
    container_thread: u32,
    /// `false` off-Windows — no child-HWND observation exists there.
    supported: bool,
}

/// Windows implementation — query-only Win32 (`poc.rs` precedent):
/// enumeration and property reads only; this module never creates,
/// destroys, moves, or messages a window.
#[cfg(windows)]
mod oschild {
    use windows_sys::Win32::Foundation::{HWND, LPARAM};
    use windows_sys::Win32::System::Threading::GetCurrentThreadId;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumChildWindows, GetParent, GetWindowThreadProcessId, IsWindow, IsWindowVisible,
    };

    use super::{ChildAttachFacts, ChildWindowFacts, ChildWindowObservation};

    /// Collects `container`'s direct child HWNDs and their contract
    /// facts, plus the caller/container owning thread ids.
    /// `container` is the opaque `container_token` `u64` (= the
    /// container HWND on Windows). Query-only — safe on any thread.
    #[expect(unsafe_code)]
    pub(super) fn observe(container: u64) -> ChildWindowObservation {
        unsafe extern "system" fn collect(hwnd: HWND, lparam: LPARAM) -> i32 {
            // SAFETY: `lparam` is the `*mut Vec<u64>` the caller
            // passed; it outlives this synchronous enumeration.
            unsafe { (*(lparam as *mut Vec<u64>)).push(hwnd as usize as u64) };
            1
        }
        let mut found: Vec<u64> = Vec::new();
        // SAFETY: query-only enumeration; `found` outlives the call.
        unsafe {
            EnumChildWindows(container as HWND, Some(collect), &raw mut found as LPARAM);
        }
        // SAFETY: query-only thread-id reads.
        let (caller_thread, container_thread) = unsafe {
            let mut pid = 0u32;
            (
                GetCurrentThreadId(),
                GetWindowThreadProcessId(container as HWND, &raw mut pid),
            )
        };
        let children = found
            .into_iter()
            .map(|hwnd| {
                // SAFETY: query-only property reads on an HWND this
                // enumeration just returned — still valid while the
                // lease (and thus the container) is held.
                unsafe {
                    let mut pid = 0u32;
                    ChildWindowFacts {
                        hwnd,
                        thread_id: GetWindowThreadProcessId(hwnd as HWND, &raw mut pid),
                        attach: ChildAttachFacts {
                            parent_matches: GetParent(hwnd as HWND) == container as HWND,
                            visible: IsWindowVisible(hwnd as HWND) != 0,
                            alive: IsWindow(hwnd as HWND) != 0,
                        },
                        destroyed: false,
                    }
                }
            })
            .collect();
        ChildWindowObservation {
            children,
            caller_thread,
            container_thread,
            supported: true,
        }
    }

    /// An observation for the no-container-token path — the platform
    /// supports observation but there is nothing to enumerate, so
    /// the child checks fail honestly downstream.
    #[expect(unsafe_code)]
    pub(super) fn empty() -> ChildWindowObservation {
        // SAFETY: query-only.
        let caller_thread = unsafe { GetCurrentThreadId() };
        ChildWindowObservation {
            children: Vec::new(),
            caller_thread,
            container_thread: 0,
            supported: true,
        }
    }

    /// Post-close verification — `IsWindow(child) == 0` for every
    /// recorded child (the `removed()` → `DestroyWindow` effect).
    #[expect(unsafe_code)]
    pub(super) fn mark_destroyed(observation: &mut ChildWindowObservation) {
        for child in &mut observation.children {
            // SAFETY: query-only; `IsWindow` accepts any HWND value —
            // a destroyed handle simply reports false.
            child.destroyed = unsafe { IsWindow(child.hwnd as HWND) } == 0;
        }
    }
}

/// Non-Windows stub — the `NativePluginDock` capability (and thus
/// this case body) is Windows-only, so this path is unreachable in
/// practice; it exists so the crate compiles on every target and an
/// accepted open would record `Unsupported`, never a fabricated
/// observation.
#[cfg(not(windows))]
mod oschild {
    use super::ChildWindowObservation;

    fn unsupported() -> ChildWindowObservation {
        ChildWindowObservation {
            children: Vec::new(),
            caller_thread: 0,
            container_thread: 0,
            supported: false,
        }
    }

    /// Always the unsupported, empty observation off-Windows.
    pub(super) fn observe(_container: u64) -> ChildWindowObservation {
        unsupported()
    }

    /// Same unsupported answer for the no-container path.
    pub(super) fn empty() -> ChildWindowObservation {
        unsupported()
    }

    /// No-op off-Windows — there are never children to mark.
    pub(super) fn mark_destroyed(_observation: &mut ChildWindowObservation) {}
}

/// Records a failed observation step and remembers the first
/// failure for the verdict detail — a violated invariant is never
/// hidden behind a later pass.
fn record_obs_fail(
    steps: &mut Vec<Value>,
    failure: &mut Option<String>,
    name: &str,
    detail: impl Into<String>,
) {
    let detail = detail.into();
    if failure.is_none() {
        *failure = Some(format!("{name}: {detail}"));
    }
    steps.push(json!({
        "step": name,
        "verdict": "fail",
        "error": detail,
    }));
}

/// The verdict-carrying result of the post-open child-HWND
/// observation — `verified` gates `Pass`; `failure` names the first
/// violated check for the `Fail` detail; `extras` merges into
/// `observed`.
struct ChildVerification {
    /// `false` off-Windows — the observation half then reports
    /// `Unsupported`, never a fabricated pass.
    supported: bool,
    /// Every contract check agreed: children exist under the
    /// container, parented, visible, live; destroyed after close;
    /// session concluded with `[Attached{Docked}, Closed]` events.
    verified: bool,
    /// First violated check (`name: detail`) for the `Fail` detail.
    failure: Option<String>,
    /// `observed` extras — container/child HWND ids, thread ids,
    /// drained events.
    extras: Value,
}

/// Phase-1 attach checks while the editor is still attached:
/// `child_enumerated` (at least one real child HWND under the
/// container), `child_contract` (every child parented into the
/// container, visible, alive), and `thread_affinity` (owning thread
/// ids recorded as observation — the Issue #389 D1 deviation is a
/// documented fact, never a verdict input). Returns whether every
/// check agreed.
fn check_attached_children(
    observation: &ChildWindowObservation,
    container: Option<u64>,
    steps: &mut Vec<Value>,
    failure: &mut Option<String>,
) -> bool {
    let mut ok = true;

    // child_enumerated — at least one real child HWND under the
    // container.
    if observation.children.is_empty() {
        record_obs_fail(
            steps,
            failure,
            "child_enumerated",
            "EnumChildWindows found no child under the container HWND",
        );
        ok = false;
    } else {
        let children: Vec<Value> = observation
            .children
            .iter()
            .map(|child| {
                json!({
                    "hwnd": child.hwnd,
                    "parent_matches": child.attach.parent_matches,
                    "visible": child.attach.visible,
                    "alive": child.attach.alive,
                    "thread_id": child.thread_id,
                })
            })
            .collect();
        steps.push(json!({
            "step": "child_enumerated",
            "verdict": "pass",
            "container": container,
            "count": observation.children.len(),
            "children": children,
        }));
    }

    // child_contract — every observed child parented into the
    // container, visible, and alive.
    if observation.children.is_empty() {
        record_obs_fail(steps, failure, "child_contract", "no children to check");
        ok = false;
    } else {
        let violations: Vec<String> = observation
            .children
            .iter()
            .enumerate()
            .filter_map(|(index, child)| {
                let mut problems = Vec::new();
                if !child.attach.parent_matches {
                    problems.push("parent-mismatch");
                }
                if !child.attach.visible {
                    problems.push("invisible");
                }
                if !child.attach.alive {
                    problems.push("not-alive");
                }
                (!problems.is_empty()).then(|| {
                    format!(
                        "child[{index}] hwnd={:#x}: {}",
                        child.hwnd,
                        problems.join("+")
                    )
                })
            })
            .collect();
        if violations.is_empty() {
            steps.push(json!({ "step": "child_contract", "verdict": "pass" }));
        } else {
            record_obs_fail(
                steps,
                failure,
                "child_contract",
                format!("child HWND contract violated: {}", violations.join("; ")),
            );
            ok = false;
        }
    }

    // thread_affinity — recorded as `note`, never a verdict input:
    // the Issue #389 D1 contract wants attach on the container
    // HWND's pump-owning thread; the scenario drives `open` on its
    // own caller thread. The ids are evidence and the deviation is
    // a documented observation — not a Pass criterion, never
    // hidden, and not dressed up as a passed check.
    steps.push(json!({
        "step": "thread_affinity",
        "verdict": "note",
        "caller_thread": observation.caller_thread,
        "container_thread": observation.container_thread,
        "child_threads": observation
            .children
            .iter()
            .map(|child| child.thread_id)
            .collect::<Vec<_>>(),
        "note": "session.open/attached ran on the scenario caller \
                 thread, not the container HWND's pump-owning \
                 thread — Issue #389 D1 deviation recorded as \
                 observation, not a Pass criterion",
    }));
    ok
}

/// Phase-2 close — `session.close` runs the production `close_gui`
/// → `removed()` → `DestroyWindow` path. The rejecting redeemer is
/// never consulted (close carries no lease).
fn close_editor_session(
    sessions: &mut PluginEditorSessions,
    session_id: PluginEditorSessionId,
    host: &mut InProcessHost,
    handle: PluginHandle,
    steps: &mut Vec<Value>,
    failure: &mut Option<String>,
) -> bool {
    match sessions.session_mut(session_id).map(|session| {
        let mut close_ops = rejecting_editor_ops(host, handle);
        session.close(&mut close_ops)
    }) {
        Some(Ok(())) => {
            steps.push(json!({ "step": "session.close", "verdict": "pass" }));
            true
        }
        Some(Err(error)) => {
            record_obs_fail(steps, failure, "session.close", error.to_string());
            false
        }
        None => {
            record_obs_fail(
                steps,
                failure,
                "session.close",
                "editor session vanished before close",
            );
            false
        }
    }
}

/// Phase-3 post-detach check — every recorded child must be dead
/// (`IsWindow(child) == 0`, the `removed()` → `DestroyWindow`
/// effect). Runs only on Windows; the caller skips it off-Windows
/// where the `child_hwnd_observation` row already recorded
/// `unsupported`.
fn check_detached_children(
    observation: &mut ChildWindowObservation,
    steps: &mut Vec<Value>,
    failure: &mut Option<String>,
) -> bool {
    oschild::mark_destroyed(observation);
    if observation.children.is_empty() {
        record_obs_fail(
            steps,
            failure,
            "child_hwnd_detached",
            "no children recorded — nothing to verify destroyed",
        );
        return false;
    }
    let alive: Vec<String> = observation
        .children
        .iter()
        .filter(|child| !child.destroyed)
        .map(|child| format!("hwnd={:#x}", child.hwnd))
        .collect();
    if alive.is_empty() {
        steps.push(json!({
            "step": "child_hwnd_detached",
            "verdict": "pass",
            "children": observation
                .children
                .iter()
                .map(|child| json!({
                    "hwnd": child.hwnd,
                    "destroyed": child.destroyed,
                }))
                .collect::<Vec<_>>(),
        }));
        true
    } else {
        record_obs_fail(
            steps,
            failure,
            "child_hwnd_detached",
            format!(
                "child HWND(s) still alive after close: {}",
                alive.join(", ")
            ),
        );
        false
    }
}

/// Phase-4 session convergence — `Closed`+`concluded` and the
/// drained event stream must be exactly `[Attached{Docked},
/// Closed]`; an unexpected event (degrade, failure, terminate) is
/// a contract surprise, never a pass. Returns the checks' outcome
/// plus the rendered event list for `observed`.
fn check_session_converged(
    sessions: &mut PluginEditorSessions,
    session_id: PluginEditorSessionId,
    steps: &mut Vec<Value>,
    failure: &mut Option<String>,
) -> (bool, Vec<String>) {
    let mut ok = true;
    if session_closed_concluded(sessions, session_id) {
        steps.push(json!({ "step": "session_concluded", "verdict": "pass" }));
    } else {
        record_obs_fail(
            steps,
            failure,
            "session_concluded",
            "session did not converge to Closed+concluded",
        );
        ok = false;
    }

    let events = sessions
        .session_mut(session_id)
        .map_or_else(Vec::new, PluginEditorSession::drain_events);
    let rendered = events_debug(&events);
    let events_ok = events.len() == 2
        && events[0]
            == EditorSessionEvent::Attached {
                mode: AttachMode::Docked,
            }
        && events[1] == EditorSessionEvent::Closed;
    if events_ok {
        steps.push(json!({
            "step": "session_events",
            "verdict": "pass",
            "events": rendered,
        }));
    } else {
        record_obs_fail(
            steps,
            failure,
            "session_events",
            format!("expected [Attached{{Docked}}, Closed], got {rendered:?}"),
        );
        ok = false;
    }
    (ok, rendered)
}

/// `observed` extras — container/child HWND ids, owning thread ids,
/// and the drained event list.
fn child_extras(
    container: Option<u64>,
    observation: &ChildWindowObservation,
    events: &[String],
) -> Map<String, Value> {
    Map::from_iter([
        ("container".to_string(), json!(container)),
        (
            "child_hwnds".to_string(),
            json!(
                observation
                    .children
                    .iter()
                    .map(|child| child.hwnd)
                    .collect::<Vec<_>>()
            ),
        ),
        (
            "caller_thread".to_string(),
            json!(observation.caller_thread),
        ),
        (
            "container_thread".to_string(),
            json!(observation.container_thread),
        ),
        (
            "child_threads".to_string(),
            json!(
                observation
                    .children
                    .iter()
                    .map(|child| child.thread_id)
                    .collect::<Vec<_>>()
            ),
        ),
        ("events".to_string(), json!(events)),
    ])
}

/// Post-open child-HWND observation + close convergence for an
/// accepted `session.open(Docked)` — runs only when `open` returned
/// `Ok`, i.e. the plugin claims an editor surface exists and every
/// contract check below must agree for `Pass`.
///
/// Sequence: `EnumChildWindows(container)` while attached →
/// `session.close` (production `close_gui` → `removed()` →
/// `DestroyWindow`) → `IsWindow(child) == 0` per recorded child →
/// `session_closed_concluded` → `drain_events`. Every check lands
/// as a pass/fail row in `steps` (`unsupported` off-Windows);
/// `verified` is the Pass gate and `failure` names the first
/// violated check.
fn observe_editor_child_lifecycle(
    container: Option<u64>,
    sessions: &mut PluginEditorSessions,
    session_id: PluginEditorSessionId,
    host: &mut InProcessHost,
    handle: PluginHandle,
    steps: &mut Vec<Value>,
) -> ChildVerification {
    let mut failure: Option<String> = None;

    // Phase 1 — enumerate the container's children while the editor
    // is still attached. `container` is the opaque `container_token`
    // u64 (= the container HWND on Windows); `None` means the token
    // observation already failed — the child checks then fail
    // honestly downstream.
    let mut observation = match container {
        Some(token) => oschild::observe(token),
        None => oschild::empty(),
    };

    // The attach-phase checks run only where an HWND concept exists;
    // off-Windows the observation half records `Unsupported` and the
    // session still converges below.
    let attach_ok = if observation.supported {
        check_attached_children(&observation, container, steps, &mut failure)
    } else {
        steps.push(json!({
            "step": "child_hwnd_observation",
            "verdict": "unsupported",
            "detail": "OS child-HWND observation is Windows-only — an \
                       accepted open cannot be verified off-Windows",
        }));
        false
    };

    // Phase 2 — `session.close` runs the production `close_gui` →
    // `removed()` → `DestroyWindow` path.
    let close_ok = close_editor_session(sessions, session_id, host, handle, steps, &mut failure);

    // Phase 3 — post-detach: every recorded child must be dead
    // (`IsWindow(child) == 0`). Skipped off-Windows — the
    // `child_hwnd_observation` row already recorded `unsupported`.
    let detach_ok =
        !observation.supported || check_detached_children(&mut observation, steps, &mut failure);

    // Phase 4 — session convergence: `Closed`+`concluded` and the
    // drained event stream must be exactly `[Attached{Docked},
    // Closed]`.
    let (converged_ok, rendered) =
        check_session_converged(sessions, session_id, steps, &mut failure);

    ChildVerification {
        supported: observation.supported,
        verified: attach_ok && close_ok && detach_ok && converged_ok,
        failure,
        extras: Value::Object(child_extras(container, &observation, &rendered)),
    }
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

/// Observes the live container's opaque token while the lease is
/// held — the same HWND `redeem` hands to `attached`. A missing or
/// failed token is recorded honestly; the attach attempt still runs
/// on the lease itself and the child checks then fail downstream.
fn observe_container_token(
    gui: &RealOsGuiTest,
    slot: SurfaceSlotId,
    steps: &mut Vec<Value>,
) -> Option<u64> {
    match gui.native_container_token(slot) {
        Ok(Some(token)) => {
            steps.push(json!({
                "step": "container_token",
                "verdict": "pass",
                "token": token,
            }));
            Some(token)
        }
        Ok(None) => {
            steps.push(json!({
                "step": "container_token",
                "verdict": "fail",
                "error": "no live container after lease_parent",
            }));
            None
        }
        Err(error) => {
            steps.push(json!({
                "step": "container_token",
                "verdict": "fail",
                "error": error.to_string(),
            }));
            None
        }
    }
}

/// Drives `session.open(Docked)` through the real lease — the
/// session-facing `ParentSlotLease` wraps the registry's opaque
/// payload (never an OS handle) and the issuer-side redeem happens
/// inside `InProcessEditorOps`: `ParentSlotLease.raw()` →
/// `redeem_native_lease` (adapter validates against its live
/// ledger) → `ParentLease::new(seq, container_token)`.
fn open_session_docked(
    gui: &RealOsGuiTest,
    host: &mut InProcessHost,
    handle: PluginHandle,
    sessions: &mut PluginEditorSessions,
    session_id: PluginEditorSessionId,
    parent_slot_lease: ParentSlotLease,
) -> Option<Result<EditorCapabilities, EditorSessionError>> {
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
}

/// The real attach attempt — session begin, real-slot
/// declare/bind/project/`lease_parent` (real container HWND),
/// `container_token` observation, `ParentSlotLease` →
/// `InProcessEditorOps` redeem through
/// [`RealOsGuiTest::redeem_native_lease`], then `session.open(Docked)`.
/// An accepted open additionally runs the child-HWND observation +
/// close convergence ([`observe_editor_child_lifecycle`]). Always
/// tears down (session end, slot/lease release, instance destroy).
/// Returns the open outcome (or the setup failure), cleanup notes,
/// and — only when `open` returned `Ok` — the child verification
/// that gates the verdict.
fn attempt_real_attach(
    gui: &mut RealOsGuiTest,
    window: WindowId,
    view: ViewInstanceId,
    host: &mut InProcessHost,
    handle: PluginHandle,
    steps: &mut Vec<Value>,
) -> (RealAttachAttempt, Vec<String>, Option<ChildVerification>) {
    let mut cleanup = Vec::new();
    let mut sessions = PluginEditorSessions::new();

    let (slot, lease) = match lease_real_slot(gui, window, view, steps) {
        Ok(pair) => pair,
        Err(detail) => {
            let _ = host.execute(HostCommand::Destroy { handle });
            cleanup.push("slot-setup-failed/instance-destroyed".to_string());
            return (RealAttachAttempt::Setup(detail), cleanup, None);
        }
    };
    let key = slot_key();

    let container_token = observe_container_token(gui, slot, steps);

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
                None,
            );
        }
    };

    let parent_slot_lease = ParentSlotLease::new(lease.raw());
    let open_result = open_session_docked(
        gui,
        host,
        handle,
        &mut sessions,
        session_id,
        parent_slot_lease,
    );

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

    // An accepted open means the plugin claims an editor surface —
    // observe the real child HWND under the container, converge the
    // session through `close` (production `removed()` →
    // `DestroyWindow`), and verify detach + events. The observation
    // decides Pass vs Fail; it runs while the lease is still live.
    let child = match &open_result {
        Some(Ok(_)) => Some(observe_editor_child_lifecycle(
            container_token,
            &mut sessions,
            session_id,
            host,
            handle,
            steps,
        )),
        _ => None,
    };

    // Converge + teardown — a refused/failed open never produced an
    // adapter editor (`editor_live` stays false), so `close` is a
    // no-op adapter call and the rejecting redeemer is never
    // consulted. An accepted open was already closed inside the
    // child observation — `close` is then a defined `InvalidState`
    // no-op, skipped explicitly.
    if child.is_none()
        && let Some(session) = sessions.session_mut(session_id)
    {
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
    (attempt, cleanup, child)
}

/// `real-editor-attach`: drives the real attach path —
/// `PluginEditorSessions` → `InProcessEditorOps` → `OpenGui` —
/// against a *real* container-HWND lease redeemed through
/// [`RealOsGuiTest::redeem_native_lease`], now against the
/// `test_editor` fixture whose `IPlugView` creates a real child
/// HWND under the container. `Pass` requires `session.open(Docked)`
/// → `Ok` AND every OS observation agreeing (child exists/parented/
/// visible/live → destroyed after `close` → concluded →
/// `[Attached{Docked}, Closed]` events); any mismatch is `Fail`, a
/// defined refusal stays `Unsupported`.
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
    let (attempt, cleanup, child) =
        attempt_real_attach(gui, window, view, &mut host, handle, &mut steps);

    let mut outcome = match attempt {
        RealAttachAttempt::Open(Ok(_capabilities)) => match child.as_ref() {
            Some(verification) if verification.supported && verification.verified => {
                // The real `test_editor` IPlugView attached a real
                // child HWND under the leased container and detached
                // cleanly — every OS observation agreed.
                let mut outcome = ScenarioOutcome::new(ScenarioVerdict::Pass);
                outcome.detail = Some(
                    "test_editor real IPlugView attached a real child HWND \
                     under the leased container HWND and detached on close \
                     (EnumChildWindows/GetParent/IsWindowVisible observed; \
                     IsWindow(child)==0 after removed(); events \
                     [Attached{Docked}, Closed])"
                        .to_string(),
                );
                outcome
                    .evidence
                    .push("editor-attach:real-child-hwnd-lifecycle".to_string());
                outcome
            }
            Some(verification) if !verification.supported => {
                // An accepted open off-Windows — the OS observation
                // cannot verify the child contract there, so the
                // case honestly stays `Unsupported` (unreachable
                // today: `NativePluginDock` is Windows-only).
                let mut outcome = ScenarioOutcome::new(ScenarioVerdict::Unsupported);
                outcome.detail = Some(
                    "editor opened through the REAL container lease but OS \
                     child-HWND observation is unsupported on this platform"
                        .to_string(),
                );
                outcome
            }
            Some(verification) => fail(format!(
                "editor opened through the REAL container lease but the \
                 child-HWND observation failed: {}",
                verification
                    .failure
                    .as_deref()
                    .unwrap_or("unknown mismatch")
            )),
            None => fail("editor opened but no child-HWND observation was recorded".to_string()),
        },
        RealAttachAttempt::Open(Err(EditorSessionError::Ops(EditorOpError::Unsupported))) => {
            let mut outcome = ScenarioOutcome::new(ScenarioVerdict::Unsupported);
            outcome.detail = Some(
                "plugin declined OpenGui through the REAL container lease: \
                 createView/isPlatformTypeSupported/attached refused → \
                 HostError::Unsupported → EditorOpError::Unsupported; \
                 the slot/lease/redeem chain itself executed on a real \
                 OS window"
                    .to_string(),
            );
            outcome
                .evidence
                .push("editor-attach:open-gui-declined-real-lease".to_string());
            outcome
        }
        RealAttachAttempt::Open(Err(error)) => fail(format!("session.open: {error}")),
        RealAttachAttempt::Setup(detail) => fail(detail),
    };
    let mut observed = Map::from_iter([
        ("steps".to_string(), json!(steps)),
        ("cleanup".to_string(), json!(cleanup)),
    ]);
    if let Some(verification) = &child
        && let Value::Object(fields) = &verification.extras
    {
        observed.extend(fields.clone());
    }
    outcome.observed = Some(Value::Object(observed));
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
/// this case never attaches an editor (attach is
/// `real-editor-attach`'s job).
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
                     editor surface is attached in this case",
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
    use crate::real_catalog::REAL_CATALOG;

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

    /// The console's `real_catalog::REAL_CATALOG` metadata must agree
    /// with the executable `real_entries` table — case id, scenario
    /// id, and required capabilities are asserted pairwise so drift
    /// between the listing projection and the live catalog fails
    /// here, not in evidence review.
    #[test]
    fn real_catalog_matches_entries() {
        let entries: [ScenarioEntry<RealDockContext<'static>>; 4] = real_entries();
        assert_eq!(entries.len(), REAL_CATALOG.len(), "catalog row count");
        for (entry, row) in entries.iter().zip(REAL_CATALOG.iter()) {
            assert_eq!(entry.case, row.case, "case id drift");
            assert_eq!(entry.scenario, row.scenario, "scenario id drift");
            assert_eq!(
                entry.required, row.required,
                "required-capability drift for `{}`",
                entry.case
            );
        }
    }
}
