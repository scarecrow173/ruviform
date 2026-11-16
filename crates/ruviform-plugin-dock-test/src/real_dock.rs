//! Real-window plugin-dock lane (Issue #631 real-window slice —
//! feature `real-dock`; `real-dcomp-pixel` added by the Issue #532
//! dcomp-pixel slice).
//!
//! [`run_all`] mounts [`plugin_dock_view_spec`] into *real OS
//! windows* through [`RealOsGuiTest`] and records the thirteen cases
//! of the `rf631.plugin-dock-real` scenario:
//!
//! | case                    | driver      | honest outcome today                              |
//! |-------------------------|-------------|----------------------------------------------------|
//! | `real-window-mount`     | `RealOs`    | `Pass` — spec mounts; `NativeSlot` resolves/observes |
//! | `real-native-slot`      | `RealOs`    | `Pass`/`Fail` — declare→bind→project→`lease_parent`→ |
//! |                         |             | validate→`container_token`→release + stale/forged  |
//! |                         |             | lease rejection, all real ops                      |
//! | `real-editor-attach`    | `RealOs`    | `Pass`/`Fail`/`Unsupported` — `test_editor` real   |
//! |                         |             | `IPlugView` child HWND under the real container    |
//! |                         |             | lease, OS-observed attach → detach (`IsWindow==0`) |
//! | `real-capture`          | `RealOs`    | `Pass`/`Unsupported`/`NotRun` — window/slot-region   |
//! |                         |             | pixels, explicitly NOT plugin editor pixels        |
//! | `real-dcomp-pixel`      | `RealOs`    | `Pass`/`Fail`/`NotRun`/`Unsupported` — docked       |
//! |                         |             | `test_editor` child HWND の**画面合成画素**に       |
//! |                         |             | `EDITOR_COLOR` 緑が出るか（screen `BitBlt` が主     |
//! |                         |             | oracle、`PrintWindow(child)` は補助診断）。既知の    |
//! |                         |             | GPUI `DComp` topmost occlusion 下では正直な `Fail` |
//! | `real-multiwindow-focus`| `RealOs`    | `Pass`/`Fail`/`NotRun` — 2 window の `request_focus` |
//! |                         |             | → `window_focused` 観測による focus 移譲            |
//! | `real-os-input`         | `RealOs`    | `Pass`/`Fail`/`NotRun`/`Unsupported` — preflight     |
//! |                         |             | effective な実 OS pointer/key 注入が `Injected`     |
//! | `real-window-dpi`       | `RealOs`    | `Unsupported` — scale-factor/DPI API が公開面に無い   |
//! | `real-window-ime`       | `RealOs`    | `Unsupported` — `Ime` capability は未 claim（#389）  |
//! | `real-window-close`     | `RealOs`    | `Unsupported` — window close API が公開面に無い       |
//! | `real-editor-resize`    | `RealOs`    | `Unsupported`/`Fail` — `set_size` の定義済み拒否を    |
//! |                         |             | 非画素 oracle（session 状態・event・HWND）で観測      |
//! | `real-editor-redock`    | `RealOs`    | `Unsupported`/`Fail` — `reattach` の定義済み拒否 →    |
//! |                         |             | `Failed` → close 収束を event 系列で観測             |
//! | `real-session-terminate`| `RealOs`    | `Pass`/`Fail`/`Unsupported` — attach 中の            |
//! |                         |             | `HostCommand::Destroy` → `Terminated{HostLost}`       |
//! |                         |             | exactly-once → child HWND teardown → host 存続        |
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
    EditorSessionState, EditorSize, HostCommand, HostError, HostReply, InProcessEditorOps,
    InProcessHost, ParentLease, ParentSlotLease, PluginEditorSession, PluginEditorSessionId,
    PluginEditorSessions, PluginHandle, SessionFailCause,
};
use ruviform_plugin_vst3::host::Vst3Library;
use ruviform_ui::dock::ViewInstanceId;
use ruviform_ui::native_slot::{
    LogicalRect, SlotCapabilities, SlotGeometry, SlotLease, SlotSpec, SlotVisibility, Support,
    SurfaceSlotId,
};
use ruviform_ui::view::{AttachEpochAllocator, ComponentKey, ViewHost};
use ruviform_ui::{LogicalPosition, WindowId, WindowSpec};
use ruviform_ui_gpui::os_input::{OsInputOutcome, OsKey, PreflightScope};
use ruviform_ui_gpui::{RealOsGuiTest, RealWindowMount, RealWindowOptions, ViewMount};
use ruviform_ui_test::scenario::plugin_dock::{
    PLUGIN_DOCK_SLOT_KEY, PluginArtifactEvidence, plugin_dock_view_spec,
};
use ruviform_ui_test::scenario::visual::{CaptureOutcome, VisualGuiTest};
use ruviform_ui_test::{
    CaseId, RunId, ScenarioCapability, ScenarioDriver, ScenarioEntry, ScenarioError, ScenarioId,
    ScenarioOutcome, ScenarioReport, ScenarioSpec, ScenarioSpecExtras, ScenarioTarget,
    ScenarioVerdict, SemanticGuiTest, TargetQuery, run_entries, run_scenario, settle_until,
};
use std::path::{Path, PathBuf};

use ruviform_ui_test::golden::RgbaImage;
use serde_json::{Map, Value, json};

use crate::LaneEnvironment;
use crate::artifact;
use crate::osoccl;
use crate::ospix;
use crate::real_catalog::{LANE, SCENARIO};
// The run id is shared with the console's catalog metadata —
// re-exported so `real_dock::RUN_ID` keeps working for the lane.
pub use crate::real_catalog::RUN_ID;
use crate::scenario::{
    activate_test_gain, editor_size, events_debug, fail, finalize, host_error_outcome, not_run,
    register_factories, rejecting_editor_ops, session_closed_concluded, session_failed_unsupported,
    session_id_json,
};

/// The resolved plugin binary + its evidence identity — only the
/// editor cases (`real-editor-attach`, `real-dcomp-pixel`) consume
/// it.
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
/// environment, the plugin pair/error/evidence resolved up front,
/// and the opt-in pixel-artifact directory. Only the editor cases
/// consume the plugin/artifact fields; the window-only cases ignore
/// them.
struct RealDockContext<'a> {
    /// The live session driver — `run_scenario`'s `G`.
    gui: &'a mut RealOsGuiTest,
    /// The first mounted window id (`windows()` order = mount order).
    window: WindowId,
    /// The second mounted window id — `real-multiwindow-focus`'s
    /// focus-transfer target. `None` only if the session mounted a
    /// single window (never on the live path — two mounts are
    /// requested up front).
    window_b: Option<WindowId>,
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
    /// Opt-in pixel-artifact directory for `real-dcomp-pixel`
    /// (`--pixel-artifacts`); `None` records stats only.
    pixel_artifacts: Option<&'a Path>,
}

/// Runs the catalog's real-window cases inside one `RealOsGuiTest`
/// session and returns their reports in case order — a thin
/// projection over [`real_entries`], the session-scoped catalog the
/// blocked path also reads so case ids/required caps never drift.
///
/// The plugin artifact is resolved/hashed/loaded once up front — a
/// missing or unloadable binary degrades only the editor cases
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
    run_all_with_pixel_artifacts(env, None)
}

/// [`run_all`] plus the opt-in pixel-artifact path: when `dir` is
/// `Some`, `real-dcomp-pixel` writes each captured frame as a PNG
/// under `dir` (created on demand). Artifact write failures land as
/// step notes — they never change a verdict. The canonical lane
/// calls [`run_all`], so no artifacts are written by default.
///
/// # Panics
/// Same contract as [`run_all`].
#[must_use]
pub fn run_all_with_pixel_artifacts(
    env: &LaneEnvironment,
    pixel_artifacts: Option<&Path>,
) -> Vec<ScenarioReport> {
    let run_id = RunId::new(RUN_ID);
    let plugin = load_plugin(env);
    let plugin_evidence = plugin.as_ref().ok().map(|(_, evidence)| evidence.clone());
    let plugin_error = plugin.as_ref().err().cloned();

    let view = ViewInstanceId::new(631).expect("view instance id 631 is non-zero");
    let view_b = ViewInstanceId::new(632).expect("view instance id 632 is non-zero");
    let mut epochs = AttachEpochAllocator::new();
    let mut host = ViewHost::new(view, epochs.mint());
    host.mount(plugin_dock_view_spec())
        .expect("plugin dock view spec mounts");
    host.drain_events();
    let mut host_b = ViewHost::new(view_b, epochs.mint());
    host_b
        .mount(plugin_dock_view_spec())
        .expect("plugin dock view spec mounts");
    host_b.drain_events();

    // Two mounts so `real-multiwindow-focus` has a real second
    // window; window B is offset right of window A so the windows
    // never overlap (an OS click on A's panel center must land on A).
    let window_spec_b = WindowSpec {
        title: "ruviform plugin-dock B".to_string(),
        position: Some(LogicalPosition::new(1450, 80)),
        ..WindowSpec::default()
    };
    let outcome = RealOsGuiTest::run(
        vec![
            RealWindowMount {
                window: WindowSpec::default(),
                view: ViewMount { host, theme: None },
            },
            RealWindowMount {
                window: window_spec_b,
                view: ViewMount {
                    host: host_b,
                    theme: None,
                },
            },
        ],
        RealWindowOptions::default(),
        |gui| {
            let mut windows = gui.windows().into_iter();
            let Some(window) = windows.next() else {
                return blocked_reports(
                    &run_id,
                    env,
                    plugin_evidence.as_ref(),
                    "real-window session reported no mounted window",
                );
            };
            let window_b = windows.next();
            // The session-scoped catalog — `real_entries` rows run
            // through `run_entries` against the live context.
            let entries = real_entries();
            let mut ctx = RealDockContext {
                gui,
                window,
                window_b,
                view,
                env,
                plugin: plugin.as_ref().ok(),
                plugin_error: plugin_error.as_ref(),
                artifact: plugin_evidence.as_ref(),
                pixel_artifacts,
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

/// `real-editor-attach`/`real-dcomp-pixel` spec extras — the lane
/// marker plus the plugin artifact identity when the binary
/// resolved (what `real_spec` used to stamp under
/// `environment.plugin_artifact`).
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

/// The raw `GPUI_DISABLE_DIRECT_COMPOSITION` value for evidence —
/// `null` when unset. GPUI reads it once in `WindowsPlatform::new`
/// at process start, so the recorded value is exactly what this run
/// saw (方式 A' — the Issue #532 audit accepts only `"true"`/`"1"`;
/// any other value is recorded verbatim, never interpreted here).
fn dcomp_env_extra() -> Map<String, Value> {
    Map::from_iter([(
        "gpui_disable_direct_composition".to_string(),
        std::env::var_os("GPUI_DISABLE_DIRECT_COMPOSITION").map_or(Value::Null, |value| {
            Value::String(value.to_string_lossy().into_owned())
        }),
    )])
}

/// [`finalize`] for real-lane reports — merges the raw
/// `GPUI_DISABLE_DIRECT_COMPOSITION` value into `environment` so
/// every JSONL line self-describes which compositor path the
/// process actually took (unset → `null`).
fn finalize_real(
    report: ScenarioReport,
    env: &LaneEnvironment,
    kind: EnvironmentKind,
    artifact: Option<&PluginArtifactEvidence>,
    extra: Map<String, Value>,
) -> ScenarioReport {
    let mut merged = dcomp_env_extra();
    merged.extend(extra);
    finalize(report, env, kind, artifact, merged)
}

/// The `rf631.plugin-dock-real` catalog rows — session-scoped: the
/// entries' `run` needs the live `RealOsGuiTest`, so the table is
/// produced per call (the entries themselves are `'static` fn items;
/// only the context type carries the session borrow).
/// `blocked_reports` reads the same rows for case ids and required
/// capabilities, so the blocked path can never drift from the live
/// catalog.
fn real_entries<'a>() -> [ScenarioEntry<RealDockContext<'a>>; 13] {
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
        ScenarioEntry {
            case: "real-dcomp-pixel",
            scenario: SCENARIO,
            required: &[
                ScenarioCapability::NativePluginDock,
                ScenarioCapability::VisualCapture,
            ],
            spec_extras: editor_attach_extras,
            run: real_dcomp_pixel_case,
        },
        ScenarioEntry {
            case: "real-multiwindow-focus",
            scenario: SCENARIO,
            required: &[ScenarioCapability::MultiWindow],
            spec_extras: real_window_extras,
            run: real_multiwindow_focus_case,
        },
        ScenarioEntry {
            case: "real-os-input",
            scenario: SCENARIO,
            required: &[ScenarioCapability::RealOsPointerOrKeyboard],
            spec_extras: real_window_extras,
            run: real_os_input_case,
        },
        ScenarioEntry {
            case: "real-window-dpi",
            scenario: SCENARIO,
            required: &[ScenarioCapability::SemanticAction],
            spec_extras: real_window_extras,
            run: real_window_dpi_case,
        },
        ScenarioEntry {
            case: "real-window-ime",
            scenario: SCENARIO,
            required: &[ScenarioCapability::Ime],
            spec_extras: real_window_extras,
            run: real_window_ime_case,
        },
        ScenarioEntry {
            case: "real-window-close",
            scenario: SCENARIO,
            required: &[ScenarioCapability::SemanticAction],
            spec_extras: real_window_extras,
            run: real_window_close_case,
        },
        ScenarioEntry {
            case: "real-editor-resize",
            scenario: SCENARIO,
            required: &[ScenarioCapability::NativePluginDock],
            spec_extras: editor_attach_extras,
            run: real_editor_resize_case,
        },
        ScenarioEntry {
            case: "real-editor-redock",
            scenario: SCENARIO,
            required: &[ScenarioCapability::NativePluginDock],
            spec_extras: editor_attach_extras,
            run: real_editor_redock_case,
        },
        ScenarioEntry {
            case: "real-session-terminate",
            scenario: SCENARIO,
            required: &[ScenarioCapability::NativePluginDock],
            spec_extras: editor_attach_extras,
            run: real_session_terminate_case,
        },
    ]
}

/// Resolves + hashes + loads the test-plugin cdylib once for the
/// lane and selects its `test_editor` descriptor — only the editor
/// cases consume it. `Err` carries the honest
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

/// The fail-closed reports for a session that never started —
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
    let entries: [ScenarioEntry<RealDockContext<'static>>; 13] = real_entries();
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
            finalize_real(
                report,
                env,
                EnvironmentKind::DeterministicBackend,
                artifact,
                Map::new(),
            )
        })
        .collect()
}

/// `--blocked-probe` reports — the fail-closed mapping a
/// `RealOsGuiTest::run` startup failure produces (verdict,
/// `observed`, and `environment_kind` identical; artifact evidence
/// aside, since no session resolved one), generated WITHOUT
/// opening a session so the proof is deterministic on any host
/// (interactive or headless). The detail string says plainly that
/// no session start was attempted — these reports evidence the
/// *mapping*, never an observed headless run.
#[must_use]
pub fn blocked_probe(env: &LaneEnvironment) -> Vec<ScenarioReport> {
    blocked_reports(
        &RunId::new(RUN_ID),
        env,
        None,
        "blocked-probe: RealOsGuiTest session start not attempted — \
         exercising the fail-closed mapping a headless host hits",
    )
}

/// Self-check for the blocked-probe lane: every report must be
/// `NotRun` + `observed.environment_blocked` and none may claim
/// `environment_kind == "real-window"` — the fail-closed shape a
/// headless host must produce. Returns violation strings; empty
/// means the shape held for every catalog row.
#[must_use]
pub fn blocked_probe_violations(reports: &[ScenarioReport]) -> Vec<String> {
    let entries: [ScenarioEntry<RealDockContext<'static>>; 13] = real_entries();
    let mut violations = Vec::new();
    if reports.len() != entries.len() {
        violations.push(format!(
            "report count {} != catalog rows {}",
            reports.len(),
            entries.len()
        ));
    }
    for (index, report) in reports.iter().enumerate() {
        let case = report
            .case
            .as_ref()
            .map_or_else(|| format!("row[{index}]"), |id| id.as_str().to_string());
        if report.verdict != ScenarioVerdict::NotRun {
            violations.push(format!("{case}: verdict {:?} != NotRun", report.verdict));
        }
        if let Some(entry) = entries.get(index)
            && report.case.as_ref().map(CaseId::as_str) != Some(entry.case)
        {
            violations.push(format!(
                "{case}: case id does not match catalog row {index} ({})",
                entry.case
            ));
        }
        if report.driver != ScenarioDriver::RealOs {
            violations.push(format!("{case}: driver {:?} != RealOs", report.driver));
        }
        let observed_ok = report
            .observed
            .as_ref()
            .and_then(|observed| observed["observed"].as_str())
            == Some("environment_blocked");
        if !observed_ok {
            violations.push(format!("{case}: observed.environment_blocked missing"));
        }
        let kind = report
            .environment
            .as_ref()
            .and_then(|env| env["environment_kind"].as_str());
        match kind {
            Some("real-window") => {
                violations.push(format!("{case}: claims environment_kind=real-window"));
            }
            None => {
                violations.push(format!("{case}: environment_kind missing"));
            }
            _ => {}
        }
    }
    violations
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
    finalize_real(
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
    finalize_real(
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
    /// Every descendant `EnumChildWindows` found under the
    /// container (the API recurses), in enumeration order.
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

    /// Collects `container`'s descendant child HWNDs (the
    /// `EnumChildWindows` walk recurses) and their contract
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

/// Phases 2-4 shared by the attach and pixel lifecycles —
/// `session.close` (production `close_gui` → `removed()` →
/// `DestroyWindow`), the post-detach `IsWindow(child) == 0` check
/// (skipped off-Windows where `child_hwnd_observation` already
/// recorded `unsupported`), and session convergence
/// (`Closed`+`concluded`, drained events exactly
/// `[Attached{Docked}, Closed]`). Returns `(close_ok, detach_ok,
/// converged_ok, rendered_events)`.
fn conclude_editor_session(
    observation: &mut ChildWindowObservation,
    sessions: &mut PluginEditorSessions,
    session_id: PluginEditorSessionId,
    host: &mut InProcessHost,
    handle: PluginHandle,
    steps: &mut Vec<Value>,
    failure: &mut Option<String>,
) -> (bool, bool, bool, Vec<String>) {
    // Phase 2 — `session.close` runs the production `close_gui` →
    // `removed()` → `DestroyWindow` path.
    let close_ok = close_editor_session(sessions, session_id, host, handle, steps, failure);

    // Phase 3 — post-detach: every recorded child must be dead
    // (`IsWindow(child) == 0`). Skipped off-Windows — the
    // `child_hwnd_observation` row already recorded `unsupported`.
    let detach_ok = !observation.supported || check_detached_children(observation, steps, failure);

    // Phase 4 — session convergence: `Closed`+`concluded` and the
    // drained event stream must be exactly `[Attached{Docked},
    // Closed]`.
    let (converged_ok, rendered) = check_session_converged(sessions, session_id, steps, failure);
    (close_ok, detach_ok, converged_ok, rendered)
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
    _lease_raw: u64,
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
    let (mut observation, attach_ok) = observe_attached_children(container, steps, &mut failure);

    let (close_ok, detach_ok, converged_ok, rendered) = conclude_editor_session(
        &mut observation,
        sessions,
        session_id,
        host,
        handle,
        steps,
        &mut failure,
    );

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
/// An accepted open additionally runs `observe_open` — the case's
/// post-open observation + close convergence — while the lease is
/// still live. Always tears down (session end, slot/lease release,
/// instance destroy). Returns the open outcome (or the setup
/// failure), cleanup notes, and — only when `open` returned `Ok` —
/// the observation result that gates the verdict.
fn attempt_real_attach<V>(
    gui: &mut RealOsGuiTest,
    window: WindowId,
    view: ViewInstanceId,
    host: &mut InProcessHost,
    handle: PluginHandle,
    steps: &mut Vec<Value>,
    observe_open: impl FnOnce(
        Option<u64>,
        u64,
        &mut PluginEditorSessions,
        PluginEditorSessionId,
        &mut InProcessHost,
        PluginHandle,
        &mut Vec<Value>,
    ) -> V,
) -> (RealAttachAttempt, Vec<String>, Option<V>) {
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
    // run the case's post-open observation (child-HWND contract for
    // `real-editor-attach`, screen pixels for `real-dcomp-pixel`)
    // and close convergence. The observation decides Pass vs Fail;
    // it runs while the lease is still live.
    let observed = match &open_result {
        Some(Ok(_)) => Some(observe_open(
            container_token,
            lease.raw(),
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
    // observation — `close` is then a defined `InvalidState`
    // no-op, skipped explicitly.
    if observed.is_none()
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
    (attempt, cleanup, observed)
}

/// Shared preamble for the editor cases — resolve the plugin pair,
/// build the host, activate the `test_editor` instance. Returns the
/// honest early `ScenarioOutcome` (`NotRun` for an absent artifact,
/// `Fail` for a present-but-broken one, the mapped host error)
/// when any prerequisite fails. The outcome is boxed — it is large
/// and only ever returned, never inspected.
fn prepare_editor_host(
    plugin: Option<&PluginPair>,
    plugin_error: Option<&PluginUnavailable>,
) -> Result<(InProcessHost, PluginHandle), Box<ScenarioOutcome>> {
    let Some((library, artifact)) = plugin else {
        return Err(Box::new(match plugin_error {
            Some(unavailable) if unavailable.prerequisite => not_run(format!(
                "plugin artifact unavailable: {}",
                unavailable.detail
            )),
            Some(unavailable) => fail(format!(
                "plugin artifact failed to load: {}",
                unavailable.detail
            )),
            None => not_run("plugin artifact unavailable".to_string()),
        }));
    };
    let plugin_id = artifact.plugin_id.clone();
    let mut host = InProcessHost::new();
    register_factories(&mut host, library);

    let handle = match activate_test_gain(&mut host, &plugin_id) {
        Ok(handle) => handle,
        Err(error) => return Err(Box::new(host_error_outcome("activate", error))),
    };
    Ok((host, handle))
}

/// The honest outcome for a refused/failed attach attempt — `None`
/// when `open` returned `Ok` (the case's observation then decides).
/// The defined `Unsupported` refusal stays `Unsupported`; everything
/// else is `Fail`. Shared by the editor cases' verdict mapping.
fn attach_attempt_failure(attempt: &RealAttachAttempt) -> Option<ScenarioOutcome> {
    match attempt {
        RealAttachAttempt::Open(Ok(_)) => None,
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
            Some(outcome)
        }
        RealAttachAttempt::Open(Err(error)) => Some(fail(format!("session.open: {error}"))),
        RealAttachAttempt::Setup(detail) => Some(fail(detail.clone())),
    }
}

/// Case identity for [`editor_child_body`] — the strings that name
/// this lifecycle path in reports: the `Pass` detail, the evidence
/// tag, and the observation label used in `Fail` details.
struct ChildCaseIdentity<'a> {
    /// `Pass` detail — names which lifecycle path ran (graceful
    /// close vs host loss).
    pass_detail: &'a str,
    /// Evidence tag pushed onto `Pass` outcomes.
    evidence: &'a str,
    /// Observation label named in `Fail` details.
    label: &'a str,
}

/// Shared body for the child-HWND lifecycle cases
/// (`real-editor-attach`, `real-session-terminate`) — identical
/// prerequisite/attempt mapping; the case-supplied `observe_open`
/// drives the post-open observation and decides the verdict:
/// verified → `Pass` (`pass_detail`/`evidence`), platform-unsupported
/// observation → `Unsupported`, any contract mismatch → `Fail`.
fn editor_child_body(
    gui: &mut RealOsGuiTest,
    window: WindowId,
    view: ViewInstanceId,
    plugin: Option<&PluginPair>,
    plugin_error: Option<&PluginUnavailable>,
    observe_open: impl FnOnce(
        Option<u64>,
        u64,
        &mut PluginEditorSessions,
        PluginEditorSessionId,
        &mut InProcessHost,
        PluginHandle,
        &mut Vec<Value>,
    ) -> ChildVerification,
    case: &ChildCaseIdentity<'_>,
) -> ScenarioOutcome {
    let (mut host, handle) = match prepare_editor_host(plugin, plugin_error) {
        Ok(pair) => pair,
        Err(outcome) => return *outcome,
    };

    let mut steps = Vec::new();
    let (attempt, cleanup, verification) = attempt_real_attach(
        gui,
        window,
        view,
        &mut host,
        handle,
        &mut steps,
        observe_open,
    );

    let mut outcome = match attach_attempt_failure(&attempt) {
        Some(outcome) => outcome,
        None => match verification.as_ref() {
            Some(verification) if verification.supported && verification.verified => {
                // The real `test_editor` IPlugView attached a real
                // child HWND under the leased container and every OS
                // observation agreed — the case's own detail names
                // which lifecycle path ran (graceful close vs host
                // loss).
                let mut outcome = ScenarioOutcome::new(ScenarioVerdict::Pass);
                outcome.detail = Some(case.pass_detail.to_string());
                outcome.evidence.push(case.evidence.to_string());
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
                 {} observation failed: {}",
                case.label,
                verification
                    .failure
                    .as_deref()
                    .unwrap_or("unknown mismatch")
            )),
            None => fail(format!(
                "editor opened but no {} observation was recorded",
                case.label
            )),
        },
    };
    let mut observed = Map::from_iter([
        ("steps".to_string(), json!(steps)),
        ("cleanup".to_string(), json!(cleanup)),
    ]);
    if let Some(verification) = &verification
        && let Value::Object(fields) = &verification.extras
    {
        observed.extend(fields.clone());
    }
    outcome.observed = Some(Value::Object(observed));
    outcome
}

/// `real-editor-attach` body — the graceful-close child-HWND
/// lifecycle through [`editor_child_body`].
fn editor_attach_body(
    gui: &mut RealOsGuiTest,
    window: WindowId,
    view: ViewInstanceId,
    plugin: Option<&PluginPair>,
    plugin_error: Option<&PluginUnavailable>,
) -> ScenarioOutcome {
    editor_child_body(
        gui,
        window,
        view,
        plugin,
        plugin_error,
        observe_editor_child_lifecycle,
        &ChildCaseIdentity {
            pass_detail: "test_editor real IPlugView attached a real child \
                          HWND under the leased container HWND and detached \
                          on close (EnumChildWindows/GetParent/IsWindowVisible \
                          observed; IsWindow(child)==0 after removed(); events \
                          [Attached{Docked}, Closed])",
            evidence: "editor-attach:real-child-hwnd-lifecycle",
            label: "child-HWND",
        },
    )
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
    finalize_real(
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
    finalize_real(
        report,
        ctx.env,
        EnvironmentKind::RealWindow,
        None,
        Map::new(),
    )
}

// ---------------------------------------------------------------------
// case: real-dcomp-pixel (docked editor screen-composited pixels —
// the Issue #532 DComp topmost occlusion regression)
// ---------------------------------------------------------------------

/// `EDITOR_COLOR` visibility threshold — the fraction of a child's
/// screen-captured pixels that must match the `test_editor` green
/// for the surface to count as "on screen". The fixture fills its
/// whole client area with one solid brush, so a correctly composed
/// surface lands near 1.0 and a fully occluded one near 0.0; 0.5
/// separates the two unambiguously. The measured fraction is always
/// recorded in the step rows, so the threshold stays reviewable.
const GREEN_VISIBLE_FRACTION: f64 = 0.5;

/// `EDITOR_COLOR` (`0x00_00_FF_00` = RGB(0,255,0)) tolerance box —
/// a solid-brush fill survives compositing unchanged, so any pixel
/// inside this box counts as editor green.
fn is_editor_green(r: u8, g: u8, b: u8) -> bool {
    g >= 200 && r <= 80 && b <= 80
}

/// `(green_pixels, total_pixels)` over an RGBA8 frame.
fn green_stats(rgba: &[u8]) -> (u64, u64) {
    let mut green = 0u64;
    let mut total = 0u64;
    for px in rgba.as_chunks::<4>().0 {
        total += 1;
        if is_editor_green(px[0], px[1], px[2]) {
            green += 1;
        }
    }
    (green, total)
}

/// `green/total` as `f64` — counts saturate at `u32::MAX` before the
/// (exact) `u32`→`f64` conversion so no lossy `u64` cast is needed;
/// real captures are far below that bound. `total == 0` reports 0.0.
fn green_fraction(green: u64, total: u64) -> f64 {
    let g = u32::try_from(green).unwrap_or(u32::MAX);
    let t = u32::try_from(total).unwrap_or(u32::MAX).max(1);
    f64::from(g) / f64::from(t)
}

/// Writes one captured frame as PNG under `dir` — opt-in artifact
/// evidence for `real-dcomp-pixel`. The directory is created on
/// demand; the returned path lands in the step row. A write failure
/// is returned to the caller, which records it as a note — it never
/// changes a verdict.
fn save_pixel_artifact(
    dir: &Path,
    name: &str,
    capture: &ospix::PixelCapture,
) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let image = RgbaImage::new(capture.width, capture.height, capture.rgba.clone())
        .map_err(|e| format!("RgbaImage::new: {e}"))?;
    let png = image.to_png().map_err(|e| format!("to_png: {e}"))?;
    let path = dir.join(format!("{name}.png"));
    std::fs::write(&path, png).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(path)
}

/// The aggregated screen-pixel outcome — this case's verdict input
/// on top of the shared lifecycle gate.
enum PixelObservation {
    /// At least one child's screen-composited pixels met the green
    /// threshold — the editor surface is actually on screen.
    GreenSeen,
    /// Every evaluable child was captured and none met the
    /// threshold — the honest `DComp`-occlusion input. Carries the
    /// best observed fraction/backend for the `Fail` detail.
    NoGreen {
        /// Highest `green_fraction` observed across children.
        best_fraction: f64,
        /// Backend that produced the best frame.
        best_backend: &'static str,
        /// How many children produced a screen capture.
        evaluated: usize,
    },
    /// Screen pixels could not be evaluated at all (window not
    /// presented, no container token, every capture failed) — the
    /// honest `NotRun` input, never a fabricated `Fail`.
    NotEvaluable(String),
}

/// The verdict-carrying result of the post-open pixel observation —
/// `lifecycle_ok` is the same gate `real-editor-attach` applies;
/// `pixels` is this case's additional oracle.
struct PixelVerification {
    /// `false` off-Windows — the observation half then reports
    /// `Unsupported`, never a fabricated pass.
    supported: bool,
    /// Every lifecycle contract check agreed (attach contract,
    /// close, detach, session convergence) — the same checks as
    /// `real-editor-attach`.
    lifecycle_ok: bool,
    /// First violated lifecycle check (`name: detail`).
    lifecycle_failure: Option<String>,
    /// The aggregated screen-pixel outcome.
    pixels: PixelObservation,
    /// `observed` extras — container/child ids, threads, events,
    /// pixel summary (an `Object`, like `ChildVerification`).
    extras: Value,
}

/// Per-child screen-pixel observation while the editor is still
/// attached — the Issue #532 regression oracle. For each observed
/// child HWND: `GetWindowRect` → screen-DC `BitBlt` of the screen
/// rectangle (the composited result the user sees — the primary
/// oracle) → `PrintWindow(child)` and `GetDC`+`BitBlt` of the
/// child's own surface (two `note`-level diagnostics that separate
/// "the plugin painted green" from "the screen shows green").
/// Every capture lands as a step row with backend + green stats;
/// opt-in PNG artifacts are written when `artifacts` is `Some`.
fn observe_child_pixels(
    observation: &ChildWindowObservation,
    container: Option<u64>,
    artifacts: Option<&Path>,
    steps: &mut Vec<Value>,
) -> PixelObservation {
    // Evaluability — the top-level window must be presented or the
    // screen says nothing about the editor (minimized/hidden →
    // honest `not_run`, never a fabricated Fail).
    fn not_evaluable(steps: &mut Vec<Value>, reason: &str) -> PixelObservation {
        steps.push(json!({
            "step": "screen_pixels",
            "verdict": "not_run",
            "reason": reason,
        }));
        PixelObservation::NotEvaluable(reason.to_string())
    }
    let Some(token) = container else {
        return not_evaluable(
            steps,
            "no container token — children cannot be located on screen",
        );
    };
    match ospix::presented(token) {
        Some(true) => {}
        Some(false) => {
            return not_evaluable(
                steps,
                "top-level window is minimized or hidden — no screen pixels",
            );
        }
        None => return not_evaluable(steps, "top-level presentation query failed"),
    }

    let mut scan = PixelScan::default();
    for (index, child) in observation.children.iter().enumerate() {
        observe_one_child_pixels(index, child, token, artifacts, steps, &mut scan);
    }

    if scan.evaluated == 0 {
        PixelObservation::NotEvaluable(format!(
            "no child produced a screen capture{}",
            scan.first_error
                .map_or_else(String::new, |e| format!(" — {e}"))
        ))
    } else if scan.any_green {
        PixelObservation::GreenSeen
    } else {
        let (best_fraction, best_backend) = scan.best.unwrap_or((0.0, "none"));
        PixelObservation::NoGreen {
            best_fraction,
            best_backend,
            evaluated: scan.evaluated,
        }
    }
}

/// Running totals for [`observe_child_pixels`] — how many children
/// produced a screen capture, whether any met the green threshold,
/// the best fraction/backend seen, and the first capture error.
#[derive(Default)]
struct PixelScan {
    /// Children whose screen capture succeeded.
    evaluated: usize,
    /// Any child met [`GREEN_VISIBLE_FRACTION`].
    any_green: bool,
    /// Best `(green_fraction, backend)` across children.
    best: Option<(f64, &'static str)>,
    /// First screen-capture error, for the `NotEvaluable` detail.
    first_error: Option<String>,
}

/// Records the opt-in PNG artifact for one capture into its step
/// row — `artifact` on success, `artifact_error` on failure; a write
/// failure never changes a verdict.
fn record_pixel_artifact(
    row: &mut Value,
    dir: Option<&Path>,
    name: &str,
    capture: &ospix::PixelCapture,
) {
    let Some(dir) = dir else { return };
    match save_pixel_artifact(dir, name, capture) {
        Ok(path) => row["artifact"] = json!(path.display().to_string()),
        Err(error) => row["artifact_error"] = json!(error),
    }
}

/// One child's pixel observation while attached: `GetWindowRect` →
/// screen-DC `BitBlt` of the screen rectangle (primary oracle —
/// what the user sees) → `PrintWindow(child)` and `GetDC`+`BitBlt`
/// of the child's own surface (two `note`-level diagnostics that
/// separate "the plugin painted green" from "the screen shows
/// green") → query-only occlusion diagnostics (root exstyle,
/// sibling z-order, `WindowFromPoint` — `note` level only, they
/// never feed the oracle).
fn observe_one_child_pixels(
    index: usize,
    child: &ChildWindowFacts,
    container: u64,
    artifacts: Option<&Path>,
    steps: &mut Vec<Value>,
    scan: &mut PixelScan,
) {
    let hwnd = child.hwnd;

    // screen_rect — GetWindowRect in virtual-screen coordinates.
    let rect = ospix::window_screen_rect(hwnd);
    match &rect {
        Ok(r) => steps.push(json!({
            "step": "screen_rect",
            "verdict": "pass",
            "hwnd": hwnd,
            "left": r.left,
            "top": r.top,
            "width": r.width,
            "height": r.height,
        })),
        Err(error) => steps.push(json!({
            "step": "screen_rect",
            "verdict": "not_run",
            "hwnd": hwnd,
            "error": error,
        })),
    }

    // Primary oracle — screen-DC BitBlt of the child's screen
    // rectangle: what the user actually sees (DComp visual,
    // other HWNDs, cursor included).
    let screen = match rect {
        Ok(r) => ospix::capture_screen_rect(r),
        Err(error) => Err(error),
    };
    match &screen {
        Ok(capture) => {
            scan.evaluated += 1;
            let (green, total) = green_stats(&capture.rgba);
            let fraction = green_fraction(green, total);
            if fraction >= GREEN_VISIBLE_FRACTION {
                scan.any_green = true;
            }
            if scan.best.is_none_or(|(f, _)| fraction > f) {
                scan.best = Some((fraction, capture.backend));
            }
            let mut row = json!({
                "step": "screen_pixels",
                "verdict": if fraction >= GREEN_VISIBLE_FRACTION {
                    "pass"
                } else {
                    "fail"
                },
                "hwnd": hwnd,
                "backend": capture.backend,
                "width": capture.width,
                "height": capture.height,
                "green_pixels": green,
                "total_pixels": total,
                "green_fraction": fraction,
                "threshold": GREEN_VISIBLE_FRACTION,
            });
            record_pixel_artifact(
                &mut row,
                artifacts,
                &format!("real-dcomp-pixel-screen-{index}"),
                capture,
            );
            steps.push(row);
        }
        Err(error) => {
            if scan.first_error.is_none() {
                scan.first_error = Some(format!("hwnd={hwnd:#x}: {error}"));
            }
            steps.push(json!({
                "step": "screen_pixels",
                "verdict": "not_run",
                "hwnd": hwnd,
                "error": error,
            }));
        }
    }

    record_printwindow_pixels(index, hwnd, artifacts, steps);
    record_windowdc_pixels(index, hwnd, artifacts, steps);
    record_occlusion_notes(hwnd, container, steps);
}

/// Query-only occlusion diagnostics for one docked child — three
/// `note`-level step rows that separate *which layer* hides the
/// child's pixels without touching the oracle or any verdict:
///
/// - `occlusion_exstyle` — the GPUI root HWND's `GWL_EXSTYLE`
///   (`WS_EX_NOREDIRECTIONBITMAP` proves whether
///   `GPUI_DISABLE_DIRECT_COMPOSITION` reached GPUI's window
///   creation on this run).
/// - `occlusion_zorder` — direct children of the child's parent
///   and of the root in z-order, with role labels and
///   above-reference flags — HWND-level cover candidates.
/// - `occlusion_hit_test` — `WindowFromPoint` at five points
///   inside the child's screen rect — the HWND the window manager
///   considers topmost there.
///
/// All reads go through [`osoccl::probe`]; every row is
/// `verdict: "note"` and failures degrade to `error` fields,
/// never to fabricated observations.
fn record_occlusion_notes(hwnd: u64, container: u64, steps: &mut Vec<Value>) {
    let facts = osoccl::probe(hwnd);
    if !facts.supported {
        steps.push(json!({
            "step": "occlusion",
            "verdict": "note",
            "hwnd": hwnd,
            "supported": false,
            "note": "OS occlusion observation is unsupported on this platform",
        }));
        return;
    }
    steps.push(occlusion_exstyle_row(hwnd, &facts));
    steps.push(occlusion_zorder_row(hwnd, container, &facts));
    steps.push(occlusion_hit_test_row(hwnd, container, &facts));
}

/// `occlusion_exstyle` row — the extended style of every window
/// on the probed chain (root, parent/container, child), not just
/// the root. `noredirectionbitmap` on the root is the DComp-path
/// marker: GPUI sets `WS_EX_NOREDIRECTIONBITMAP` only when direct
/// composition is enabled, so its per-run value is the env-var
/// effectiveness evidence. The child/parent styles carry the
/// hit-test-relevant bits (`WS_EX_TRANSPARENT` etc.) that the
/// root-only record cannot see.
fn occlusion_exstyle_row(hwnd: u64, facts: &osoccl::OcclusionFacts) -> Value {
    let exstyle = |e: &Option<osoccl::ExstyleFacts>| {
        e.as_ref().map(|e| {
            json!({
                "value": e.value,
                "noredirectionbitmap": e.noredirectionbitmap(),
                "topmost": e.topmost(),
                "layered": e.layered(),
                "transparent": e.transparent(),
                "noactivate": e.noactivate(),
                "toolwindow": e.toolwindow(),
                "appwindow": e.appwindow(),
            })
        })
    };
    json!({
        "step": "occlusion_exstyle",
        "verdict": "note",
        "hwnd": hwnd,
        "root": facts.root,
        "exstyle": exstyle(&facts.root_exstyle).unwrap_or(Value::Null),
        "exstyle_error": facts.root_exstyle_error,
        "child_exstyle": exstyle(&facts.child_exstyle).unwrap_or(Value::Null),
        "child_exstyle_error": facts.child_exstyle_error,
        "parent_exstyle": exstyle(&facts.parent_exstyle).unwrap_or(Value::Null),
        "parent_exstyle_error": facts.parent_exstyle_error,
        "note": "GWL_EXSTYLE of the probed chain (root/parent/child) \
                — root WS_EX_NOREDIRECTIONBITMAP marks the DComp path \
                (absent only when GPUI_DISABLE_DIRECT_COMPOSITION took \
                effect); child/parent styles carry hit-test-relevant \
                bits such as WS_EX_TRANSPARENT",
    })
}

/// `occlusion_zorder` row — both sibling walks with role labels.
/// `parent_children` is annotated `above_editor` (position above
/// the docked child); `root_children` is annotated
/// `above_container` (position above the container). A `null`
/// flag means the reference window was not found in that list —
/// unknown, never assumed.
fn occlusion_zorder_row(hwnd: u64, container: u64, facts: &osoccl::OcclusionFacts) -> Value {
    json!({
        "step": "occlusion_zorder",
        "verdict": "note",
        "hwnd": hwnd,
        "parent": facts.parent,
        "parent_children": occlusion_zorder_list(
            facts.parent_children.as_ref(),
            "GetParent returned null — no parent to enumerate",
            hwnd,
            "above_editor",
            hwnd,
            container,
            facts.root,
        ),
        "root_children": occlusion_zorder_list(
            facts.root_children.as_ref(),
            "GetAncestor(GA_ROOT) returned null — no root to enumerate",
            container,
            "above_container",
            hwnd,
            container,
            facts.root,
        ),
        "note": "direct-child z-order walks (topmost first) — \
                siblings above the child/container are HWND-level \
                occlusion candidates",
    })
}

/// Serializes one [`osoccl::ZOrder`] option: `{children, truncated}`
/// when the walk ran, `{error}` when there was no window to
/// enumerate (`skipped_reason` names which query came back null).
/// Each entry carries its z-order position, role, the `above_key`
/// flag relative to `reference`, and the window's identifying
/// facts.
fn occlusion_zorder_list(
    result: Option<&osoccl::ZOrder>,
    skipped_reason: &str,
    reference: u64,
    above_key: &str,
    child: u64,
    container: u64,
    root: Option<u64>,
) -> Value {
    match result {
        None => json!({ "error": skipped_reason }),
        Some(zorder) => {
            let reference_index = zorder.children.iter().position(|row| row.hwnd == reference);
            let children: Vec<Value> = zorder
                .children
                .iter()
                .enumerate()
                .map(|(index, row)| {
                    let mut entry = occlusion_window_json(row);
                    if let Some(obj) = entry.as_object_mut() {
                        obj.insert(
                            "role".to_string(),
                            json!(occlusion_role(row.hwnd, child, container, root)),
                        );
                        obj.insert(
                            above_key.to_string(),
                            json!(reference_index.map(|r| index < r)),
                        );
                    }
                    entry
                })
                .collect();
            json!({
                "children": children,
                "truncated": zorder.truncated,
            })
        }
    }
}

/// `occlusion_hit_test` row — `WindowFromPoint` at the
/// representative points inside the child's screen rectangle,
/// each resolved to a role. `WindowFromPoint` sees the window
/// manager's z-order/visibility only — `DComp` visuals and
/// flip-model presentation planes are not HWNDs and cannot appear
/// here, so a child answer does not by itself prove compositor-
/// level occlusion (recorded limitation, not a verdict input).
/// The hit-test also *skips* hidden and disabled windows and
/// their subtrees, so a non-child answer means "covered OR
/// skipped" — the `child_window`/`parent_window`/`root_window`
/// rows record the chain's probe-time `visible`/`disabled`/
/// `alive`/exstyle state that disambiguates the two.
fn occlusion_hit_test_row(hwnd: u64, container: u64, facts: &osoccl::OcclusionFacts) -> Value {
    let points: Vec<Value> = facts
        .hits
        .iter()
        .map(|hit| {
            // Uniform key set for resolved and NULL hits — absent
            // window facts serialize as `null`, never fabricated.
            let mut entry = match &hit.window {
                Some(row) => {
                    let mut entry = occlusion_window_json(row);
                    if let Some(obj) = entry.as_object_mut() {
                        obj.insert(
                            "role".to_string(),
                            json!(occlusion_role(row.hwnd, hwnd, container, facts.root)),
                        );
                    }
                    entry
                }
                None => json!({
                    "hwnd": Value::Null,
                    "z_index": Value::Null,
                    "class_name": Value::Null,
                    "rect": Value::Null,
                    "visible": Value::Null,
                    "alive": Value::Null,
                    "disabled": Value::Null,
                    "role": "none",
                }),
            };
            if let Some(obj) = entry.as_object_mut() {
                obj.insert("x".to_string(), json!(hit.x));
                obj.insert("y".to_string(), json!(hit.y));
            }
            entry
        })
        .collect();
    json!({
        "step": "occlusion_hit_test",
        "verdict": "note",
        "hwnd": hwnd,
        "child_rect": occlusion_rect_json(facts.child_rect),
        "child_window": occlusion_window_json(&facts.child),
        "parent_window": facts
            .parent_window
            .as_ref()
            .map_or(Value::Null, occlusion_window_json),
        "root_window": facts
            .root_window
            .as_ref()
            .map_or(Value::Null, occlusion_window_json),
        "points": points,
        "note": "WindowFromPoint at representative points inside the \
                child rect — HWND-level hit-test only; compositor \
                visuals/planes are not HWNDs. Hidden/disabled windows \
                are skipped: child/parent/root_window rows carry the \
                chain's probe-time visible/disabled/alive state to \
                separate 'covered' from 'skipped'",
    })
}

/// Role label for one observed HWND relative to the docked child,
/// the container token, and the GPUI root — `other` covers any
/// unrelated window (including GPUI-owned children that are not
/// the container).
fn occlusion_role(hwnd: u64, child: u64, container: u64, root: Option<u64>) -> &'static str {
    if hwnd == child {
        "editor_child"
    } else if hwnd == container {
        "container"
    } else if root == Some(hwnd) {
        "root"
    } else {
        "other"
    }
}

/// Plain-JSON form of one [`osoccl::WindowRow`] — `z_index` is
/// `null` for rows that did not come from a z-order walk.
fn occlusion_window_json(row: &osoccl::WindowRow) -> Value {
    json!({
        "hwnd": row.hwnd,
        "z_index": row.z_index,
        "class_name": row.class_name,
        "rect": occlusion_rect_json(row.rect),
        "visible": row.visible,
        "alive": row.alive,
        "disabled": row.disabled,
    })
}

/// Plain-JSON form of an [`ospix::ScreenRect`].
fn occlusion_rect_json(rect: Option<ospix::ScreenRect>) -> Value {
    rect.map_or(Value::Null, |r| {
        json!({
            "left": r.left,
            "top": r.top,
            "width": r.width,
            "height": r.height,
        })
    })
}

/// The `GetDC`+`BitBlt` diagnostic — the child HWND's own backing
/// surface as GDI last painted it, read without screen composition
/// or `WM_PRINT`. `note` level only, like [`record_printwindow_pixels`]:
/// together the two diagnostics separate "child painted green but
/// the compositor hides it" from "the child surface never received
/// the fill".
fn record_windowdc_pixels(
    index: usize,
    hwnd: u64,
    artifacts: Option<&Path>,
    steps: &mut Vec<Value>,
) {
    match ospix::capture_window_dc(hwnd) {
        Ok(capture) => {
            let (green, total) = green_stats(&capture.rgba);
            let fraction = green_fraction(green, total);
            let mut row = json!({
                "step": "windowdc_pixels",
                "verdict": "note",
                "hwnd": hwnd,
                "backend": capture.backend,
                "width": capture.width,
                "height": capture.height,
                "green_pixels": green,
                "total_pixels": total,
                "green_fraction": fraction,
                "note": "child's own window-DC surface via GetDC+BitBlt — \
                        NOT the on-screen composition; diagnostic only",
            });
            record_pixel_artifact(
                &mut row,
                artifacts,
                &format!("real-dcomp-pixel-windowdc-{index}"),
                &capture,
            );
            steps.push(row);
        }
        Err(error) => steps.push(json!({
            "step": "windowdc_pixels",
            "verdict": "note",
            "hwnd": hwnd,
            "error": error,
        })),
    }
}

/// The `PrintWindow(child)` diagnostic — the child's own raster,
/// NOT the on-screen composition. `note` level only: it can prove
/// the plugin painted green even when the screen never shows it
/// (the `DComp` occlusion signature).
fn record_printwindow_pixels(
    index: usize,
    hwnd: u64,
    artifacts: Option<&Path>,
    steps: &mut Vec<Value>,
) {
    match ospix::capture_print_window(hwnd) {
        Ok(capture) => {
            let (green, total) = green_stats(&capture.rgba);
            let fraction = green_fraction(green, total);
            let mut row = json!({
                "step": "printwindow_pixels",
                "verdict": "note",
                "hwnd": hwnd,
                "backend": capture.backend,
                "width": capture.width,
                "height": capture.height,
                "green_pixels": green,
                "total_pixels": total,
                "green_fraction": fraction,
                "note": "child's own raster via PrintWindow — NOT \
                        the on-screen composition; diagnostic only",
            });
            record_pixel_artifact(
                &mut row,
                artifacts,
                &format!("real-dcomp-pixel-printwindow-{index}"),
                &capture,
            );
            steps.push(row);
        }
        Err(error) => steps.push(json!({
            "step": "printwindow_pixels",
            "verdict": "note",
            "hwnd": hwnd,
            "error": error,
        })),
    }
}

/// `observed` extras for the pixel case — the shared child/session
/// fields plus the aggregated pixel outcome.
fn pixel_extras(
    container: Option<u64>,
    observation: &ChildWindowObservation,
    events: &[String],
    pixels: &PixelObservation,
) -> Map<String, Value> {
    let mut extras = child_extras(container, observation, events);
    let summary = match pixels {
        PixelObservation::GreenSeen => json!({ "green_observed": true }),
        PixelObservation::NoGreen {
            best_fraction,
            best_backend,
            evaluated,
        } => json!({
            "green_observed": false,
            "best_green_fraction": best_fraction,
            "best_backend": best_backend,
            "evaluated_children": evaluated,
        }),
        PixelObservation::NotEvaluable(reason) => json!({
            "green_observed": Value::Null,
            "not_evaluable": reason,
        }),
    };
    extras.insert("screen_pixels".to_string(), summary);
    extras
}

/// Post-open pixel observation + close convergence for an accepted
/// `session.open(Docked)` — the `real-dcomp-pixel` counterpart of
/// [`observe_editor_child_lifecycle`]. Same attach contract checks,
/// then — while still attached — the screen-composited pixel
/// observation ([`observe_child_pixels`]), then the shared close →
/// detach → convergence tail ([`conclude_editor_session`]).
fn observe_editor_pixel_lifecycle(
    container: Option<u64>,
    sessions: &mut PluginEditorSessions,
    session_id: PluginEditorSessionId,
    host: &mut InProcessHost,
    handle: PluginHandle,
    artifacts: Option<&Path>,
    steps: &mut Vec<Value>,
) -> PixelVerification {
    let mut failure: Option<String> = None;

    // Phase 1 — enumerate the container's children while the editor
    // is still attached (same observation as `real-editor-attach`).
    let mut observation = match container {
        Some(token) => oschild::observe(token),
        None => oschild::empty(),
    };

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

    // Phase 1b — screen-composited pixels while still attached (the
    // Issue #532 oracle). Off-Windows there is no pixel concept
    // either — recorded as not evaluable, never fabricated.
    let pixels = if observation.supported {
        observe_child_pixels(&observation, container, artifacts, steps)
    } else {
        steps.push(json!({
            "step": "screen_pixels",
            "verdict": "unsupported",
            "detail": "OS pixel observation is Windows-only",
        }));
        PixelObservation::NotEvaluable("OS pixel observation is Windows-only".to_string())
    };

    let (close_ok, detach_ok, converged_ok, rendered) = conclude_editor_session(
        &mut observation,
        sessions,
        session_id,
        host,
        handle,
        steps,
        &mut failure,
    );

    PixelVerification {
        supported: observation.supported,
        lifecycle_ok: attach_ok && close_ok && detach_ok && converged_ok,
        lifecycle_failure: failure,
        extras: Value::Object(pixel_extras(container, &observation, &rendered, &pixels)),
        pixels,
    }
}

/// `real-dcomp-pixel`: the same real attach path as
/// `real-editor-attach`, plus the screen-composited pixel oracle —
/// does `EDITOR_COLOR` green actually appear in the pixels the user
/// sees? Under the known GPUI `DComp` topmost occlusion the honest
/// answer is `Fail` (the regression this case pins); `Pass` is only
/// claimed when green is really observed on screen, and an
/// unevaluable environment reports `NotRun`, never a fabricated
/// verdict.
fn editor_pixel_body(
    gui: &mut RealOsGuiTest,
    window: WindowId,
    view: ViewInstanceId,
    plugin: Option<&PluginPair>,
    plugin_error: Option<&PluginUnavailable>,
    artifacts: Option<&Path>,
) -> ScenarioOutcome {
    let (mut host, handle) = match prepare_editor_host(plugin, plugin_error) {
        Ok(pair) => pair,
        Err(outcome) => return *outcome,
    };

    let mut steps = Vec::new();
    let (attempt, cleanup, verification) = attempt_real_attach(
        gui,
        window,
        view,
        &mut host,
        handle,
        &mut steps,
        |container, _lease_raw, sessions, session_id, host, handle, steps| {
            observe_editor_pixel_lifecycle(
                container, sessions, session_id, host, handle, artifacts, steps,
            )
        },
    );

    let mut outcome = match attach_attempt_failure(&attempt) {
        Some(outcome) => outcome,
        None => match verification.as_ref() {
            Some(verification) if !verification.supported => {
                // An accepted open off-Windows — neither the child
                // contract nor the pixels can be observed there, so
                // the case honestly stays `Unsupported` (unreachable
                // today: `NativePluginDock` is Windows-only).
                let mut outcome = ScenarioOutcome::new(ScenarioVerdict::Unsupported);
                outcome.detail = Some(
                    "editor opened through the REAL container lease but OS \
                     pixel observation is unsupported on this platform"
                        .to_string(),
                );
                outcome
            }
            Some(verification) if !verification.lifecycle_ok => fail(format!(
                "editor opened through the REAL container lease but the \
                 lifecycle observation failed: {}",
                verification
                    .lifecycle_failure
                    .as_deref()
                    .unwrap_or("unknown mismatch")
            )),
            Some(verification) => match &verification.pixels {
                PixelObservation::GreenSeen => {
                    let mut outcome = ScenarioOutcome::new(ScenarioVerdict::Pass);
                    outcome.detail = Some(
                        "docked test_editor child HWND's screen-composited \
                         pixels show EDITOR_COLOR green — the editor \
                         surface is actually visible on screen"
                            .to_string(),
                    );
                    outcome
                        .evidence
                        .push("dcomp-pixel:editor-green-on-screen".to_string());
                    outcome
                }
                PixelObservation::NoGreen {
                    best_fraction,
                    best_backend,
                    evaluated,
                } => fail(format!(
                    "editor lifecycle verified but {evaluated} child screen \
                     capture(s) show no EDITOR_COLOR green (best \
                     green_fraction={best_fraction:.3} via {best_backend}) — \
                     consistent with the known GPUI DComp topmost occlusion"
                )),
                PixelObservation::NotEvaluable(reason) => not_run(format!(
                    "editor lifecycle verified but screen pixels are not \
                     evaluable: {reason}"
                )),
            },
            None => fail("editor opened but no pixel observation was recorded".to_string()),
        },
    };
    let mut observed = Map::from_iter([
        ("steps".to_string(), json!(steps)),
        ("cleanup".to_string(), json!(cleanup)),
    ]);
    if let Some(verification) = &verification
        && let Value::Object(fields) = &verification.extras
    {
        observed.extend(fields.clone());
    }
    outcome.observed = Some(Value::Object(observed));
    outcome
}

/// `real-dcomp-pixel` case: pre-flight requires `NativePluginDock`
/// and `VisualCapture` (the adapter claims the former only where
/// the surface backend mints real container parents; the latter
/// marks the pixel oracle), then the body drives the attach chain
/// plus the screen-pixel observation — or reports the honest
/// artifact/session/environment outcome.
fn real_dcomp_pixel_case(ctx: &mut RealDockContext<'_>, spec: &ScenarioSpec) -> ScenarioReport {
    let window = ctx.window;
    let view = ctx.view;
    let plugin = ctx.plugin;
    let plugin_error = ctx.plugin_error;
    let artifact = ctx.artifact;
    let pixel_artifacts = ctx.pixel_artifacts;
    let report = run_scenario(&mut *ctx.gui, spec, |gui| {
        Ok(editor_pixel_body(
            gui,
            window,
            view,
            plugin,
            plugin_error,
            pixel_artifacts,
        ))
    });
    finalize_real(
        report,
        ctx.env,
        EnvironmentKind::RealWindow,
        artifact,
        Map::new(),
    )
}

// ---------------------------------------------------------------------
// case: real-multiwindow-focus (AC1 — 2-window focus transfer)
// ---------------------------------------------------------------------

/// Requests focus on `window` through the driver's own op, settles
/// until `window_focused` reports it, and pushes the per-step rows.
/// `Ok(true)` = focus observed; `Ok(false)` = settle timed out
/// (recorded as a failure); `Err` = driver contract error — the
/// caller decides whether to halt the sequence.
fn drive_focus(
    gui: &mut RealOsGuiTest,
    steps: &mut Vec<Value>,
    failure: &mut Option<String>,
    window: WindowId,
    request_step: &str,
    observe_step: &str,
) -> Result<bool, String> {
    step(steps, request_step, gui.request_focus(window, true), |()| {
        Value::Null
    })?;
    match settle_until(gui, |g| g.window_focused(window), 120) {
        Ok(true) => {
            steps.push(json!({ "step": observe_step, "verdict": "pass" }));
            Ok(true)
        }
        Ok(false) => {
            record_obs_fail(
                steps,
                failure,
                observe_step,
                format!("window {window} never reported focused after request_focus"),
            );
            Ok(false)
        }
        Err(error) => {
            record_obs_fail(steps, failure, observe_step, error.to_string());
            Ok(false)
        }
    }
}

/// `real-multiwindow-focus`: two real OS windows are mounted; the
/// case drives `request_focus` on the driver and observes
/// `window_focused` transitions — the AC1 "2Window Focus" judgment.
/// Focus *release* (`focused = false`) is recorded as an observation
/// only: the driver may honestly report `Unsupported`, which is
/// evidence, never a verdict input. Window close is NOT exercised —
/// the `RealOsGuiTest` public surface exposes no close request (see
/// `real-window-close`); this case's scope is focus transfer only.
fn multiwindow_focus_body(
    gui: &mut RealOsGuiTest,
    window_a: WindowId,
    window_b: Option<WindowId>,
) -> ScenarioOutcome {
    let mut steps = Vec::new();
    let windows = gui.windows();
    let ids: Vec<String> = windows.iter().map(ToString::to_string).collect();
    steps.push(json!({
        "step": "window_count",
        "verdict": if windows.len() >= 2 { "pass" } else { "fail" },
        "count": windows.len(),
        "windows": ids,
    }));

    let mut failure: Option<String> = None;
    let attempt = (|| -> Result<Value, String> {
        if windows.len() < 2 {
            return Err("session mounted fewer than 2 windows".to_string());
        }
        let wb = window_b.ok_or_else(|| "second window id not recorded".to_string())?;

        // Establish the A baseline first — the A→B→A claim requires
        // proof A actually held focus before B was requested (the
        // second-created window may take foreground at mount, which
        // would make an unobserved "A→B" leg unfalsifiable).
        drive_focus(
            gui,
            &mut steps,
            &mut failure,
            window_a,
            "focus_request_a_baseline",
            "focus_a_baseline_observed",
        )?;

        // A → B focus transfer through the driver's own focus op.
        drive_focus(
            gui,
            &mut steps,
            &mut failure,
            wb,
            "focus_request_b",
            "focus_b_observed",
        )?;
        // Observation only — whether A released focus is platform
        // behavior, recorded for evidence, never a verdict input.
        steps.push(json!({
            "step": "focus_a_state_while_b",
            "verdict": "note",
            "focused": gui.window_focused(window_a),
        }));

        // Focus release honesty — `focused = false` may be a defined
        // `Unsupported` on this backend; record the driver's answer.
        match gui.request_focus(wb, false) {
            Ok(()) => steps.push(json!({
                "step": "focus_release_b",
                "verdict": "note",
                "detail": "release request accepted",
            })),
            Err(error) => steps.push(json!({
                "step": "focus_release_b",
                "verdict": "note",
                "detail": format!("release refused/unsupported: {error}"),
            })),
        }

        // B → A transfer back — proves focus moves both directions.
        drive_focus(
            gui,
            &mut steps,
            &mut failure,
            window_a,
            "focus_request_a",
            "focus_a_observed",
        )?;
        steps.push(json!({
            "step": "focus_b_state_after_a",
            "verdict": "note",
            "focused": gui.window_focused(wb),
        }));

        if let Some(detail) = &failure {
            return Err(detail.clone());
        }
        Ok(json!({
            "windows": windows.iter().map(ToString::to_string).collect::<Vec<_>>(),
            "window_a": window_a.to_string(),
            "window_b": wb.to_string(),
        }))
    })();

    steps_outcome(
        &steps,
        attempt,
        "two real OS windows mounted; request_focus moved observed \
         window_focused state A→B→A (release recorded as observation \
         only — window close is out of this case's scope)",
        &["multiwindow:focus-transfer:real-os"],
    )
}

/// `real-multiwindow-focus` case: pre-flight requires `MultiWindow`,
/// then the body drives the two-window focus transfer — or reports
/// the honest mount/focus failure.
fn real_multiwindow_focus_case(
    ctx: &mut RealDockContext<'_>,
    spec: &ScenarioSpec,
) -> ScenarioReport {
    let window_a = ctx.window;
    let window_b = ctx.window_b;
    let report = run_scenario(&mut *ctx.gui, spec, |gui| {
        Ok(multiwindow_focus_body(gui, window_a, window_b))
    });
    finalize_real(
        report,
        ctx.env,
        EnvironmentKind::RealWindow,
        None,
        Map::new(),
    )
}

// ---------------------------------------------------------------------
// case: real-os-input (AC1 — real OS pointer/keyboard injection)
// ---------------------------------------------------------------------

/// Maps an [`OsInputOutcome`] to its step verdict + detail — the
/// lane verdict vocabulary mirrored honestly (`Injected` is the only
/// pass-grade landing; `EnvironmentBlocked`/`NotRun`/`Unsupported`
/// stay distinct, never collapsed into a failure). `OsInputOutcome`
/// is `#[non_exhaustive]`: an unlisted variant maps to `not_run`,
/// never a neutral tag the verdict fold could promote to `Pass`.
fn os_outcome_tag(outcome: &OsInputOutcome) -> (&'static str, String) {
    match outcome {
        OsInputOutcome::Injected { detail, .. } => ("pass", detail.clone()),
        OsInputOutcome::Unsupported { reason } => ("unsupported", reason.clone()),
        OsInputOutcome::EnvironmentBlocked { detail } => ("environment_blocked", detail.clone()),
        OsInputOutcome::NotRun { reason } => ("not_run", reason.clone()),
        _ => (
            "not_run",
            "unlisted OsInputOutcome variant — injection effect cannot be proven".to_string(),
        ),
    }
}

/// Pushes one `observed.steps` row for an OS-input op outcome —
/// the verdict tag mirrors the lane vocabulary honestly.
fn push_os_step(steps: &mut Vec<Value>, name: &str, outcome: &OsInputOutcome) {
    let (tag, detail) = os_outcome_tag(outcome);
    steps.push(json!({ "step": name, "verdict": tag, "detail": detail }));
}

/// Runs the family-effect preflight (Issue #552) and pushes the
/// `os_preflight` step row with every per-family check. Returns
/// `false` when injection cannot take effect — the caller then
/// records the honest `environment_blocked`, never a fake pass.
fn os_input_preflight_effective(gui: &mut RealOsGuiTest, steps: &mut Vec<Value>) -> bool {
    let preflight = gui.os_input_preflight(PreflightScope::PointerAndKeyboard);
    let checks: Vec<Value> = preflight
        .checks
        .iter()
        .map(|check| {
            json!({
                "family": check.family,
                "effective": check.effective,
                "detail": check.detail,
            })
        })
        .collect();
    steps.push(json!({
        "step": "os_preflight",
        "verdict": if preflight.effective { "pass" } else { "environment_blocked" },
        "checks": checks,
    }));
    preflight.effective
}

/// Pushes one op call's result as a step row — `Err` is a contract
/// error and lands as `fail`.
fn push_op_result(
    steps: &mut Vec<Value>,
    name: &str,
    result: Result<OsInputOutcome, ScenarioError>,
) {
    match result {
        Ok(outcome) => push_os_step(steps, name, &outcome),
        Err(error) => steps.push(json!({
            "step": name,
            "verdict": "fail",
            "error": error.to_string(),
        })),
    }
}

/// Runs the real OS injection ops — pointer aim at `component`'s
/// center (logical bounds → physical pixels with cursor read-back)
/// first, then click down/up, and an unmodified `Tab` keystroke —
/// pushing one step row per op. `Injected` is the only pass-grade
/// landing; every other `OsInputOutcome` keeps its honest tag and a
/// contract error lands as `fail`.
///
/// The click ops are gated on the pointer move landing `Injected`:
/// the driver injects buttons at the *current* cursor position, so
/// when the aim cannot be proven the cursor stays wherever it was —
/// firing a click there would hit an uncontrolled screen target.
/// The skipped ops record `not_run`, never a fabricated pass. `Tab`
/// is keyboard input gated only on window focus, so it runs
/// regardless of the pointer leg.
fn push_injection_steps(
    gui: &mut RealOsGuiTest,
    target: &ScenarioTarget,
    component: &ComponentKey,
    steps: &mut Vec<Value>,
) {
    let aimed = match gui.os_pointer_move_to(target, component, (0.5, 0.5)) {
        Ok(outcome) => {
            let injected = matches!(&outcome, OsInputOutcome::Injected { .. });
            push_os_step(steps, "pointer_move", &outcome);
            injected
        }
        Err(error) => {
            steps.push(json!({
                "step": "pointer_move",
                "verdict": "fail",
                "error": error.to_string(),
            }));
            false
        }
    };
    if aimed {
        push_op_result(steps, "mouse_down", gui.os_mouse_down(target));
        push_op_result(steps, "mouse_up", gui.os_mouse_up(target));
    } else {
        for name in ["mouse_down", "mouse_up"] {
            steps.push(json!({
                "step": name,
                "verdict": "not_run",
                "detail": "skipped — pointer aim did not inject; a click \
                           at the unverified cursor position would hit \
                           an uncontrolled target",
            }));
        }
    }
    push_op_result(steps, "key_tab", gui.os_press_key(target, OsKey::Tab));
}

/// Binds a real native slot so `plugin.editor` records painted
/// bounds (the dock panel paints no canvas, so `plugin.dock` can
/// never be an aim target), runs the four OS injection ops aimed at
/// that dock region, then tears the slot down — pushing every step
/// row. `Err` means the slot chain itself failed; op outcomes land
/// as step rows instead.
fn inject_into_bound_slot(
    gui: &mut RealOsGuiTest,
    window: WindowId,
    view: ViewInstanceId,
    target: &ScenarioTarget,
    steps: &mut Vec<Value>,
) -> Result<(), String> {
    let key = slot_key();
    let (slot, lease) = lease_real_slot(gui, window, view, steps)?;

    push_injection_steps(gui, target, &key, steps);

    // Success-path teardown — unbind → release lease → release
    // slot; notes never mask the case verdict.
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
    Ok(())
}

/// `real-os-input`: verifies real OS pointer/keyboard injection
/// reaches the focused real window — the AC1 "実 Window … 入力"
/// judgment at transport level. `os_input_preflight` must prove
/// *effect* (Issue #552) before any injection; every op's
/// `OsInputOutcome` is recorded verbatim. The dock panel paints no
/// canvas, so a real native slot is bound first and the pointer
/// aims at `plugin.editor`'s painted bounds (the actual dock
/// region). Pass requires the pointer move (with physical cursor
/// read-back), the click, and the `Tab` keystroke all `Injected`
/// while the window stays focused. The view has no component-level
/// input oracle, so semantic effect beyond injection+focus is
/// honestly out of scope — recorded in `detail`, never claimed.
fn os_input_body(
    gui: &mut RealOsGuiTest,
    window: WindowId,
    view: ViewInstanceId,
) -> ScenarioOutcome {
    let mut steps = Vec::new();

    // Family-effect preflight — a `false` report means the session
    // cannot deliver effective injection; the honest verdict is
    // `NotRun` + `environment_blocked`, never a fake pass.
    if !os_input_preflight_effective(gui, &mut steps) {
        let mut outcome = not_run(
            "OS input injection preflight proved ineffective on this \
             session — pointer/keyboard events would not take effect",
        );
        outcome.observed = Some(json!({
            "observed": "environment_blocked",
            "steps": steps,
        }));
        return outcome;
    }

    // OS injection ops only reach the *observed focused* window —
    // focus it through the driver's own op first. A settle timeout
    // is an unmet precondition (`not_run` — the ops below record
    // their own `NotRun` on an unfocused window); a driver contract
    // error stays `fail`.
    let _ = gui.request_focus(window, true);
    match settle_until(gui, |g| g.window_focused(window), 120) {
        Ok(true) => steps.push(json!({
            "step": "window_focus_for_input",
            "verdict": "pass",
            "focused": true,
        })),
        Ok(false) => steps.push(json!({
            "step": "window_focus_for_input",
            "verdict": "not_run",
            "focused": false,
            "detail": "window never reported focused — injection ops \
                       will record their own NotRun",
        })),
        Err(error) => steps.push(json!({
            "step": "window_focus_for_input",
            "verdict": "fail",
            "focused": false,
            "error": error.to_string(),
        })),
    }

    let target = match gui.resolve_target(&TargetQuery::new(window, view)) {
        Ok(target) => target,
        Err(error) => {
            steps.push(json!({
                "step": "resolve_target",
                "verdict": "fail",
                "error": error.to_string(),
            }));
            let mut outcome = fail(format!("resolve_target: {error}"));
            outcome.observed = Some(json!({ "steps": steps }));
            return outcome;
        }
    };
    steps.push(json!({ "step": "resolve_target", "verdict": "pass" }));

    if let Err(detail) = inject_into_bound_slot(gui, window, view, &target, &mut steps) {
        let mut outcome = fail(detail);
        outcome.observed = Some(json!({ "steps": steps }));
        return outcome;
    }

    // Verdict aggregation — the worst honest landing wins:
    // environment_blocked → NotRun, unsupported → Unsupported,
    // not_run → NotRun, any contract error → Fail, all Injected →
    // Pass. `steps` rows already carry the per-op detail.
    let worst =
        steps
            .iter()
            .filter_map(|row| row["verdict"].as_str())
            .fold("pass", |acc, verdict| match (acc, verdict) {
                ("fail", _) | (_, "fail") => "fail",
                ("environment_blocked", _) | (_, "environment_blocked") => "environment_blocked",
                ("unsupported", _) | (_, "unsupported") => "unsupported",
                ("not_run", _) | (_, "not_run") => "not_run",
                _ => acc,
            });
    let mut observed = Map::from_iter([("steps".to_string(), json!(steps))]);
    if worst == "environment_blocked" {
        observed.insert("observed".to_string(), json!("environment_blocked"));
    }
    let mut outcome = match worst {
        "pass" => {
            let mut outcome = ScenarioOutcome::new(ScenarioVerdict::Pass);
            outcome.detail = Some(
                "preflight proved injection effect; pointer move \
                 (cursor read-back), click, and Tab keystroke all \
                 Injected into the focused real OS window — transport- \
                 level input evidence; the dock view has no component- \
                 level input oracle, so semantic effect is not claimed"
                    .to_string(),
            );
            outcome
                .evidence
                .push("os-input:injected:real-window".to_string());
            outcome
        }
        "environment_blocked" => not_run(
            "an OS input op reported environment_blocked — injection \
             would not take effect on this session",
        ),
        "unsupported" => ScenarioOutcome::new(ScenarioVerdict::Unsupported),
        "not_run" => not_run(
            "an OS input op could not run (target unfocused, no painted \
             bounds, or run precondition unmet) — see steps",
        ),
        _ => fail("an OS input op hit a contract error — see steps"),
    };
    if worst == "unsupported" {
        outcome.detail =
            Some("OS input injection is unsupported on this platform/backend".to_string());
    }
    outcome.observed = Some(Value::Object(observed));
    outcome
}

/// `real-os-input` case: pre-flight requires
/// `RealOsPointerOrKeyboard` (claimed only where the OS injection
/// backend exists), then the body runs the injection sequence — or
/// reports the honest environment/capability outcome.
fn real_os_input_case(ctx: &mut RealDockContext<'_>, spec: &ScenarioSpec) -> ScenarioReport {
    let window = ctx.window;
    let view = ctx.view;
    let report = run_scenario(&mut *ctx.gui, spec, |gui| {
        Ok(os_input_body(gui, window, view))
    });
    finalize_real(
        report,
        ctx.env,
        EnvironmentKind::RealWindow,
        None,
        Map::new(),
    )
}

// ---------------------------------------------------------------------
// cases: real-window-dpi / real-window-ime / real-window-close
// (AC1 — honest Unsupported judgments for absent adapter surface)
// ---------------------------------------------------------------------

/// `real-window-dpi`: per-case DPI judgment needs scale-factor
/// observation and/or a display-scale change. `RealOsGuiTest`
/// exposes neither — the honest verdict is `Unsupported`, recorded
/// inside the real session so the gap is machine-visible in the
/// catalog projection instead of silently absent.
fn real_window_dpi_case(ctx: &mut RealDockContext<'_>, spec: &ScenarioSpec) -> ScenarioReport {
    let report = run_scenario(&mut *ctx.gui, spec, |_gui| {
        let mut outcome = ScenarioOutcome::new(ScenarioVerdict::Unsupported);
        outcome.detail = Some(
            "RealOsGuiTest exposes no scale-factor/DPI observation or \
             change API — DPI adoption judgment deferred until the \
             adapter surface grows (window move across mixed-DPI \
             monitors is likewise not drivable)"
                .to_string(),
        );
        outcome
            .evidence
            .push("window:dpi:api-not-exposed".to_string());
        Ok(outcome)
    });
    finalize_real(
        report,
        ctx.env,
        EnvironmentKind::RealWindow,
        None,
        Map::new(),
    )
}

/// `real-window-ime`: `Ime` is never claimed by the adapter's
/// `supported_capabilities()` (IME composition is Issue #389
/// scope), so the capability preflight reports `Unsupported`
/// before the body runs. The body stays as a defensive honest
/// `NotRun` — unreachable today, correct if the vocabulary ever
/// changes.
fn real_window_ime_case(ctx: &mut RealDockContext<'_>, spec: &ScenarioSpec) -> ScenarioReport {
    let report = run_scenario(&mut *ctx.gui, spec, |_gui| {
        Ok(not_run(
            "Ime capability is not provided by the adapter — IME \
             composition judgment stays deferred (Issue #389)",
        ))
    });
    finalize_real(
        report,
        ctx.env,
        EnvironmentKind::RealWindow,
        None,
        Map::new(),
    )
}

/// `real-window-close`: window close through the normal
/// `CloseRequested` flow needs `WindowDriver::request_close` /
/// `respond_close` (or an Alt+F4 chord), none of which the
/// `RealOsGuiTest` public surface forwards — the driver supports
/// them internally but the seam is outside this slice's lease. The
/// honest verdict is `Unsupported`, recorded inside the real
/// session.
fn real_window_close_case(ctx: &mut RealDockContext<'_>, spec: &ScenarioSpec) -> ScenarioReport {
    let report = run_scenario(&mut *ctx.gui, spec, |_gui| {
        let mut outcome = ScenarioOutcome::new(ScenarioVerdict::Unsupported);
        outcome.detail = Some(
            "RealOsGuiTest exposes no window-close request/respond \
             API and os_input has no modifier-chord injection — the \
             CloseRequested flow cannot be driven from this lane yet \
             (adapter surface extension is a follow-up slice)"
                .to_string(),
        );
        outcome
            .evidence
            .push("window:close:api-not-exposed".to_string());
        Ok(outcome)
    });
    finalize_real(
        report,
        ctx.env,
        EnvironmentKind::RealWindow,
        None,
        Map::new(),
    )
}

// ---------------------------------------------------------------------
// AC3 non-pixel editor op cases (resize / re-dock)
// ---------------------------------------------------------------------

/// How the attempted editor op landed through the production ops
/// adapter — the case body's verdict input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OpLanding {
    /// The op returned the defined `EditorOpError::Unsupported`.
    Refused,
    /// The op returned `Ok` — recorded, but no non-pixel oracle can
    /// confirm the effect took place (honest `NotRun`, never Pass).
    Accepted,
    /// Any other landing — a contract error or a vanished session;
    /// `EditorOpVerification::failure` carries the detail.
    Other,
}

/// Post-open observation for the non-pixel editor-op cases
/// (`real-editor-resize`, `real-editor-redock`): the same child-HWND
/// attach contract as `real-editor-attach`, plus the op attempt's
/// honest landing — the defined `Unsupported` refusal, an accepted
/// op (no non-pixel oracle exists to confirm effect → never Pass),
/// or a real failure.
struct EditorOpVerification {
    /// `false` off-Windows — the OS observation half then reports
    /// `Unsupported`, never a fabricated pass.
    supported: bool,
    /// How the attempted op landed.
    landing: OpLanding,
    /// Every contract check agreed (attach children, session state
    /// after the op, close/detach convergence, expected events).
    verified: bool,
    /// First violated check (`name: detail`) for the `Fail` detail.
    failure: Option<String>,
    /// `observed` extras — container/child HWND ids, thread ids,
    /// drained events.
    extras: Value,
}

/// Phase-1 child-HWND attach observation shared by the editor-op
/// cases — identical contract to `observe_editor_child_lifecycle`'s
/// first phase: enumerate the container's children while attached
/// and run `check_attached_children` where an HWND concept exists.
/// Returns the observation plus whether every check agreed.
fn observe_attached_children(
    container: Option<u64>,
    steps: &mut Vec<Value>,
    failure: &mut Option<String>,
) -> (ChildWindowObservation, bool) {
    let observation = match container {
        Some(token) => oschild::observe(token),
        None => oschild::empty(),
    };
    let attach_ok = if observation.supported {
        check_attached_children(&observation, container, steps, failure)
    } else {
        steps.push(json!({
            "step": "child_hwnd_observation",
            "verdict": "unsupported",
            "detail": "OS child-HWND observation is Windows-only — an \
                       accepted open cannot be verified off-Windows",
        }));
        false
    };
    (observation, attach_ok)
}

/// `real-editor-resize` post-open observation: child-HWND attach
/// contract → `session.set_size` through the production
/// `InProcessEditorOps` (the defined `Unsupported` refusal today —
/// no `HostCommand` resize verb exists) → the session must stay
/// `Attached` (a refused `set_size` is non-fatal by contract) →
/// close convergence with the unchanged `[Attached{Docked},
/// Closed]` event sequence. No pixel capture is touched — the
/// oracle is session state + events + the HWND contract only.
fn observe_editor_resize_lifecycle(
    container: Option<u64>,
    _lease_raw: u64,
    sessions: &mut PluginEditorSessions,
    session_id: PluginEditorSessionId,
    host: &mut InProcessHost,
    handle: PluginHandle,
    steps: &mut Vec<Value>,
) -> EditorOpVerification {
    let mut failure: Option<String> = None;
    let (mut observation, attach_ok) = observe_attached_children(container, steps, &mut failure);

    // The resize attempt — `rejecting_editor_ops` is the production
    // `InProcessEditorOps`; `set_size` refuses before any lease
    // redeem, so the rejecting redeemer is never consulted.
    let mut landing = OpLanding::Other;
    match sessions.session_mut(session_id).map(|session| {
        let mut ops = rejecting_editor_ops(host, handle);
        session.set_size(
            &mut ops,
            EditorSize {
                width: 800.0,
                height: 600.0,
            },
        )
    }) {
        Some(Err(EditorSessionError::Ops(EditorOpError::Unsupported))) => {
            steps.push(json!({
                "step": "editor_resize",
                "verdict": "unsupported",
                "detail": "session.set_size → InProcessEditorOps::set_size \
                           → EditorOpError::Unsupported — no HostCommand \
                           resize verb exists on the L0 surface",
            }));
            landing = OpLanding::Refused;
        }
        Some(Ok(())) => {
            steps.push(json!({
                "step": "editor_resize",
                "verdict": "pass",
                "detail": "set_size accepted — no non-pixel oracle exists \
                           yet to confirm the resize took effect",
            }));
            landing = OpLanding::Accepted;
        }
        Some(Err(error)) => {
            record_obs_fail(steps, &mut failure, "editor_resize", error.to_string());
        }
        None => {
            record_obs_fail(
                steps,
                &mut failure,
                "editor_resize",
                "editor session vanished before set_size",
            );
        }
    }

    // Non-fatal contract: an accepted or refused `set_size` leaves
    // the session `Attached` — any other landing is a violation.
    let mut state_ok = true;
    if landing != OpLanding::Other {
        let attached = sessions
            .session(session_id)
            .is_some_and(|session| matches!(session.state(), EditorSessionState::Attached { .. }));
        if attached {
            steps.push(json!({
                "step": "session_state_after_op",
                "verdict": "pass",
                "state": "Attached",
            }));
        } else {
            record_obs_fail(
                steps,
                &mut failure,
                "session_state_after_op",
                "session did not stay Attached after set_size",
            );
            state_ok = false;
        }
    }

    let (close_ok, detach_ok, converged_ok, rendered) = conclude_editor_session(
        &mut observation,
        sessions,
        session_id,
        host,
        handle,
        steps,
        &mut failure,
    );
    EditorOpVerification {
        supported: observation.supported,
        landing,
        verified: attach_ok && state_ok && close_ok && detach_ok && converged_ok,
        failure,
        extras: Value::Object(child_extras(container, &observation, &rendered)),
    }
}

/// Drained-event contract for the re-dock lifecycle — the refused
/// path must produce exactly `[Attached{Docked}, Detached,
/// Failed{Unsupported}, Closed]`; an accepted re-attach produces
/// `[Attached{Docked}, Detached, Reattached{Docked}, Closed]`.
/// Anything else is a contract surprise, never a pass.
fn check_redock_events(
    sessions: &mut PluginEditorSessions,
    session_id: PluginEditorSessionId,
    refused: bool,
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
    let expected = if refused {
        vec![
            EditorSessionEvent::Attached {
                mode: AttachMode::Docked,
            },
            EditorSessionEvent::Detached,
            EditorSessionEvent::Failed {
                cause: SessionFailCause::Unsupported,
            },
            EditorSessionEvent::Closed,
        ]
    } else {
        vec![
            EditorSessionEvent::Attached {
                mode: AttachMode::Docked,
            },
            EditorSessionEvent::Detached,
            EditorSessionEvent::Reattached {
                mode: AttachMode::Docked,
            },
            EditorSessionEvent::Closed,
        ]
    };
    if events == expected {
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
            format!("unexpected re-dock event sequence: {rendered:?}"),
        );
        ok = false;
    }
    (ok, rendered)
}

/// `real-editor-redock` post-open observation: child-HWND attach
/// contract → `session.reattach` under the same real lease through
/// the production `InProcessEditorOps` (the defined `Unsupported`
/// refusal today — no re-dock verb on the L0 surface) → the session
/// must land `Failed{Unsupported}` per contract → `close` →
/// `IsWindow(child) == 0` → the exact `[Attached{Docked}, Detached,
/// Failed{Unsupported}, Closed]` event sequence. No pixel capture —
/// the oracle is session events + HWND contract + session state.
fn observe_editor_redock_lifecycle(
    container: Option<u64>,
    lease_raw: u64,
    sessions: &mut PluginEditorSessions,
    session_id: PluginEditorSessionId,
    host: &mut InProcessHost,
    handle: PluginHandle,
    steps: &mut Vec<Value>,
) -> EditorOpVerification {
    let mut failure: Option<String> = None;
    let (mut observation, mut attach_ok) =
        observe_attached_children(container, steps, &mut failure);

    // The re-dock attempt — same lease (`ParentSlotLease` wraps the
    // registry's opaque payload; the verb refuses before any redeem
    // today, and the lease itself stays honest evidence).
    let mut landing = OpLanding::Other;
    match sessions.session_mut(session_id).map(|session| {
        let mut ops = rejecting_editor_ops(host, handle);
        session.reattach(&mut ops, &ParentSlotLease::new(lease_raw))
    }) {
        Some(Err(EditorSessionError::Ops(EditorOpError::Unsupported))) => {
            steps.push(json!({
                "step": "editor_redock",
                "verdict": "unsupported",
                "detail": "session.reattach → InProcessEditorOps::reattach \
                           → EditorOpError::Unsupported — no re-dock verb \
                           on the L0 surface; the session lands Failed \
                           per contract",
            }));
            landing = OpLanding::Refused;
            // Contract: a refused re-attach lands `Failed`, never
            // silently `Attached`.
            if session_failed_unsupported(sessions, session_id) {
                steps.push(json!({
                    "step": "session_state_after_op",
                    "verdict": "pass",
                    "state": "Failed{Unsupported}",
                }));
            } else {
                record_obs_fail(
                    steps,
                    &mut failure,
                    "session_state_after_op",
                    "refused reattach did not land Failed{Unsupported}",
                );
            }
        }
        Some(Ok(())) => {
            steps.push(json!({
                "step": "editor_redock",
                "verdict": "pass",
                "detail": "reattach accepted — a Reattached event and a \
                           live child contract are expected",
            }));
            landing = OpLanding::Accepted;
            // An accepted re-dock may recreate the editor surface —
            // re-run the child contract against the live container.
            let (re_observation, re_ok) = observe_attached_children(container, steps, &mut failure);
            attach_ok = attach_ok && re_ok;
            observation = re_observation;
        }
        Some(Err(error)) => {
            record_obs_fail(steps, &mut failure, "editor_redock", error.to_string());
        }
        None => {
            record_obs_fail(
                steps,
                &mut failure,
                "editor_redock",
                "editor session vanished before reattach",
            );
        }
    }

    // Convergence — close (production `close_gui` → `removed()` →
    // `DestroyWindow`), post-detach `IsWindow(child) == 0`, and the
    // re-dock-specific event sequence.
    let close_ok = close_editor_session(sessions, session_id, host, handle, steps, &mut failure);
    let detach_ok =
        !observation.supported || check_detached_children(&mut observation, steps, &mut failure);
    let (events_ok, rendered) = check_redock_events(
        sessions,
        session_id,
        landing == OpLanding::Refused,
        steps,
        &mut failure,
    );
    EditorOpVerification {
        supported: observation.supported,
        landing,
        verified: attach_ok && close_ok && detach_ok && events_ok,
        failure,
        extras: Value::Object(child_extras(container, &observation, &rendered)),
    }
}

/// Shared body for the non-pixel editor-op cases — identical
/// prerequisite/attempt mapping to `editor_attach_body`; the
/// case-supplied `observe_open` drives the op attempt and decides
/// the verdict: verified + defined refusal → `Unsupported`,
/// accepted-but-unverifiable → `NotRun`, any contract mismatch →
/// `Fail`.
fn editor_op_body(
    gui: &mut RealOsGuiTest,
    window: WindowId,
    view: ViewInstanceId,
    plugin: Option<&PluginPair>,
    plugin_error: Option<&PluginUnavailable>,
    observe_open: impl FnOnce(
        Option<u64>,
        u64,
        &mut PluginEditorSessions,
        PluginEditorSessionId,
        &mut InProcessHost,
        PluginHandle,
        &mut Vec<Value>,
    ) -> EditorOpVerification,
) -> ScenarioOutcome {
    let (mut host, handle) = match prepare_editor_host(plugin, plugin_error) {
        Ok(pair) => pair,
        Err(outcome) => return *outcome,
    };

    let mut steps = Vec::new();
    let (attempt, cleanup, verification) = attempt_real_attach(
        gui,
        window,
        view,
        &mut host,
        handle,
        &mut steps,
        observe_open,
    );

    let mut outcome = match attach_attempt_failure(&attempt) {
        Some(outcome) => outcome,
        None => match verification.as_ref() {
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
            Some(verification)
                if verification.verified && verification.landing == OpLanding::Refused =>
            {
                // The op ran through the production ops adapter on a
                // real lease and returned the defined `Unsupported`
                // refusal; every non-pixel contract check agreed —
                // the honest per-case judgment is `Unsupported`.
                let mut outcome = ScenarioOutcome::new(ScenarioVerdict::Unsupported);
                outcome.detail = Some(
                    "editor op attempted through the REAL container lease \
                     returned the defined EditorOpError::Unsupported — \
                     the verb is absent on the L0 ops surface; child-HWND \
                     contract, session state, events, and close \
                     convergence all verified without pixel evidence"
                        .to_string(),
                );
                outcome
                    .evidence
                    .push("editor-op:defined-refusal-real-lease".to_string());
                outcome
            }
            Some(verification)
                if verification.verified && verification.landing == OpLanding::Accepted =>
            {
                // The op was accepted AND every non-pixel contract
                // check agreed — but those oracles verify lifecycle,
                // not the op's effect, so the honest verdict stays
                // `NotRun`. An accepted op whose verification FAILED
                // falls through to `Fail` below: a contract violation
                // is never demoted to `NotRun`.
                not_run(
                    "editor op was accepted and every lifecycle/HWND/ \
                     event check agreed, but no oracle confirms the \
                     op's effect took place — judgment deferred until \
                     a session/HWND oracle lands",
                )
            }
            Some(verification) => fail(format!(
                "editor opened through the REAL container lease but the \
                 op observation failed: {}",
                verification
                    .failure
                    .as_deref()
                    .unwrap_or("unknown mismatch")
            )),
            None => fail("editor opened but no op observation was recorded".to_string()),
        },
    };
    let mut observed = Map::from_iter([
        ("steps".to_string(), json!(steps)),
        ("cleanup".to_string(), json!(cleanup)),
    ]);
    if let Some(verification) = &verification
        && let Value::Object(fields) = &verification.extras
    {
        observed.extend(fields.clone());
    }
    outcome.observed = Some(Value::Object(observed));
    outcome
}

/// `real-editor-resize` case: pre-flight requires `NativePluginDock`,
/// then the body drives the real attach chain plus the `set_size`
/// attempt — judged by session state/events/HWND contract only,
/// never pixels.
fn real_editor_resize_case(ctx: &mut RealDockContext<'_>, spec: &ScenarioSpec) -> ScenarioReport {
    let window = ctx.window;
    let view = ctx.view;
    let plugin = ctx.plugin;
    let plugin_error = ctx.plugin_error;
    let artifact = ctx.artifact;
    let report = run_scenario(&mut *ctx.gui, spec, |gui| {
        Ok(editor_op_body(
            gui,
            window,
            view,
            plugin,
            plugin_error,
            observe_editor_resize_lifecycle,
        ))
    });
    finalize_real(
        report,
        ctx.env,
        EnvironmentKind::RealWindow,
        artifact,
        Map::new(),
    )
}

/// `real-editor-redock` case: pre-flight requires `NativePluginDock`,
/// then the body drives the real attach chain plus the `reattach`
/// attempt under the same lease — judged by session events, child
/// HWND contract, and session state only, never pixels.
fn real_editor_redock_case(ctx: &mut RealDockContext<'_>, spec: &ScenarioSpec) -> ScenarioReport {
    let window = ctx.window;
    let view = ctx.view;
    let plugin = ctx.plugin;
    let plugin_error = ctx.plugin_error;
    let artifact = ctx.artifact;
    let report = run_scenario(&mut *ctx.gui, spec, |gui| {
        Ok(editor_op_body(
            gui,
            window,
            view,
            plugin,
            plugin_error,
            observe_editor_redock_lifecycle,
        ))
    });
    finalize_real(
        report,
        ctx.env,
        EnvironmentKind::RealWindow,
        artifact,
        Map::new(),
    )
}

// ---------------------------------------------------------------------
// case: real-session-terminate (host loss → Terminated exactly-once)
// ---------------------------------------------------------------------

/// Host-loss phase — `Deactivate` then `Destroy` on the live
/// instance (an `Activated` plugin must be deactivated before
/// `Destroy`; the headless `host-terminate` case uses the same
/// order). Each command lands as its own step row and both run
/// independently: a failed `Deactivate` does not skip `Destroy` —
/// a `Failed` landing makes `Destroy` legal, and the extra
/// evidence never masks the first failure. The instance drop runs
/// the adapter's `close_view` → `removed()` → `DestroyWindow`
/// synchronously, so the child HWNDs are already dead when this
/// returns. Returns whether both commands landed.
fn destroy_editor_instance(
    host: &mut InProcessHost,
    handle: PluginHandle,
    steps: &mut Vec<Value>,
    failure: &mut Option<String>,
) -> bool {
    let mut ok = true;
    for (name, command) in [
        ("deactivate", HostCommand::Deactivate { handle }),
        ("destroy", HostCommand::Destroy { handle }),
    ] {
        match host.execute(command) {
            Ok(_) => steps.push(json!({ "step": name, "verdict": "pass" })),
            Err(error) => {
                record_obs_fail(steps, failure, name, error.to_string());
                ok = false;
            }
        }
    }
    ok
}

/// Registry termination phase — `notify_terminated(HostLost)`
/// converges the attached session: `Closed`+`concluded`, exactly one
/// `Terminated{HostLost}` event (the `Closed` event is graceful-close
/// only and never appears here), and a repeated notify is a no-op
/// because the instance binding was already removed. Returns
/// `(converged_ok, rendered_events)`.
fn terminate_editor_session(
    sessions: &mut PluginEditorSessions,
    session_id: PluginEditorSessionId,
    handle: PluginHandle,
    steps: &mut Vec<Value>,
    failure: &mut Option<String>,
) -> (bool, Vec<String>) {
    sessions.notify_terminated(handle, SessionFailCause::HostLost);
    steps.push(json!({
        "step": "notify_terminated",
        "verdict": "pass",
        "cause": "host-lost",
    }));

    let mut ok = true;
    if session_closed_concluded(sessions, session_id) {
        steps.push(json!({ "step": "session_concluded", "verdict": "pass" }));
    } else {
        record_obs_fail(
            steps,
            failure,
            "session_concluded",
            "session did not converge to Closed+concluded after Terminated",
        );
        ok = false;
    }

    let events = sessions
        .session_mut(session_id)
        .map_or_else(Vec::new, PluginEditorSession::drain_events);
    let rendered = events_debug(&events);
    let expected = [
        EditorSessionEvent::Attached {
            mode: AttachMode::Docked,
        },
        EditorSessionEvent::Terminated {
            cause: SessionFailCause::HostLost,
        },
    ];
    if events == expected {
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
            format!("expected [Attached{{Docked}}, Terminated{{HostLost}}], got {rendered:?}"),
        );
        ok = false;
    }

    // exactly-once — the first notify removed the instance binding,
    // so a repeated notify (or any late callback) emits nothing.
    sessions.notify_terminated(handle, SessionFailCause::HostLost);
    let after = sessions
        .session_mut(session_id)
        .map_or_else(Vec::new, PluginEditorSession::drain_events);
    if after.is_empty() {
        steps.push(json!({
            "step": "notify_terminated_again",
            "verdict": "pass",
            "result": "no-op",
        }));
    } else {
        record_obs_fail(
            steps,
            failure,
            "notify_terminated_again",
            format!(
                "second notify_terminated produced events: {:?}",
                events_debug(&after)
            ),
        );
        ok = false;
    }
    (ok, rendered)
}

/// Host-survival phase — the host command surface still answers
/// (`Scan` → `Scanned`) while the destroyed instance's handle is
/// rejected with `InvalidHandle`: the containing host outlives the
/// plugin instance, and the DSP/host lifecycle is not conflated
/// with the editor-session lifecycle. Returns whether both checks
/// agreed.
fn check_host_survives(
    host: &mut InProcessHost,
    handle: PluginHandle,
    steps: &mut Vec<Value>,
    failure: &mut Option<String>,
) -> bool {
    let mut ok = true;
    match host.execute(HostCommand::Scan) {
        Ok(HostReply::Scanned(_)) => {
            steps.push(json!({ "step": "host_survives", "verdict": "pass" }));
        }
        Ok(_) => {
            record_obs_fail(
                steps,
                failure,
                "host_survives",
                "Scan returned an unexpected reply shape",
            );
            ok = false;
        }
        Err(error) => {
            record_obs_fail(steps, failure, "host_survives", error.to_string());
            ok = false;
        }
    }
    match host.execute(HostCommand::Initialize { handle }) {
        Err(HostError::InvalidHandle) => {
            steps.push(json!({
                "step": "destroyed_handle_rejected",
                "verdict": "pass",
            }));
        }
        other => {
            record_obs_fail(
                steps,
                failure,
                "destroyed_handle_rejected",
                format!("expected HostError::InvalidHandle, got {other:?}"),
            );
            ok = false;
        }
    }
    ok
}

/// `real-session-terminate` post-open observation: child-HWND
/// attach contract → host loss (`Deactivate`+`Destroy` — the
/// instance drop runs `close_view` → `removed()` → `DestroyWindow`
/// synchronously) → registry `notify_terminated(HostLost)` →
/// `Closed`+`concluded` with exactly `[Attached{Docked},
/// Terminated{HostLost}]` → repeated notify no-op → every recorded
/// child `IsWindow(child) == 0` → host survival (`Scan` succeeds,
/// destroyed handle `InvalidHandle`). No graceful `session.close`
/// runs — the terminal event is `Terminated`, never `Closed`, and
/// the child teardown is observed at OS level, not inferred from
/// the session state.
fn observe_session_terminate_lifecycle(
    container: Option<u64>,
    _lease_raw: u64,
    sessions: &mut PluginEditorSessions,
    session_id: PluginEditorSessionId,
    host: &mut InProcessHost,
    handle: PluginHandle,
    steps: &mut Vec<Value>,
) -> ChildVerification {
    let mut failure: Option<String> = None;
    let (mut observation, attach_ok) = observe_attached_children(container, steps, &mut failure);

    let destroy_ok = destroy_editor_instance(host, handle, steps, &mut failure);
    let (terminated_ok, rendered) =
        terminate_editor_session(sessions, session_id, handle, steps, &mut failure);
    let detach_ok =
        !observation.supported || check_detached_children(&mut observation, steps, &mut failure);
    let host_ok = check_host_survives(host, handle, steps, &mut failure);

    ChildVerification {
        supported: observation.supported,
        verified: attach_ok && destroy_ok && terminated_ok && detach_ok && host_ok,
        failure,
        extras: Value::Object(child_extras(container, &observation, &rendered)),
    }
}

/// `real-session-terminate` body — the host-loss child-HWND
/// lifecycle through [`editor_child_body`].
fn session_terminate_body(
    gui: &mut RealOsGuiTest,
    window: WindowId,
    view: ViewInstanceId,
    plugin: Option<&PluginPair>,
    plugin_error: Option<&PluginUnavailable>,
) -> ScenarioOutcome {
    editor_child_body(
        gui,
        window,
        view,
        plugin,
        plugin_error,
        observe_session_terminate_lifecycle,
        &ChildCaseIdentity {
            pass_detail: "test_editor real IPlugView attached under the \
                          leased container HWND; host Destroy converged the \
                          session to Terminated{HostLost} exactly once \
                          (repeated notify is a no-op), the child HWND tore \
                          down (IsWindow(child)==0), and the host survived",
            evidence: "session-terminate:host-loss-exactly-once",
            label: "session-terminate",
        },
    )
}

/// `real-session-terminate` case: pre-flight requires
/// `NativePluginDock`, then the body drives the real attach chain
/// plus the host-loss lifecycle — judged by session events, child
/// HWND teardown, and host survival only, never pixels.
fn real_session_terminate_case(
    ctx: &mut RealDockContext<'_>,
    spec: &ScenarioSpec,
) -> ScenarioReport {
    let window = ctx.window;
    let view = ctx.view;
    let plugin = ctx.plugin;
    let plugin_error = ctx.plugin_error;
    let artifact = ctx.artifact;
    let report = run_scenario(&mut *ctx.gui, spec, |gui| {
        Ok(session_terminate_body(
            gui,
            window,
            view,
            plugin,
            plugin_error,
        ))
    });
    finalize_real(
        report,
        ctx.env,
        EnvironmentKind::RealWindow,
        artifact,
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

    /// A session that never started must produce `NotRun`
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
        assert_eq!(reports.len(), 13, "thirteen real-dock cases");

        // The blocked path reads the same catalog rows — case ids
        // and required caps must match exactly.
        let entries: [ScenarioEntry<RealDockContext<'static>>; 13] = real_entries();
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
        let entries: [ScenarioEntry<RealDockContext<'static>>; 13] = real_entries();
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
