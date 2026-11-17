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

/// The twenty-two real-window dock cases in run order — metadata
/// only. The AC1 remainder rows (`real-multiwindow-focus`,
/// `real-os-input`, `real-window-dpi`, `real-window-ime`,
/// `real-window-close`) judge the remaining adopted capabilities
/// per case — executable where a real oracle exists, honestly
/// `Unsupported` where the adapter exposes no API. The catalog-flip
/// slice turned `real-window-dpi`/`real-window-close` into real
/// verdicts on the window-surface observation API
/// (`window_geometry`/`window_events`/`request_close`/
/// `respond_close`/`arm_close_response`); `real-window-ime` stays
/// `Unsupported` (`Ime` unclaimed, Issue #389 scope). The
/// command-receipt slice added `real-command-receipt`: the AC1
/// Command→Receipt link plus AC4 receipt population — a real
/// window hosts the command-minting fader fixture, semantic
/// dispatch mints receipt-expecting `Commit` intents, and
/// applied/rejected `result` events posted through `post_event`
/// settle the binding phases while the decoded `CommandReceipt`s
/// land in the report's `receipts`. The AC3 rows
/// (`real-editor-resize`, `real-editor-redock`) judge editor ops
/// through non-pixel oracles (session events, child-HWND contract,
/// session state) only. The AC1/AC6 remainder rows
/// (`real-session-terminate` and the Issue #389 lifecycle
/// remainder: `real-session-degraded`, `real-session-recovery`,
/// `real-session-timeout`, `real-editor-visibility`,
/// `real-session-crash`, `real-session-hang`) judge the session
/// lifecycle paths through session events, child-HWND, and session
/// state oracles only — crash/hang stay honest `Unsupported`
/// boundary rows because the in-process lane cannot survive the
/// fixture's process-killing/wedging fault modes.
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
        required: &[ScenarioCapability::Ime],
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
        case: "real-session-crash",
        scenario: SCENARIO,
        required: &[ScenarioCapability::NativePluginDock],
    },
    RealCatalogRow {
        case: "real-session-hang",
        scenario: SCENARIO,
        required: &[ScenarioCapability::NativePluginDock],
    },
];
