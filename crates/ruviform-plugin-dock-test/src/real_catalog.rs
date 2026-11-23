//! Real-window dock lane catalog metadata (Issue #631 console
//! slice).
//!
//! Driver-neutral declaration data for the `rf631.plugin-dock-real`
//! scenario. The `real-dock` feature's *executable* entries live in
//! [`crate::real_dock`], but their identity — run/scenario/lane/
//! driver and per-case required capabilities — is GPUI-free data the
//! headless console needs in **every** build configuration:
//! `plugin-dock-console --list --include-real` rows and `--run`
//! real-selector refusal must work even when the lane itself is not
//! compiled in.
//!
//! Listing these rows never claims execution: the real lane runs
//! only through the opt-in `plugin-dock-real-scenario` binary inside
//! a `RealOsGuiTest` session (`session_required`), and console
//! `--run` refuses real selectors outright.
//!
//! **RT contract:** non-RT only — static data.

use ruviform_ui_test::{ScenarioCapability, ScenarioDriver};

/// The run id shared by every case of the real-window dock lane —
/// the same value `real_dock::RUN_ID` re-exports for the
/// feature-gated lane.
pub const RUN_ID: &str = "rf631-plugin-dock-real";

/// Scenario id — distinct from the headless lane's
/// `rf631.plugin-dock` so evidence never conflates the two drivers.
pub const SCENARIO: &str = "rf631.plugin-dock-real";

/// Lane label recorded in the real lane's spec extras and in
/// console `--list --include-real` rows — shared so the listing and
/// the reports' `environment.lane` can never drift apart.
pub const LANE: &str = "real-window-dock";

/// The driver every real-window dock case reports — real OS window
/// session evidence, never headless proof.
pub const DRIVER: ScenarioDriver = ScenarioDriver::RealOs;

/// The run id shared by every case of the CLAP real-window dock
/// lane (Issue #630 AC4 CLAP leg) — deliberately distinct from the
/// VST3 lane's [`RUN_ID`] so the two formats' evidence can never be
/// conflated.
pub const CLAP_RUN_ID: &str = "rf630-plugin-dock-real-clap";

/// CLAP lane scenario id — [`SCENARIO`]'s CLAP counterpart: a
/// separate run, a separate case-id family (`real-clap-*`), and a
/// separate JSONL ledger, never interleaved with the VST3 catalog.
pub const CLAP_SCENARIO: &str = "rf630.plugin-dock-real-clap";

/// One `rf631.plugin-dock-real` catalog row — metadata only. The
/// executable entries (run fn + spec extras) live behind the
/// `real-dock` feature in `real_dock::real_entries`; a feature-gated
/// test asserts the two tables agree on `case`/`scenario`/`required`.
#[derive(Debug)]
pub struct RealCatalogRow {
    /// Case id (`real-window-mount`, ...).
    pub case: &'static str,
    /// Scenario id — [`SCENARIO`].
    pub scenario: &'static str,
    /// Capabilities the case requires, in declaration order —
    /// identical to the matching `real_entries` entry's `required`.
    pub required: &'static [ScenarioCapability],
}

/// The thirty-two real-window dock cases in run order — metadata
/// only. The AC1 remainder rows (`real-multiwindow-focus`,
/// `real-os-input`, `real-window-dpi`, `real-window-ime`,
/// `real-window-close`) judge the remaining adopted capabilities
/// per case — executable where a real oracle exists, honestly
/// `Unsupported` where the adapter exposes no API. The catalog-flip
/// slice turned `real-window-dpi`/`real-window-close` into real
/// verdicts on the window-surface observation API
/// (`window_geometry`/`window_events`/`request_close`/
/// `respond_close`/`arm_close_response`); `real-window-ime` judges an
/// OS-level IMM32 composition+commit drive into the docked
/// `test_editor` child HWND through the caller-side subclass
/// `WM_IME_*` delivery oracle — `Ime` claimed on backend presence,
/// IME absent/disabled stays `NotRun` + `environment_blocked`
/// (Issue #389). The
/// command-receipt slice added `real-command-receipt`: the AC1
/// Command→Receipt link plus AC4 receipt population — a real
/// window hosts the command-minting fader fixture, semantic
/// dispatch mints receipt-expecting `Commit` intents, and
/// applied/rejected `result` events posted through `post_event`
/// settle the binding phases while the decoded `CommandReceipt`s
/// land in the report's `receipts`. The AC3 rows
/// (`real-editor-resize`, `real-editor-redock`,
/// `real-editor-visibility`, `real-editor-input`,
/// `real-editor-focus`) judge editor ops through non-pixel
/// oracles — session events, child-HWND contract/state, plus the
/// verbs' own OS-level effect oracles (child `GetWindowRect`
/// geometry for resize, hide→destroy/show→recreate for visibility,
/// attach-generation title advance for redock, and the fixture's
/// `GetWindowTextW` event-log title tokens for input/focus) —
/// never pixels. `real-editor-popup` (the Issue #630 AC4 shared
/// remainder) extends the same non-pixel family: `send_key` `VK_P`
/// toggles the fixture's popup twice and the verdict is judged by
/// the `po:`/`pc:` title event-log tokens plus a real top-level
/// `WS_POPUP` HWND oracle (own-process `RuviformTestEditor` class,
/// `rvf-test-editor-popup-{seq}` title, `GW_OWNER` == the child's
/// `GA_ROOT`, (0,0) 200x100 rect) and its close/teardown
/// disappearance.
/// The AC1/AC6 remainder rows
/// (`real-session-terminate` and the Issue #389 lifecycle
/// remainder: `real-session-degraded`, `real-session-recovery`,
/// `real-session-timeout`, `real-session-crash`, `real-session-hang`)
/// judge the session
/// lifecycle paths through session events, child-HWND, and session
/// state oracles only. Crash/hang (Issue #886) run the fixture on a
/// dedicated out-of-process L1 child (`OutOfProcessPluginHost` —
/// session-probed: runner + fixture resolvable and the child
/// advertises `GUI_FLOATING`), attach `test_editor` as a floating
/// child-owned toplevel, and judge `InstanceFailed` →
/// `Terminated{HostLost}` → `Closed` plus lane survival — a host
/// where the capability was not proven reports the capability-gated
/// `Unsupported`, never a fabricated verdict.
/// The Issue #874 rows (`real-session-rt-samethread`,
/// `real-session-rt-fault`, `real-session-rt-boundary`) are the
/// real-window RT non-interference oracle: the same-thread leg
/// interleaves `InProcessHost::process` blocks around the real
/// session verbs and judges `Continue` + bit-exact pass-through +
/// `Processing` maintained, and the negative-control leg proves
/// the observation path catches `test_fault`'s `rt_alloc` marker.
/// The boundary row (Issue #920, route (b)) proves the
/// cross-thread same-instance pump on the public API: a dedicated
/// L1 child hosts `test_editor`, `PluginHost::prepare_endpoint`
/// detaches its `RtLeg` into a `Send` `PluginEndpoint` a
/// `std::thread` pumps, and the UI thread concurrently runs the
/// same instance's `L1EditorOps` floating session verbs —
/// `host.process` answers the defined `EndpointUnavailable` while
/// checked out and resumes when the leg returns. The
/// `OutOfProcessPluginHost` claim gates it: a host that never
/// proved the route reports the capability-gated `Unsupported`.
pub const REAL_CATALOG: &[RealCatalogRow] = &[
    RealCatalogRow {
        case: "real-window-mount",
        scenario: SCENARIO,
        required: &[ScenarioCapability::SemanticAction],
    },
    RealCatalogRow {
        case: "real-native-slot",
        scenario: SCENARIO,
        required: &[ScenarioCapability::NativePluginDock],
    },
    RealCatalogRow {
        case: "real-editor-attach",
        scenario: SCENARIO,
        required: &[ScenarioCapability::NativePluginDock],
    },
    RealCatalogRow {
        case: "real-capture",
        scenario: SCENARIO,
        required: &[ScenarioCapability::VisualCapture],
    },
    RealCatalogRow {
        case: "real-dcomp-pixel",
        scenario: SCENARIO,
        required: &[
            ScenarioCapability::NativePluginDock,
            ScenarioCapability::VisualCapture,
        ],
    },
    RealCatalogRow {
        case: "real-dcomp-foreground",
        scenario: SCENARIO,
        required: &[
            ScenarioCapability::NativePluginDock,
            ScenarioCapability::VisualCapture,
        ],
    },
    RealCatalogRow {
        case: "real-dcomp-present",
        scenario: SCENARIO,
        required: &[
            ScenarioCapability::NativePluginDock,
            ScenarioCapability::VisualCapture,
        ],
    },
    RealCatalogRow {
        case: "real-dcomp-uncovered",
        scenario: SCENARIO,
        required: &[
            ScenarioCapability::NativePluginDock,
            ScenarioCapability::VisualCapture,
        ],
    },
    RealCatalogRow {
        case: "real-dcomp-bandescape",
        scenario: SCENARIO,
        required: &[
            ScenarioCapability::NativePluginDock,
            ScenarioCapability::VisualCapture,
        ],
    },
    RealCatalogRow {
        case: "real-dcomp-childinv",
        scenario: SCENARIO,
        required: &[
            ScenarioCapability::NativePluginDock,
            ScenarioCapability::VisualCapture,
        ],
    },
    RealCatalogRow {
        case: "real-dcomp-rasterfix",
        scenario: SCENARIO,
        required: &[
            ScenarioCapability::NativePluginDock,
            ScenarioCapability::VisualCapture,
        ],
    },
    RealCatalogRow {
        case: "real-multiwindow-focus",
        scenario: SCENARIO,
        required: &[ScenarioCapability::MultiWindow],
    },
    RealCatalogRow {
        case: "real-os-input",
        scenario: SCENARIO,
        required: &[ScenarioCapability::RealOsPointerOrKeyboard],
    },
    RealCatalogRow {
        case: "real-command-receipt",
        scenario: SCENARIO,
        required: &[ScenarioCapability::SemanticAction],
    },
    RealCatalogRow {
        case: "real-window-dpi",
        scenario: SCENARIO,
        required: &[ScenarioCapability::SemanticAction],
    },
    RealCatalogRow {
        case: "real-window-ime",
        scenario: SCENARIO,
        required: &[
            ScenarioCapability::NativePluginDock,
            ScenarioCapability::Ime,
        ],
    },
    RealCatalogRow {
        case: "real-window-close",
        scenario: SCENARIO,
        required: &[ScenarioCapability::MultiWindow],
    },
    RealCatalogRow {
        case: "real-editor-resize",
        scenario: SCENARIO,
        required: &[ScenarioCapability::NativePluginDock],
    },
    RealCatalogRow {
        case: "real-editor-redock",
        scenario: SCENARIO,
        required: &[ScenarioCapability::NativePluginDock],
    },
    RealCatalogRow {
        case: "real-session-terminate",
        scenario: SCENARIO,
        required: &[ScenarioCapability::NativePluginDock],
    },
    RealCatalogRow {
        case: "real-session-degraded",
        scenario: SCENARIO,
        required: &[ScenarioCapability::NativePluginDock],
    },
    RealCatalogRow {
        case: "real-session-recovery",
        scenario: SCENARIO,
        required: &[ScenarioCapability::NativePluginDock],
    },
    RealCatalogRow {
        case: "real-session-timeout",
        scenario: SCENARIO,
        required: &[ScenarioCapability::NativePluginDock],
    },
    RealCatalogRow {
        case: "real-editor-visibility",
        scenario: SCENARIO,
        required: &[ScenarioCapability::NativePluginDock],
    },
    RealCatalogRow {
        case: "real-editor-input",
        scenario: SCENARIO,
        required: &[ScenarioCapability::NativePluginDock],
    },
    RealCatalogRow {
        case: "real-editor-focus",
        scenario: SCENARIO,
        required: &[ScenarioCapability::NativePluginDock],
    },
    RealCatalogRow {
        case: "real-editor-popup",
        scenario: SCENARIO,
        required: &[ScenarioCapability::NativePluginDock],
    },
    RealCatalogRow {
        case: "real-session-crash",
        scenario: SCENARIO,
        required: &[
            ScenarioCapability::NativePluginDock,
            ScenarioCapability::OutOfProcessPluginHost,
        ],
    },
    RealCatalogRow {
        case: "real-session-hang",
        scenario: SCENARIO,
        required: &[
            ScenarioCapability::NativePluginDock,
            ScenarioCapability::OutOfProcessPluginHost,
        ],
    },
    RealCatalogRow {
        case: "real-session-rt-samethread",
        scenario: SCENARIO,
        required: &[ScenarioCapability::NativePluginDock],
    },
    RealCatalogRow {
        case: "real-session-rt-fault",
        scenario: SCENARIO,
        required: &[ScenarioCapability::NativePluginDock],
    },
    RealCatalogRow {
        case: "real-session-rt-boundary",
        scenario: SCENARIO,
        required: &[
            ScenarioCapability::NativePluginDock,
            ScenarioCapability::OutOfProcessPluginHost,
        ],
    },
];

/// The fourteen CLAP real-window dock cases (Issue #630 AC4 CLAP
/// leg) in run order — metadata only, mirroring
/// [`real_dock::real_clap_entries`]. The CLAP lane loads
/// `ruviform_plugin_test_clap.dll` and runs the same
/// session/child-HWND oracle family as the VST3 catalog, but every
/// format-specific expectation is the CLAP fixture's own:
///
/// - pixels: the CLAP `test_editor` paints blue
///   (`EDITOR_COLOR = 0x00FF0000`), not VST3 green;
/// - visibility: CLAP `hide`/`show` flips `IsWindowVisible` on the
///   live child — it does not destroy/recreate the way VST3's
///   `removed()`/`attached` cycle does;
/// - re-dock: the CLAP embedded-GUI contract is single-embed —
///   in-place `reattach` lands the fixture's defined
///   `gui already embedded` plugin refusal
///   (`Failed{PluginFailed}`), and the format's re-open path is
///   close + re-open on a fresh session (a new child generation);
/// - input/focus/popup: `send_gui_key`/`set_gui_focus` are the
///   trait-default `Err(Unsupported)` on `ClapPlugin` and embedded
///   `open_gui` never reaches floating — these cases run the verbs
///   through the real lease and record the defined `Unsupported`
///   landing honestly (Coordinator decision 1);
/// - crash/hang: the CLAP `test_editor` is parameterless (no
///   in-band Fault arm) and `PluginHost` routes are keyed by
///   `plugin_id` with per-route Bundled pools, so a co-resident
///   `test_fault` kill arm cannot exist — the cases record
///   `Unsupported` with that structural ground (Coordinator
///   decision 2, option (c)).
///
/// Session lifecycle (`terminate`/`degraded`/`recovery`/`timeout`)
/// uses the same format-agnostic session drives and oracles as the
/// VST3 rows — the verbs and expectations do not differ by format.
pub const REAL_CLAP_CATALOG: &[RealCatalogRow] = &[
    RealCatalogRow {
        case: "real-clap-editor-attach",
        scenario: CLAP_SCENARIO,
        required: &[ScenarioCapability::NativePluginDock],
    },
    RealCatalogRow {
        case: "real-clap-dcomp-pixel",
        scenario: CLAP_SCENARIO,
        required: &[
            ScenarioCapability::NativePluginDock,
            ScenarioCapability::VisualCapture,
        ],
    },
    RealCatalogRow {
        case: "real-clap-editor-resize",
        scenario: CLAP_SCENARIO,
        required: &[ScenarioCapability::NativePluginDock],
    },
    RealCatalogRow {
        case: "real-clap-editor-redock",
        scenario: CLAP_SCENARIO,
        required: &[ScenarioCapability::NativePluginDock],
    },
    RealCatalogRow {
        case: "real-clap-editor-visibility",
        scenario: CLAP_SCENARIO,
        required: &[ScenarioCapability::NativePluginDock],
    },
    RealCatalogRow {
        case: "real-clap-editor-input",
        scenario: CLAP_SCENARIO,
        required: &[ScenarioCapability::NativePluginDock],
    },
    RealCatalogRow {
        case: "real-clap-editor-focus",
        scenario: CLAP_SCENARIO,
        required: &[ScenarioCapability::NativePluginDock],
    },
    RealCatalogRow {
        case: "real-clap-editor-popup",
        scenario: CLAP_SCENARIO,
        required: &[ScenarioCapability::NativePluginDock],
    },
    RealCatalogRow {
        case: "real-clap-session-terminate",
        scenario: CLAP_SCENARIO,
        required: &[ScenarioCapability::NativePluginDock],
    },
    RealCatalogRow {
        case: "real-clap-session-degraded",
        scenario: CLAP_SCENARIO,
        required: &[ScenarioCapability::NativePluginDock],
    },
    RealCatalogRow {
        case: "real-clap-session-recovery",
        scenario: CLAP_SCENARIO,
        required: &[ScenarioCapability::NativePluginDock],
    },
    RealCatalogRow {
        case: "real-clap-session-timeout",
        scenario: CLAP_SCENARIO,
        required: &[ScenarioCapability::NativePluginDock],
    },
    RealCatalogRow {
        case: "real-clap-session-crash",
        scenario: CLAP_SCENARIO,
        required: &[ScenarioCapability::NativePluginDock],
    },
    RealCatalogRow {
        case: "real-clap-session-hang",
        scenario: CLAP_SCENARIO,
        required: &[ScenarioCapability::NativePluginDock],
    },
];
