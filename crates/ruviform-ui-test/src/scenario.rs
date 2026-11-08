//! Consumer-neutral GUI Scenario Contract (Issue #464).
//!
//! A thin, driver-neutral layer that lets a consumer write one
//! ordinary-Rust scenario and run it against any [`SemanticGuiTest`]
//! adapter — the [`fake`] `ViewHost` adapter here, or the GPUI
//! adapter in `ruviform-ui-gpui` (`gpui-test` feature). There is no
//! JSON DSL, no second runner, and no new gesture/state authority:
//! every operation reuses [`ControlInput`], [`ViewHost`],
//! [`ComponentSnapshot`], [`CommandIntent`]/[`CommandReceipt`], and
//! the `rvf.ipc.v1` [`Event`] envelope from `ruviform-ui`.
//!
//! Targets are logical triples — [`WindowId`] + [`ViewInstanceId`] +
//! [`ComponentKey`] — resolved dynamically and pinned with
//! [`AttachEpoch`] + `generation` at resolve time. Every operation
//! and observation re-checks the pins, so a view that moved,
//! regenerated, or was recreated surfaces an explicit
//! [`ScenarioError`] instead of silently acting on a dead handle.
//! Display text and absolute pixel coordinates are never identity.
//!
//! [`SemanticGuiTest::capabilities`] declares which evidence levels
//! an adapter can honestly produce. A Fake `pass` is semantic-action
//! evidence only — it is never promoted to simulated-input, real-OS,
//! draw-plan, or pixel evidence. `GpuiObserver::dispatch_input`
//! bypasses hit-testing and is likewise not pointer proof.
//!
//! **RT contract:** non-RT only.

use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};

use ruviform_ui::behavior::{
    CancelReason, CommandId, CommandIntent, CommandReceipt, ControlInput, InputSource, ValueInput,
};
use ruviform_ui::dock::ViewInstanceId;
use ruviform_ui::ipc::Event;
use ruviform_ui::view::{AttachEpoch, ComponentKey, ComponentSnapshot, ViewError};
use ruviform_ui::{UiError, WindowId};
use serde::{Deserialize, Serialize};

pub mod fake;

// ---------------------------------------------------------------------
// identifiers
// ---------------------------------------------------------------------

/// Newtype id macro-free helper: scenario-side string ids are
/// consumer-namespaced plain strings; the newtypes only give the
/// report fields distinct types.
macro_rules! string_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub String);

        impl $name {
            /// Wraps a raw string as the id.
            #[must_use]
            pub fn new(raw: impl Into<String>) -> Self {
                Self(raw.into())
            }

            /// The raw id string.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl From<&str> for $name {
            fn from(s: &str) -> Self {
                Self(s.to_string())
            }
        }

        impl From<String> for $name {
            fn from(s: String) -> Self {
                Self(s)
            }
        }

        impl Display for $name {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

string_id! {
    /// Identifies one scenario definition.
    ScenarioId
}

string_id! {
    /// Identifies one case inside a scenario.
    CaseId
}

string_id! {
    /// Identifies one execution run of a scenario/case pair.
    RunId
}

// ---------------------------------------------------------------------
// target
// ---------------------------------------------------------------------

/// Logical target query — the consumer-facing handle.
///
/// `window` + `view` are required: a [`ComponentKey`] alone cannot
/// identify a view once views move or regenerate. `component`
/// optionally narrows the query to one mounted component.
/// `attach_epoch`/`generation` are optional staleness pins checked at
/// resolve time — pass the values a previous [`ScenarioTarget`]
/// reported to prove the view has not been recreated since.
#[derive(Debug, Clone, PartialEq)]
pub struct TargetQuery {
    /// Window hosting the view.
    pub window: WindowId,
    /// Logical view instance id.
    pub view: ViewInstanceId,
    /// Optional component narrowing.
    pub component: Option<ComponentKey>,
    /// Optional expected attach epoch.
    pub attach_epoch: Option<AttachEpoch>,
    /// Optional expected generation.
    pub generation: Option<u64>,
}

impl TargetQuery {
    /// A query for `view` mounted in `window`, unpinned.
    #[must_use]
    pub fn new(window: WindowId, view: ViewInstanceId) -> Self {
        Self {
            window,
            view,
            component: None,
            attach_epoch: None,
            generation: None,
        }
    }

    /// Narrows the query to one component.
    #[must_use]
    pub fn component(mut self, key: ComponentKey) -> Self {
        self.component = Some(key);
        self
    }

    /// Pins the expected attach epoch.
    #[must_use]
    pub fn attach_epoch(mut self, epoch: AttachEpoch) -> Self {
        self.attach_epoch = Some(epoch);
        self
    }

    /// Pins the expected generation.
    #[must_use]
    pub fn generation(mut self, generation: u64) -> Self {
        self.generation = Some(generation);
        self
    }
}

/// A resolved, pinned target — the value every operation re-checks.
///
/// `attach_epoch` and `generation` were live when [`SemanticGuiTest::
/// resolve_target`] returned; each subsequent call compares them
/// against the adapter's current state and fails with
/// [`ScenarioError::StaleEpoch`]/[`ScenarioError::StaleGeneration`]
/// when the view was recreated or regenerated in between.
#[derive(Debug, Clone, PartialEq)]
pub struct ScenarioTarget {
    /// Window hosting the view.
    pub window: WindowId,
    /// Logical view instance id.
    pub view: ViewInstanceId,
    /// Pinned runtime-instance epoch.
    pub attach_epoch: AttachEpoch,
    /// Pinned host generation.
    pub generation: u64,
    /// Optional component narrowing carried from the query.
    pub component: Option<ComponentKey>,
}

// ---------------------------------------------------------------------
// capability / driver / verdict / error
// ---------------------------------------------------------------------

/// Evidence-level capability an adapter may honestly claim.
///
/// The variants are distinct on purpose: a pass under `SemanticAction`
/// says nothing about `SimulatedPointerOrKeyboard`, `RealOsPointerOrKeyboard`,
/// `Ime`, `VisualCapture`, `MultiWindow`, or `NativePluginDock`. Fake
/// and GPUI-simulated adapters never claim the stronger levels.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ScenarioCapability {
    /// Semantic `ControlInput` dispatch, select/invoke, observe,
    /// settle, and intent/receipt flow — every adapter provides this.
    SemanticAction,
    /// Simulated pointer/keyboard injection at the toolkit level.
    /// `GpuiObserver::dispatch_input` is *not* this: it bypasses
    /// hit-testing, so it is semantic dispatch, not pointer proof.
    SimulatedPointerOrKeyboard,
    /// Real OS pointer/keyboard injection (Issue #313 scope).
    RealOsPointerOrKeyboard,
    /// IME composition input.
    Ime,
    /// Pixel/screenshot capture (Issue #314 scope).
    VisualCapture,
    /// Multiple live windows addressable in one scenario.
    MultiWindow,
    /// Native plugin dock surface (Issues #388/#389 + consumer side).
    NativePluginDock,
}

/// Which adapter produced a [`ScenarioReport`]. The value is the
/// evidence level — `GpuiSimulated` is toolkit-simulated input, never
/// real-OS proof.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ScenarioDriver {
    /// `ViewHost`-backed fake adapter in `ruviform-ui-test`.
    Fake,
    /// `GpuiObserver`-backed simulated adapter in `ruviform-ui-gpui`
    /// (`gpui-test` feature).
    GpuiSimulated,
    /// Real OS input driver (Issue #313 scope).
    RealOs,
}

/// Scenario verdict — the same five-value vocabulary as the #390
/// driver-lab manifest (`pass`/`fail`/`not_run`/`unsupported`/
/// `blocked`). Consumer-verify lanes map `blocked` to `not_run` with
/// a reason, unchanged.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScenarioVerdict {
    /// Every check held.
    Pass,
    /// A check failed; `detail`/`observed` carry the evidence.
    Fail,
    /// The scenario was not executed (prerequisite absent).
    NotRun,
    /// A required capability is not provided by the adapter.
    Unsupported,
    /// Execution was blocked by an external prerequisite.
    Blocked,
}

impl ScenarioVerdict {
    /// The manifest vocabulary spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Fail => "fail",
            Self::NotRun => "not_run",
            Self::Unsupported => "unsupported",
            Self::Blocked => "blocked",
        }
    }
}

/// Errors returned by [`SemanticGuiTest`] operations.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum ScenarioError {
    /// No window with the queried id is known to the adapter.
    NoSuchWindow {
        /// The missing window.
        window: WindowId,
    },
    /// The window exists but hosts no view with the queried id.
    NoSuchView {
        /// The window that was searched.
        window: WindowId,
        /// The missing view instance id.
        view: ViewInstanceId,
    },
    /// The pinned attach epoch no longer matches the live host — the
    /// view was recreated since the target was resolved.
    StaleEpoch {
        /// Epoch pinned at resolve time.
        expected: AttachEpoch,
        /// Epoch the live host reports now.
        got: AttachEpoch,
    },
    /// The pinned generation no longer matches the live host — the
    /// view spec changed (mount/update/detach) since resolution.
    StaleGeneration {
        /// Generation pinned at resolve time.
        expected: u64,
        /// Generation the live host reports now.
        got: u64,
    },
    /// The component key is not mounted on the resolved view.
    UnknownComponent {
        /// The unmounted key.
        key: ComponentKey,
    },
    /// A `ruviform-ui` view-layer error (stale generation inside the
    /// host, invalid input, bad spec, ...).
    View(ViewError),
    /// The adapter/driver refused the operation.
    Driver(String),
    /// A required capability is not provided by this adapter.
    Unsupported {
        /// The first missing capability.
        capability: ScenarioCapability,
    },
}

impl Display for ScenarioError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoSuchWindow { window } => write!(f, "no such window {window}"),
            Self::NoSuchView { window, view } => {
                write!(f, "no view {view} in window {window}")
            }
            Self::StaleEpoch { expected, got } => {
                write!(f, "stale attach epoch: pinned {expected}, live {got}")
            }
            Self::StaleGeneration { expected, got } => {
                write!(f, "stale generation: pinned {expected}, live {got}")
            }
            Self::UnknownComponent { key } => write!(f, "unknown component {key}"),
            Self::View(e) => write!(f, "view error: {e}"),
            Self::Driver(m) => write!(f, "driver error: {m}"),
            Self::Unsupported { capability } => {
                write!(f, "unsupported capability: {capability:?}")
            }
        }
    }
}

impl std::error::Error for ScenarioError {}

impl From<ViewError> for ScenarioError {
    fn from(e: ViewError) -> Self {
        Self::View(e)
    }
}

impl From<UiError> for ScenarioError {
    fn from(e: UiError) -> Self {
        Self::Driver(e.to_string())
    }
}

// ---------------------------------------------------------------------
// action / minted intent / report
// ---------------------------------------------------------------------

/// One semantic operation a scenario can perform on a target.
///
/// `Input` carries an existing [`ControlInput`] verbatim — including
/// `Begin`/`Preview`/`Commit`/`Cancel`/`Set`/`Adjust`/`Reset`/
/// `FocusLost`/`Detach`. The convenience constructors below only
/// fill the `ControlInput` fields; they do not introduce a second
/// gesture or state authority.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum GuiAction {
    /// Dispatch a [`ControlInput`] to `key`.
    Input {
        /// Component key the input targets.
        key: ComponentKey,
        /// The control input.
        input: ControlInput,
    },
    /// Select `row` of the selectable `Table` `key`.
    SelectRow {
        /// Table component key.
        key: ComponentKey,
        /// Row index.
        row: usize,
    },
    /// Select tab `index` of the `Tabs` `key`.
    SelectTab {
        /// Tabs component key.
        key: ComponentKey,
        /// Tab index.
        index: usize,
    },
    /// Press the `Button` `key`.
    Invoke {
        /// Button component key.
        key: ComponentKey,
    },
}

/// A command intent minted by the view under test, drained through
/// [`SemanticGuiTest::drain_intents`]. Carries the binding-minted
/// `command_id` so the consumer can correlate the later
/// [`CommandReceipt`] it posts back through
/// [`SemanticGuiTest::post_event`].
#[derive(Debug, Clone, PartialEq)]
pub struct MintedIntent {
    /// Binding-minted command id.
    pub command_id: CommandId,
    /// Whether the binding expects a receipt for this command.
    pub expect_receipt: bool,
    /// The intent itself.
    pub intent: CommandIntent,
}

/// Execution record for one scenario/case run.
///
/// Every field is evidence, not control flow: the report never
/// upgrades a weaker adapter's result into a stronger claim. A Fake
/// `pass` recorded here stays `driver: Fake` + `capabilities:
/// {semantic-action}` — it is not simulated-input, real-OS,
/// draw-plan, or pixel evidence.
#[derive(Debug, Clone)]
pub struct ScenarioReport {
    /// Execution run id.
    pub run_id: RunId,
    /// Scenario definition id.
    pub scenario: ScenarioId,
    /// Case id inside the scenario, when the run is case-scoped.
    pub case: Option<CaseId>,
    /// Adapter that executed the scenario.
    pub driver: ScenarioDriver,
    /// Capabilities the adapter actually provided.
    pub capabilities: BTreeSet<ScenarioCapability>,
    /// Capabilities the scenario required (pre-flight checked).
    pub required: BTreeSet<ScenarioCapability>,
    /// Final verdict.
    pub verdict: ScenarioVerdict,
    /// Resolved targets the scenario touched.
    pub targets: Vec<ScenarioTarget>,
    /// What the scenario expected (consumer-defined JSON).
    pub expected: Option<serde_json::Value>,
    /// What was actually observed (consumer-defined JSON).
    pub observed: Option<serde_json::Value>,
    /// Receipts the consumer posted back and the view accepted.
    pub receipts: Vec<CommandReceipt>,
    /// Evidence references (fixture paths, journal ids, ...).
    pub evidence: Vec<String>,
    /// Environment facts (tested SHAs, OS, theme, ...).
    pub environment: Option<serde_json::Value>,
    /// Human-readable detail for non-pass verdicts.
    pub detail: Option<String>,
}

impl ScenarioReport {
    /// A minimal report skeleton for `scenario`/`case` on `driver`.
    #[must_use]
    pub fn new(
        run_id: RunId,
        scenario: ScenarioId,
        case: Option<CaseId>,
        driver: ScenarioDriver,
    ) -> Self {
        Self {
            run_id,
            scenario,
            case,
            driver,
            capabilities: BTreeSet::new(),
            required: BTreeSet::new(),
            verdict: ScenarioVerdict::NotRun,
            targets: Vec::new(),
            expected: None,
            observed: None,
            receipts: Vec::new(),
            evidence: Vec::new(),
            environment: None,
            detail: None,
        }
    }
}

// ---------------------------------------------------------------------
// gesture helpers — thin `ControlInput` constructors only
// ---------------------------------------------------------------------

/// `ControlInput::Begin` for `gesture_id` with programmatic source —
/// semantic scenario input, never pointer evidence.
#[must_use]
pub fn begin(gesture_id: &str) -> ControlInput {
    ControlInput::Begin {
        gesture_id: gesture_id.to_string(),
        source: InputSource::from(InputSource::PROGRAMMATIC),
        origin_view: None,
        target: None,
        at: None,
        modifiers: None,
        seq: None,
    }
}

/// `ControlInput::Preview` carrying `value` for `gesture_id`.
#[must_use]
pub fn preview(gesture_id: &str, value: ValueInput) -> ControlInput {
    ControlInput::Preview {
        gesture_id: gesture_id.to_string(),
        value,
        modifiers: None,
        seq: None,
    }
}

/// `ControlInput::Commit` for `gesture_id`.
#[must_use]
pub fn commit(gesture_id: &str) -> ControlInput {
    ControlInput::Commit {
        gesture_id: gesture_id.to_string(),
        value: None,
        modifiers: None,
        seq: None,
    }
}

/// `ControlInput::Cancel` for `gesture_id`.
#[must_use]
pub fn cancel(gesture_id: &str) -> ControlInput {
    ControlInput::Cancel {
        gesture_id: gesture_id.to_string(),
        reason: Some(CancelReason::from(CancelReason::EXPLICIT)),
        seq: None,
    }
}

// ---------------------------------------------------------------------
// the port
// ---------------------------------------------------------------------

/// The thin semantic port every scenario adapter implements.
///
/// All types in the signature are `ruviform-ui` contract types — no
/// adapter type (GPUI `Window`, `TestAppContext`, ...) ever appears
/// here. Implementations stay on the caller's thread: GPUI `!Send`
/// contexts are never moved across threads.
///
/// Observation is read-only: [`observe`](Self::observe) and
/// [`drain_intents`](Self::drain_intents) never mutate product state
/// or bypass authorization. [`post_event`](Self::post_event) is the
/// consumer→view direction for receipts/projections — it goes through
/// the same [`ViewHost::deliver_event`] decode every driver uses.
pub trait SemanticGuiTest {
    /// Which adapter is executing — the evidence level for reports.
    fn driver(&self) -> ScenarioDriver;

    /// Capabilities this adapter actually provides right now.
    fn capabilities(&self) -> BTreeSet<ScenarioCapability>;

    /// Windows currently known to the adapter.
    fn windows(&self) -> Vec<WindowId>;

    /// Resolves a [`TargetQuery`] to a pinned [`ScenarioTarget`].
    ///
    /// Fails with [`ScenarioError::NoSuchWindow`]/
    /// [`ScenarioError::NoSuchView`] when the triple does not exist,
    /// [`ScenarioError::UnknownComponent`] when `component` is not
    /// mounted, and [`ScenarioError::StaleEpoch`] /
    /// [`ScenarioError::StaleGeneration`] when the query's optional
    /// pins no longer match.
    fn resolve_target(&mut self, query: &TargetQuery) -> Result<ScenarioTarget, ScenarioError>;

    /// Dispatches `input` to `key` on `target` after re-checking the
    /// target's `attach_epoch`/`generation` pins.
    fn dispatch_input(
        &mut self,
        target: &ScenarioTarget,
        key: &ComponentKey,
        input: ControlInput,
    ) -> Result<(), ScenarioError>;

    /// Selects `row` of the table `key` on `target`.
    fn select_row(
        &mut self,
        target: &ScenarioTarget,
        key: &ComponentKey,
        row: usize,
    ) -> Result<(), ScenarioError>;

    /// Selects tab `index` of the tab strip `key` on `target`.
    fn select_tab(
        &mut self,
        target: &ScenarioTarget,
        key: &ComponentKey,
        index: usize,
    ) -> Result<(), ScenarioError>;

    /// Presses the button `key` on `target`.
    fn invoke(&mut self, target: &ScenarioTarget, key: &ComponentKey) -> Result<(), ScenarioError>;

    /// Read-only snapshot of `key` on `target`. Never mutates state.
    fn observe(
        &self,
        target: &ScenarioTarget,
        key: &ComponentKey,
    ) -> Result<ComponentSnapshot, ScenarioError>;

    /// One bounded settle step — flush queued work/events so intents
    /// and projections land. Never sleeps unconditionally.
    fn settle(&mut self) -> Result<(), ScenarioError>;

    /// Drains command intents minted since the previous drain.
    fn drain_intents(&mut self) -> Vec<MintedIntent>;

    /// Delivers a wire [`Event`] (receipt `Result` or state `Notify`)
    /// toward `target`'s view — the consumer→view direction. The event
    /// is queued like any driver `post_event`; [`settle`](Self::settle)
    /// is the flush point on asynchronous adapters.
    fn post_event(&mut self, target: &ScenarioTarget, event: &Event) -> Result<(), ScenarioError>;

    /// Pre-flight capability check — the scenario's required set must
    /// be a subset of [`capabilities`](Self::capabilities).
    ///
    /// # Errors
    /// [`ScenarioError::Unsupported`] naming the first missing
    /// capability (ordered by [`ScenarioCapability`]'s `Ord`).
    fn require_capabilities(
        &self,
        required: &BTreeSet<ScenarioCapability>,
    ) -> Result<(), ScenarioError> {
        let supported = self.capabilities();
        if let Some(&capability) = required.difference(&supported).next() {
            return Err(ScenarioError::Unsupported { capability });
        }
        Ok(())
    }

    /// Applies one [`GuiAction`] on `target`.
    ///
    /// # Errors
    /// Propagates the underlying operation's [`ScenarioError`].
    fn perform(&mut self, target: &ScenarioTarget, action: GuiAction) -> Result<(), ScenarioError> {
        match action {
            GuiAction::Input { key, input } => self.dispatch_input(target, &key, input),
            GuiAction::SelectRow { key, row } => self.select_row(target, &key, row),
            GuiAction::SelectTab { key, index } => self.select_tab(target, &key, index),
            GuiAction::Invoke { key } => self.invoke(target, &key),
        }
    }
}

/// Shared Fake/GPUI parity scenario — one ordinary Rust function every
/// [`SemanticGuiTest`] adapter must satisfy (Issue #464 acceptance).
///
/// Runs a `Begin → Preview → Commit` gesture on `key` with a bounded
/// settle, drains the minted intents, and returns them with the final
/// [`ComponentSnapshot`]. The caller asserts on the returned evidence
/// identically for Fake and GPUI — the scenario itself stays
/// adapter-agnostic.
///
/// # Errors
/// Propagates the first [`ScenarioError`] from resolve-time checks or
/// any operation.
pub fn parity_gesture_scenario(
    gui: &mut impl SemanticGuiTest,
    target: &ScenarioTarget,
    key: &ComponentKey,
    gesture_id: &str,
    value: ValueInput,
) -> Result<(Vec<MintedIntent>, ComponentSnapshot), ScenarioError> {
    gui.dispatch_input(target, key, begin(gesture_id))?;
    gui.dispatch_input(target, key, preview(gesture_id, value))?;
    gui.dispatch_input(target, key, commit(gesture_id))?;
    gui.settle()?;
    let intents = gui.drain_intents();
    let snapshot = gui.observe(target, key)?;
    Ok((intents, snapshot))
}
