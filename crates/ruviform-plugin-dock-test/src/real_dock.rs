//! Real-window plugin-dock lane (Issue #631 real-window slice —
//! feature `real-dock`; `real-dcomp-pixel`/`real-dcomp-foreground`/
//! `real-dcomp-present` added by the Issue #532
//! dcomp-pixel/foreground-probe/presentation-probe slices).
//!
//! [`run_all`] mounts [`plugin_dock_view_spec`] into *real OS
//! windows* through [`RealOsGuiTest`] and records the twenty-two
//! cases of the `rf631.plugin-dock-real` scenario:
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
//! | `real-dcomp-foreground` | `RealOs`    | `Pass`/`Fail`/`NotRun`/`Unsupported` — GPUI root の |
//! |                         |             | z-order/foreground を段階変異し hit-test+screen     |
//! |                         |             | green を再観測して外部 stacking vs compositor 内部  |
//! |                         |             | 遮蔽を二分。verdict は probe 実行+復元のみ           |
//! | `real-dcomp-present`    | `RealOs`    | `Pass`/`Fail`/`NotRun`/`Unsupported` — 変異なしの   |
//! |                         |             | presentation probe: cover inventory・presentation    |
//! |                         |             | attributes・PrintWindow flag 変種・screen-vs-suspect |
//! |                         |             | fingerprint で残余 3 候補を判別。verdict は probe    |
//! |                         |             | 実行+lifecycle のみ（観測値はデータ）                |
//! | `real-multiwindow-focus`| `RealOs`    | `Pass`/`Fail`/`NotRun` — 2 window の `request_focus` |
//! |                         |             | → `window_focused` 観測による focus 移譲            |
//! | `real-os-input`         | `RealOs`    | `Pass`/`Fail`/`NotRun`/`Unsupported` — preflight     |
//! |                         |             | effective な実 OS pointer/key 注入が `Injected`     |
//! | `real-command-receipt`  | `RealOs`    | `Pass`/`Fail`/`NotRun` — fader fixture の            |
//! |                         |             | Begin→Preview→Commit が receipt-expecting intent を  |
//! |                         |             | mint し、`post_event` の applied/rejected `result`   |
//! |                         |             | が `command_id` で binding を Applied→Observed /     |
//! |                         |             | Rejected に settle。decode 済み `CommandReceipt` を   |
//! |                         |             | report.receipts へ populate（AC1 link + AC4）        |
//! | `real-window-dpi`       | `RealOs`    | `Pass`/`Fail`/`NotRun` — live `window_geometry` +   |
//! |                         |             | lifecycle `Resized`/`ScaleFactorChanged` 整合を      |
//! |                         |             | 観測判定（DPI 変更経路は未駆動 — 観測のみ）           |
//! | `real-window-ime`       | `RealOs`    | `Unsupported` — `Ime` capability は未 claim（#389）  |
//! | `real-window-close`     | `RealOs`    | `Pass`/`Fail`/`NotRun` — armed `Veto`/`Defer`→        |
//! |                         |             | `Approve`/既定 `Approve` の `CloseRequested`→         |
//! |                         |             | `Destroyed` を B/C window で実観測（A は sentinel）   |
//! | `real-editor-resize`    | `RealOs`    | `Pass`/`Fail`/`NotRun`/`Unsupported` — `set_size` を     |
//! |                         |             | 実 lease 経由で実行し、child HWND の `GetWindowRect` が  |
//! |                         |             | 要求サイズと一致することを非画素 oracle で検証           |
//! | `real-editor-redock`    | `RealOs`    | `Pass`/`Fail`/`NotRun`/`Unsupported` — `reattach` を     |
//! |                         |             | 同一 live lease で実行し、旧 child HWND の破棄 + 新      |
//! |                         |             | attach generation（window title）の出現を検証            |
//! | `real-session-terminate`| `RealOs`    | `Pass`/`Fail`/`Unsupported` — attach 中の            |
//! |                         |             | `HostCommand::Destroy` → `Terminated{HostLost}`       |
//! |                         |             | exactly-once → child HWND teardown → host 存続        |
//! | `real-session-degraded` | `RealOs`    | `Pass`/`Fail`/`Unsupported` — `open_or_degrade`      |
//! |                         |             | (Floating→Docked) の `Degraded{Docked}` event +       |
//! |                         |             | `OpenOutcome::Degraded` + child HWND 契約             |
//! | `real-session-recovery` | `RealOs`    | `Pass`/`Fail`/`Unsupported` — refused open →         |
//! |                         |             | `Failed{Unsupported}` → `recover(Docked)` →           |
//! |                         |             | `Recovered{Docked}` + child HWND 契約                 |
//! | `real-session-timeout`  | `RealOs`    | `Pass`/`Fail`/`Unsupported` — ZERO deadline policy +  |
//! |                         |             | `begin_detach` + `tick_all` → `Failed{Timeout}` +     |
//! |                         |             | child HWND 契約                                       |
//! | `real-editor-visibility`| `RealOs`    | `Pass`/`Fail`/`NotRun`/`Unsupported` — `set_visibility` |
//! |                         |             | の hide→destroy / show→recreate cycle を child HWND     |
//! |                         |             | 生死 + `Hidden`/`Shown` event で検証                    |
//! | `real-session-crash`    | `RealOs`    | `Unsupported` — in-process lane は fixture の         |
//! |                         |             | process-killing fault を生存不能（out-of-process      |
//! |                         |             | host 未実装）                                         |
//! | `real-session-hang`     | `RealOs`    | `Unsupported` — 同上（wedging fault で lane 非生存）  |
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
    InProcessHost, MonotonicSessionClock, OpenOutcome, ParentLease, ParentSlotLease,
    PluginEditorSession, PluginEditorSessionId, PluginEditorSessions, PluginHandle,
    SessionDeadlinePolicy, SessionFailCause,
};
use ruviform_plugin_vst3::host::Vst3Library;
use ruviform_ui::behavior::{CommandIntent, CommandPhase, CommandReceipt, ValueInput};
use ruviform_ui::dock::ViewInstanceId;
use ruviform_ui::ipc::{ErrorBody, Event};
use ruviform_ui::native_slot::{
    LogicalRect, SlotCapabilities, SlotGeometry, SlotLease, SlotSpec, SlotVisibility, Support,
    SurfaceSlotId,
};
use ruviform_ui::view::{AttachEpochAllocator, ComponentKey, ComponentSnapshot, ViewHost};
use ruviform_ui::{CloseDecision, LogicalPosition, WindowId, WindowSpec};
use ruviform_ui_gpui::os_input::{OsInputOutcome, OsKey, PreflightScope};
use ruviform_ui_gpui::{
    OsWindowGeometry, RealOsGuiTest, RealWindowMount, RealWindowOptions, ViewMount,
    WindowLifecycleEvent,
};
use ruviform_ui_test::scenario::plugin_dock::{
    PLUGIN_DOCK_SLOT_KEY, PluginArtifactEvidence, plugin_dock_view_spec,
};
use ruviform_ui_test::scenario::visual::{CaptureOutcome, VisualGuiTest};
use ruviform_ui_test::{
    CaseId, MintedIntent, RunId, ScenarioCapability, ScenarioDriver, ScenarioEntry, ScenarioError,
    ScenarioId, ScenarioOutcome, ScenarioReport, ScenarioSpec, ScenarioSpecExtras, ScenarioTarget,
    ScenarioVerdict, SemanticGuiTest, TargetQuery, parity_gesture_scenario, run_entries,
    run_scenario, settle_until, sv_fader_dock_fixture,
};
use std::path::{Path, PathBuf};
use std::time::Duration;

use ruviform_ui_test::golden::RgbaImage;
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

use crate::LaneEnvironment;
use crate::artifact;
use crate::osmut;
use crate::osoccl;
use crate::ospix;
use crate::ospres;
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
/// editor/session cases (`real-editor-attach`, `real-dcomp-pixel`,
/// `real-dcomp-foreground`, `real-dcomp-present`,
/// `real-editor-resize`, `real-editor-redock`,
/// `real-session-terminate`, `real-session-degraded`,
/// `real-session-recovery`, `real-session-timeout`,
/// `real-editor-visibility`) consume it.
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
    /// focus-transfer target and `real-window-close`'s
    /// veto/defer/approve target. `None` only if the session
    /// mounted fewer windows (never on the live path — three
    /// mounts are requested up front).
    window_b: Option<WindowId>,
    /// The third mounted window id — `real-window-close`'s
    /// default-approve target; window A stays the sentinel the
    /// remaining cases build on. `None` only if the session
    /// mounted fewer than three windows.
    window_c: Option<WindowId>,
    /// The fourth mounted window id — `real-command-receipt`'s
    /// command-minting fader view (`sv_fader_dock_fixture`). The
    /// dock spec mounts no command bindings, so the receipt case
    /// gets its own window rather than disturbing A/B/C. `None`
    /// only if the session mounted fewer than four windows.
    window_d: Option<WindowId>,
    /// The primary window's live scale factor measured once at
    /// session start through `window_geometry` — stamped into
    /// every case's `environment.scale_factor` (AC4 evidence;
    /// `None` keeps the honest `null` when unmeasured).
    session_scale_factor: Option<f64>,
    /// The mounted view instance id.
    view: ViewInstanceId,
    /// The fourth window's view instance id — the fader-dock
    /// fixture `real-command-receipt` drives.
    view_d: ViewInstanceId,
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
    /// Opt-in pixel-artifact directory for `real-dcomp-pixel` and
    /// `real-dcomp-foreground` (`--pixel-artifacts`); `None` records
    /// stats only.
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
/// `Some`, `real-dcomp-pixel` and `real-dcomp-foreground` write each
/// captured frame as a PNG under `dir` (created on demand).
/// Artifact write failures land as step notes — they never change a
/// verdict. The canonical lane calls [`run_all`], so no artifacts
/// are written by default.
///
/// # Panics
/// Same contract as [`run_all`].
#[expect(clippy::too_many_lines)]
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
    let view_c = ViewInstanceId::new(633).expect("view instance id 633 is non-zero");
    let view_d = ViewInstanceId::new(634).expect("view instance id 634 is non-zero");
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
    let mut host_c = ViewHost::new(view_c, epochs.mint());
    host_c
        .mount(plugin_dock_view_spec())
        .expect("plugin dock view spec mounts");
    host_c.drain_events();
    // Window D hosts the command-minting fader fixture — the dock
    // spec carries no command bindings, so `real-command-receipt`
    // needs a view whose controls mint receipt-expecting intents.
    let mut host_d = ViewHost::new(view_d, epochs.mint());
    host_d
        .mount(sv_fader_dock_fixture().spec)
        .expect("fader dock fixture spec mounts");
    host_d.drain_events();

    // Four mounts: window A stays the sentinel every later case
    // builds on (the GPUI platform loop exits when its last window
    // closes), window B gives `real-multiwindow-focus` a
    // focus-transfer target and `real-window-close` its
    // veto/defer/approve target, window C is the close case's
    // default-approve target, and window D hosts the fader fixture
    // `real-command-receipt` drives. B/C/D are offset right of A so
    // the windows never overlap (an OS click on A's panel center
    // must land on A).
    let window_spec_b = WindowSpec {
        title: "ruviform plugin-dock B".to_string(),
        position: Some(LogicalPosition::new(1450, 80)),
        ..WindowSpec::default()
    };
    let window_spec_c = WindowSpec {
        title: "ruviform plugin-dock C".to_string(),
        position: Some(LogicalPosition::new(2900, 80)),
        ..WindowSpec::default()
    };
    let window_spec_d = WindowSpec {
        title: "ruviform fader-dock D".to_string(),
        position: Some(LogicalPosition::new(4350, 80)),
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
            RealWindowMount {
                window: window_spec_c,
                view: ViewMount {
                    host: host_c,
                    theme: None,
                },
            },
            RealWindowMount {
                window: window_spec_d,
                view: ViewMount {
                    host: host_d,
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
            let window_c = windows.next();
            let window_d = windows.next();
            // The session's primary-window scale factor, measured
            // once — stamped into every case's `environment.
            // scale_factor` (AC4; a failed read stays the honest
            // `null`).
            let session_scale_factor = gui
                .window_geometry(window)
                .ok()
                .map(|geometry| geometry.scale_factor);
            // The session-scoped catalog — `real_entries` rows run
            // through `run_entries` against the live context.
            let entries = real_entries();
            let mut ctx = RealDockContext {
                gui,
                window,
                window_b,
                window_c,
                window_d,
                session_scale_factor,
                view,
                view_d,
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

/// [`finalize_real`] plus the session's measured primary-window
/// scale factor — stamped into `environment.scale_factor` when the
/// session recorded one (AC4 evidence; `extra` overrides the honest
/// `null` base in `case_environment`). `blocked_reports` keeps
/// [`finalize`] — no session exists there, so `null` stays the
/// honest record.
fn finalize_case(
    report: ScenarioReport,
    ctx: &RealDockContext<'_>,
    kind: EnvironmentKind,
    artifact: Option<&PluginArtifactEvidence>,
    extra: Map<String, Value>,
) -> ScenarioReport {
    let mut merged = extra;
    if let Some(scale_factor) = ctx.session_scale_factor {
        merged.insert("scale_factor".to_string(), json!(scale_factor));
    }
    finalize_real(report, ctx.env, kind, artifact, merged)
}

/// The `rf631.plugin-dock-real` catalog rows — session-scoped: the
/// entries' `run` needs the live `RealOsGuiTest`, so the table is
/// produced per call (the entries themselves are `'static` fn items;
/// only the context type carries the session borrow).
/// `blocked_reports` reads the same rows for case ids and required
/// capabilities, so the blocked path can never drift from the live
/// catalog.
#[expect(clippy::too_many_lines)]
fn real_entries<'a>() -> [ScenarioEntry<RealDockContext<'a>>; 22] {
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
            case: "real-dcomp-foreground",
            scenario: SCENARIO,
            required: &[
                ScenarioCapability::NativePluginDock,
                ScenarioCapability::VisualCapture,
            ],
            spec_extras: editor_attach_extras,
            run: real_dcomp_foreground_case,
        },
        ScenarioEntry {
            case: "real-dcomp-present",
            scenario: SCENARIO,
            required: &[
                ScenarioCapability::NativePluginDock,
                ScenarioCapability::VisualCapture,
            ],
            spec_extras: editor_attach_extras,
            run: real_dcomp_present_case,
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
            case: "real-command-receipt",
            scenario: SCENARIO,
            required: &[ScenarioCapability::SemanticAction],
            spec_extras: real_window_extras,
            run: real_command_receipt_case,
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
            required: &[ScenarioCapability::MultiWindow],
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
        ScenarioEntry {
            case: "real-session-degraded",
            scenario: SCENARIO,
            required: &[ScenarioCapability::NativePluginDock],
            spec_extras: editor_attach_extras,
            run: real_session_degraded_case,
        },
        ScenarioEntry {
            case: "real-session-recovery",
            scenario: SCENARIO,
            required: &[ScenarioCapability::NativePluginDock],
            spec_extras: editor_attach_extras,
            run: real_session_recovery_case,
        },
        ScenarioEntry {
            case: "real-session-timeout",
            scenario: SCENARIO,
            required: &[ScenarioCapability::NativePluginDock],
            spec_extras: editor_attach_extras,
            run: real_session_timeout_case,
        },
        ScenarioEntry {
            case: "real-editor-visibility",
            scenario: SCENARIO,
            required: &[ScenarioCapability::NativePluginDock],
            spec_extras: editor_attach_extras,
            run: real_editor_visibility_case,
        },
        ScenarioEntry {
            case: "real-session-crash",
            scenario: SCENARIO,
            required: &[ScenarioCapability::NativePluginDock],
            spec_extras: real_window_extras,
            run: real_session_crash_case,
        },
        ScenarioEntry {
            case: "real-session-hang",
            scenario: SCENARIO,
            required: &[ScenarioCapability::NativePluginDock],
            spec_extras: real_window_extras,
            run: real_session_hang_case,
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

/// Count of `window`'s recorded lifecycle events equal to `event`
/// — a non-destructive snapshot read, safe inside `settle_until`
/// predicates (the log is append-only for the session).
fn lifecycle_event_count(
    gui: &RealOsGuiTest,
    window: WindowId,
    event: WindowLifecycleEvent,
) -> usize {
    gui.window_events(window)
        .iter()
        .filter(|observed| **observed == event)
        .count()
}

/// The ordered `CloseRequested`/`Destroyed` subsequence of
/// `window`'s lifecycle log — the close-case's verdict-relevant
/// record.
fn close_sequence(gui: &RealOsGuiTest, window: WindowId) -> Vec<&'static str> {
    gui.window_events(window)
        .iter()
        .filter_map(|event| match event {
            WindowLifecycleEvent::CloseRequested => Some("close_requested"),
            WindowLifecycleEvent::Destroyed => Some("destroyed"),
            _ => None,
        })
        .collect()
}

/// Evidence summary of `window`'s recorded lifecycle log — counts
/// per variant, latest geometry-carrying values, and the ordered
/// close-relevant subsequence. Observation only: the log is the
/// app thread's record, never synthesized caller-side.
fn lifecycle_summary(gui: &RealOsGuiTest, window: WindowId) -> Value {
    let events = gui.window_events(window);
    let mut created = 0usize;
    let mut ready = 0usize;
    let mut resized = 0usize;
    let mut moved = 0usize;
    let mut focused = 0usize;
    let mut scale_factor_changed = 0usize;
    let mut latest_resized: Option<(u32, u32)> = None;
    let mut latest_scale_factor: Option<f64> = None;
    let mut close_events: Vec<&'static str> = Vec::new();
    for event in &events {
        match event {
            WindowLifecycleEvent::Created => created += 1,
            WindowLifecycleEvent::Ready => ready += 1,
            WindowLifecycleEvent::Resized { width, height } => {
                resized += 1;
                latest_resized = Some((*width, *height));
            }
            WindowLifecycleEvent::Moved { .. } => moved += 1,
            WindowLifecycleEvent::Focused(_) => focused += 1,
            WindowLifecycleEvent::ScaleFactorChanged { factor } => {
                scale_factor_changed += 1;
                latest_scale_factor = Some(*factor);
            }
            WindowLifecycleEvent::CloseRequested => close_events.push("close_requested"),
            WindowLifecycleEvent::Destroyed => close_events.push("destroyed"),
            _ => {}
        }
    }
    json!({
        "total": events.len(),
        "created": created,
        "ready": ready,
        "resized": resized,
        "moved": moved,
        "focused": focused,
        "scale_factor_changed": scale_factor_changed,
        "latest_resized": latest_resized.map(|(width, height)| json!({
            "width": width,
            "height": height,
        })),
        "latest_scale_factor": latest_scale_factor,
        "close_sequence": close_events,
    })
}

/// `steps`/`observed` JSON for one [`OsWindowGeometry`] — origin,
/// physical client size, and scale factor; the same physical-pixel
/// unit the lifecycle `Resized`/`Moved` events report.
fn geometry_json(geometry: &OsWindowGeometry) -> Value {
    json!({
        "origin_x": geometry.origin_x,
        "origin_y": geometry.origin_y,
        "width": geometry.width,
        "height": geometry.height,
        "scale_factor": geometry.scale_factor,
    })
}

/// JSON projection of one [`ComponentSnapshot`] for semantic-tree
/// evidence — every field type is `Serialize`; the snapshot struct
/// itself is not, so the evidence boundary maps it field-by-field
/// (AC4 — additive, observation only).
fn component_snapshot_json(snapshot: &ComponentSnapshot) -> Value {
    json!({
        "display": snapshot.display,
        "committed": snapshot.committed,
        "phase": snapshot.phase,
        "observed": snapshot.observed,
        "selected": snapshot.selected,
        "mounted": snapshot.mounted,
    })
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
    let entries: [ScenarioEntry<RealDockContext<'static>>; 22] = real_entries();
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
    let entries: [ScenarioEntry<RealDockContext<'static>>; 22] = real_entries();
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
        let snapshot = gui.observe(&target, &key)?;
        // AC4 semantic-tree evidence — best-effort `ComponentSnapshot`
        // projection of the dock panel and slot: observation only,
        // never a verdict input; an unobservable component records
        // `null` rather than a fabricated tree.
        let panel_key = ComponentKey("plugin.dock".to_string());
        let panel_snapshot = (|| {
            let panel_target = gui
                .resolve_target(&TargetQuery::new(window, view).component(panel_key.clone()))
                .ok()?;
            gui.observe(&panel_target, &panel_key).ok()
        })();
        let mut tree = Map::new();
        tree.insert(
            "plugin.dock".to_string(),
            panel_snapshot
                .as_ref()
                .map_or(Value::Null, component_snapshot_json),
        );
        tree.insert(
            PLUGIN_DOCK_SLOT_KEY.to_string(),
            component_snapshot_json(&snapshot),
        );
        let mut outcome = ScenarioOutcome::new(ScenarioVerdict::Pass);
        outcome.detail = Some(format!(
            "real OS window mounts plugin_dock_view_spec; NativeSlot \
             `{PLUGIN_DOCK_SLOT_KEY}` resolved and observed"
        ));
        outcome.observed = Some(json!({
            "window": window.to_string(),
            "component": PLUGIN_DOCK_SLOT_KEY,
            "semantic_tree": Value::Object(tree),
        }));
        outcome
            .evidence
            .push("real-window:semantic-mount:plugin.editor".to_string());
        Ok(outcome)
    });
    finalize_case(report, ctx, EnvironmentKind::RealWindow, None, Map::new())
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
    finalize_case(report, ctx, EnvironmentKind::RealWindow, None, Map::new())
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
    /// `GetWindowRect(child)` width/height in device pixels — the
    /// editor-op effect oracle for `set_size` (the plugin's `onSize`
    /// resizes its own window; the host only reports the size).
    width: i32,
    /// `GetWindowRect(child)` height in device pixels — see `width`.
    height: i32,
    /// `GetWindowTextW(child)` — the fixture stamps its attach
    /// generation here (`rvf-test-editor-attach-{n}`), the reliable
    /// detach/`removed` + `attached` recreation marker (an HWND
    /// value alone can be reused by Windows).
    title: String,
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
    use windows_sys::Win32::Foundation::{HWND, LPARAM, RECT};
    use windows_sys::Win32::System::Threading::GetCurrentThreadId;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumChildWindows, GetParent, GetWindowRect, GetWindowTextLengthW, GetWindowTextW,
        GetWindowThreadProcessId, IsWindow, IsWindowVisible,
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
                    let mut rect = RECT::default();
                    GetWindowRect(hwnd as HWND, &raw mut rect);
                    ChildWindowFacts {
                        hwnd,
                        thread_id: GetWindowThreadProcessId(hwnd as HWND, &raw mut pid),
                        width: rect.right - rect.left,
                        height: rect.bottom - rect.top,
                        title: window_text(hwnd),
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

    /// `GetWindowTextW` as a `String` — empty on any failure (a
    /// title is evidence garnish, never a verdict input by itself).
    /// Query-only.
    #[expect(unsafe_code)]
    #[expect(clippy::cast_sign_loss)]
    fn window_text(hwnd: u64) -> String {
        // SAFETY: query-only; `hwnd` is an HWND this enumeration just
        // returned.
        unsafe {
            let len = GetWindowTextLengthW(hwnd as HWND);
            if len <= 0 {
                return String::new();
            }
            let mut buf = vec![0u16; len as usize + 1];
            let got = GetWindowTextW(hwnd as HWND, buf.as_mut_ptr(), len + 1);
            if got <= 0 {
                return String::new();
            }
            String::from_utf16_lossy(&buf[..got as usize])
        }
    }

    /// `IsWindow(hwnd) != 0` — the editor-op effect oracles use this
    /// to prove a pre-op child really died (`removed()` →
    /// `DestroyWindow`) before the op's replacement was observed.
    /// Query-only; accepts any HWND value.
    #[expect(unsafe_code)]
    pub(super) fn is_alive(hwnd: u64) -> bool {
        // SAFETY: query-only handle-validity check.
        unsafe { IsWindow(hwnd as HWND) != 0 }
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

    /// No HWND concept off-Windows — never alive.
    pub(super) fn is_alive(_hwnd: u64) -> bool {
        false
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

/// The graceful-close event tail a plain docked attach must
/// produce — `[Attached{Docked}, Closed]`.
const ATTACH_CLOSE_EVENTS: &[EditorSessionEvent] = &[
    EditorSessionEvent::Attached {
        mode: AttachMode::Docked,
    },
    EditorSessionEvent::Closed,
];

/// Phase-4 session convergence — `Closed`+`concluded` and the
/// drained event stream must equal `expected` exactly; an
/// unexpected event (degrade, failure, terminate) is a contract
/// surprise, never a pass. Returns the checks' outcome plus the
/// rendered event list for `observed`.
fn check_session_converged(
    sessions: &mut PluginEditorSessions,
    session_id: PluginEditorSessionId,
    steps: &mut Vec<Value>,
    failure: &mut Option<String>,
    expected: &[EditorSessionEvent],
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
    if events.as_slice() == expected {
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
            format!(
                "unexpected session event sequence: expected {:?}, got {rendered:?}",
                events_debug(expected)
            ),
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
/// (`Closed`+`concluded`, drained events exactly `expected`).
/// Returns `(close_ok, detach_ok, converged_ok, rendered_events)`.
#[expect(clippy::too_many_arguments)]
fn conclude_editor_session(
    observation: &mut ChildWindowObservation,
    sessions: &mut PluginEditorSessions,
    session_id: PluginEditorSessionId,
    host: &mut InProcessHost,
    handle: PluginHandle,
    steps: &mut Vec<Value>,
    failure: &mut Option<String>,
    expected: &[EditorSessionEvent],
) -> (bool, bool, bool, Vec<String>) {
    // Phase 2 — `session.close` runs the production `close_gui` →
    // `removed()` → `DestroyWindow` path.
    let close_ok = close_editor_session(sessions, session_id, host, handle, steps, failure);

    // Phase 3 — post-detach: every recorded child must be dead
    // (`IsWindow(child) == 0`). Skipped off-Windows — the
    // `child_hwnd_observation` row already recorded `unsupported`.
    let detach_ok = !observation.supported || check_detached_children(observation, steps, failure);

    // Phase 4 — session convergence: `Closed`+`concluded` and the
    // drained event stream must equal `expected` exactly.
    let (converged_ok, rendered) =
        check_session_converged(sessions, session_id, steps, failure, expected);
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
#[expect(clippy::too_many_arguments)]
fn observe_editor_child_lifecycle(
    _gui: &RealOsGuiTest,
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
        ATTACH_CLOSE_EVENTS,
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

/// `InProcessEditorOps` redeeming session leases against the real
/// slot ledger through [`RealOsGuiTest::redeem_native_lease`] — the
/// issuer-side seam every real session op shares: the
/// session-facing `ParentSlotLease` payload decodes to a `SlotLease`,
/// the adapter validates it against its live ledger, and the minted
/// `ParentLease` carries the container token as payload. A
/// stale/forged lease never reaches the plugin.
fn real_editor_ops<'a>(
    gui: &'a RealOsGuiTest,
    host: &'a mut InProcessHost,
    handle: PluginHandle,
) -> InProcessEditorOps<'a> {
    InProcessEditorOps::with_redeemer(
        host,
        handle,
        Box::new(move |session_lease| {
            gui.redeem_native_lease(session_lease.raw())
                .map(|token| ParentLease::new(SlotLease::new(session_lease.raw()).seq(), token))
                .map_err(|_| EditorOpError::LeaseRejected)
        }),
    )
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
    let mut ops = real_editor_ops(gui, host, handle);
    sessions.session_mut(session_id).map(|session| {
        session.open(
            &mut ops,
            &parent_slot_lease,
            editor_size(),
            AttachMode::Docked,
        )
    })
}

/// The default open drive — `session.open(Docked)` through the real
/// lease. Records the same `session.open(Docked)` step row the
/// pre-seam `attempt_real_attach` emitted (verdict mapping and the
/// `failed_unsupported` flag unchanged) so existing cases' evidence
/// keeps its shape.
fn drive_open_docked(
    gui: &RealOsGuiTest,
    host: &mut InProcessHost,
    handle: PluginHandle,
    sessions: &mut PluginEditorSessions,
    session_id: PluginEditorSessionId,
    parent_slot_lease: ParentSlotLease,
    steps: &mut Vec<Value>,
) -> Option<Result<EditorCapabilities, EditorSessionError>> {
    let open_result =
        open_session_docked(gui, host, handle, sessions, session_id, parent_slot_lease);
    // Session state at this point is the honest landing — a refused
    // open sits `Failed{Unsupported}`.
    let failed_unsupported = session_failed_unsupported(sessions, session_id);
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
    open_result
}

/// The case-supplied session drive for [`attempt_real_attach`] —
/// which open to drive and which post-open observation to run,
/// bundled so the attempt stays within the argument limit.
struct AttachDrive<O, V> {
    /// Session-open drive — records its own step rows and returns
    /// the open landing (`None` = session vanished → `Setup`).
    drive_open: O,
    /// Post-open observation run while the lease is live.
    observe_open: V,
}

/// The real attach attempt — session begin, real-slot
/// declare/bind/project/`lease_parent` (real container HWND),
/// `container_token` observation, `ParentSlotLease` →
/// `InProcessEditorOps` redeem through
/// [`RealOsGuiTest::redeem_native_lease`], then the case's
/// [`AttachDrive`] session ops. An accepted open additionally runs
/// `observe_open` — the case's post-open observation + close
/// convergence — while the lease is still live. Always tears down
/// (session end, slot/lease release, instance destroy). Returns the
/// open outcome (or the setup failure), cleanup notes, and — only
/// when `open` returned `Ok` — the observation result that gates the
/// verdict.
fn attempt_real_attach<O, V, R>(
    gui: &mut RealOsGuiTest,
    window: WindowId,
    view: ViewInstanceId,
    host: &mut InProcessHost,
    handle: PluginHandle,
    steps: &mut Vec<Value>,
    drive: AttachDrive<O, V>,
) -> (RealAttachAttempt, Vec<String>, Option<R>)
where
    O: FnOnce(
        &RealOsGuiTest,
        &mut InProcessHost,
        PluginHandle,
        &mut PluginEditorSessions,
        PluginEditorSessionId,
        ParentSlotLease,
        &mut Vec<Value>,
    ) -> Option<Result<EditorCapabilities, EditorSessionError>>,
    V: FnOnce(
        &RealOsGuiTest,
        Option<u64>,
        u64,
        &mut PluginEditorSessions,
        PluginEditorSessionId,
        &mut InProcessHost,
        PluginHandle,
        &mut Vec<Value>,
    ) -> R,
{
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
    // The case's open drive records its own step rows and returns
    // the open landing — `None` means the session vanished (Setup).
    let open_result = (drive.drive_open)(
        gui,
        host,
        handle,
        &mut sessions,
        session_id,
        parent_slot_lease,
        steps,
    );

    // An accepted open means the plugin claims an editor surface —
    // run the case's post-open observation (child-HWND contract for
    // `real-editor-attach`, screen pixels for `real-dcomp-pixel`)
    // and close convergence. The observation decides Pass vs Fail;
    // it runs while the lease is still live.
    let observed = match &open_result {
        Some(Ok(_)) => Some((drive.observe_open)(
            gui,
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

/// The session-open drive signature [`attempt_real_attach`] invokes
/// after `begin_session` — records its own step rows and returns
/// the open landing (`None` = session vanished → `Setup`).
type DriveOpenFn = fn(
    &RealOsGuiTest,
    &mut InProcessHost,
    PluginHandle,
    &mut PluginEditorSessions,
    PluginEditorSessionId,
    ParentSlotLease,
    &mut Vec<Value>,
) -> Option<Result<EditorCapabilities, EditorSessionError>>;

/// The post-open observation signature [`attempt_real_attach`]
/// invokes while the lease is live — decides `verified`/`failure`/
/// `extras` for the verdict. `gui` is the live session driver so an
/// observation can redeem leases through the real slot ledger
/// (`real_editor_ops`) — `reattach` is the only verb that consults
/// the redeemer.
type ObserveOpenFn = fn(
    &RealOsGuiTest,
    Option<u64>,
    u64,
    &mut PluginEditorSessions,
    PluginEditorSessionId,
    &mut InProcessHost,
    PluginHandle,
    &mut Vec<Value>,
) -> ChildVerification;

/// A child-lifecycle case's session drive — the open drive, the
/// post-open observation, and the verdict identity bundled into one
/// value so [`editor_child_body`] stays within the argument limit.
#[derive(Clone, Copy)]
struct ChildLifecycle<'a> {
    /// Session-open drive invoked after `begin_session`.
    drive_open: DriveOpenFn,
    /// Post-open observation run while the lease is live.
    observe_open: ObserveOpenFn,
    /// Verdict identity — pass detail, evidence tag, label.
    case: &'a ChildCaseIdentity<'a>,
}

/// Shared body for the child-HWND lifecycle cases
/// (`real-editor-attach`, `real-session-terminate`,
/// `real-session-degraded`, `real-session-recovery`,
/// `real-session-timeout`) — identical prerequisite/attempt
/// mapping; the case-supplied [`ChildLifecycle`] drives the open
/// and the post-open observation, which decides the verdict:
/// verified → `Pass` (`pass_detail`/`evidence`),
/// platform-unsupported observation → `Unsupported`, any contract
/// mismatch → `Fail`.
fn editor_child_body(
    gui: &mut RealOsGuiTest,
    window: WindowId,
    view: ViewInstanceId,
    plugin: Option<&PluginPair>,
    plugin_error: Option<&PluginUnavailable>,
    lifecycle: ChildLifecycle<'_>,
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
        AttachDrive {
            drive_open: lifecycle.drive_open,
            observe_open: lifecycle.observe_open,
        },
    );

    let mut outcome = match attach_attempt_failure(&attempt) {
        Some(outcome) => outcome,
        None => match verification.as_ref() {
            Some(verification) if verification.supported && verification.verified => {
                // The real `test_editor` IPlugView attached a real
                // child HWND under the leased container and every OS
                // observation agreed — the case's own detail names
                // which lifecycle path ran (graceful close vs host
                // loss vs degrade/recovery/timeout).
                let mut outcome = ScenarioOutcome::new(ScenarioVerdict::Pass);
                outcome.detail = Some(lifecycle.case.pass_detail.to_string());
                outcome.evidence.push(lifecycle.case.evidence.to_string());
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
                lifecycle.case.label,
                verification
                    .failure
                    .as_deref()
                    .unwrap_or("unknown mismatch")
            )),
            None => fail(format!(
                "editor opened but no {} observation was recorded",
                lifecycle.case.label
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
        ChildLifecycle {
            drive_open: drive_open_docked,
            observe_open: observe_editor_child_lifecycle,
            case: &ChildCaseIdentity {
                pass_detail: "test_editor real IPlugView attached a real child \
                              HWND under the leased container HWND and detached \
                              on close (EnumChildWindows/GetParent/IsWindowVisible \
                              observed; IsWindow(child)==0 after removed(); events \
                              [Attached{Docked}, Closed])",
                evidence: "editor-attach:real-child-hwnd-lifecycle",
                label: "child-HWND",
            },
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
    finalize_case(
        report,
        ctx,
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
    finalize_case(report, ctx, EnvironmentKind::RealWindow, None, Map::new())
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

/// `real-dcomp-pixel`'s post-open observation bound to its artifact
/// directory — a named `impl FnOnce` wrapper so the closure's
/// parameter types are inferred against the [`AttachDrive`] contract
/// (a bare closure literal inside the struct cannot infer them).
fn pixel_observe(
    artifacts: Option<&Path>,
) -> impl FnOnce(
    &RealOsGuiTest,
    Option<u64>,
    u64,
    &mut PluginEditorSessions,
    PluginEditorSessionId,
    &mut InProcessHost,
    PluginHandle,
    &mut Vec<Value>,
) -> PixelVerification
+ '_ {
    move |_gui, container, _lease_raw, sessions, session_id, host, handle, steps| {
        observe_editor_pixel_lifecycle(
            container, sessions, session_id, host, handle, artifacts, steps,
        )
    }
}

/// `real-dcomp-foreground`'s post-open observation bound to its
/// artifact directory — see [`pixel_observe`].
fn foreground_observe(
    artifacts: Option<&Path>,
) -> impl FnOnce(
    &RealOsGuiTest,
    Option<u64>,
    u64,
    &mut PluginEditorSessions,
    PluginEditorSessionId,
    &mut InProcessHost,
    PluginHandle,
    &mut Vec<Value>,
) -> ForegroundVerification
+ '_ {
    move |_gui, container, _lease_raw, sessions, session_id, host, handle, steps| {
        observe_editor_foreground_probe(
            container, sessions, session_id, host, handle, artifacts, steps,
        )
    }
}

/// `real-dcomp-present`'s post-open observation bound to its
/// artifact directory — see [`pixel_observe`].
fn present_observe(
    artifacts: Option<&Path>,
) -> impl FnOnce(
    &RealOsGuiTest,
    Option<u64>,
    u64,
    &mut PluginEditorSessions,
    PluginEditorSessionId,
    &mut InProcessHost,
    PluginHandle,
    &mut Vec<Value>,
) -> PresentationVerification
+ '_ {
    move |_gui, container, _lease_raw, sessions, session_id, host, handle, steps| {
        observe_editor_presentation_probe(
            container, sessions, session_id, host, handle, artifacts, steps,
        )
    }
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
        ATTACH_CLOSE_EVENTS,
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
        AttachDrive {
            drive_open: drive_open_docked,
            observe_open: pixel_observe(artifacts),
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
    finalize_case(
        report,
        ctx,
        EnvironmentKind::RealWindow,
        artifact,
        Map::new(),
    )
}

// ---------------------------------------------------------------------
// case: real-dcomp-foreground (Issue #532 foreground/z-order probe)
// ---------------------------------------------------------------------

/// The aggregated foreground/z-order probe outcome — whether the
/// baseline could be taken, whether every planned mutation stage
/// executed, and whether state was restored. The bisect *result*
/// (did green appear, which HWND answered the hit-test) lives in
/// the step rows as data; this struct carries only the probe's own
/// execution/restore verdict inputs.
#[derive(Debug, Default)]
struct ProbeOutcome {
    /// `false` when the baseline could not be taken (no container,
    /// window not presented, no children, root unresolvable) — the
    /// case then reports `NotRun`, never a fabricated verdict.
    evaluated: bool,
    /// Every planned z-order mutation applied (`SetWindowPos`
    /// TRUE) and the foreground attempt was issued. A refused
    /// `SetForegroundWindow` still counts as executed — the
    /// refusal is recorded data, not a skipped stage.
    executed: bool,
    /// `osmut::restore` verification passed — topmost flag back to
    /// baseline and every `SetWindowPos` restore op applied.
    restored: bool,
    /// First violated probe check (`name: detail`).
    failure: Option<String>,
}

impl ProbeOutcome {
    /// Probe could not be evaluated — records a `not_run` row and
    /// returns the honest outcome. The reason is carried in
    /// `failure` so the case-level `NotRun` detail names the cause,
    /// not a generic fallback.
    fn not_evaluable(steps: &mut Vec<Value>, reason: &str) -> Self {
        steps.push(json!({
            "step": "fg_probe",
            "verdict": "not_run",
            "reason": reason,
        }));
        Self {
            failure: Some(reason.to_string()),
            ..Self::default()
        }
    }

    /// Off-Windows / unsupported platform — records an
    /// `unsupported` row; unreachable today (`NativePluginDock` is
    /// Windows-only) but kept honest.
    fn unsupported(steps: &mut Vec<Value>) -> Self {
        steps.push(json!({
            "step": "fg_probe",
            "verdict": "unsupported",
            "detail": "OS window-state mutation is Windows-only",
        }));
        Self::default()
    }
}

/// Verification for `real-dcomp-foreground` — same lifecycle
/// contract as [`PixelVerification`] plus the probe outcome. The
/// case verdict reads `lifecycle_ok` + `probe.{evaluated,
/// executed,restored}` only; screen green and hit-test answers are
/// bisect *data*, never verdict inputs.
struct ForegroundVerification {
    /// `false` off-Windows — the observation half then reports
    /// `Unsupported`, never a fabricated pass.
    supported: bool,
    /// Every lifecycle contract check agreed (attach contract,
    /// close, detach, session convergence) — the same checks as
    /// `real-editor-attach`.
    lifecycle_ok: bool,
    /// First violated lifecycle check (`name: detail`).
    lifecycle_failure: Option<String>,
    /// The aggregated probe outcome.
    probe: ProbeOutcome,
    /// `observed` extras — container/child ids, threads, events,
    /// probe summary (an `Object`, like `PixelVerification`).
    extras: Value,
}

/// Plain-JSON form of one [`osmut::RootSnapshot`] — every field is
/// already a plain value; `exstyle` is flattened to its
/// compositor-relevant bits.
fn snapshot_json(snapshot: &osmut::RootSnapshot) -> Value {
    json!({
        "supported": snapshot.supported,
        "root": snapshot.root,
        "exstyle": snapshot.exstyle.as_ref().map(|e| json!({
            "value": e.value,
            "noredirectionbitmap": e.noredirectionbitmap(),
            "topmost": e.topmost(),
            "layered": e.layered(),
            "transparent": e.transparent(),
            "noactivate": e.noactivate(),
            "toolwindow": e.toolwindow(),
            "appwindow": e.appwindow(),
        })),
        "topmost": snapshot.topmost,
        "above_count": snapshot.above_count,
        "above_hwnd": snapshot.above_hwnd,
        "below_hwnd": snapshot.below_hwnd,
        "foreground": snapshot.foreground,
        "truncated": snapshot.truncated,
    })
}

/// Plain-JSON form of one [`osmut::MutationOutcome`].
fn mutation_json(outcome: &osmut::MutationOutcome) -> Value {
    json!({
        "api": outcome.api,
        "applied": outcome.applied,
        "win32_error": outcome.win32_error,
        "after": snapshot_json(&outcome.after),
    })
}

/// Plain-JSON form of one [`osmut::RestoreOutcome`] — every restore
/// op plus the verification fields (`topmost_restored` is the only
/// deterministic check; z-order/foreground are recorded observed
/// values, never claimed restored).
fn restore_json(outcome: &osmut::RestoreOutcome) -> Value {
    json!({
        "ops": outcome.ops.iter().map(mutation_json).collect::<Vec<_>>(),
        "topmost_restored": outcome.topmost_restored,
        "zorder_before": {
            "above_count": outcome.zorder_before.0,
            "above_hwnd": outcome.zorder_before.1,
        },
        "zorder_after": {
            "above_count": outcome.zorder_after.0,
            "above_hwnd": outcome.zorder_after.1,
        },
        "foreground_before": outcome.foreground_before,
        "foreground_attempted": outcome.foreground_attempted,
        "foreground_final": outcome.foreground_final,
        "final_state": snapshot_json(&outcome.final_state),
    })
}

/// One per-child re-observation at one probe stage — `WindowFromPoint`
/// hit-test roles plus the screen-composited green fraction. Both are
/// `note`-level bisect *data*: they never feed the case verdict. The
/// stage name distinguishes baseline/raise/foreground/topmost/restore.
fn record_foreground_observation(
    stage: &str,
    index: usize,
    hwnd: u64,
    container: u64,
    artifacts: Option<&Path>,
    steps: &mut Vec<Value>,
) {
    let facts = osoccl::probe(hwnd);
    let root = facts.root;
    let hits: Vec<Value> = facts
        .hits
        .iter()
        .map(|hit| match &hit.window {
            Some(row) => json!({
                "x": hit.x,
                "y": hit.y,
                "hwnd": row.hwnd,
                "class_name": row.class_name,
                "role": occlusion_role(row.hwnd, hwnd, container, root),
                "visible": row.visible,
                "alive": row.alive,
            }),
            None => json!({
                "x": hit.x,
                "y": hit.y,
                "hwnd": Value::Null,
                "role": "none",
            }),
        })
        .collect();

    // Screen-composited green fraction — the same oracle
    // `real-dcomp-pixel` reads, re-sampled after each mutation. A
    // capture failure degrades to an `error` field, never a verdict.
    let screen = ospix::window_screen_rect(hwnd).and_then(ospix::capture_screen_rect);
    let mut screen_json = match &screen {
        Ok(capture) => {
            let (green, total) = green_stats(&capture.rgba);
            let fraction = green_fraction(green, total);
            let mut row = json!({
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
                &format!("real-dcomp-foreground-{stage}-{index}"),
                capture,
            );
            row
        }
        Err(error) => json!({ "error": error }),
    };
    if let Some(obj) = screen_json.as_object_mut() {
        obj.insert("stage".to_string(), json!(stage));
    }

    steps.push(json!({
        "step": format!("fg_observe_{stage}"),
        "verdict": "note",
        "stage": stage,
        "child_index": index,
        "hwnd": hwnd,
        "root": root,
        "hits": hits,
        "screen": screen_json,
    }));
}

/// The mutation probe itself — runs while the editor is still
/// attached (lease live, child HWNDs present). Baseline snapshot +
/// per-child observation, then the three mutation stages (raise →
/// foreground → topmost) each followed by a per-child re-observation,
/// then `osmut::restore` + a final per-child observation. The root
/// is shared across children, so mutations apply once per stage and
/// every child is re-observed at each stage.
fn foreground_probe_children(
    observation: &ChildWindowObservation,
    container: Option<u64>,
    artifacts: Option<&Path>,
    steps: &mut Vec<Value>,
) -> ProbeOutcome {
    let Some(token) = container else {
        return ProbeOutcome::not_evaluable(
            steps,
            "no container token — the GPUI root cannot be resolved",
        );
    };
    match ospix::presented(token) {
        Some(true) => {}
        Some(false) => {
            return ProbeOutcome::not_evaluable(
                steps,
                "top-level window is minimized or hidden — no screen pixels",
            );
        }
        None => {
            return ProbeOutcome::not_evaluable(steps, "top-level presentation query failed");
        }
    }
    if observation.children.is_empty() {
        return ProbeOutcome::not_evaluable(steps, "no observed child HWNDs to probe");
    }

    // Baseline — resolve the shared GPUI root through the first
    // child's ancestry and snapshot its mutable state before any
    // mutation. `osmut::observe` returns `supported:false` only on
    // the non-Windows stub (unreachable — `observation.supported`
    // already gated this path).
    let first = &observation.children[0];
    let baseline = osmut::observe(first.hwnd);
    let Some(root) = baseline.root else {
        steps.push(json!({
            "step": "fg_baseline_state",
            "verdict": "note",
            "snapshot": snapshot_json(&baseline),
        }));
        return ProbeOutcome::not_evaluable(steps, "GA_ROOT returned null for the child HWND");
    };
    steps.push(json!({
        "step": "fg_baseline_state",
        "verdict": "note",
        "root": root,
        "snapshot": snapshot_json(&baseline),
    }));

    observe_all_children("baseline", observation, token, artifacts, steps);

    let (mut executed, mut failure) = run_probe_stages(root, observation, token, artifacts, steps);

    // Restore — always attempted regardless of stage outcomes. The
    // verification is `topmost_restored` plus every `SetWindowPos`
    // restore op applying; the `SetForegroundWindow` hand-back is
    // note-level (foreign foreground is not forcibly restorable).
    let restore = osmut::restore(&baseline);
    let mut restore_ops_ok = true;
    for op in &restore.ops {
        if op.api.starts_with("SetWindowPos") && !op.applied {
            restore_ops_ok = false;
        }
    }
    let restored = restore.topmost_restored && restore_ops_ok;
    if !restored {
        executed = false;
        failure.get_or_insert_with(|| {
            format!(
                "restore verification failed (topmost_restored={}, \
                 setwindowpos_ops_ok={restore_ops_ok})",
                restore.topmost_restored
            )
        });
    }
    steps.push(json!({
        "step": "fg_restore",
        "verdict": if restored { "pass" } else { "fail" },
        "restore": restore_json(&restore),
        "note": "topmost flag restoration is verified; z-order and \
                 foreground are best-effort — zorder_before/after and \
                 foreground_before/final record what could and could \
                 not be restored, never claimed",
    }));
    observe_all_children("restore", observation, token, artifacts, steps);

    ProbeOutcome {
        evaluated: true,
        executed,
        restored,
        failure,
    }
}

/// Re-observe every child at one probe stage — one
/// [`record_foreground_observation`] row per child.
fn observe_all_children(
    stage: &str,
    observation: &ChildWindowObservation,
    token: u64,
    artifacts: Option<&Path>,
    steps: &mut Vec<Value>,
) {
    for (index, child) in observation.children.iter().enumerate() {
        record_foreground_observation(stage, index, child.hwnd, token, artifacts, steps);
    }
}

/// One mutation apply step row — `pass` when the API applied,
/// `fail` for a refused `SetWindowPos` (strict stage), `note` for a
/// refused `SetForegroundWindow` (foreground-lock refusal is
/// spec-expected data, never a failure).
fn record_apply_row(
    steps: &mut Vec<Value>,
    step: &str,
    outcome: &osmut::MutationOutcome,
    strict: bool,
) {
    let verdict = if outcome.applied {
        "pass"
    } else if strict {
        "fail"
    } else {
        "note"
    };
    let mut row = json!({
        "step": step,
        "verdict": verdict,
        "mutation": mutation_json(outcome),
    });
    if !strict {
        row["note"] = json!(
            "SetForegroundWindow may be refused by the foreground lock — \
             a refusal is recorded data (the foreground branch stays \
             unresolved), never disguised as success"
        );
    }
    steps.push(row);
}

/// Apply the three mutation stages in order — (a) raise to top of
/// the non-topmost band, (b) `SetForegroundWindow`, (c) topmost
/// band — recording each apply row and re-observing every child.
/// Returns `(executed, first_failure)`: `executed` clears when a
/// `SetWindowPos` stage is refused (anomalous for our own window);
/// a refused `SetForegroundWindow` is recorded data and does not
/// clear it.
fn run_probe_stages(
    root: u64,
    observation: &ChildWindowObservation,
    token: u64,
    artifacts: Option<&Path>,
    steps: &mut Vec<Value>,
) -> (bool, Option<String>) {
    let mut executed = true;
    let mut failure: Option<String> = None;

    // (a) raise — strict `SetWindowPos` (refusal on our own window
    // is anomalous → `fail` step + probe failure).
    let raise = osmut::raise_to_top(root);
    record_apply_row(steps, "fg_apply_raise", &raise, true);
    if !raise.applied {
        executed = false;
        failure.get_or_insert_with(|| {
            format!(
                "raise_to_top refused (win32 error {})",
                raise
                    .win32_error
                    .map_or_else(|| "unknown".to_string(), |e| e.to_string())
            )
        });
    }
    observe_all_children("raise", observation, token, artifacts, steps);

    // (b) foreground — non-strict (foreground-lock refusal is data).
    let foreground = osmut::set_foreground(root);
    record_apply_row(steps, "fg_apply_foreground", &foreground, false);
    observe_all_children("foreground", observation, token, artifacts, steps);

    // (c) topmost — strict `SetWindowPos`.
    let topmost = osmut::set_topmost(root);
    record_apply_row(steps, "fg_apply_topmost", &topmost, true);
    if !topmost.applied {
        executed = false;
        failure.get_or_insert_with(|| {
            format!(
                "set_topmost refused (win32 error {})",
                topmost
                    .win32_error
                    .map_or_else(|| "unknown".to_string(), |e| e.to_string())
            )
        });
    }
    observe_all_children("topmost", observation, token, artifacts, steps);

    (executed, failure)
}

/// `observed` extras for the foreground case — same child/container
/// fields as [`child_extras`] plus the probe summary. The full
/// per-stage data lives in `steps`; this carries only the verdict-
/// relevant aggregate.
fn foreground_extras(
    container: Option<u64>,
    observation: &ChildWindowObservation,
    events: &[String],
    probe: &ProbeOutcome,
) -> Map<String, Value> {
    let mut extras = child_extras(container, observation, events);
    extras.insert(
        "foreground_probe".to_string(),
        json!({
            "evaluated": probe.evaluated,
            "executed": probe.executed,
            "restored": probe.restored,
            "failure": probe.failure,
        }),
    );
    extras
}

/// Post-open foreground-probe observation + close convergence —
/// the `real-dcomp-foreground` counterpart of
/// [`observe_editor_pixel_lifecycle`]. Same attach contract checks,
/// then — while still attached — the z-order/foreground mutation
/// probe ([`foreground_probe_children`]), then the shared close →
/// detach → convergence tail ([`conclude_editor_session`]).
fn observe_editor_foreground_probe(
    container: Option<u64>,
    sessions: &mut PluginEditorSessions,
    session_id: PluginEditorSessionId,
    host: &mut InProcessHost,
    handle: PluginHandle,
    artifacts: Option<&Path>,
    steps: &mut Vec<Value>,
) -> ForegroundVerification {
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

    // Phase 1b — the foreground/z-order mutation probe while still
    // attached (the Issue #532 bisect). Off-Windows there is no
    // window-state mutation either — recorded as unsupported.
    let probe = if observation.supported {
        foreground_probe_children(&observation, container, artifacts, steps)
    } else {
        ProbeOutcome::unsupported(steps)
    };

    let (close_ok, detach_ok, converged_ok, rendered) = conclude_editor_session(
        &mut observation,
        sessions,
        session_id,
        host,
        handle,
        steps,
        &mut failure,
        ATTACH_CLOSE_EVENTS,
    );

    ForegroundVerification {
        supported: observation.supported,
        lifecycle_ok: attach_ok && close_ok && detach_ok && converged_ok,
        lifecycle_failure: failure,
        extras: Value::Object(foreground_extras(
            container,
            &observation,
            &rendered,
            &probe,
        )),
        probe,
    }
}

/// `real-dcomp-foreground`: the same real attach path as
/// `real-dcomp-pixel`, plus the foreground/z-order mutation probe —
/// does displacing external HWND stacking (and then the topmost
/// band) let the docked child's pixels reach the screen? The case
/// verdict is the *probe's own* honest oracle (executed + restored +
/// lifecycle); whether green appeared is bisect data recorded in the
/// step rows, never a verdict input.
fn editor_foreground_body(
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
        AttachDrive {
            drive_open: drive_open_docked,
            observe_open: foreground_observe(artifacts),
        },
    );

    let mut outcome = match attach_attempt_failure(&attempt) {
        Some(outcome) => outcome,
        None => match verification.as_ref() {
            Some(verification) if !verification.supported => {
                let mut outcome = ScenarioOutcome::new(ScenarioVerdict::Unsupported);
                outcome.detail = Some(
                    "editor opened through the REAL container lease but OS \
                     window-state observation is unsupported on this platform"
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
            Some(verification) if !verification.probe.evaluated => not_run(format!(
                "editor lifecycle verified but the foreground probe is not \
                 evaluable: {}",
                verification
                    .probe
                    .failure
                    .as_deref()
                    .unwrap_or("baseline could not be taken")
            )),
            Some(verification) if !verification.probe.executed || !verification.probe.restored => {
                fail(format!(
                    "editor lifecycle verified but the foreground probe did \
                     not complete: {}",
                    verification
                        .probe
                        .failure
                        .as_deref()
                        .unwrap_or("probe execution or restore verification failed")
                ))
            }
            Some(_) => {
                let mut outcome = ScenarioOutcome::new(ScenarioVerdict::Pass);
                outcome.detail = Some(
                    "foreground/z-order probe executed all mutation \
                     stages; topmost band verified restored (z-order \
                     and foreground restoration are best-effort — \
                     see steps). Bisect data (hit-test answers, screen \
                     green per stage) is recorded in steps"
                        .to_string(),
                );
                outcome
                    .evidence
                    .push("dcomp-foreground:probe-executed-restored".to_string());
                outcome
            }
            None => fail("editor opened but no foreground observation was recorded".to_string()),
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

/// `real-dcomp-foreground` case: pre-flight requires `NativePluginDock`
/// and `VisualCapture` (same capabilities as `real-dcomp-pixel` — the
/// probe re-uses the screen-pixel oracle per stage), then the body
/// drives the attach chain plus the mutation probe — or reports the
/// honest artifact/session/environment outcome.
fn real_dcomp_foreground_case(
    ctx: &mut RealDockContext<'_>,
    spec: &ScenarioSpec,
) -> ScenarioReport {
    let window = ctx.window;
    let view = ctx.view;
    let plugin = ctx.plugin;
    let plugin_error = ctx.plugin_error;
    let artifact = ctx.artifact;
    let pixel_artifacts = ctx.pixel_artifacts;
    let report = run_scenario(&mut *ctx.gui, spec, |gui| {
        Ok(editor_foreground_body(
            gui,
            window,
            view,
            plugin,
            plugin_error,
            pixel_artifacts,
        ))
    });
    finalize_case(
        report,
        ctx,
        EnvironmentKind::RealWindow,
        artifact,
        Map::new(),
    )
}

// ---------------------------------------------------------------------
// case: real-dcomp-present (Issue #532 presentation probe)
// ---------------------------------------------------------------------

/// `PrintWindow` `nflags` variants exercised per target — `0`
/// (whole window), `PW_CLIENTONLY`, `PW_RENDERFULLCONTENT`, and
/// both. The ABI values match `ospix`'s local constants.
const PRESENT_FLAGS: [u32; 4] = [0x0, 0x1, 0x2, 0x3];
/// Cap on cover candidates recorded per child — the inventory
/// itself is already capped at `ospres::INVENTORY_CAP`; this bounds
/// the per-child attribute queries and JSON size.
const COVER_CAP: usize = 16;
/// Cap on fingerprint suspects per child — hit-test HWNDs plus the
/// topmost cover candidates.
const FINGERPRINT_CAP: usize = 8;

/// The aggregated presentation-probe outcome — whether the
/// baseline could be taken and whether every planned observation
/// family ran. The probe *result* (which mechanism the data
/// supports) lives in the step rows as data; this struct carries
/// only the probe's own execution verdict inputs.
#[derive(Debug, Default)]
struct PresentationProbeOutcome {
    /// `false` when the baseline could not be taken (no container,
    /// window not presented, no children) — the case then reports
    /// `NotRun`, never a fabricated verdict.
    evaluated: bool,
    /// Every planned observation family ran its platform call. A
    /// family reporting `supported:false` mid-probe (defensive —
    /// unreachable on Windows) clears this → `Fail`.
    completed: bool,
    /// The planned observation families, for the `observed` summary.
    families: Vec<String>,
    /// First violated probe check (`name: detail`).
    failure: Option<String>,
}

impl PresentationProbeOutcome {
    /// Probe could not be evaluated — records a `not_run` row and
    /// returns the honest outcome with the reason carried through.
    fn not_evaluable(steps: &mut Vec<Value>, reason: &str) -> Self {
        steps.push(json!({
            "step": "pres_probe",
            "verdict": "not_run",
            "reason": reason,
        }));
        Self {
            failure: Some(reason.to_string()),
            ..Self::default()
        }
    }

    /// Off-Windows / unsupported platform — records an
    /// `unsupported` row; unreachable today (`NativePluginDock` is
    /// Windows-only) but kept honest.
    fn unsupported(steps: &mut Vec<Value>) -> Self {
        steps.push(json!({
            "step": "pres_probe",
            "verdict": "unsupported",
            "detail": "OS presentation observation is Windows-only",
        }));
        Self::default()
    }
}

/// Verification for `real-dcomp-present` — same lifecycle contract
/// as [`ForegroundVerification`] plus the probe outcome. The case
/// verdict reads `lifecycle_ok` + `probe.{evaluated,completed}`
/// only; cover/fingerprint/capture observations are bisect *data*,
/// never verdict inputs.
struct PresentationVerification {
    /// `false` off-Windows — the observation half then reports
    /// `Unsupported`, never a fabricated pass.
    supported: bool,
    /// Every lifecycle contract check agreed (attach contract,
    /// close, detach, session convergence) — the same checks as
    /// `real-editor-attach`.
    lifecycle_ok: bool,
    /// First violated lifecycle check (`name: detail`).
    lifecycle_failure: Option<String>,
    /// The aggregated probe outcome.
    probe: PresentationProbeOutcome,
    /// `observed` extras — container/child ids, threads, events,
    /// probe summary (an `Object`, like `ForegroundVerification`).
    extras: Value,
}

/// Plain-JSON form of one [`ospres::TopLevelWindow`] — `own_process`
/// is derived from `pid == own_pid` so own-process (GPUI/plugin)
/// covers never masquerade as foreign ones.
fn top_level_window_json(window: &ospres::TopLevelWindow, own_pid: u32) -> Value {
    json!({
        "hwnd": window.hwnd,
        "z_index": window.z_index,
        "class_name": window.class_name,
        "rect": occlusion_rect_json(window.rect),
        "visible": window.visible,
        "alive": window.alive,
        "disabled": window.disabled,
        "exstyle": window.exstyle.as_ref().map(|e| json!({
            "value": e.value,
            "noredirectionbitmap": e.noredirectionbitmap(),
            "topmost": e.topmost(),
            "layered": e.layered(),
            "transparent": e.transparent(),
            "noactivate": e.noactivate(),
            "toolwindow": e.toolwindow(),
            "appwindow": e.appwindow(),
        })),
        "owner": window.owner,
        "pid": window.pid,
        "owner_pid": window.owner_pid,
        "own_process": window.pid == Some(own_pid),
    })
}

/// Plain-JSON form of one [`ospres::PresentationAttributes`] —
/// every field is already a plain value or `None`.
fn attributes_json(attributes: &ospres::PresentationAttributes) -> Value {
    json!({
        "supported": attributes.supported,
        "cloaked": attributes.cloaked,
        "region": attributes.region.as_ref().map(|r| json!({
            "region_type": r.region_type,
            // GetWindowRgn/GetRgnBox coordinates — relative to the
            // window's upper-left corner, NOT screen space.
            "bounds_space": "window",
            "bounds": occlusion_rect_json(r.bounds),
            "error": r.error,
        })),
        "update": attributes.update.as_ref().map(|u| json!({
            "has_update": u.has_update,
            // GetUpdateRect coordinates — client-area space, NOT
            // screen space.
            "rect_space": "client",
            "rect": occlusion_rect_json(u.rect),
            "error": u.error,
        })),
        "layered": attributes.layered.as_ref().map(|l| json!({
            "alpha": l.alpha,
            "color_key": l.color_key,
            "flags": l.flags,
        })),
    })
}

/// `true` when two screen-space rectangles overlap by at least one
/// pixel — the cover-candidate predicate.
fn rects_intersect(a: ospix::ScreenRect, b: ospix::ScreenRect) -> bool {
    rect_intersection(a, b).is_some()
}

/// The overlapping region of two screen-space rectangles — `None`
/// when they do not intersect or the result is degenerate. `i64`
/// intermediates keep the edge arithmetic overflow-free.
fn rect_intersection(a: ospix::ScreenRect, b: ospix::ScreenRect) -> Option<ospix::ScreenRect> {
    let left = a.left.max(b.left);
    let top = a.top.max(b.top);
    let right =
        (i64::from(a.left) + i64::from(a.width)).min(i64::from(b.left) + i64::from(b.width));
    let bottom =
        (i64::from(a.top) + i64::from(a.height)).min(i64::from(b.top) + i64::from(b.height));
    let width = u32::try_from(right - i64::from(left)).ok()?;
    let height = u32::try_from(bottom - i64::from(top)).ok()?;
    (width > 0 && height > 0).then_some(ospix::ScreenRect {
        left,
        top,
        width,
        height,
    })
}

/// Whether `inner` is fully contained in `outer` (screen coords) —
/// `i64` intermediates keep the edge arithmetic overflow-free.
fn rect_contains(outer: ospix::ScreenRect, inner: ospix::ScreenRect) -> bool {
    i64::from(inner.left) >= i64::from(outer.left)
        && i64::from(inner.top) >= i64::from(outer.top)
        && i64::from(inner.left) + i64::from(inner.width)
            <= i64::from(outer.left) + i64::from(outer.width)
        && i64::from(inner.top) + i64::from(inner.height)
            <= i64::from(outer.top) + i64::from(outer.height)
}

/// The identity label a fingerprint suspect carries — chain windows
/// keep their chain role regardless of which list (hit-test or
/// cover inventory) surfaced them; `foreign` labels everything
/// else (`hit_test` for `WindowFromPoint` hits, `cover_candidate`
/// for inventory-only windows).
fn suspect_role(hwnd: u64, facts: &osoccl::OcclusionFacts, foreign: &'static str) -> &'static str {
    if Some(hwnd) == facts.root {
        "root"
    } else if Some(hwnd) == facts.parent {
        "container"
    } else {
        foreign
    }
}

/// Quantized RGB color bin — 5 bits per channel (`>>3`), the
/// fingerprint comparison unit. Solid surfaces survive
/// compositing/capture-format differences inside one bin.
fn color_bin(r: u8, g: u8, b: u8) -> u16 {
    (u16::from(r >> 3) << 10) | (u16::from(g >> 3) << 5) | u16::from(b >> 3)
}

/// The bin of the RGBA8 pixel at `index` (pixel index, not byte).
fn bin_at(rgba: &[u8], index: usize) -> u16 {
    let base = index * 4;
    color_bin(rgba[base], rgba[base + 1], rgba[base + 2])
}

/// `(match_fraction, overlap_pixels)` between the screen capture
/// (covering `screen_rect`) and a suspect raster (covering
/// `raster_rect`): the fraction of overlap pixels whose quantized
/// color bin matches. `None` when either capture's dimensions do
/// not match its declared rect (a misaligned comparison would
/// fabricate a match rate — skipped honestly instead).
fn fingerprint_match(
    screen: &ospix::PixelCapture,
    screen_rect: ospix::ScreenRect,
    raster: &ospix::PixelCapture,
    raster_rect: ospix::ScreenRect,
) -> Option<(f64, u64)> {
    if screen.width != screen_rect.width || screen.height != screen_rect.height {
        return None;
    }
    if raster.width != raster_rect.width || raster.height != raster_rect.height {
        return None;
    }
    let overlap = rect_intersection(screen_rect, raster_rect)?;
    let sw = usize::try_from(screen.width).ok()?;
    let rw = usize::try_from(raster.width).ok()?;
    let mut matched = 0u64;
    let mut total = 0u64;
    let y_end = overlap.top + i32::try_from(overlap.height).ok()?;
    let x_end = overlap.left + i32::try_from(overlap.width).ok()?;
    for y in overlap.top..y_end {
        for x in overlap.left..x_end {
            let si = usize::try_from(y - screen_rect.top).ok()? * sw
                + usize::try_from(x - screen_rect.left).ok()?;
            let ri = usize::try_from(y - raster_rect.top).ok()? * rw
                + usize::try_from(x - raster_rect.left).ok()?;
            total += 1;
            if bin_at(&screen.rgba, si) == bin_at(&raster.rgba, ri) {
                matched += 1;
            }
        }
    }
    // u32 saturation before the exact u32→f64 conversion — same
    // pattern as `green_fraction`; real captures are far below
    // the bound.
    (total > 0).then(|| {
        let m = u32::try_from(matched).unwrap_or(u32::MAX);
        let t = u32::try_from(total).unwrap_or(u32::MAX).max(1);
        (f64::from(m) / f64::from(t), total)
    })
}

/// The top `n` quantized color bins of a capture by pixel count —
/// the human-readable "what is actually on screen" summary.
fn top_colors(rgba: &[u8], n: usize) -> Vec<Value> {
    let mut hist = vec![0u32; 1 << 15];
    for px in rgba.as_chunks::<4>().0 {
        hist[usize::from(color_bin(px[0], px[1], px[2]))] += 1;
    }
    let mut bins: Vec<(u16, u32)> = hist
        .iter()
        .enumerate()
        .filter(|(_, count)| **count > 0)
        .map(|(bin, count)| (u16::try_from(bin).unwrap_or(u16::MAX), *count))
        .collect();
    bins.sort_by_key(|bin| std::cmp::Reverse(bin.1));
    bins.into_iter()
        .take(n)
        .map(|(bin, count)| {
            json!({
                "r": ((bin >> 10) & 0x1f) << 3,
                "g": ((bin >> 5) & 0x1f) << 3,
                "b": (bin & 0x1f) << 3,
                "count": count,
            })
        })
        .collect()
}

/// One `PrintWindow`/`windowdc` capture step row for the
/// presentation probe — `note` level only, green stats + opt-in
/// artifact, never a verdict input. Takes the result by reference
/// so the caller keeps the `flags=0` raster for fingerprinting.
#[expect(
    clippy::too_many_arguments,
    reason = "per-capture identity (index/role/hwnd/path/flags) + result + artifacts"
)]
fn record_present_capture(
    steps: &mut Vec<Value>,
    child_index: usize,
    role: &str,
    hwnd: u64,
    path: &str,
    flags: Option<u32>,
    result: &Result<ospix::PixelCapture, String>,
    artifacts: Option<&Path>,
    artifact_name: &str,
) {
    match result {
        Ok(capture) => {
            let (green, total) = green_stats(&capture.rgba);
            let fraction = green_fraction(green, total);
            let mut row = json!({
                "step": "pres_capture",
                "verdict": "note",
                "child_index": child_index,
                "target_role": role,
                "hwnd": hwnd,
                "path": path,
                "flags": flags,
                "backend": capture.backend,
                "width": capture.width,
                "height": capture.height,
                "green_pixels": green,
                "total_pixels": total,
                "green_fraction": fraction,
            });
            record_pixel_artifact(&mut row, artifacts, artifact_name, capture);
            steps.push(row);
        }
        Err(error) => steps.push(json!({
            "step": "pres_capture",
            "verdict": "note",
            "child_index": child_index,
            "target_role": role,
            "hwnd": hwnd,
            "path": path,
            "flags": flags,
            "error": error,
        })),
    }
}

/// A captured `PrintWindow(flags=0)` raster kept for the
/// fingerprint comparison — window-rect basis by construction.
struct FingerprintRaster {
    /// The target's role label (`root`/`container`/`editor_child`/
    /// `hit_test`/`cover_candidate`).
    role: &'static str,
    /// The HWND the raster belongs to.
    hwnd: u64,
    /// The window rect the raster covers (flags=0 → `GetWindowRect`).
    rect: ospix::ScreenRect,
    /// The captured pixels.
    capture: ospix::PixelCapture,
}

/// Per-child probe result — `Completed` ran every observation
/// family, `Failed` hit an unsupported observation mid-probe (the
/// case reports `Fail`), `NotEvaluable` means the child's baseline
/// could not be established — the top-level root HWND is
/// unresolvable, so the chain's root attributes/captures/fingerprint
/// cannot run (the case reports `NotRun`).
enum ChildProbeResult {
    /// Every planned family ran.
    Completed,
    /// A platform observation reported unsupported mid-probe.
    Failed,
    /// The baseline itself could not be taken for this child.
    NotEvaluable,
}

/// Per-child presentation observation while attached — cover
/// inventory intersection, presentation attributes, `PrintWindow`
/// flag variants, screen capture, and fingerprint matching. Every
/// family records `note`-level step rows; failures degrade to
/// `error`/`skipped` fields, never fabricated observations.
#[expect(clippy::too_many_lines)]
fn probe_one_child_presentation(
    index: usize,
    child: &ChildWindowFacts,
    container: u64,
    inventory: &ospres::Inventory,
    artifacts: Option<&Path>,
    steps: &mut Vec<Value>,
) -> ChildProbeResult {
    let hwnd = child.hwnd;
    let facts = osoccl::probe(hwnd);
    if !facts.supported {
        steps.push(json!({
            "step": "pres_probe",
            "verdict": "fail",
            "child_index": index,
            "error": "osoccl observation unsupported mid-probe",
        }));
        return ChildProbeResult::Failed;
    }
    // The approved baseline requires the top-level root to resolve
    // — without it the chain's root attributes, captures, and
    // fingerprint have no target, so the probe is not evaluable
    // (recorded as `not_run` by the caller, never a partial pass).
    if facts.root.is_none() {
        return ChildProbeResult::NotEvaluable;
    }

    // pres_attributes — the docked chain (child/container/root).
    let mut chain_targets: Vec<(&'static str, u64)> =
        vec![("editor_child", hwnd), ("container", container)];
    if let Some(root) = facts.root {
        chain_targets.push(("root", root));
    }
    let chain: Vec<Value> = chain_targets
        .iter()
        .map(|(role, target)| {
            json!({
                "role": role,
                "hwnd": target,
                "attributes": attributes_json(&ospres::attributes(*target)),
            })
        })
        .collect();
    steps.push(json!({
        "step": "pres_attributes",
        "verdict": "note",
        "child_index": index,
        "hwnd": hwnd,
        "targets": chain,
        "note": "DWMWA_CLOAKED / window region / update region / \
                layered attributes per chain window — each query \
                degrades independently",
    }));

    // pres_cover — top-level windows intersecting the child's
    // screen rect, z-order, with presentation attributes inline.
    let child_rect = facts
        .child_rect
        .or_else(|| ospix::window_screen_rect(hwnd).ok());
    let covers: Vec<Value> = match child_rect {
        Some(rect) => inventory
            .windows
            .iter()
            .filter(|window| {
                window
                    .rect
                    .is_some_and(|candidate| rects_intersect(rect, candidate))
            })
            .take(COVER_CAP)
            .map(|window| {
                let mut entry = top_level_window_json(window, inventory.own_pid);
                if let Some(obj) = entry.as_object_mut() {
                    obj.insert(
                        "covers_child".to_string(),
                        json!(window.rect.is_some_and(|r| rect_contains(r, rect))),
                    );
                    obj.insert(
                        "attributes".to_string(),
                        attributes_json(&ospres::attributes(window.hwnd)),
                    );
                }
                entry
            })
            .collect(),
        None => Vec::new(),
    };
    steps.push(json!({
        "step": "pres_cover",
        "verdict": "note",
        "child_index": index,
        "hwnd": hwnd,
        "child_rect": occlusion_rect_json(child_rect),
        "cover_count": covers.len(),
        "covers": covers,
        "note": "top-level windows intersecting the child rect \
                (z-order, cap {}) — WindowFromPoint skips hidden/\
                disabled/transparent/layered/cloaked windows, so \
                this inventory sees covers the hit-test cannot",
        "cap": COVER_CAP,
    }));

    // pres_capture — PrintWindow flag variants + window DC per
    // chain target. The flags=0 printwindow rasters are kept for
    // the fingerprint comparison (window-rect basis).
    let mut fingerprint_rasters: Vec<FingerprintRaster> = Vec::new();
    for &(role, target) in &chain_targets {
        for flags in PRESENT_FLAGS {
            let result = ospix::capture_print_window_flags(target, flags);
            record_present_capture(
                steps,
                index,
                role,
                target,
                "printwindow",
                Some(flags),
                &result,
                artifacts,
                &format!("real-dcomp-present-{role}-f{flags}-c{index}"),
            );
            if flags == 0x0
                && let Ok(capture) = result
                && let Ok(rect) = ospix::window_screen_rect(target)
            {
                fingerprint_rasters.push(FingerprintRaster {
                    role,
                    hwnd: target,
                    rect,
                    capture,
                });
            }
        }
        let result = ospix::capture_window_dc(target);
        record_present_capture(
            steps,
            index,
            role,
            target,
            "windowdc",
            None,
            &result,
            artifacts,
            &format!("real-dcomp-present-{role}-windowdc-c{index}"),
        );
    }

    // pres_screen — the composited child-rect capture, same oracle
    // as `real-dcomp-pixel`; kept for the fingerprint comparison.
    let screen = child_rect.map_or_else(
        || Err("no child rect".to_string()),
        ospix::capture_screen_rect,
    );
    let screen_capture = match &screen {
        Ok(capture) => {
            let (green, total) = green_stats(&capture.rgba);
            let fraction = green_fraction(green, total);
            let mut row = json!({
                "step": "pres_screen",
                "verdict": "note",
                "child_index": index,
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
                &format!("real-dcomp-present-screen-{index}"),
                capture,
            );
            steps.push(row);
            Some(capture.clone())
        }
        Err(error) => {
            steps.push(json!({
                "step": "pres_screen",
                "verdict": "note",
                "child_index": index,
                "hwnd": hwnd,
                "error": error,
            }));
            None
        }
    };

    // pres_fingerprint — screen capture vs chain rasters and
    // suspect rasters, quantized-bin match fraction over the
    // overlap region.
    record_fingerprint(
        index,
        hwnd,
        child_rect,
        &facts,
        &fingerprint_rasters,
        screen_capture.as_ref(),
        inventory,
        steps,
    );

    ChildProbeResult::Completed
}

/// The `pres_fingerprint` step — compares the screen capture of
/// the child rect against (a) the chain targets' `PrintWindow(0)`
/// rasters and (b) suspect rasters (hit-test HWNDs + top cover
/// candidates). `match_fraction` is the quantized-bin agreement
/// over the screen-rect ∩ raster-rect overlap; `None`/skipped
/// reasons are recorded, never fabricated.
#[expect(clippy::too_many_lines)]
#[expect(
    clippy::too_many_arguments,
    reason = "per-child inputs (index/hwnd/rect/facts/rasters/screen/inventory) + steps"
)]
fn record_fingerprint(
    index: usize,
    hwnd: u64,
    child_rect: Option<ospix::ScreenRect>,
    facts: &osoccl::OcclusionFacts,
    chain_rasters: &[FingerprintRaster],
    screen: Option<&ospix::PixelCapture>,
    inventory: &ospres::Inventory,
    steps: &mut Vec<Value>,
) {
    let (Some(screen_rect), Some(screen_capture)) = (child_rect, screen) else {
        steps.push(json!({
            "step": "pres_fingerprint",
            "verdict": "note",
            "child_index": index,
            "hwnd": hwnd,
            "skipped": "no screen capture or child rect — fingerprint not evaluable",
        }));
        return;
    };

    // Chain matches — root/container/editor_child flags=0 rasters.
    let chain_matches: Vec<Value> = chain_rasters
        .iter()
        .map(|raster| {
            match fingerprint_match(screen_capture, screen_rect, &raster.capture, raster.rect) {
                Some((fraction, pixels)) => json!({
                    "target": raster.role,
                    "hwnd": raster.hwnd,
                    "match_fraction": fraction,
                    "overlap_pixels": pixels,
                }),
                None => json!({
                    "target": raster.role,
                    "hwnd": raster.hwnd,
                    "skipped": "size_mismatch_or_no_overlap",
                }),
            }
        })
        .collect();

    // Suspects — hit-test HWNDs (any level, dedup) plus the top
    // cover candidates, capped at FINGERPRINT_CAP. A suspect that
    // IS a chain window keeps its chain role (`suspect_role`) —
    // the same HWND never carries two different labels.
    let mut suspects: Vec<(u64, &'static str)> = Vec::new();
    let mut seen: Vec<u64> = vec![hwnd];
    for hit in &facts.hits {
        if let Some(window) = &hit.window
            && !seen.contains(&window.hwnd)
        {
            seen.push(window.hwnd);
            suspects.push((window.hwnd, suspect_role(window.hwnd, facts, "hit_test")));
        }
    }
    let cover_candidates: Vec<u64> = inventory
        .windows
        .iter()
        .filter(|window| {
            window.alive
                && window.visible
                && window
                    .rect
                    .is_some_and(|rect| rects_intersect(screen_rect, rect))
                && !seen.contains(&window.hwnd)
        })
        .take(FINGERPRINT_CAP.saturating_sub(suspects.len()))
        .map(|window| window.hwnd)
        .collect();
    for candidate in cover_candidates {
        seen.push(candidate);
        suspects.push((candidate, suspect_role(candidate, facts, "cover_candidate")));
    }

    let suspect_matches: Vec<Value> = suspects
        .iter()
        .map(|(suspect, role)| {
            let rect = match ospix::window_screen_rect(*suspect) {
                Ok(rect) => rect,
                Err(error) => {
                    return json!({
                        "hwnd": suspect,
                        "role": role,
                        "error": error,
                    });
                }
            };
            // `PrintWindow` sends `WM_PRINT` via `SendMessage` — it
            // blocks until the target thread pumps messages, so a
            // hung foreign window would stall the lane forever.
            // `IsHungAppWindow` narrows (not eliminates) that risk;
            // the skip is recorded as data, not hidden.
            if ospix::window_hung(*suspect) {
                return json!({
                    "hwnd": suspect,
                    "role": role,
                    "skipped": "hung_app_window",
                });
            }
            match ospix::capture_print_window_flags(*suspect, 0x0) {
                Ok(raster) => {
                    let mut entry =
                        match fingerprint_match(screen_capture, screen_rect, &raster, rect) {
                            Some((fraction, pixels)) => json!({
                                "match_fraction": fraction,
                                "overlap_pixels": pixels,
                            }),
                            None => json!({ "skipped": "size_mismatch_or_no_overlap" }),
                        };
                    if let Some(obj) = entry.as_object_mut() {
                        obj.insert("hwnd".to_string(), json!(suspect));
                        obj.insert("role".to_string(), json!(role));
                        obj.insert("raster_backend".to_string(), json!(raster.backend));
                        obj.insert("raster_width".to_string(), json!(raster.width));
                        obj.insert("raster_height".to_string(), json!(raster.height));
                    }
                    entry
                }
                Err(error) => json!({
                    "hwnd": suspect,
                    "role": role,
                    "error": error,
                }),
            }
        })
        .collect();

    steps.push(json!({
        "step": "pres_fingerprint",
        "verdict": "note",
        "child_index": index,
        "hwnd": hwnd,
        "screen": {
            "width": screen_capture.width,
            "height": screen_capture.height,
            "top_colors": top_colors(&screen_capture.rgba, 3),
        },
        "chain_matches": chain_matches,
        "suspect_matches": suspect_matches,
        "note": "quantized-bin (5bit/channel) match fraction over \
                the screen∩raster overlap — which surface the screen \
                actually shows; data only, never a verdict input",
    }));
}

/// The presentation probe itself — runs while the editor is still
/// attached (lease live, child HWNDs present). One global
/// `ospres::inventory` snapshot, then per child: `osoccl::probe`
/// chain facts → presentation attributes → cover candidates →
/// `PrintWindow` flag variants + window DC → screen capture →
/// fingerprint. No mutation — every observation is read-only.
fn presentation_probe_children(
    observation: &ChildWindowObservation,
    container: Option<u64>,
    artifacts: Option<&Path>,
    steps: &mut Vec<Value>,
) -> PresentationProbeOutcome {
    let Some(token) = container else {
        return PresentationProbeOutcome::not_evaluable(
            steps,
            "no container token — the presentation probe cannot locate children",
        );
    };
    match ospix::presented(token) {
        Some(true) => {}
        Some(false) => {
            return PresentationProbeOutcome::not_evaluable(
                steps,
                "top-level window is minimized or hidden — no screen pixels",
            );
        }
        None => {
            return PresentationProbeOutcome::not_evaluable(
                steps,
                "top-level presentation query failed",
            );
        }
    }
    if observation.children.is_empty() {
        return PresentationProbeOutcome::not_evaluable(steps, "no observed child HWNDs to probe");
    }

    // Inventory once — the global top-level snapshot every child's
    // cover analysis shares.
    let inventory = ospres::inventory();
    let inventory_ran = inventory.supported;
    steps.push(json!({
        "step": "pres_inventory",
        "verdict": if inventory_ran { "note" } else { "fail" },
        "supported": inventory.supported,
        "own_pid": inventory.own_pid,
        "total": inventory.total,
        "truncated": inventory.truncated,
        "windows": inventory
            .windows
            .iter()
            .map(|window| top_level_window_json(window, inventory.own_pid))
            .collect::<Vec<_>>(),
        "note": "EnumWindows top-level inventory in z-order — sees \
                covers WindowFromPoint skips (hidden/disabled/\
                transparent/layered/cloaked); same-desktop top-level \
                only, recorded limitation",
    }));

    let mut families = vec!["inventory".to_string()];
    let mut completed = inventory_ran;
    let mut failure =
        (!inventory_ran).then(|| "top-level window inventory unsupported mid-probe".to_string());

    for (index, child) in observation.children.iter().enumerate() {
        match probe_one_child_presentation(index, child, token, &inventory, artifacts, steps) {
            ChildProbeResult::Completed => {}
            ChildProbeResult::Failed => {
                completed = false;
                failure.get_or_insert_with(|| {
                    format!("child[{index}] osoccl observation unsupported mid-probe")
                });
            }
            ChildProbeResult::NotEvaluable => {
                return PresentationProbeOutcome::not_evaluable(
                    steps,
                    &format!(
                        "child[{index}] top-level root HWND unresolvable — \
                         baseline not evaluable"
                    ),
                );
            }
        }
    }
    families.extend([
        "attributes".to_string(),
        "cover".to_string(),
        "capture_variants".to_string(),
        "screen".to_string(),
        "fingerprint".to_string(),
    ]);

    PresentationProbeOutcome {
        evaluated: true,
        completed,
        families,
        failure,
    }
}

/// `observed` extras for the presentation case — same
/// child/container fields as [`child_extras`] plus the probe
/// summary. The full per-family data lives in `steps`; this
/// carries only the verdict-relevant aggregate.
fn presentation_extras(
    container: Option<u64>,
    observation: &ChildWindowObservation,
    events: &[String],
    probe: &PresentationProbeOutcome,
) -> Map<String, Value> {
    let mut extras = child_extras(container, observation, events);
    extras.insert(
        "presentation_probe".to_string(),
        json!({
            "evaluated": probe.evaluated,
            "completed": probe.completed,
            "families": probe.families,
            "failure": probe.failure,
        }),
    );
    extras
}

/// Post-open presentation-probe observation + close convergence —
/// the `real-dcomp-present` counterpart of
/// [`observe_editor_foreground_probe`]. Same attach contract
/// checks, then — while still attached — the read-only
/// presentation probe ([`presentation_probe_children`]), then the
/// shared close → detach → convergence tail
/// ([`conclude_editor_session`]).
fn observe_editor_presentation_probe(
    container: Option<u64>,
    sessions: &mut PluginEditorSessions,
    session_id: PluginEditorSessionId,
    host: &mut InProcessHost,
    handle: PluginHandle,
    artifacts: Option<&Path>,
    steps: &mut Vec<Value>,
) -> PresentationVerification {
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

    // Phase 1b — the presentation probe while still attached (the
    // Issue #532 mechanism discrimination). Off-Windows there is
    // no presentation observation either — recorded as unsupported.
    let probe = if observation.supported {
        presentation_probe_children(&observation, container, artifacts, steps)
    } else {
        PresentationProbeOutcome::unsupported(steps)
    };

    let (close_ok, detach_ok, converged_ok, rendered) = conclude_editor_session(
        &mut observation,
        sessions,
        session_id,
        host,
        handle,
        steps,
        &mut failure,
        ATTACH_CLOSE_EVENTS,
    );

    PresentationVerification {
        supported: observation.supported,
        lifecycle_ok: attach_ok && close_ok && detach_ok && converged_ok,
        lifecycle_failure: failure,
        extras: Value::Object(presentation_extras(
            container,
            &observation,
            &rendered,
            &probe,
        )),
        probe,
    }
}

/// `real-dcomp-present`: the same real attach path as
/// `real-dcomp-foreground`, plus the read-only presentation probe —
/// does the cover inventory / presentation attributes / capture
/// variants / fingerprint data discriminate the remaining
/// hit-test-transparent / child-presentation / capture-blind-spot
/// candidates? The case verdict is the *probe's own* honest oracle
/// (executed + lifecycle); the observations are bisect data
/// recorded in the step rows, never a verdict input.
fn editor_present_body(
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
        AttachDrive {
            drive_open: drive_open_docked,
            observe_open: present_observe(artifacts),
        },
    );

    let mut outcome = match attach_attempt_failure(&attempt) {
        Some(outcome) => outcome,
        None => match verification.as_ref() {
            Some(verification) if !verification.supported => {
                let mut outcome = ScenarioOutcome::new(ScenarioVerdict::Unsupported);
                outcome.detail = Some(
                    "editor opened through the REAL container lease but OS \
                     presentation observation is unsupported on this platform"
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
            Some(verification) if !verification.probe.evaluated => not_run(format!(
                "editor lifecycle verified but the presentation probe is not \
                 evaluable: {}",
                verification
                    .probe
                    .failure
                    .as_deref()
                    .unwrap_or("baseline could not be taken")
            )),
            Some(verification) if !verification.probe.completed => fail(format!(
                "editor lifecycle verified but the presentation probe did \
                 not complete: {}",
                verification
                    .probe
                    .failure
                    .as_deref()
                    .unwrap_or("an observation family failed mid-probe")
            )),
            Some(_) => {
                let mut outcome = ScenarioOutcome::new(ScenarioVerdict::Pass);
                outcome.detail = Some(
                    "presentation probe executed all observation families \
                     (cover inventory, presentation attributes, capture \
                     variants, screen, fingerprint). Bisect data (cover \
                     candidates, attribute states, per-variant green, \
                     match fractions) is recorded in steps"
                        .to_string(),
                );
                outcome
                    .evidence
                    .push("dcomp-present:probe-executed".to_string());
                outcome
            }
            None => fail("editor opened but no presentation observation was recorded".to_string()),
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

/// `real-dcomp-present` case: pre-flight requires `NativePluginDock`
/// and `VisualCapture` (same capabilities as `real-dcomp-pixel` —
/// the probe re-uses the screen-pixel oracle plus OS-level
/// presentation reads), then the body drives the attach chain plus
/// the read-only presentation probe — or reports the honest
/// artifact/session/environment outcome.
fn real_dcomp_present_case(ctx: &mut RealDockContext<'_>, spec: &ScenarioSpec) -> ScenarioReport {
    let window = ctx.window;
    let view = ctx.view;
    let plugin = ctx.plugin;
    let plugin_error = ctx.plugin_error;
    let artifact = ctx.artifact;
    let pixel_artifacts = ctx.pixel_artifacts;
    let report = run_scenario(&mut *ctx.gui, spec, |gui| {
        Ok(editor_present_body(
            gui,
            window,
            view,
            plugin,
            plugin_error,
            pixel_artifacts,
        ))
    });
    finalize_case(
        report,
        ctx,
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
    finalize_case(report, ctx, EnvironmentKind::RealWindow, None, Map::new())
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
    finalize_case(report, ctx, EnvironmentKind::RealWindow, None, Map::new())
}

// ---------------------------------------------------------------------
// case: real-command-receipt (AC1 Command→Receipt link + AC4 receipts)
// ---------------------------------------------------------------------

/// The component key `real-command-receipt` drives on window D — the
/// `sv_fader_dock_fixture` walk key, a command-minting fader.
const RECEIPT_FADER_KEY: &str = "fader-ch1";

/// The receipt-expecting `Commit` minted for `gesture_id`, if the
/// drained intents carry one — the command→receipt correlation key
/// this case exists to prove. `expect_receipt` plus the gesture id
/// pin the match so a stray intent from another window's view can
/// never be mistaken for this command.
fn commit_for_gesture<'a>(
    intents: &'a [MintedIntent],
    gesture_id: &str,
) -> Option<&'a MintedIntent> {
    intents.iter().find(|minted| {
        minted.expect_receipt
            && matches!(
                &minted.intent,
                CommandIntent::Commit { gesture_id: g, .. } if g == gesture_id
            )
    })
}

/// The `control_id` a `Commit` intent carries — the binding id the
/// later `Notify` projection must name to reach the same binding.
fn commit_control_id(minted: &MintedIntent) -> Option<String> {
    match &minted.intent {
        CommandIntent::Commit { control_id, .. } => Some(control_id.clone()),
        _ => None,
    }
}

/// The command→receipt attempt on window D's fader view: two
/// `Begin → Preview → Commit` gestures mint receipt-expecting
/// intents, `result` events carrying the minted `command_id`s are
/// posted back through `post_event` — `applied` for the first
/// command (then an authoritative `Notify` projection), `result_err`
/// for the second — and the binding phases plus the decoded
/// [`CommandReceipt`]s are the verdict inputs. Receipts are decoded
/// with the same [`CommandReceipt::from_result`] `deliver_event`
/// runs, so a recorded receipt is exactly what the view applied —
/// never a hand-built stand-in. Collected receipts escape through
/// `receipts` even on the failure path: whatever the view accepted
/// is honest evidence.
#[expect(clippy::too_many_lines)]
fn command_receipt_body(
    gui: &mut RealOsGuiTest,
    window: Option<WindowId>,
    view: ViewInstanceId,
) -> ScenarioOutcome {
    let mut steps = Vec::new();
    let mut receipts = Vec::new();
    let key = ComponentKey(RECEIPT_FADER_KEY.to_string());
    let attempt = (|| -> Result<Value, String> {
        let window = window.ok_or_else(|| "fourth window id not recorded".to_string())?;
        let target = step(
            &mut steps,
            "resolve_target",
            gui.resolve_target(&TargetQuery::new(window, view).component(key.clone())),
            |_| Value::Null,
        )?;

        // -- command 1: commit → applied receipt → observed projection --
        let (intents, _snapshot) = step(
            &mut steps,
            "gesture_apply",
            parity_gesture_scenario(gui, &target, &key, "rc-apply", ValueInput::Plain(0.7)),
            |(intents, _)| json!({ "intents": intents.len() }),
        )?;
        let commit = commit_for_gesture(&intents, "rc-apply")
            .ok_or_else(|| "gesture_apply: no receipt-expecting Commit minted".to_string())?;
        let command_id = commit.command_id.clone();
        let control_id = commit_control_id(commit)
            .ok_or_else(|| "commit intent carried no control_id".to_string())?;
        steps.push(json!({
            "step": "commit_minted",
            "verdict": "pass",
            "command_id": command_id,
            "control_id": control_id,
        }));

        // The receipt the consumer gateway posts back — decoded by
        // the same `from_result` the view's `deliver_event` runs, so
        // the recorded receipt is exactly what the binding applies.
        let applied_event = Event::result_ok(
            command_id.clone(),
            json!({ "status": "applied", "revision": 1 }),
        );
        let applied_receipt = CommandReceipt::from_result(&applied_event)
            .ok_or_else(|| "applied result did not decode to a receipt".to_string())?;
        if applied_receipt.command_id != command_id {
            return Err("decoded receipt command_id diverged from the minted id".to_string());
        }
        receipts.push(applied_receipt);
        step(
            &mut steps,
            "post_applied_receipt",
            gui.post_event(&target, &applied_event),
            |()| json!({ "command_id": command_id }),
        )?;
        step(&mut steps, "settle_applied", gui.settle(), |()| Value::Null)?;
        let applied_snapshot = step(
            &mut steps,
            "observe_applied",
            gui.observe(&target, &key),
            component_snapshot_json,
        )?;
        if applied_snapshot.phase != CommandPhase::Applied {
            return Err(format!(
                "applied receipt did not settle the binding: phase {:?}",
                applied_snapshot.phase
            ));
        }
        steps.push(json!({ "step": "phase_applied", "verdict": "pass" }));

        // Authoritative projection through the same event path —
        // the receipt-routed binding must also accept state.
        let notify = Event::Notify {
            seq: 1,
            data: json!({
                "control_id": control_id,
                "revision": 2,
                "value": { "normalized": 0.7 },
            }),
        };
        step(
            &mut steps,
            "post_projection",
            gui.post_event(&target, &notify),
            |()| Value::Null,
        )?;
        step(&mut steps, "settle_projection", gui.settle(), |()| {
            Value::Null
        })?;
        let observed_snapshot = step(
            &mut steps,
            "observe_projection",
            gui.observe(&target, &key),
            component_snapshot_json,
        )?;
        if observed_snapshot.phase != CommandPhase::Observed || observed_snapshot.observed.is_none()
        {
            return Err(format!(
                "projection did not reach Observed: phase {:?}",
                observed_snapshot.phase
            ));
        }
        steps.push(json!({ "step": "phase_observed", "verdict": "pass" }));

        // -- command 2: commit → rejected receipt --
        let (intents2, _snapshot2) = step(
            &mut steps,
            "gesture_reject",
            parity_gesture_scenario(gui, &target, &key, "rc-reject", ValueInput::Plain(0.4)),
            |(intents, _)| json!({ "intents": intents.len() }),
        )?;
        let commit2 = commit_for_gesture(&intents2, "rc-reject")
            .ok_or_else(|| "gesture_reject: no receipt-expecting Commit minted".to_string())?;
        let command_id2 = commit2.command_id.clone();
        steps.push(json!({
            "step": "commit_minted_rejected",
            "verdict": "pass",
            "command_id": command_id2,
        }));
        let rejected_event = Event::result_err(
            command_id2.clone(),
            ErrorBody::new("command_rejected", "consumer refused the commit"),
        );
        let rejected_receipt = CommandReceipt::from_result(&rejected_event)
            .ok_or_else(|| "rejected result did not decode to a receipt".to_string())?;
        receipts.push(rejected_receipt);
        step(
            &mut steps,
            "post_rejected_receipt",
            gui.post_event(&target, &rejected_event),
            |()| json!({ "command_id": command_id2 }),
        )?;
        step(&mut steps, "settle_rejected", gui.settle(), |()| {
            Value::Null
        })?;
        let rejected_snapshot = step(
            &mut steps,
            "observe_rejected",
            gui.observe(&target, &key),
            component_snapshot_json,
        )?;
        if rejected_snapshot.phase != CommandPhase::Rejected {
            return Err(format!(
                "rejected receipt did not settle the binding: phase {:?}",
                rejected_snapshot.phase
            ));
        }
        steps.push(json!({ "step": "phase_rejected", "verdict": "pass" }));

        Ok(json!({
            "window": window.to_string(),
            "view": view.to_string(),
            "component": RECEIPT_FADER_KEY,
            "semantic_tree": {
                RECEIPT_FADER_KEY: component_snapshot_json(&rejected_snapshot),
            },
        }))
    })();

    let mut outcome = steps_outcome(
        &steps,
        attempt,
        "real OS window hosted the fader fixture; Begin→Preview→Commit \
         minted receipt-expecting intents whose command_ids were \
         answered through post_event — applied settled Applied→Observed \
         and rejected settled Rejected, with the decoded receipts \
         recorded on the report",
        &["real-window:command-receipt:fader-ch1"],
    );
    outcome.receipts = receipts;
    outcome
}

/// `real-command-receipt`: the AC1 Command→Receipt link plus AC4
/// receipt population on a real OS window. Window D mounts the
/// command-minting `sv_fader_dock_fixture` view; semantic dispatch
/// mints receipt-expecting `Commit` intents, `result` events carrying
/// the minted `command_id`s return through `post_event`, and the
/// binding phases plus decoded [`CommandReceipt`]s prove the link —
/// the first command takes the applied→observed path, the second the
/// rejected path.
fn real_command_receipt_case(ctx: &mut RealDockContext<'_>, spec: &ScenarioSpec) -> ScenarioReport {
    let window = ctx.window_d;
    let view = ctx.view_d;
    let report = run_scenario(&mut *ctx.gui, spec, |gui| {
        Ok(command_receipt_body(gui, window, view))
    });
    finalize_case(report, ctx, EnvironmentKind::RealWindow, None, Map::new())
}

// ---------------------------------------------------------------------
// cases: real-window-dpi / real-window-ime / real-window-close
// (AC1 — dpi/close flipped to real verdicts on the window-surface
// observation API; ime stays Unsupported — `Ime` unclaimed, #389)
// ---------------------------------------------------------------------

/// The DPI/geometry attempt: two `window_geometry` reads must
/// agree, the live physical client rect must be positive with a
/// finite positive scale factor, and the lifecycle log's latest
/// `Resized`/`ScaleFactorChanged` must agree within rounding slack
/// — the window-surface slice's observation contract turned into a
/// case verdict. No DPI-change path is driven: the driver exposes
/// no move/display-scale verb, so the case judges observation
/// consistency only and records that honestly.
fn dpi_body(gui: &mut RealOsGuiTest, window: WindowId) -> ScenarioOutcome {
    let mut steps = Vec::new();
    let attempt = (|| -> Result<Value, String> {
        // Flush pending drains first — the geometry reads and the
        // lifecycle log must describe the same instant.
        step(&mut steps, "settle", gui.settle(), |()| Value::Null)?;
        let first = step(
            &mut steps,
            "window_geometry_1",
            gui.window_geometry(window),
            geometry_json,
        )?;
        let second = step(
            &mut steps,
            "window_geometry_2",
            gui.window_geometry(window),
            geometry_json,
        )?;
        if first != second {
            return Err(format!(
                "window_geometry reads disagree: {first:?} vs {second:?}"
            ));
        }
        if first.width == 0 || first.height == 0 {
            return Err(format!(
                "window_geometry reported empty client rect {}x{}",
                first.width, first.height
            ));
        }
        if !(first.scale_factor.is_finite() && first.scale_factor > 0.0) {
            return Err(format!(
                "window_geometry reported invalid scale_factor {}",
                first.scale_factor
            ));
        }
        steps.push(json!({ "step": "geometry_stable", "verdict": "pass" }));

        let events = gui.window_events(window);
        let resized: Vec<(u32, u32)> = events
            .iter()
            .filter_map(|event| match event {
                WindowLifecycleEvent::Resized { width, height } => Some((*width, *height)),
                _ => None,
            })
            .collect();
        let Some((resized_w, resized_h)) = resized.last().copied() else {
            return Err("no physical Resized lifecycle record to cross-check".to_string());
        };
        // The lifecycle log and the live OS read describe the same
        // physical client area — the same ±2px rounding slack the
        // window-surface lifecycle test proved.
        let dw = i64::from(resized_w) - i64::from(first.width);
        let dh = i64::from(resized_h) - i64::from(first.height);
        if dw.abs() > 2 || dh.abs() > 2 {
            return Err(format!(
                "latest Resized {resized_w}x{resized_h} disagrees with live \
                 geometry {}x{} beyond rounding",
                first.width, first.height
            ));
        }
        let latest_scale = events.iter().rev().find_map(|event| match event {
            WindowLifecycleEvent::ScaleFactorChanged { factor } => Some(*factor),
            _ => None,
        });
        if let Some(factor) = latest_scale
            && (factor - first.scale_factor).abs() > 1e-6
        {
            return Err(format!(
                "latest ScaleFactorChanged {factor} disagrees with live \
                 scale_factor {}",
                first.scale_factor
            ));
        }
        steps.push(json!({ "step": "lifecycle_consistent", "verdict": "pass" }));

        // Window-identity digest — canonical geometry string hashed
        // with the crate's existing sha2 dep (AC4 window hash; no
        // lockfile change).
        let canonical = format!(
            "{window}|{},{},{},{}|{}",
            first.origin_x, first.origin_y, first.width, first.height, first.scale_factor
        );
        let digest = format!("{:x}", Sha256::digest(canonical.as_bytes()));
        Ok(json!({
            "window": window.to_string(),
            "geometry": geometry_json(&first),
            "geometry_stable_reads": 2,
            "resized_transitions": resized.len(),
            "latest_resized": {
                "width": resized_w,
                "height": resized_h,
            },
            "latest_scale_factor_changed": latest_scale,
            "window_geometry_sha256": digest,
            "dpi_change_driven": false,
            "lifecycle": lifecycle_summary(gui, window),
        }))
    })();

    steps_outcome(
        &steps,
        attempt,
        "live OS client geometry observed twice identically (origin, \
         physical px, scale factor) and consistent with the lifecycle \
         Resized/ScaleFactorChanged record within rounding slack — \
         DPI-change paths are not driven (no move/display-scale verb)",
        &["window:dpi:geometry:real-window"],
    )
}

/// `real-window-dpi`: live `window_geometry` (origin, physical
/// pixels, scale factor) observed twice and cross-checked against
/// the lifecycle `Resized`/`ScaleFactorChanged` record — the
/// window-surface slice's observation contract turned into a case
/// verdict. DPI *change* paths (mixed-monitor move) have no driver
/// verb and are honestly recorded as not driven.
fn real_window_dpi_case(ctx: &mut RealDockContext<'_>, spec: &ScenarioSpec) -> ScenarioReport {
    let window = ctx.window;
    let report = run_scenario(&mut *ctx.gui, spec, |gui| Ok(dpi_body(gui, window)));
    finalize_case(report, ctx, EnvironmentKind::RealWindow, None, Map::new())
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
    finalize_case(report, ctx, EnvironmentKind::RealWindow, None, Map::new())
}

/// Bounded wait for `window`'s lifecycle `event` count to reach
/// `expected` — pushes the observation step row and returns the
/// halt detail on timeout or driver error. `>=` (not `==`): the
/// final `close_sequence` assertion is the exact-count check;
/// the wait only proves the transition landed.
fn wait_event_count(
    gui: &mut RealOsGuiTest,
    steps: &mut Vec<Value>,
    window: WindowId,
    event: WindowLifecycleEvent,
    expected: usize,
    name: &str,
) -> Result<(), String> {
    let seen = settle_until(
        gui,
        |g| lifecycle_event_count(g, window, event) >= expected,
        40,
    )
    .map_err(|error| error.to_string())?;
    if seen {
        steps.push(json!({ "step": name, "verdict": "pass" }));
        Ok(())
    } else {
        steps.push(json!({
            "step": name,
            "verdict": "fail",
            "error": format!("lifecycle event count {expected} not observed"),
        }));
        Err(format!(
            "{name}: lifecycle event count {expected} not observed"
        ))
    }
}

/// Bounded wait asserting `window`'s lifecycle `event` count stays
/// at zero — the veto/defer survival proof: a
/// wrongly-approved close lands `Destroyed` within a few drains,
/// so a short budget that sees none is the evidence (same shape as
/// the window-surface lifecycle test).
fn assert_event_absent(
    gui: &mut RealOsGuiTest,
    steps: &mut Vec<Value>,
    window: WindowId,
    event: WindowLifecycleEvent,
    name: &str,
) -> Result<(), String> {
    let appeared = settle_until(gui, |g| lifecycle_event_count(g, window, event) >= 1, 8)
        .map_err(|error| error.to_string())?;
    if appeared || !gui.windows().contains(&window) {
        steps.push(json!({
            "step": name,
            "verdict": "fail",
            "error": "window destroyed despite the armed decision",
        }));
        return Err(format!("{name}: window destroyed despite armed decision"));
    }
    steps.push(json!({ "step": name, "verdict": "pass" }));
    Ok(())
}

/// The armed-Veto leg: `CloseRequested` observed, the window
/// survives — the armed decision is consumed inside the
/// synchronous close-request flush.
fn close_veto_leg(
    gui: &mut RealOsGuiTest,
    steps: &mut Vec<Value>,
    window: WindowId,
) -> Result<(), String> {
    gui.arm_close_response(window, CloseDecision::Veto);
    step(
        steps,
        "request_close_veto",
        gui.request_close(window),
        |()| Value::Null,
    )?;
    wait_event_count(
        gui,
        steps,
        window,
        WindowLifecycleEvent::CloseRequested,
        1,
        "close_requested_veto",
    )?;
    assert_event_absent(
        gui,
        steps,
        window,
        WindowLifecycleEvent::Destroyed,
        "veto_survived",
    )
}

/// The armed-Defer leg: a second `CloseRequested` is observed and
/// the window stays open close-pending.
fn close_defer_leg(
    gui: &mut RealOsGuiTest,
    steps: &mut Vec<Value>,
    window: WindowId,
) -> Result<(), String> {
    gui.arm_close_response(window, CloseDecision::Defer);
    step(
        steps,
        "request_close_defer",
        gui.request_close(window),
        |()| Value::Null,
    )?;
    wait_event_count(
        gui,
        steps,
        window,
        WindowLifecycleEvent::CloseRequested,
        2,
        "close_requested_defer",
    )?;
    assert_event_absent(
        gui,
        steps,
        window,
        WindowLifecycleEvent::Destroyed,
        "defer_pending",
    )
}

/// The deferred-approve leg plus terminal-retirement probes: the
/// pending close is answered through `respond_close`, the window
/// is destroyed, and the retired id answers honest errors only.
fn close_approve_retire_leg(
    gui: &mut RealOsGuiTest,
    steps: &mut Vec<Value>,
    window: WindowId,
) -> Result<(), String> {
    step(
        steps,
        "respond_close_approve",
        gui.respond_close(window, CloseDecision::Approve),
        |()| Value::Null,
    )?;
    wait_event_count(
        gui,
        steps,
        window,
        WindowLifecycleEvent::Destroyed,
        1,
        "destroyed",
    )?;
    if gui.windows().contains(&window) {
        return Err("approve: window still listed after Destroyed".to_string());
    }
    steps.push(json!({ "step": "destroyed", "verdict": "pass" }));

    // Terminal retirement — the destroyed id is dead: a further
    // close request is an honest no-op, `respond_close` and
    // `window_geometry` are defined errors, and no
    // `CloseRequested` may follow `Destroyed` in its log.
    step(
        steps,
        "request_close_retired",
        gui.request_close(window),
        |()| Value::Null,
    )?;
    gui.settle().map_err(|error| error.to_string())?;
    let events = gui.window_events(window);
    if events
        .iter()
        .skip_while(|event| **event != WindowLifecycleEvent::Destroyed)
        .any(|event| matches!(event, WindowLifecycleEvent::CloseRequested))
    {
        return Err("retired: CloseRequested observed after Destroyed".to_string());
    }
    if gui.respond_close(window, CloseDecision::Approve).is_ok() {
        steps.push(json!({
            "step": "respond_close_retired",
            "verdict": "fail",
            "error": "respond_close succeeded on a destroyed window",
        }));
        return Err("retired: respond_close on destroyed window must fail".to_string());
    }
    steps.push(json!({ "step": "respond_close_retired", "verdict": "pass" }));
    if gui.window_geometry(window).is_ok() {
        steps.push(json!({
            "step": "geometry_retired",
            "verdict": "fail",
            "error": "window_geometry succeeded on a destroyed window",
        }));
        return Err("retired: window_geometry on destroyed window must fail".to_string());
    }
    steps.push(json!({ "step": "geometry_retired", "verdict": "pass" }));
    Ok(())
}

/// The unanswered-default leg: a `request_close` with no armed
/// decision takes the driver's `Approve` default and destroys the
/// window.
fn close_default_leg(
    gui: &mut RealOsGuiTest,
    steps: &mut Vec<Value>,
    window: WindowId,
) -> Result<(), String> {
    step(
        steps,
        "request_close_default",
        gui.request_close(window),
        |()| Value::Null,
    )?;
    wait_event_count(
        gui,
        steps,
        window,
        WindowLifecycleEvent::CloseRequested,
        1,
        "close_requested_default",
    )?;
    wait_event_count(
        gui,
        steps,
        window,
        WindowLifecycleEvent::Destroyed,
        1,
        "destroyed_default",
    )
}

/// The close-case attempt: drives every close-decision route on
/// the two target windows — `window_b` takes armed `Veto`,
/// armed `Defer`, `respond_close(Approve)`, and terminal-retirement
/// probes; `window_c` takes the unanswered default `Approve`.
/// `window_a` and every other non-target window stay alive (the
/// GPUI platform loop exits when its last window closes) and must
/// observe no close lifecycle. Returns the observation extras on
/// success — the same sequence shape the window-surface lifecycle
/// test proved on real windows.
#[expect(clippy::too_many_lines)]
fn close_body(
    gui: &mut RealOsGuiTest,
    window_a: WindowId,
    window_b: Option<WindowId>,
    window_c: Option<WindowId>,
) -> ScenarioOutcome {
    let mut steps = Vec::new();
    let windows = gui.windows();
    steps.push(json!({
        "step": "window_count",
        "verdict": if windows.len() >= 3 { "pass" } else { "fail" },
        "count": windows.len(),
        "windows": windows.iter().map(ToString::to_string).collect::<Vec<_>>(),
    }));

    let attempt = (|| -> Result<Value, String> {
        if windows.len() < 3 {
            return Err("session mounted fewer than 3 windows".to_string());
        }
        let wb = window_b.ok_or_else(|| "second window id not recorded".to_string())?;
        let wc = window_c.ok_or_else(|| "third window id not recorded".to_string())?;
        for window in [window_a, wb, wc] {
            if !windows.contains(&window) {
                return Err(format!("mounted window {window} missing from windows()"));
            }
        }

        // Honest-error probe: answering a close that was never
        // requested is a defined refusal, not a silent no-op.
        match gui.respond_close(wb, CloseDecision::Approve) {
            Ok(()) => {
                steps.push(json!({
                    "step": "respond_close_without_pending",
                    "verdict": "fail",
                    "error": "respond_close succeeded with no pending close",
                }));
                return Err("respond_close without a pending close must fail".to_string());
            }
            Err(error) => steps.push(json!({
                "step": "respond_close_without_pending",
                "verdict": "pass",
                "detail": error.to_string(),
            })),
        }

        close_veto_leg(gui, &mut steps, wb)?;
        close_defer_leg(gui, &mut steps, wb)?;
        close_approve_retire_leg(gui, &mut steps, wb)?;
        close_default_leg(gui, &mut steps, wc)?;

        // Terminal state — every non-target window survives (the
        // sentinel plus any fixture windows later cases still
        // need); no survivor's log may carry close lifecycle, and
        // B/C logs must end at Destroyed with the expected close
        // subsequence.
        let survivors: Vec<WindowId> = windows
            .iter()
            .copied()
            .filter(|window| *window != wb && *window != wc)
            .collect();
        let remaining = gui.windows();
        if remaining != survivors {
            return Err(format!(
                "expected non-target windows {survivors:?} to remain, got {remaining:?}"
            ));
        }
        steps.push(json!({
            "step": "survivors_only",
            "verdict": "pass",
            "windows": remaining.iter().map(ToString::to_string).collect::<Vec<_>>(),
        }));
        for survivor in &survivors {
            let survivor_events = gui.window_events(*survivor);
            if survivor_events.contains(&WindowLifecycleEvent::CloseRequested)
                || survivor_events.contains(&WindowLifecycleEvent::Destroyed)
            {
                return Err(format!(
                    "non-target window {survivor} observed close lifecycle"
                ));
            }
        }
        steps.push(json!({ "step": "survivors_isolated", "verdict": "pass" }));
        let b_sequence = close_sequence(gui, wb);
        if b_sequence.as_slice() != ["close_requested", "close_requested", "destroyed"] {
            return Err(format!("window B close sequence {b_sequence:?} unexpected"));
        }
        let c_sequence = close_sequence(gui, wc);
        if c_sequence.as_slice() != ["close_requested", "destroyed"] {
            return Err(format!("window C close sequence {c_sequence:?} unexpected"));
        }
        steps.push(json!({
            "step": "close_sequences",
            "verdict": "pass",
            "window_b": b_sequence.as_slice(),
            "window_c": c_sequence.as_slice(),
        }));

        Ok(json!({
            "window_a": window_a.to_string(),
            "window_b": wb.to_string(),
            "window_c": wc.to_string(),
            "windows_after": remaining.iter().map(ToString::to_string).collect::<Vec<_>>(),
            "close_sequence_b": b_sequence,
            "close_sequence_c": c_sequence,
            "lifecycle_a": lifecycle_summary(gui, window_a),
            "lifecycle_b": lifecycle_summary(gui, wb),
            "lifecycle_c": lifecycle_summary(gui, wc),
        }))
    })();

    steps_outcome(
        &steps,
        attempt,
        "three real OS windows; window B drove armed Veto → armed \
         Defer → respond_close(Approve) → Destroyed plus terminal-\
         retirement probes, window C drove the unanswered default \
         Approve → Destroyed, every non-target window observed no \
         close lifecycle and survived",
        &["window:close:lifecycle:real-window"],
    )
}

/// `real-window-close`: drives the full close-decision matrix on
/// the two target windows — window B takes armed `Veto`,
/// armed `Defer`, `respond_close(Approve)`, and terminal-retirement
/// probes; window C takes the unanswered default `Approve`. Every
/// non-target window (sentinel A plus the command-receipt fader
/// window D) survives untouched.
fn real_window_close_case(ctx: &mut RealDockContext<'_>, spec: &ScenarioSpec) -> ScenarioReport {
    let window_a = ctx.window;
    let window_b = ctx.window_b;
    let window_c = ctx.window_c;
    let report = run_scenario(&mut *ctx.gui, spec, |gui| {
        Ok(close_body(gui, window_a, window_b, window_c))
    });
    finalize_case(report, ctx, EnvironmentKind::RealWindow, None, Map::new())
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
    /// The op returned `Ok` — an `effect_confirmed` observation then
    /// gates `Pass`; without it the honest verdict stays `NotRun`.
    Accepted,
    /// Any other landing — a contract error or a vanished session;
    /// `EditorOpVerification::failure` carries the detail.
    Other,
}

/// Post-open observation for the non-pixel editor-op cases
/// (`real-editor-resize`, `real-editor-redock`,
/// `real-editor-visibility`): the same child-HWND attach contract as
/// `real-editor-attach`, plus the op attempt's honest landing — the
/// defined `Unsupported` refusal, an accepted op whose OS-level
/// effect oracle either confirmed the change (`Pass`) or stayed
/// unobservable (`NotRun`), or a real failure. An accepted op whose
/// effect oracle observed a *mismatch* is a contract failure
/// (`verified == false`), never demoted to `NotRun`.
struct EditorOpVerification {
    /// `false` off-Windows — the OS observation half then reports
    /// `Unsupported`, never a fabricated pass.
    supported: bool,
    /// How the attempted op landed.
    landing: OpLanding,
    /// The op's OS-observable effect was confirmed: child-HWND
    /// geometry for `set_size`, the hide→destroy/show→recreate cycle
    /// for `set_visibility`, the attach-generation advance for
    /// `reattach`.
    effect_confirmed: bool,
    /// Every contract check agreed (attach children, session state
    /// after the op, close/detach convergence, expected events).
    verified: bool,
    /// First violated check (`name: detail`) for the `Fail` detail.
    failure: Option<String>,
    /// `observed` extras — container/child HWND ids, thread ids,
    /// drained events.
    extras: Value,
}

/// Verdict identity for an editor-op case — the `Pass` detail and
/// evidence tag `editor_op_body` reports when the op's effect oracle
/// confirmed the change (the refusal/`NotRun`/`Fail` arms keep their
/// own generic wording).
struct EditorOpIdentity {
    /// `Pass` detail — names the op, the oracle, and the contract.
    pass_detail: &'static str,
    /// Evidence tag pushed on `Pass`.
    evidence: &'static str,
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
/// contract → `session.set_size(800×600)` through the production
/// `InProcessEditorOps` redeeming against the real slot ledger →
/// the child HWND's `GetWindowRect` must report the requested size
/// (the fixture's `onSize` resizes its own window — the host only
/// reports the size) → the session must stay `Attached` (a refused
/// `set_size` is non-fatal by contract) → close convergence with
/// the unchanged `[Attached{Docked}, Closed]` event sequence. No
/// pixel capture — the oracle is child-window geometry, never
/// pixels.
#[expect(clippy::too_many_arguments)]
#[expect(clippy::too_many_lines)]
fn observe_editor_resize_lifecycle(
    gui: &RealOsGuiTest,
    container: Option<u64>,
    _lease_raw: u64,
    sessions: &mut PluginEditorSessions,
    session_id: PluginEditorSessionId,
    host: &mut InProcessHost,
    handle: PluginHandle,
    steps: &mut Vec<Value>,
) -> EditorOpVerification {
    /// The size the case requests — distinct from whatever geometry
    /// the attach produced so the oracle can tell "took effect"
    /// from "never moved".
    const REQUESTED: EditorSize = EditorSize {
        width: 800.0,
        height: 600.0,
    };
    let mut failure: Option<String> = None;
    let (mut observation, mut attach_ok) =
        observe_attached_children(container, steps, &mut failure);

    let mut landing = OpLanding::Other;
    let mut effect_confirmed = false;
    // An accepted op whose geometry oracle observed a mismatch is a
    // contract failure — tracked separately so `verified` (and thus
    // the `Fail` verdict) sees it.
    let mut effect_ok = true;
    match sessions.session_mut(session_id).map(|session| {
        let mut ops = real_editor_ops(gui, host, handle);
        session.set_size(&mut ops, REQUESTED)
    }) {
        Some(Err(EditorSessionError::Ops(EditorOpError::Unsupported))) => {
            steps.push(json!({
                "step": "editor_resize",
                "verdict": "unsupported",
                "detail": "session.set_size → InProcessEditorOps::set_size \
                           → HostCommand::SetGuiSize → the plugin declined \
                           resize (canResize) — the defined \
                           EditorOpError::Unsupported",
            }));
            landing = OpLanding::Refused;
        }
        Some(Ok(())) => {
            steps.push(json!({
                "step": "editor_resize",
                "verdict": "pass",
                "detail": "set_size accepted — verifying the child HWND \
                           geometry took the requested size",
                "requested": {
                    "width": REQUESTED.width,
                    "height": REQUESTED.height,
                },
            }));
            landing = OpLanding::Accepted;
            // Effect oracle — re-enumerate while attached: the child
            // contract must still hold AND every child's
            // `GetWindowRect` must report the requested size. An
            // accepted op whose geometry did not move is a contract
            // failure, never a `NotRun`.
            if observation.supported {
                let (resized, resized_ok) =
                    observe_attached_children(container, steps, &mut failure);
                attach_ok = attach_ok && resized_ok;
                let mismatches: Vec<String> = resized
                    .children
                    .iter()
                    .enumerate()
                    .filter(|(_, child)| {
                        f64::from(child.width) != REQUESTED.width
                            || f64::from(child.height) != REQUESTED.height
                    })
                    .map(|(index, child)| {
                        format!(
                            "child[{index}] hwnd={:#x}: {}x{} != {}x{}",
                            child.hwnd,
                            child.width,
                            child.height,
                            REQUESTED.width,
                            REQUESTED.height,
                        )
                    })
                    .collect();
                if resized.children.is_empty() {
                    // `child_enumerated`/`child_contract` already
                    // recorded the failure.
                    effect_ok = false;
                } else if mismatches.is_empty() {
                    effect_confirmed = true;
                    steps.push(json!({
                        "step": "editor_resize_effect",
                        "verdict": "pass",
                        "children": resized
                            .children
                            .iter()
                            .map(|child| json!({
                                "hwnd": child.hwnd,
                                "width": child.width,
                                "height": child.height,
                                "title": child.title,
                            }))
                            .collect::<Vec<_>>(),
                    }));
                } else {
                    record_obs_fail(
                        steps,
                        &mut failure,
                        "editor_resize_effect",
                        format!(
                            "accepted set_size but child HWND geometry did \
                             not take {}x{}: {}",
                            REQUESTED.width,
                            REQUESTED.height,
                            mismatches.join("; "),
                        ),
                    );
                    effect_ok = false;
                }
                observation = resized;
            }
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
        ATTACH_CLOSE_EVENTS,
    );
    EditorOpVerification {
        supported: observation.supported,
        landing,
        effect_confirmed,
        verified: attach_ok && state_ok && effect_ok && close_ok && detach_ok && converged_ok,
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
    let expected: &[EditorSessionEvent] = if refused {
        &[
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
        &[
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
    check_session_converged(sessions, session_id, steps, failure, expected)
}

/// `real-editor-redock` post-open observation: child-HWND attach
/// contract → `session.reattach` under the same live lease through
/// the production `InProcessEditorOps` redeeming against the real
/// slot ledger → the pre-reattach child HWNDs must be dead
/// (`removed()` → `DestroyWindow`) and the re-observed child
/// contract must hold under a new attach generation (the fixture
/// stamps `rvf-test-editor-attach-{n}` into the window text — an
/// HWND value alone can be reused) → `close` → `IsWindow(child) ==
/// 0` → the re-dock event sequence (`[Attached{Docked}, Detached,
/// Reattached{Docked}, Closed]` accepted / `Failed{Unsupported}`
/// refused). No pixel capture — the oracle is session events +
/// HWND contract + attach generation + session state.
#[expect(clippy::too_many_arguments)]
#[expect(clippy::too_many_lines)]
fn observe_editor_redock_lifecycle(
    gui: &RealOsGuiTest,
    container: Option<u64>,
    lease_raw: u64,
    sessions: &mut PluginEditorSessions,
    session_id: PluginEditorSessionId,
    host: &mut InProcessHost,
    handle: PluginHandle,
    steps: &mut Vec<Value>,
) -> EditorOpVerification {
    let mut failure: Option<String> = None;
    let (pre_observation, mut attach_ok) =
        observe_attached_children(container, steps, &mut failure);
    // Pre-reattach child identities — the recreation oracle: every
    // recorded child must die (`removed()` → `DestroyWindow`) and
    // the post-reattach generation must carry a new attach title.
    let pre_children: Vec<(u64, String)> = pre_observation
        .children
        .iter()
        .map(|child| (child.hwnd, child.title.clone()))
        .collect();
    let mut observation = pre_observation;

    let mut landing = OpLanding::Other;
    let mut effect_confirmed = false;
    // An accepted re-dock whose recreation oracle observed a
    // mismatch is a contract failure — tracked separately so
    // `verified` (and thus the `Fail` verdict) sees it.
    let mut effect_ok = true;
    let mut state_ok = true;
    match sessions.session_mut(session_id).map(|session| {
        let mut ops = real_editor_ops(gui, host, handle);
        session.reattach(&mut ops, &ParentSlotLease::new(lease_raw))
    }) {
        Some(Err(EditorSessionError::Ops(EditorOpError::Unsupported))) => {
            steps.push(json!({
                "step": "editor_redock",
                "verdict": "unsupported",
                "detail": "session.reattach → InProcessEditorOps::reattach \
                           → HostCommand::SetGuiParent → the plugin \
                           declined re-parent — the defined \
                           EditorOpError::Unsupported; the session lands \
                           Failed per contract",
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
                state_ok = false;
            }
        }
        Some(Ok(())) => {
            steps.push(json!({
                "step": "editor_redock",
                "verdict": "pass",
                "detail": "reattach accepted — verifying the old child \
                           HWNDs died and a new attach generation lives \
                           under the container",
            }));
            landing = OpLanding::Accepted;
            // Contract: an accepted re-dock lands `Attached` again.
            let attached = sessions
                .session(session_id)
                .is_some_and(|session| {
                    matches!(session.state(), EditorSessionState::Attached { mode } if mode == AttachMode::Docked)
                });
            if attached {
                steps.push(json!({
                    "step": "session_state_after_op",
                    "verdict": "pass",
                    "state": "Attached{Docked}",
                }));
            } else {
                record_obs_fail(
                    steps,
                    &mut failure,
                    "session_state_after_op",
                    "accepted reattach did not land Attached{Docked}",
                );
                state_ok = false;
            }
            // Effect oracle — only meaningful where HWNDs exist.
            if observation.supported {
                // Half 1 — every pre-reattach child must be dead:
                // `set_gui_parent` ran `removed()` → `DestroyWindow`.
                let survivors: Vec<String> = pre_children
                    .iter()
                    .filter(|(hwnd, _)| oschild::is_alive(*hwnd))
                    .map(|(hwnd, _)| format!("hwnd={hwnd:#x}"))
                    .collect();
                // Half 2 — the child contract must hold again under
                // the container, on a new attach generation.
                let (reattached, reattached_ok) =
                    observe_attached_children(container, steps, &mut failure);
                attach_ok = attach_ok && reattached_ok;
                let pre_titles: Vec<&str> = pre_children
                    .iter()
                    .map(|(_, title)| title.as_str())
                    .collect();
                let generation_advanced = !reattached.children.is_empty()
                    && reattached.children.iter().all(|child| {
                        !child.title.is_empty()
                            && pre_titles.iter().all(|title| *title != child.title)
                    });
                if survivors.is_empty() && !reattached.children.is_empty() && generation_advanced {
                    effect_confirmed = true;
                    steps.push(json!({
                        "step": "editor_redock_effect",
                        "verdict": "pass",
                        "detail": "pre-reattach children destroyed; new \
                                   attach generation observed under the \
                                   container",
                        "pre_titles": pre_children
                            .iter()
                            .map(|(_, title)| title.clone())
                            .collect::<Vec<_>>(),
                        "post_titles": reattached
                            .children
                            .iter()
                            .map(|child| child.title.clone())
                            .collect::<Vec<_>>(),
                    }));
                } else {
                    let mut problems = Vec::new();
                    if !survivors.is_empty() {
                        problems.push(format!(
                            "pre-reattach child HWND(s) still alive: {}",
                            survivors.join(", ")
                        ));
                    }
                    if reattached.children.is_empty() {
                        problems.push("no child re-appeared under the container".to_string());
                    } else if !generation_advanced {
                        problems.push(
                            "post-reattach children carry no new attach \
                             generation title"
                                .to_string(),
                        );
                    }
                    record_obs_fail(
                        steps,
                        &mut failure,
                        "editor_redock_effect",
                        format!(
                            "accepted reattach without a verified \
                                 detach/reattach cycle: {}",
                            problems.join("; ")
                        ),
                    );
                    effect_ok = false;
                }
                observation = reattached;
            }
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
        effect_confirmed,
        verified: attach_ok && state_ok && effect_ok && close_ok && detach_ok && events_ok,
        failure,
        extras: Value::Object(child_extras(container, &observation, &rendered)),
    }
}

/// Shared body for the non-pixel editor-op cases — identical
/// prerequisite/attempt mapping to `editor_attach_body`; the
/// case-supplied `observe_open` drives the op attempt and decides
/// the verdict: verified + accepted + effect-confirmed → `Pass`
/// (the case's [`EditorOpIdentity`]), verified + defined refusal →
/// `Unsupported`, accepted-but-unverifiable → `NotRun`, any
/// contract mismatch → `Fail`.
fn editor_op_body(
    gui: &mut RealOsGuiTest,
    window: WindowId,
    view: ViewInstanceId,
    plugin: Option<&PluginPair>,
    plugin_error: Option<&PluginUnavailable>,
    case: &EditorOpIdentity,
    observe_open: impl FnOnce(
        &RealOsGuiTest,
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
        AttachDrive {
            drive_open: drive_open_docked,
            observe_open,
        },
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
                if verification.verified
                    && verification.landing == OpLanding::Accepted
                    && verification.effect_confirmed =>
            {
                // The op was accepted AND its OS-level effect oracle
                // confirmed the change (child-HWND geometry /
                // hide-show cycle / attach generation) with every
                // lifecycle, state, event, and close check agreeing —
                // an honest `Pass`, never pixel-dependent.
                let mut outcome = ScenarioOutcome::new(ScenarioVerdict::Pass);
                outcome.detail = Some(case.pass_detail.to_string());
                outcome.evidence.push(case.evidence.to_string());
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
/// attempt — judged by the child-HWND geometry oracle
/// (`GetWindowRect` == requested size), session state/events, and
/// close convergence, never pixels.
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
            &EditorOpIdentity {
                pass_detail: "session.set_size(800x600) accepted through the \
                              REAL container lease; the child HWND's \
                              GetWindowRect reported the requested size \
                              (the fixture's onSize resized its own window), \
                              the session stayed Attached, events \
                              [Attached{Docked}, Closed], and close \
                              convergence verified — no pixels",
                evidence: "editor-resize:real-hwnd-geometry-effect",
            },
            observe_editor_resize_lifecycle,
        ))
    });
    finalize_case(
        report,
        ctx,
        EnvironmentKind::RealWindow,
        artifact,
        Map::new(),
    )
}

/// `real-editor-redock` case: pre-flight requires `NativePluginDock`,
/// then the body drives the real attach chain plus the `reattach`
/// attempt under the same live lease — judged by the recreation
/// oracle (pre-reattach children destroyed, new attach generation
/// under the container), session events, child HWND contract, and
/// session state, never pixels.
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
            &EditorOpIdentity {
                pass_detail: "session.reattach accepted through the REAL \
                              container lease; the pre-reattach child HWNDs \
                              were destroyed (removed() → DestroyWindow) and \
                              a new attach generation re-appeared under the \
                              container (rvf-test-editor-attach-{n} title \
                              advanced), events [Attached{Docked}, Detached, \
                              Reattached{Docked}, Closed], and close \
                              convergence verified — no pixels",
                evidence: "editor-redock:real-attach-generation-cycle",
            },
            observe_editor_redock_lifecycle,
        ))
    });
    finalize_case(
        report,
        ctx,
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

    // `Terminated` is the abnormal terminal event — the drained
    // stream must be exactly `[Attached{Docked}, Terminated{HostLost}]`
    // (no graceful `Closed`); `session_closed_concluded` still holds
    // because `notify_terminated` converges the session to `Closed`.
    let (converged_ok, rendered) = check_session_converged(
        sessions,
        session_id,
        steps,
        failure,
        &[
            EditorSessionEvent::Attached {
                mode: AttachMode::Docked,
            },
            EditorSessionEvent::Terminated {
                cause: SessionFailCause::HostLost,
            },
        ],
    );
    let mut ok = converged_ok;

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
#[expect(clippy::too_many_arguments)]
fn observe_session_terminate_lifecycle(
    _gui: &RealOsGuiTest,
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
        ChildLifecycle {
            drive_open: drive_open_docked,
            observe_open: observe_session_terminate_lifecycle,
            case: &ChildCaseIdentity {
                pass_detail: "test_editor real IPlugView attached under the \
                              leased container HWND; host Destroy converged the \
                              session to Terminated{HostLost} exactly once \
                              (repeated notify is a no-op), the child HWND tore \
                              down (IsWindow(child)==0), and the host survived",
                evidence: "session-terminate:host-loss-exactly-once",
                label: "session-terminate",
            },
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
    finalize_case(
        report,
        ctx,
        EnvironmentKind::RealWindow,
        artifact,
        Map::new(),
    )
}

// ---------------------------------------------------------------------
// cases: real-session-{degraded,recovery,timeout},
//        real-editor-visibility, real-session-{crash,hang}
//        (Issue #389 session-lifecycle remainder)
// ---------------------------------------------------------------------

/// `real-session-degraded` open drive — `open_or_degrade` with
/// `Floating` preferred and `Docked` fallback through the real
/// lease. `InProcessEditorOps` refuses the non-docked preferred
/// mode with the defined `Unsupported` before any lease redeem, so
/// the fallback runs the real `OpenGui` attach and lands
/// `Attached{Docked}` emitting `Degraded{Docked}` — never
/// `Attached` — returning `OpenOutcome::Degraded`. An
/// `OpenOutcome::Attached` landing means the degrade path was not
/// exercised and is recorded `fail`; the observe half's exact
/// event check decides the verdict either way. An unrecognized
/// `OpenOutcome` variant (the enum is `#[non_exhaustive]`) maps to
/// `Ops(Internal)` — a contract surprise, never a silent pass.
fn drive_open_degraded(
    gui: &RealOsGuiTest,
    host: &mut InProcessHost,
    handle: PluginHandle,
    sessions: &mut PluginEditorSessions,
    session_id: PluginEditorSessionId,
    parent_slot_lease: ParentSlotLease,
    steps: &mut Vec<Value>,
) -> Option<Result<EditorCapabilities, EditorSessionError>> {
    let mut ops = real_editor_ops(gui, host, handle);
    let outcome = sessions.session_mut(session_id).map(|session| {
        session.open_or_degrade(
            &mut ops,
            &parent_slot_lease,
            editor_size(),
            AttachMode::Floating,
            AttachMode::Docked,
        )
    });
    let failed_unsupported = session_failed_unsupported(sessions, session_id);
    let (verdict, detail) = match &outcome {
        Some(Ok(OpenOutcome::Degraded { mode, .. })) => (
            "pass",
            format!("open_or_degrade(Floating→Docked) degraded to {mode:?}"),
        ),
        Some(Ok(OpenOutcome::Attached(_))) => (
            "fail",
            "open_or_degrade landed Attached — the degrade path was not exercised".to_string(),
        ),
        Some(Ok(_)) => (
            "fail",
            "open_or_degrade returned an unrecognized OpenOutcome variant".to_string(),
        ),
        Some(Err(error)) => {
            let verdict = if matches!(error, EditorSessionError::Ops(EditorOpError::Unsupported)) {
                "unsupported"
            } else {
                "fail"
            };
            (verdict, error.to_string())
        }
        None => (
            "fail",
            "editor session vanished before open_or_degrade".to_string(),
        ),
    };
    steps.push(json!({
        "step": "session.open_or_degrade(Floating→Docked)",
        "verdict": verdict,
        "detail": detail,
        "failed_unsupported": failed_unsupported,
    }));
    outcome.map(|result| match result {
        Ok(OpenOutcome::Attached(capabilities) | OpenOutcome::Degraded { capabilities, .. }) => {
            Ok(capabilities)
        }
        Ok(_) => Err(EditorSessionError::Ops(EditorOpError::Internal)),
        Err(error) => Err(error),
    })
}

/// `real-session-recovery` open drive — `open(Floating)` first,
/// which `InProcessEditorOps` refuses with the defined
/// `Unsupported` before any lease redeem, landing the session
/// `Failed{Unsupported}` with `editor_live == false` (a refused
/// open produced no adapter editor, and `open_gui` refuses a
/// second open on a live view — this refused-open `Failed` is the
/// only recoverable starting point). `recover(Docked)` then runs
/// the real `OpenGui` attach and lands `Attached{Docked}` emitting
/// `Recovered{Docked}`.
fn drive_open_recovery(
    gui: &RealOsGuiTest,
    host: &mut InProcessHost,
    handle: PluginHandle,
    sessions: &mut PluginEditorSessions,
    session_id: PluginEditorSessionId,
    parent_slot_lease: ParentSlotLease,
    steps: &mut Vec<Value>,
) -> Option<Result<EditorCapabilities, EditorSessionError>> {
    let mut ops = real_editor_ops(gui, host, handle);

    // The refused open that creates the recoverable
    // `Failed{Unsupported}` state — any other landing is recorded
    // honestly and the recover step plus the observe half's exact
    // event check decide the verdict.
    let refused = sessions.session_mut(session_id).map(|session| {
        session.open(
            &mut ops,
            &parent_slot_lease,
            editor_size(),
            AttachMode::Floating,
        )
    });
    let (refused_verdict, refused_detail) = match &refused {
        Some(Err(EditorSessionError::Ops(EditorOpError::Unsupported))) => (
            "unsupported",
            "session.open(Floating) → defined Unsupported refusal → \
             Failed{Unsupported} (editor_live=false — the recoverable \
             starting point)"
                .to_string(),
        ),
        Some(Ok(_)) => (
            "fail",
            "session.open(Floating) unexpectedly attached — the \
             preferred-mode refusal was not exercised"
                .to_string(),
        ),
        Some(Err(error)) => ("fail", error.to_string()),
        None => ("fail", "editor session vanished before open".to_string()),
    };
    steps.push(json!({
        "step": "session.open(Floating)",
        "verdict": refused_verdict,
        "detail": refused_detail,
        "failed_unsupported": session_failed_unsupported(sessions, session_id),
    }));

    // The recovery — `Failed{Unsupported}` → `recover(Docked)` runs
    // the real `OpenGui` attach and lands `Recovered{Docked}`.
    let recovered = sessions.session_mut(session_id).map(|session| {
        session.recover(
            &mut ops,
            &parent_slot_lease,
            editor_size(),
            AttachMode::Docked,
        )
    });
    let (recover_verdict, recover_detail) = match &recovered {
        Some(Ok(_)) => ("pass", "editor capabilities returned".to_string()),
        Some(Err(error)) => {
            let verdict = if matches!(error, EditorSessionError::Ops(EditorOpError::Unsupported)) {
                "unsupported"
            } else {
                "fail"
            };
            (verdict, error.to_string())
        }
        None => ("fail", "editor session vanished before recover".to_string()),
    };
    steps.push(json!({
        "step": "session.recover(Docked)",
        "verdict": recover_verdict,
        "detail": recover_detail,
    }));
    recovered
}

/// Post-open observation for the graceful-close lifecycle cases
/// whose drained event sequence differs from the plain-attach
/// `[Attached{Docked}, Closed]` — child-HWND attach contract →
/// `session.close` (production `close_gui` → `removed()` →
/// `DestroyWindow`) → `IsWindow(child) == 0` → `Closed`+`concluded`
/// → drained events exactly `expected`.
fn observe_session_close_events(
    container: Option<u64>,
    sessions: &mut PluginEditorSessions,
    session_id: PluginEditorSessionId,
    host: &mut InProcessHost,
    handle: PluginHandle,
    steps: &mut Vec<Value>,
    expected: &[EditorSessionEvent],
) -> ChildVerification {
    let mut failure: Option<String> = None;
    let (mut observation, attach_ok) = observe_attached_children(container, steps, &mut failure);
    let (close_ok, detach_ok, converged_ok, rendered) = conclude_editor_session(
        &mut observation,
        sessions,
        session_id,
        host,
        handle,
        steps,
        &mut failure,
        expected,
    );
    ChildVerification {
        supported: observation.supported,
        verified: attach_ok && close_ok && detach_ok && converged_ok,
        failure,
        extras: Value::Object(child_extras(container, &observation, &rendered)),
    }
}

/// `real-session-degraded` post-open observation — identical
/// close-convergence contract to the plain attach, except the
/// drained events must be exactly `[Degraded{Docked}, Closed]`:
/// the degrade landing is the case's whole point and an `Attached`
/// event here would mean the degrade path never ran.
#[expect(clippy::too_many_arguments)]
fn observe_session_degraded_lifecycle(
    _gui: &RealOsGuiTest,
    container: Option<u64>,
    _lease_raw: u64,
    sessions: &mut PluginEditorSessions,
    session_id: PluginEditorSessionId,
    host: &mut InProcessHost,
    handle: PluginHandle,
    steps: &mut Vec<Value>,
) -> ChildVerification {
    observe_session_close_events(
        container,
        sessions,
        session_id,
        host,
        handle,
        steps,
        &[
            EditorSessionEvent::Degraded {
                mode: AttachMode::Docked,
            },
            EditorSessionEvent::Closed,
        ],
    )
}

/// `real-session-recovery` post-open observation — close
/// convergence with the drained events exactly
/// `[Failed{Unsupported}, Recovered{Docked}, Closed]`: the refused
/// `open(Floating)` failure, the `recover(Docked)` landing, and the
/// graceful close — in that order, nothing else.
#[expect(clippy::too_many_arguments)]
fn observe_session_recovery_lifecycle(
    _gui: &RealOsGuiTest,
    container: Option<u64>,
    _lease_raw: u64,
    sessions: &mut PluginEditorSessions,
    session_id: PluginEditorSessionId,
    host: &mut InProcessHost,
    handle: PluginHandle,
    steps: &mut Vec<Value>,
) -> ChildVerification {
    observe_session_close_events(
        container,
        sessions,
        session_id,
        host,
        handle,
        steps,
        &[
            EditorSessionEvent::Failed {
                cause: SessionFailCause::Unsupported,
            },
            EditorSessionEvent::Recovered {
                mode: AttachMode::Docked,
            },
            EditorSessionEvent::Closed,
        ],
    )
}

/// The deadline drive for `real-session-timeout` — install a
/// ZERO-timeout `SessionDeadlinePolicy`, `begin_detach`, prove a
/// deadline armed, then `tick_all` once: the in-flight `Detaching`
/// request expires into `Failed{Timeout}`. Returns whether every
/// step agreed.
fn drive_detach_timeout(
    sessions: &mut PluginEditorSessions,
    session_id: PluginEditorSessionId,
    steps: &mut Vec<Value>,
    failure: &mut Option<String>,
) -> bool {
    let mut ok = true;
    if let Some(session) = sessions.session_mut(session_id) {
        session.set_deadline_policy(Some(SessionDeadlinePolicy {
            timeout: Duration::ZERO,
            clock: Box::new(MonotonicSessionClock::new()),
        }));
        steps.push(json!({
            "step": "deadline_policy_zero",
            "verdict": "pass",
            "timeout": "0s",
        }));
        match session.begin_detach() {
            Ok(request) => steps.push(json!({
                "step": "begin_detach",
                "verdict": "pass",
                "request": request,
            })),
            Err(error) => {
                record_obs_fail(steps, failure, "begin_detach", error.to_string());
                ok = false;
            }
        }
        if let Some(deadline) = session.request_deadline() {
            steps.push(json!({
                "step": "request_deadline",
                "verdict": "pass",
                "deadline": format!("{deadline:?}"),
            }));
        } else {
            record_obs_fail(
                steps,
                failure,
                "request_deadline",
                "no deadline armed for the in-flight detach",
            );
            ok = false;
        }
    } else {
        record_obs_fail(
            steps,
            failure,
            "deadline_policy_zero",
            "editor session vanished before the deadline drive",
        );
        ok = false;
    }

    let timed_out = sessions.tick_all();
    if timed_out == 1 {
        steps.push(json!({
            "step": "tick_all",
            "verdict": "pass",
            "timed_out": timed_out,
        }));
    } else {
        record_obs_fail(
            steps,
            failure,
            "tick_all",
            format!("expected exactly 1 timed-out request, got {timed_out}"),
        );
        ok = false;
    }

    let failed_timeout = sessions.session(session_id).is_some_and(|session| {
        matches!(
            session.state(),
            EditorSessionState::Failed {
                cause: SessionFailCause::Timeout
            }
        )
    });
    if failed_timeout {
        steps.push(json!({
            "step": "session_state_after_timeout",
            "verdict": "pass",
            "state": "Failed{Timeout}",
        }));
    } else {
        record_obs_fail(
            steps,
            failure,
            "session_state_after_timeout",
            "session did not land Failed{Timeout} after tick_all",
        );
        ok = false;
    }
    ok
}

/// `real-session-timeout` post-open observation: child-HWND attach
/// contract → [`drive_detach_timeout`] → close convergence with
/// drained events exactly `[Attached{Docked}, Failed{Timeout},
/// Closed]`. The timeout is caused by the session deadline/tick
/// API — never a synthetic verdict.
#[expect(clippy::too_many_arguments)]
fn observe_session_timeout_lifecycle(
    _gui: &RealOsGuiTest,
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

    let deadline_ok = drive_detach_timeout(sessions, session_id, steps, &mut failure);

    let (close_ok, detach_ok, converged_ok, rendered) = conclude_editor_session(
        &mut observation,
        sessions,
        session_id,
        host,
        handle,
        steps,
        &mut failure,
        &[
            EditorSessionEvent::Attached {
                mode: AttachMode::Docked,
            },
            EditorSessionEvent::Failed {
                cause: SessionFailCause::Timeout,
            },
            EditorSessionEvent::Closed,
        ],
    );
    ChildVerification {
        supported: observation.supported,
        verified: attach_ok && deadline_ok && close_ok && detach_ok && converged_ok,
        failure,
        extras: Value::Object(child_extras(container, &observation, &rendered)),
    }
}

/// `real-session-degraded` body — the degrade-fallback child-HWND
/// lifecycle through [`editor_child_body`].
fn session_degraded_body(
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
        ChildLifecycle {
            drive_open: drive_open_degraded,
            observe_open: observe_session_degraded_lifecycle,
            case: &ChildCaseIdentity {
                pass_detail: "open_or_degrade(Floating→Docked) refused the \
                              preferred mode with the defined Unsupported, \
                              fell back to Docked, attached the real \
                              IPlugView child HWND under the leased \
                              container, and emitted Degraded{Docked} — \
                              never Attached — then closed cleanly \
                              (events [Degraded{Docked}, Closed])",
                evidence: "session-degraded:open-or-degrade-docked-fallback",
                label: "session-degraded",
            },
        },
    )
}

/// `real-session-degraded` case: pre-flight requires
/// `NativePluginDock`, then the body drives the degrade-fallback
/// attach chain — judged by `OpenOutcome::Degraded`, the
/// `Degraded{Docked}` event, the child HWND contract, and close
/// convergence, never pixels.
fn real_session_degraded_case(
    ctx: &mut RealDockContext<'_>,
    spec: &ScenarioSpec,
) -> ScenarioReport {
    let window = ctx.window;
    let view = ctx.view;
    let plugin = ctx.plugin;
    let plugin_error = ctx.plugin_error;
    let artifact = ctx.artifact;
    let report = run_scenario(&mut *ctx.gui, spec, |gui| {
        Ok(session_degraded_body(
            gui,
            window,
            view,
            plugin,
            plugin_error,
        ))
    });
    finalize_case(
        report,
        ctx,
        EnvironmentKind::RealWindow,
        artifact,
        Map::new(),
    )
}

/// `real-session-recovery` body — the refused-open → recover
/// child-HWND lifecycle through [`editor_child_body`].
fn session_recovery_body(
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
        ChildLifecycle {
            drive_open: drive_open_recovery,
            observe_open: observe_session_recovery_lifecycle,
            case: &ChildCaseIdentity {
                pass_detail: "open(Floating) landed the defined \
                              Failed{Unsupported} (editor_live=false), \
                              recover(Docked) attached the real IPlugView \
                              child HWND under the leased container and \
                              emitted Recovered{Docked} (events \
                              [Failed{Unsupported}, Recovered{Docked}, \
                              Closed])",
                evidence: "session-recovery:refused-open-recover-docked",
                label: "session-recovery",
            },
        },
    )
}

/// `real-session-recovery` case: pre-flight requires
/// `NativePluginDock`, then the body drives the refused-open →
/// recover chain — judged by `Failed{Unsupported}` →
/// `Recovered{Docked}`, the child HWND contract, and close
/// convergence, never pixels.
fn real_session_recovery_case(
    ctx: &mut RealDockContext<'_>,
    spec: &ScenarioSpec,
) -> ScenarioReport {
    let window = ctx.window;
    let view = ctx.view;
    let plugin = ctx.plugin;
    let plugin_error = ctx.plugin_error;
    let artifact = ctx.artifact;
    let report = run_scenario(&mut *ctx.gui, spec, |gui| {
        Ok(session_recovery_body(
            gui,
            window,
            view,
            plugin,
            plugin_error,
        ))
    });
    finalize_case(
        report,
        ctx,
        EnvironmentKind::RealWindow,
        artifact,
        Map::new(),
    )
}

/// `real-session-timeout` body — the deadline-timeout child-HWND
/// lifecycle through [`editor_child_body`] (normal docked open;
/// the timeout drive lives in the observation).
fn session_timeout_body(
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
        ChildLifecycle {
            drive_open: drive_open_docked,
            observe_open: observe_session_timeout_lifecycle,
            case: &ChildCaseIdentity {
                pass_detail: "ZERO SessionDeadlinePolicy + begin_detach + \
                              tick_all expired the in-flight detach into \
                              Failed{Timeout} on the real attached editor \
                              (events [Attached{Docked}, Failed{Timeout}, \
                              Closed]); the child HWND tore down on close",
                evidence: "session-timeout:deadline-policy-zero-tick",
                label: "session-timeout",
            },
        },
    )
}

/// `real-session-timeout` case: pre-flight requires
/// `NativePluginDock`, then the body drives the real attach chain
/// plus the deadline-timeout lifecycle — judged by
/// `Failed{Timeout}`, the child HWND contract, and close
/// convergence, never pixels.
fn real_session_timeout_case(ctx: &mut RealDockContext<'_>, spec: &ScenarioSpec) -> ScenarioReport {
    let window = ctx.window;
    let view = ctx.view;
    let plugin = ctx.plugin;
    let plugin_error = ctx.plugin_error;
    let artifact = ctx.artifact;
    let report = run_scenario(&mut *ctx.gui, spec, |gui| {
        Ok(session_timeout_body(
            gui,
            window,
            view,
            plugin,
            plugin_error,
        ))
    });
    finalize_case(
        report,
        ctx,
        EnvironmentKind::RealWindow,
        artifact,
        Map::new(),
    )
}

/// `real-editor-visibility` post-open observation: child-HWND
/// attach contract → `session.set_visibility(false)` through the
/// production `InProcessEditorOps` redeeming against the real slot
/// ledger → the session must land `Hidden{Docked}` and every
/// pre-hide child HWND must be dead (`removed()` → `DestroyWindow`
/// — `IPlugView` has no hide verb, so hide IS detach) →
/// `session.set_visibility(true)` → `Attached{Docked}` again and
/// the child contract must hold under a new attach generation →
/// `close` → `IsWindow(child) == 0` → the visibility event sequence
/// (`[Attached{Docked}, Hidden, Shown{Docked}, Closed]` accepted /
/// `[Attached{Docked}, Closed]` refused). An accepted op whose
/// hide/show oracle observed a mismatch is a contract failure,
/// never a `NotRun`. No pixel capture — the oracle is child-HWND
/// lifecycle + session state/events.
#[expect(clippy::too_many_arguments)]
#[expect(clippy::too_many_lines)]
fn observe_editor_visibility_lifecycle(
    gui: &RealOsGuiTest,
    container: Option<u64>,
    _lease_raw: u64,
    sessions: &mut PluginEditorSessions,
    session_id: PluginEditorSessionId,
    host: &mut InProcessHost,
    handle: PluginHandle,
    steps: &mut Vec<Value>,
) -> EditorOpVerification {
    let mut failure: Option<String> = None;
    let (mut observation, mut attach_ok) =
        observe_attached_children(container, steps, &mut failure);
    // Pre-hide child identities — the hide oracle proves every one
    // of them died before the show half re-observes the container.
    let pre_hide: Vec<u64> = observation
        .children
        .iter()
        .map(|child| child.hwnd)
        .collect();
    let pre_titles: Vec<String> = observation
        .children
        .iter()
        .map(|child| child.title.clone())
        .collect();

    let mut landing = OpLanding::Other;
    let mut effect_confirmed = false;
    // An accepted cycle whose hide/show oracle observed a mismatch
    // is a contract failure — tracked separately so `verified`
    // (and thus the `Fail` verdict) sees it.
    let mut effect_ok = true;
    let mut state_ok = true;
    // Whether the show half of the cycle ran `Ok` — picks the honest
    // expected event tail.
    let mut shown = false;
    match sessions.session_mut(session_id).map(|session| {
        let mut ops = real_editor_ops(gui, host, handle);
        session.set_visibility(&mut ops, false)
    }) {
        Some(Err(EditorSessionError::Ops(EditorOpError::Unsupported))) => {
            steps.push(json!({
                "step": "editor_visibility",
                "verdict": "unsupported",
                "detail": "session.set_visibility(false) → \
                           InProcessEditorOps::set_visibility → \
                           HostCommand::SetGuiVisibility → the plugin \
                           declined hide — the defined \
                           EditorOpError::Unsupported",
            }));
            landing = OpLanding::Refused;
            // Contract: a refused hide leaves the session
            // `Attached{Docked}` — no `Hidden`/`Shown` may appear.
            let attached = sessions.session(session_id).is_some_and(|session| {
                matches!(session.state(), EditorSessionState::Attached { mode } if mode == AttachMode::Docked)
            });
            if attached {
                steps.push(json!({
                    "step": "session_state_after_op",
                    "verdict": "pass",
                    "state": "Attached{Docked}",
                }));
            } else {
                record_obs_fail(
                    steps,
                    &mut failure,
                    "session_state_after_op",
                    "refused set_visibility did not leave Attached{Docked}",
                );
                state_ok = false;
            }
        }
        Some(Ok(())) => {
            landing = OpLanding::Accepted;
            steps.push(json!({
                "step": "editor_visibility_hide",
                "verdict": "pass",
                "detail": "set_visibility(false) accepted — verifying \
                           Hidden{Docked} and child-HWND teardown",
            }));
            // Contract: hide lands `Hidden{Docked}` with a `Hidden`
            // event (checked in the event tail below).
            let hidden = sessions.session(session_id).is_some_and(|session| {
                matches!(session.state(), EditorSessionState::Hidden { mode } if mode == AttachMode::Docked)
            });
            if hidden {
                steps.push(json!({
                    "step": "session_state_after_hide",
                    "verdict": "pass",
                    "state": "Hidden{Docked}",
                }));
            } else {
                record_obs_fail(
                    steps,
                    &mut failure,
                    "session_state_after_hide",
                    "accepted hide did not land Hidden{Docked}",
                );
                state_ok = false;
            }
            // Hide effect oracle — `IPlugView` has no hide verb, so
            // hide IS `removed()` → `DestroyWindow`: every pre-hide
            // child must be dead and no live child may remain under
            // the container.
            if observation.supported {
                let survivors: Vec<String> = pre_hide
                    .iter()
                    .filter(|hwnd| oschild::is_alive(**hwnd))
                    .map(|hwnd| format!("hwnd={hwnd:#x}"))
                    .collect();
                let hidden_enum = match container {
                    Some(token) => oschild::observe(token),
                    None => oschild::empty(),
                };
                let live_remaining: Vec<String> = hidden_enum
                    .children
                    .iter()
                    .filter(|child| child.attach.alive)
                    .map(|child| format!("hwnd={:#x}", child.hwnd))
                    .collect();
                if survivors.is_empty() && live_remaining.is_empty() {
                    steps.push(json!({
                        "step": "editor_visibility_hide_effect",
                        "verdict": "pass",
                        "detail": "every pre-hide child HWND destroyed; \
                                   no live child remains under the container",
                        "pre_hwnds": pre_hide,
                    }));
                } else {
                    record_obs_fail(
                        steps,
                        &mut failure,
                        "editor_visibility_hide_effect",
                        format!(
                            "accepted hide left live child HWND(s): {}",
                            survivors
                                .iter()
                                .chain(live_remaining.iter())
                                .cloned()
                                .collect::<Vec<_>>()
                                .join(", "),
                        ),
                    );
                    effect_ok = false;
                }
            }
            // Show half — the same verb surface in reverse. A show
            // refusal after an accepted hide is a contract break.
            match sessions.session_mut(session_id).map(|session| {
                let mut ops = real_editor_ops(gui, host, handle);
                session.set_visibility(&mut ops, true)
            }) {
                Some(Ok(())) => {
                    shown = true;
                    steps.push(json!({
                        "step": "editor_visibility_show",
                        "verdict": "pass",
                        "detail": "set_visibility(true) accepted — \
                                   verifying Attached{Docked} and the \
                                   child contract on the new generation",
                    }));
                    let attached = sessions.session(session_id).is_some_and(|session| {
                        matches!(session.state(), EditorSessionState::Attached { mode } if mode == AttachMode::Docked)
                    });
                    if attached {
                        steps.push(json!({
                            "step": "session_state_after_show",
                            "verdict": "pass",
                            "state": "Attached{Docked}",
                        }));
                    } else {
                        record_obs_fail(
                            steps,
                            &mut failure,
                            "session_state_after_show",
                            "accepted show did not land Attached{Docked}",
                        );
                        state_ok = false;
                    }
                    // Show effect oracle — the child contract must
                    // hold again; the new generation's attach title
                    // is recorded as recreation evidence (the old
                    // children are already proven dead above, so a
                    // live parented child under the container is
                    // necessarily a new window).
                    if observation.supported {
                        let (reshown, reshown_ok) =
                            observe_attached_children(container, steps, &mut failure);
                        attach_ok = attach_ok && reshown_ok;
                        let generation_advanced = !reshown.children.is_empty()
                            && reshown.children.iter().all(|child| {
                                !child.title.is_empty()
                                    && pre_titles.iter().all(|title| *title != child.title)
                            });
                        if reshown.children.is_empty() {
                            // `child_enumerated` already recorded the
                            // contract failure.
                            effect_ok = false;
                        } else {
                            steps.push(json!({
                                "step": "editor_visibility_show_effect",
                                "verdict": "pass",
                                "detail": "child contract re-established \
                                           under the container",
                                "pre_titles": pre_titles,
                                "post_titles": reshown
                                    .children
                                    .iter()
                                    .map(|child| child.title.clone())
                                    .collect::<Vec<_>>(),
                                "attach_generation_advanced": generation_advanced,
                            }));
                            if reshown_ok {
                                effect_confirmed = true;
                            } else {
                                effect_ok = false;
                            }
                        }
                        observation = reshown;
                    }
                }
                Some(Err(error)) => {
                    record_obs_fail(
                        steps,
                        &mut failure,
                        "editor_visibility_show",
                        error.to_string(),
                    );
                    effect_ok = false;
                }
                None => {
                    record_obs_fail(
                        steps,
                        &mut failure,
                        "editor_visibility_show",
                        "editor session vanished before set_visibility(true)",
                    );
                    effect_ok = false;
                }
            }
        }
        Some(Err(error)) => {
            record_obs_fail(steps, &mut failure, "editor_visibility", error.to_string());
        }
        None => {
            record_obs_fail(
                steps,
                &mut failure,
                "editor_visibility",
                "editor session vanished before set_visibility",
            );
        }
    }

    // Convergence — close (production `close_gui` → `removed()` →
    // `DestroyWindow`), post-detach `IsWindow(child) == 0`, and the
    // visibility-specific event sequence: refused keeps
    // `[Attached{Docked}, Closed]` (no `Hidden`/`Shown` may appear);
    // accepted hide+show produces
    // `[Attached{Docked}, Hidden, Shown{Docked}, Closed]`; an
    // accepted hide whose show failed produces
    // `[Attached{Docked}, Hidden, Closed]`.
    let close_ok = close_editor_session(sessions, session_id, host, handle, steps, &mut failure);
    let detach_ok =
        !observation.supported || check_detached_children(&mut observation, steps, &mut failure);
    let expected: &[EditorSessionEvent] = if landing == OpLanding::Refused {
        ATTACH_CLOSE_EVENTS
    } else if shown {
        &[
            EditorSessionEvent::Attached {
                mode: AttachMode::Docked,
            },
            EditorSessionEvent::Hidden,
            EditorSessionEvent::Shown {
                mode: AttachMode::Docked,
            },
            EditorSessionEvent::Closed,
        ]
    } else {
        &[
            EditorSessionEvent::Attached {
                mode: AttachMode::Docked,
            },
            EditorSessionEvent::Hidden,
            EditorSessionEvent::Closed,
        ]
    };
    let (events_ok, rendered) =
        check_session_converged(sessions, session_id, steps, &mut failure, expected);
    EditorOpVerification {
        supported: observation.supported,
        landing,
        effect_confirmed,
        verified: attach_ok && state_ok && effect_ok && close_ok && detach_ok && events_ok,
        failure,
        extras: Value::Object(child_extras(container, &observation, &rendered)),
    }
}

/// `real-editor-visibility` body — the hide/show observation
/// through [`editor_op_body`] (normal docked open; the op attempts
/// live in the observation).
fn editor_visibility_body(
    gui: &mut RealOsGuiTest,
    window: WindowId,
    view: ViewInstanceId,
    plugin: Option<&PluginPair>,
    plugin_error: Option<&PluginUnavailable>,
) -> ScenarioOutcome {
    editor_op_body(
        gui,
        window,
        view,
        plugin,
        plugin_error,
        &EditorOpIdentity {
            pass_detail: "session.set_visibility(false→true) accepted through \
                          the REAL container lease; Hidden{Docked} destroyed \
                          every pre-hide child HWND (removed() → \
                          DestroyWindow) and Attached{Docked} re-established \
                          the child contract on a new attach generation, \
                          events [Attached{Docked}, Hidden, Shown{Docked}, \
                          Closed], and close convergence verified — no pixels",
            evidence: "editor-visibility:real-hide-show-cycle",
        },
        observe_editor_visibility_lifecycle,
    )
}

/// `real-editor-visibility` case: pre-flight requires
/// `NativePluginDock`, then the body drives the real attach chain
/// plus the `set_visibility` hide/show cycle — judged by the
/// child-HWND teardown/recreation oracles, session state, the
/// `Hidden`/`Shown` event pair, and close convergence, never pixels.
fn real_editor_visibility_case(
    ctx: &mut RealDockContext<'_>,
    spec: &ScenarioSpec,
) -> ScenarioReport {
    let window = ctx.window;
    let view = ctx.view;
    let plugin = ctx.plugin;
    let plugin_error = ctx.plugin_error;
    let artifact = ctx.artifact;
    let report = run_scenario(&mut *ctx.gui, spec, |gui| {
        Ok(editor_visibility_body(
            gui,
            window,
            view,
            plugin,
            plugin_error,
        ))
    });
    finalize_case(
        report,
        ctx,
        EnvironmentKind::RealWindow,
        artifact,
        Map::new(),
    )
}

/// `real-session-crash`: the fixture's crash fault modes
/// (`test_fault` abort/exit) kill the plugin cdylib — which shares
/// the host process in this lane — so the in-process real-window
/// lane cannot survive them, and no out-of-process plugin host
/// exists yet. `L1EditorOps` refuses every editor op with the
/// defined `Unsupported`. The honest verdict is `Unsupported`,
/// recorded inside the real session without touching GUI/session
/// state — never a fabricated `NotRun` (the catalog row is
/// executable; the boundary is architectural) and never a `Pass`.
fn real_session_crash_case(ctx: &mut RealDockContext<'_>, spec: &ScenarioSpec) -> ScenarioReport {
    let report = run_scenario(&mut *ctx.gui, spec, |_gui| {
        let mut outcome = ScenarioOutcome::new(ScenarioVerdict::Unsupported);
        outcome.detail = Some(
            "the in-process lane cannot survive the fixture's \
             process-killing crash fault (test_fault abort/exit — \
             the plugin cdylib shares the host process) and no \
             out-of-process plugin host exists yet; L1EditorOps \
             refuses every editor op with the defined Unsupported"
                .to_string(),
        );
        outcome
            .evidence
            .push("session:crash:in-process-lane-nonviable".to_string());
        Ok(outcome)
    });
    finalize_case(report, ctx, EnvironmentKind::RealWindow, None, Map::new())
}

/// `real-session-hang`: the fixture's hang fault mode
/// (`test_fault` hang) wedges the plugin call — which shares the
/// host process in this lane — so the in-process real-window lane
/// cannot survive it either, and no out-of-process plugin host
/// exists yet. `L1EditorOps` refuses every editor op with the
/// defined `Unsupported`. The honest verdict is `Unsupported`,
/// recorded inside the real session without touching GUI/session
/// state — never a fabricated `NotRun` and never a `Pass`.
fn real_session_hang_case(ctx: &mut RealDockContext<'_>, spec: &ScenarioSpec) -> ScenarioReport {
    let report = run_scenario(&mut *ctx.gui, spec, |_gui| {
        let mut outcome = ScenarioOutcome::new(ScenarioVerdict::Unsupported);
        outcome.detail = Some(
            "the in-process lane cannot survive the fixture's \
             wedging hang fault (test_fault hang — the plugin call \
             never returns and shares the host process) and no \
             out-of-process plugin host exists yet; L1EditorOps \
             refuses every editor op with the defined Unsupported"
                .to_string(),
        );
        outcome
            .evidence
            .push("session:hang:in-process-lane-nonviable".to_string());
        Ok(outcome)
    });
    finalize_case(report, ctx, EnvironmentKind::RealWindow, None, Map::new())
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
        assert_eq!(reports.len(), 22, "twenty-two real-dock cases");

        // The blocked path reads the same catalog rows — case ids
        // and required caps must match exactly.
        let entries: [ScenarioEntry<RealDockContext<'static>>; 22] = real_entries();
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
        let entries: [ScenarioEntry<RealDockContext<'static>>; 22] = real_entries();
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

    fn rect(left: i32, top: i32, width: u32, height: u32) -> ospix::ScreenRect {
        ospix::ScreenRect {
            left,
            top,
            width,
            height,
        }
    }

    /// A solid-color `width`×`height` capture — `rgba` filled with
    /// the given R,G,B (alpha 255).
    fn solid_capture(width: u32, height: u32, r: u8, g: u8, b: u8) -> ospix::PixelCapture {
        let mut rgba = vec![0u8; (width * height * 4) as usize];
        for px in rgba.as_chunks_mut::<4>().0 {
            px[0] = r;
            px[1] = g;
            px[2] = b;
            px[3] = 255;
        }
        ospix::PixelCapture {
            width,
            height,
            rgba,
            backend: "test",
        }
    }

    /// `rect_intersection` returns exactly the overlapping region —
    /// edge-touching and disjoint rectangles must not match, and the
    /// i64 intermediates must not panic on extreme coordinates.
    #[test]
    fn rect_intersection_overlap_and_edges() {
        let a = rect(0, 0, 10, 10);
        assert_eq!(
            rect_intersection(a, rect(5, 5, 10, 10)),
            Some(rect(5, 5, 5, 5))
        );
        assert_eq!(rect_intersection(a, rect(10, 0, 5, 5)), None, "edge touch");
        assert_eq!(rect_intersection(a, rect(20, 0, 5, 5)), None, "disjoint");
        assert_eq!(
            rect_intersection(rect(i32::MIN, i32::MIN, u32::MAX, u32::MAX), a),
            Some(a)
        );
    }

    /// `rect_contains` is inclusive on the far edge and rejects any
    /// pixel outside — the cover-candidate containment predicate.
    #[test]
    fn rect_contains_boundaries() {
        let outer = rect(0, 0, 10, 10);
        assert!(rect_contains(outer, rect(0, 0, 10, 10)));
        assert!(rect_contains(outer, rect(2, 2, 8, 8)));
        assert!(!rect_contains(outer, rect(0, 0, 11, 5)));
        assert!(!rect_contains(outer, rect(-1, 0, 5, 5)));
    }

    /// `color_bin` quantizes 5 bits per channel — values inside one
    /// bin share it, values across a bin edge do not.
    #[test]
    fn color_bin_quantizes() {
        assert_eq!(color_bin(0, 0, 0), 0);
        assert_eq!(color_bin(255, 255, 255), 0x7fff);
        assert_eq!(color_bin(8, 8, 8), color_bin(15, 15, 15));
        assert_ne!(color_bin(7, 0, 0), color_bin(8, 0, 0));
    }

    /// `fingerprint_match` compares only the rect overlap: identical
    /// solid rasters match at 1.0, different colors at 0.0, a
    /// capture whose dims disagree with its rect is skipped (never
    /// a fabricated fraction), and a partial overlap compares only
    /// the shared pixels.
    #[test]
    fn fingerprint_match_semantics() {
        let screen_rect = rect(0, 0, 4, 4);
        let same = solid_capture(4, 4, 10, 200, 10);
        assert_eq!(
            fingerprint_match(&same, screen_rect, &same, screen_rect),
            Some((1.0, 16))
        );
        let other = solid_capture(4, 4, 200, 10, 10);
        assert_eq!(
            fingerprint_match(&same, screen_rect, &other, screen_rect),
            Some((0.0, 16))
        );
        // Raster covering only the right half of the screen rect —
        // 8 overlap pixels, all matching.
        let half = solid_capture(2, 4, 10, 200, 10);
        assert_eq!(
            fingerprint_match(&same, screen_rect, &half, rect(2, 0, 2, 4)),
            Some((1.0, 8))
        );
        // Dims disagreeing with the declared rect → honest skip.
        let lying = ospix::PixelCapture {
            width: 2,
            height: 2,
            rgba: vec![0; 16],
            backend: "test",
        };
        assert_eq!(
            fingerprint_match(&lying, screen_rect, &same, screen_rect),
            None
        );
        assert_eq!(
            fingerprint_match(&same, screen_rect, &lying, rect(0, 0, 4, 4)),
            None
        );
        // No overlap → None.
        assert_eq!(
            fingerprint_match(&same, screen_rect, &same, rect(100, 100, 4, 4)),
            None
        );
    }

    /// `top_colors` ranks bins by pixel count and reports the
    /// quantized (bin-center) RGB values.
    #[test]
    fn top_colors_ranking() {
        let mut rgba = Vec::new();
        for _ in 0..3 {
            rgba.extend_from_slice(&[0, 0, 0, 255]);
        }
        for _ in 0..2 {
            rgba.extend_from_slice(&[255, 255, 255, 255]);
        }
        let colors = top_colors(&rgba, 2);
        assert_eq!(colors.len(), 2);
        assert_eq!(colors[0]["count"], 3);
        assert_eq!(colors[0]["r"], 0);
        assert_eq!(colors[1]["count"], 2);
        // Bin center of 255>>3=31 → 31<<3=248.
        assert_eq!(colors[1]["r"], 248);
    }
}
