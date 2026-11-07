//! Driver-neutral Component/View spec contract + headless projection
//! (Issue #387).
//!
//! [`ViewSpec`]/[`ComponentSpec`] are pure data: they carry no GPUI,
//! DOM, OS, or toolkit types. [`ViewHost`] is the authoritative
//! headless projection engine — it reconciles spec trees by
//! [`ComponentKey`], owns the [`crate::behavior`] dispatchers and
//! per-control [`crate::behavior::CommandBinding`]s, and emits typed
//! [`ProjectionOp`]/[`ViewEvent`] records. Renderers (GPUI, fake,
//! future backends) translate specs to elements and normalize input
//! into [`crate::behavior::ControlInput`] — the semantic rules live
//! here once.
//!
//! **RT contract:** non-RT only.

use std::collections::{HashMap, VecDeque};
use std::fmt::{Display, Formatter};
use std::ops::Range;

use serde::{Deserialize, Serialize};

use crate::behavior::{
    AvailabilityFlags, CommandBinding, CommandId, CommandIntent, CommandPhase, CommandReceipt,
    ControlId, ControlInput, DetachPolicy, InteractionControlOptions, InteractionDispatcher,
    InteractionEvent, LogicalTarget, ReceiptStatus, StateProjection, ValueDescriptor, ValuePorts,
    intent_from_gesture, validate_descriptor,
};
use crate::dock::ViewInstanceId;

/// Stable component identifier within a [`ViewSpec`] tree.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ComponentKey(pub String);

impl From<&str> for ComponentKey {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl Display for ComponentKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Layout orientation for a control or container.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Orientation {
    /// Left-to-right.
    Horizontal,
    /// Top-to-bottom.
    Vertical,
}

/// Backend-neutral accessibility role. Renderers map these to their
/// platform a11y model; the set is intentionally minimal for the
/// first vertical slices.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum A11yRole {
    /// Continuous value control (Fader/Seek).
    Slider,
    /// Tab strip container.
    TabList,
    /// Single tab.
    Tab,
    /// Grouping container (Panel).
    Group,
    /// Non-interactive visualization (Waveform/Meter/TimeAxis/Spectrogram).
    Img,
    /// Binary switch (Toggle).
    Switch,
    /// Static text (Label).
    Text,
    /// Table grid (Table).
    Table,
    /// Table row (Table header and data rows).
    Row,
    /// Table column header.
    ColumnHeader,
    /// Table data cell. A missing cell has no value but keeps its position.
    Cell,
}

/// Accessibility semantics carried by a spec. `value_min`/`value_max`/
/// `value_text` describe the *range*; the live value/text comes from
/// the host's display state at render time.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct A11ySpec {
    /// Component role.
    pub role: Option<A11yRole>,
    /// Accessible label.
    pub label: Option<String>,
    /// Orientation override for a11y.
    pub orientation: Option<Orientation>,
    /// Value minimum for `Slider`-like roles.
    pub value_min: Option<f64>,
    /// Value maximum for `Slider`-like roles.
    pub value_max: Option<f64>,
    /// Static value text override.
    pub value_text: Option<String>,
}

/// Visual state that does not alter behavior semantics. `read_only`/
/// `disabled`/`constant` live on [`ValueDescriptor`] as the
/// availability authority — this struct only carries the renderable
/// error state.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ComponentVisualState {
    /// Error message shown and exposed to a11y (`invalid` semantics).
    /// Does not block gestures.
    pub error: Option<String>,
}

/// Grouping container.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PanelSpec {
    /// Component key.
    pub key: ComponentKey,
    /// Optional title shown on the panel chrome.
    pub title: Option<String>,
    /// Child components, laid out vertically.
    pub children: Vec<ComponentSpec>,
    /// Visual state.
    pub visual: ComponentVisualState,
    /// Accessibility semantics.
    pub a11y: A11ySpec,
}

/// A single tab: stable key + title + child tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TabSpec {
    /// Component key.
    pub key: ComponentKey,
    /// Tab title.
    pub title: String,
    /// Tab content.
    pub child: Box<ComponentSpec>,
}

/// Tab strip + active child.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TabsSpec {
    /// Component key.
    pub key: ComponentKey,
    /// Index of the active tab (`< tabs.len()`).
    pub active: usize,
    /// Tabs in order.
    pub tabs: Vec<TabSpec>,
    /// Visual state.
    pub visual: ComponentVisualState,
    /// Accessibility semantics.
    pub a11y: A11ySpec,
}

/// Continuous-value control spec shared by `Fader` and `Seek`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControlSpec {
    /// Component key.
    pub key: ComponentKey,
    /// Binding-level control id (defaults to `key`).
    pub control_id: Option<ControlId>,
    /// Logical command target used as the intent fallback target.
    pub target: Option<LogicalTarget>,
    /// Value contract.
    pub descriptor: ValueDescriptor,
    /// Layout orientation.
    pub orientation: Orientation,
    /// Terminal policy for detach when the input does not override it.
    pub detach_policy: DetachPolicy,
    /// Marks the control unsupported: every mutating input notifies
    /// `UnavailableReason::Unsupported`.
    pub unsupported: bool,
    /// Visual state.
    pub visual: ComponentVisualState,
    /// Accessibility semantics.
    pub a11y: A11ySpec,
}

/// One resolved waveform drawing bin — the contract-layer payload for
/// `ruviform-viz::ViewBin` + `LodBucket` (same field semantics:
/// `count == 0` ⇒ empty bin, `NaN` statistics ⇒ gap/missing data).
///
/// The bounded query / LOD build / cancellation machinery stays in
/// `ruviform-viz` on the consumer side; this spec only carries the
/// already-resolved, bounded payload.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WaveformBin {
    /// First frame the bin covers (inclusive).
    pub start: u64,
    /// Last frame the bin covers (exclusive).
    pub end: u64,
    /// Minimum finite sample (`NaN` when empty).
    pub min: f64,
    /// Maximum finite sample (`NaN` when empty).
    pub max: f64,
    /// RMS over finite samples (`NaN` when empty).
    pub rms: f64,
    /// Finite samples aggregated (`0` = nothing covered).
    pub count: u64,
}

/// Upper bound on total bins a `Waveform` spec may carry — keeps spec
/// payloads bounded by construction.
pub const MAX_WAVEFORM_BINS: usize = 16384;

/// One resolved level-meter bin — peak/RMS over a frame span.
/// `count == 0` marks an empty bin (no samples covered); `NaN`
/// statistics carry the same gap/missing-data meaning as
/// [`WaveformBin`]. Level values are consumer-domain amplitudes —
/// the contract does not prescribe dB or linear scale.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MeterBin {
    /// First frame the bin covers (inclusive).
    pub start: u64,
    /// Last frame the bin covers (exclusive).
    pub end: u64,
    /// Peak absolute level over the bin (`NaN` when empty).
    pub peak: f64,
    /// RMS level over the bin (`NaN` when empty).
    pub rms: f64,
    /// Samples aggregated (`0` = nothing covered).
    pub count: u64,
}

/// Upper bound on total bins a `Meter` spec may carry.
pub const MAX_METER_BINS: usize = 8192;

/// Level-meter visualization component. Payload is resolved data,
/// not a query — renderers draw the bins they are given. Levels are
/// unipolar (no min/max pair like [`WaveformBin`]).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeterSpec {
    /// Component key.
    pub key: ComponentKey,
    /// Channel count (`>= 1`, matches `channel_levels.len()`).
    pub channels: usize,
    /// Resolved level bins per channel (`channel_levels.len() ==
    /// channels`, total bins `<= MAX_METER_BINS`).
    pub channel_levels: Vec<Vec<MeterBin>>,
    /// Viewport frame range the bins cover.
    pub viewport: Range<u64>,
    /// Visual state.
    pub visual: ComponentVisualState,
    /// Accessibility semantics.
    pub a11y: A11ySpec,
}

/// One resolved time-axis tick. `at` is expressed in the same frame
/// units as the owning [`TimeAxisSpec::viewport`] (fractional values
/// allowed — ticks need not align to whole frames); `label` is the
/// consumer-resolved display text (the contract carries no timebase
/// formatting — `ruviform-viz`'s `SampleTimebase`/`FrameGrid` are the
/// consumer-side resolution path).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AxisTick {
    /// Tick position in viewport frame units.
    pub at: f64,
    /// Display label.
    pub label: String,
    /// Major ticks render stronger than minor ones.
    pub major: bool,
}

/// Upper bound on ticks a `TimeAxis` spec may carry.
pub const MAX_TIME_AXIS_TICKS: usize = 1024;

/// Time/ruler axis component — non-interactive resolved tick marks.
/// Consumers resolve tick positions and labels through their own
/// timebase (`ruviform-viz` timeline primitives on the consumer
/// side); the spec only carries the resolved payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimeAxisSpec {
    /// Component key.
    pub key: ComponentKey,
    /// Viewport frame range the axis spans.
    pub viewport: Range<u64>,
    /// Resolved ticks (`<= MAX_TIME_AXIS_TICKS`).
    pub ticks: Vec<AxisTick>,
    /// Visual state.
    pub visual: ComponentVisualState,
    /// Accessibility semantics.
    pub a11y: A11ySpec,
}

/// Waveform visualization component. Payload is resolved data, not a
/// query — renderers draw the bins they are given.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WaveformSpec {
    /// Component key.
    pub key: ComponentKey,
    /// Samples per second (integer Hz).
    pub sample_rate: u32,
    /// Channel count (`>= 1`, matches `channel_bins.len()`).
    pub channels: usize,
    /// Per-channel total frame count.
    pub frames: u64,
    /// Viewport frame range the bins cover.
    pub viewport: Range<u64>,
    /// Resolved bins per channel (`channel_bins.len() == channels`,
    /// total bins `<= MAX_WAVEFORM_BINS`).
    pub channel_bins: Vec<Vec<WaveformBin>>,
    /// Visual state.
    pub visual: ComponentVisualState,
    /// Accessibility semantics.
    pub a11y: A11ySpec,
}

/// One resolved spectrogram time column — a frame span with one
/// magnitude per frequency row. `count == 0` ⇒ empty column (gap), and
/// a `NaN` magnitude ⇒ missing data for that cell. The gap semantics
/// match [`WaveformBin`]; the magnitudes are consumer-resolved values
/// (`ruviform-viz` `Tile` values on the consumer side).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpectrogramColumn {
    /// First frame the column covers (inclusive).
    pub start: u64,
    /// Last frame the column covers (exclusive).
    pub end: u64,
    /// Finite cells in `magnitudes` (`0..=rows`; `0` = gap column).
    /// Counts output cells, not source samples: the consumer derives
    /// it from the `ruviform-viz` tile (`TileColumn::count`).
    pub count: u64,
    /// Magnitude per frequency row, row 0 = lowest Hz
    /// (`len() == SpectrogramSpec::rows`).
    pub magnitudes: Vec<f64>,
}

/// Upper bound on total cells (`columns × rows`) a `Spectrogram` spec
/// may carry — keeps spec payloads bounded by construction.
pub const MAX_SPECTROGRAM_CELLS: usize = 65536;

/// Spectrogram visualization component. Payload is resolved data, not
/// a query — renderers draw the cells they are given. `magnitude_range`
/// is the consumer-resolved display range that renderers map onto
/// intensity (values outside it clamp); the contract does not
/// prescribe dB or linear scale.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpectrogramSpec {
    /// Component key.
    pub key: ComponentKey,
    /// Viewport frame range the columns cover.
    pub viewport: Range<u64>,
    /// Frequency rows per column (`>= 1`).
    pub rows: usize,
    /// Display range for magnitudes (`start < end`, both finite).
    pub magnitude_range: Range<f64>,
    /// Resolved time columns (`magnitudes.len() == rows` per column,
    /// `columns.len() * rows <= MAX_SPECTROGRAM_CELLS`).
    pub columns: Vec<SpectrogramColumn>,
    /// Visual state.
    pub visual: ComponentVisualState,
    /// Accessibility semantics.
    pub a11y: A11ySpec,
}

/// A region reserved for a native surface slot (Issue #388
/// [`NativeSurfaceSlot`](crate::NativeSurfaceSlot) port).
///
/// The component is **non-interactive and declarative**: it marks
/// where in the Panel/Tab layout a native child surface lives, so the
/// renderer can project the laid-out bounds — including ancestor
/// clip rects and inactive-tab suppression — onto the slot port.
/// The runtime [`SurfaceSlotId`](crate::SurfaceSlotId) is *not* part
/// of the spec: slots are minted by `declare_slot` against a live
/// window, so consumers bind `key → slot` on the driver side
/// (`GpuiDriver::bind_native_slot`) after window creation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NativeSlotSpec {
    /// Component key — also the `bind_native_slot` lookup key.
    pub key: ComponentKey,
    /// Minimum logical `[width, height]` the region reserves when the
    /// layout would otherwise give it zero extent.
    pub min_size: Option<[f64; 2]>,
    /// Visual state.
    pub visual: ComponentVisualState,
    /// Accessibility semantics.
    pub a11y: A11ySpec,
}

/// Static text label. The text is the consumer-resolved display string;
/// the contract does not format or localize it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LabelSpec {
    /// Component key.
    pub key: ComponentKey,
    /// Resolved display text.
    pub text: String,
    /// Visual state.
    pub visual: ComponentVisualState,
    /// Accessibility semantics. `role` defaults to [`A11yRole::Text`].
    pub a11y: A11ySpec,
}

/// One resolved table cell value. The consumer resolves the value;
/// [`Display`] is the single canonical text form every renderer uses.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CellValue {
    /// Resolved text.
    Text(String),
    /// Resolved finite number (non-finite values are rejected on mount).
    Number(f64),
}

impl Display for CellValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Text(s) => f.write_str(s),
            Self::Number(n) => n.fmt(f),
        }
    }
}

/// Upper bound on rows a `Table` spec may carry.
pub const MAX_TABLE_ROWS: usize = 4096;

/// Upper bound on columns a `Table` spec may carry.
pub const MAX_TABLE_COLUMNS: usize = 64;

/// Read-only table of resolved cells. Column and row order are kept as
/// declared. A `None` cell is a missing value: it stays in its position
/// so later cells in the row do not shift. A `List` is a one-column table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableSpec {
    /// Component key.
    pub key: ComponentKey,
    /// Column headers in display order.
    pub columns: Vec<String>,
    /// Rows in display order (each row has exactly `columns.len()`
    /// cells, `None` for a missing value).
    pub rows: Vec<Vec<Option<CellValue>>>,
    /// Visual state.
    pub visual: ComponentVisualState,
    /// Accessibility semantics. `role` defaults to [`A11yRole::Table`].
    pub a11y: A11ySpec,
}

/// Backend-neutral accessibility node derived from a spec. The tree is
/// the single source of truth for `Label`/`Table` semantics; renderers
/// consume it instead of re-deriving table structure.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct A11yNode {
    /// Semantic role.
    pub role: A11yRole,
    /// Accessible name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Text content. `None` for a container or a missing cell.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<CellValue>,
    /// Child nodes in document order.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<A11yNode>,
}

impl A11yNode {
    fn new(
        role: A11yRole,
        name: Option<String>,
        value: Option<CellValue>,
        children: Vec<A11yNode>,
    ) -> Self {
        Self {
            role,
            name,
            value,
            children,
        }
    }
}

impl TableSpec {
    /// Table → header row + data rows → cells. Missing cells stay as
    /// valueless `Cell` nodes so the grid positions remain stable.
    fn a11y_node(&self) -> A11yNode {
        let header = A11yNode::new(
            A11yRole::Row,
            None,
            None,
            self.columns
                .iter()
                .map(|c| {
                    A11yNode::new(
                        A11yRole::ColumnHeader,
                        None,
                        Some(CellValue::Text(c.clone())),
                        Vec::new(),
                    )
                })
                .collect(),
        );
        let rows = self.rows.iter().map(|row| {
            let cells = row
                .iter()
                .map(|cell| A11yNode::new(A11yRole::Cell, None, cell.clone(), Vec::new()))
                .collect();
            A11yNode::new(A11yRole::Row, None, None, cells)
        });
        A11yNode::new(
            self.a11y.role.unwrap_or(A11yRole::Table),
            self.a11y.label.clone(),
            None,
            std::iter::once(header).chain(rows).collect(),
        )
    }
}

/// Component tree node. The set is intentionally minimal for the
/// first two vertical slices (Synvelis Fader+Dock, `AudioLens`
/// Waveform+Seek); new kinds extend this enum rather than a generic
/// widget DSL.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ComponentSpec {
    /// Grouping container.
    Panel(PanelSpec),
    /// Tab strip + active child.
    Tabs(TabsSpec),
    /// Vertical/horizontal continuous fader.
    Fader(ControlSpec),
    /// Timeline seek control (horizontal fader semantics).
    Seek(ControlSpec),
    /// Continuous control with the same value/gesture contract as
    /// `Fader` (`Begin` → `Preview*` → `Commit|Cancel`). The first GPUI
    /// slice draws it as a vertical bar; a rotary drawing is renderer
    /// work and does not change the headless contract.
    Knob(ControlSpec),
    /// Binary switch. The descriptor must declare the fixed binary
    /// range `{min: 0, max: 1, step: 1}` (linear mapping) — toggle
    /// semantics are inherently two-state and the range is part of
    /// the component's identity, not consumer configuration.
    Toggle(ControlSpec),
    /// Resolved-waveform visualization.
    Waveform(WaveformSpec),
    /// Resolved level-meter visualization.
    Meter(MeterSpec),
    /// Resolved time-axis tick marks.
    #[serde(rename = "time_axis")]
    TimeAxis(TimeAxisSpec),
    /// Resolved spectrogram (time × frequency magnitudes).
    Spectrogram(SpectrogramSpec),
    /// Native-surface region — layout bounds feed the slot port.
    #[serde(rename = "native_slot")]
    NativeSlot(NativeSlotSpec),
    /// Static text label.
    Label(LabelSpec),
    /// Read-only table of resolved cells (a list is a one-column table).
    Table(TableSpec),
}

impl ComponentSpec {
    /// The component's stable key.
    #[must_use]
    pub fn key(&self) -> &ComponentKey {
        match self {
            Self::Panel(s) => &s.key,
            Self::Tabs(s) => &s.key,
            Self::Fader(s) | Self::Seek(s) | Self::Knob(s) | Self::Toggle(s) => &s.key,
            Self::Waveform(s) => &s.key,
            Self::Meter(s) => &s.key,
            Self::TimeAxis(s) => &s.key,
            Self::Spectrogram(s) => &s.key,
            Self::NativeSlot(s) => &s.key,
            Self::Label(s) => &s.key,
            Self::Table(s) => &s.key,
        }
    }

    /// Backend-neutral a11y tree for the kinds whose semantics this
    /// contract defines (`Label`, `Table`). `None` for other kinds.
    #[must_use]
    pub fn a11y_node(&self) -> Option<A11yNode> {
        match self {
            Self::Label(s) => Some(A11yNode::new(
                s.a11y.role.unwrap_or(A11yRole::Text),
                s.a11y.label.clone(),
                Some(CellValue::Text(s.text.clone())),
                Vec::new(),
            )),
            Self::Table(s) => Some(s.a11y_node()),
            _ => None,
        }
    }

    /// A short kind name for ops and diagnostics.
    #[must_use]
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Panel(_) => "panel",
            Self::Tabs(_) => "tabs",
            Self::Fader(_) => "fader",
            Self::Seek(_) => "seek",
            Self::Knob(_) => "knob",
            Self::Toggle(_) => "toggle",
            Self::Waveform(_) => "waveform",
            Self::Meter(_) => "meter",
            Self::TimeAxis(_) => "time_axis",
            Self::Spectrogram(_) => "spectrogram",
            Self::NativeSlot(_) => "native_slot",
            Self::Label(_) => "label",
            Self::Table(_) => "table",
        }
    }

    /// Control spec for continuous/discrete value controls
    /// (`Fader`/`Seek`/`Knob`/`Toggle`).
    #[must_use]
    pub fn control(&self) -> Option<&ControlSpec> {
        match self {
            Self::Fader(s) | Self::Seek(s) | Self::Knob(s) | Self::Toggle(s) => Some(s),
            _ => None,
        }
    }
}

/// A complete view tree rooted at one component.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ViewSpec {
    /// Root component.
    pub root: ComponentSpec,
}

/// Lifecycle ops emitted by [`ViewHost`] reconciliation.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectionOp {
    /// Key newly present in the spec.
    Mounted {
        /// Component key.
        key: ComponentKey,
    },
    /// Same key, changed props — semantic identity preserved.
    Updated {
        /// Component key.
        key: ComponentKey,
    },
    /// Key disappeared from the spec (gestures end per detach policy).
    Removed {
        /// Component key.
        key: ComponentKey,
    },
    /// Host unmounted — every mounted key is torn down.
    Unmounted {
        /// Component key.
        key: ComponentKey,
    },
}

/// Events emitted by [`ViewHost`] for the consumer/transport.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum ViewEvent {
    /// A projection lifecycle op.
    Op(ProjectionOp),
    /// A headless interaction event from one control.
    Gesture {
        /// Component key the control belongs to.
        key: ComponentKey,
        /// The interaction event.
        event: InteractionEvent,
    },
    /// A command intent minted from a gesture event, already bound
    /// with a binding-level `command_id`.
    Intent {
        /// The intent.
        intent: CommandIntent,
        /// Binding-minted command id.
        command_id: CommandId,
        /// Whether the binding expects a receipt for this command.
        expect_receipt: bool,
    },
    /// A mounted component kind the host/renderer does not support.
    Unsupported {
        /// Component key.
        key: ComponentKey,
        /// Kind name.
        kind: String,
    },
}

/// Errors returned by [`ViewHost`] operations.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum ViewError {
    /// The caller passed a generation that is not current — the input
    /// was produced under a dead view epoch and must not mutate state.
    StaleGeneration {
        /// Current generation.
        expected: u64,
        /// Supplied generation.
        got: u64,
    },
    /// The spec is structurally invalid.
    InvalidSpec {
        /// Human-readable reason.
        detail: String,
    },
    /// The host has no mounted tree — `update` reconciles an
    /// existing mount and is not a remount entry; go through
    /// [`ViewHost::mount`] (the epoch boundary) instead.
    NotMounted,
    /// No component with the given key is mounted.
    UnknownComponent {
        /// The missing key.
        key: ComponentKey,
    },
}

impl Display for ViewError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StaleGeneration { expected, got } => {
                write!(f, "stale generation {got} (expected {expected})")
            }
            Self::InvalidSpec { detail } => write!(f, "invalid spec: {detail}"),
            Self::NotMounted => write!(f, "no mounted tree — use mount()"),
            Self::UnknownComponent { key } => {
                write!(f, "no mounted component for key {key}")
            }
        }
    }
}

impl std::error::Error for ViewError {}

/// Bound on the internal event backlog — preview-class events evict
/// first; non-droppable events are never lost under this bound.
const MAX_VIEW_EVENTS: usize = 512;

/// Snapshot of a mounted control's display state.
#[derive(Debug, Clone)]
pub struct ComponentSnapshot {
    /// Current display value (preview while active, committed while
    /// idle) — `None` for non-control components.
    pub display: Option<crate::behavior::ControlValue>,
    /// Last committed value.
    pub committed: Option<crate::behavior::ControlValue>,
    /// Binding display phase (`Idle` when no binding exists).
    pub phase: CommandPhase,
    /// Latest accepted authoritative projection — the only channel
    /// that proves domain state (`None` when none has landed).
    pub observed: Option<StateProjection<serde_json::Value>>,
    /// Whether the component is mounted.
    pub mounted: bool,
}

#[derive(Debug, Clone)]
struct ControlMeta {
    control_id: ControlId,
    spec: ControlSpec,
}

#[derive(Debug, Clone)]
struct Mounted {
    kind: &'static str,
    spec: ComponentSpec,
    control: Option<ControlMeta>,
}

/// Reconcile outcome for one `(key, node)` pair.
enum ReconcileAction {
    Mount,
    KindSwap,
    Updated { contract_changed: bool },
    Unchanged,
}

/// Runtime-instance epoch distinguishing live [`ViewHost`]s that
/// share one logical [`ViewInstanceId`].
///
/// A `ViewInstanceId` is the *logical* identity — it survives
/// moves, layout restores, and host recreation by contract. The
/// `AttachEpoch` is the *runtime* identity: a fresh value for every
/// `ViewHost` instance created for that logical id, injected at
/// construction and folded into each binding's command-id
/// namespace. A receipt minted against a dead runtime instance can
/// therefore never resolve onto a command minted by its successor —
/// the same collision class as the per-host `mount_epoch`, closed
/// across `Drop`+recreate boundaries.
///
/// Mint via [`AttachEpochAllocator`]; a literal `AttachEpoch::new`
/// is legal for single-shot hosts when the caller owns uniqueness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AttachEpoch(u64);

impl AttachEpoch {
    /// Wraps a raw value as an [`AttachEpoch`]. Callers minting
    /// literals must uphold the uniqueness contract themselves —
    /// prefer [`AttachEpochAllocator`] whenever more than one host
    /// can exist per logical id.
    #[must_use]
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    /// The raw epoch value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl Display for AttachEpoch {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Session-scoped allocator of [`AttachEpoch`]s — the authority
/// that tells two runtime instances of one logical view apart.
///
/// The party that owns the logical-id lifecycle (the consumer's
/// view session / dock restore path — whatever creates and drops
/// [`ViewHost`]s) owns one allocator and injects each minted epoch
/// into `ViewHost::new`. Uniqueness is guaranteed per allocator,
/// which is honest because command ids and their receipts are
/// inbox-scoped: a receipt can only arrive through the live session
/// that minted the command, so a restarted process or session —
/// which starts a fresh allocator at the same values — can never
/// receive the dead session's receipts.
///
/// `mint` is monotonic `u64`; exhaustion panics rather than
/// silently reusing a value (2^64 mints is unreachable in any real
/// session). No `Default`/`Clone`: a second allocator mints the same
/// sequence and silently re-opens the collision it exists to close —
/// the composition root constructs exactly one per session.
#[derive(Debug)]
pub struct AttachEpochAllocator {
    next: u64,
}

#[expect(
    clippy::new_without_default,
    reason = "Default is deliberately withheld — a second allocator mints the same sequence and re-opens the collision"
)]
impl AttachEpochAllocator {
    /// An empty allocator; the first minted epoch is `1`.
    #[must_use]
    pub fn new() -> Self {
        Self { next: 0 }
    }

    /// Mints the next runtime-instance epoch.
    ///
    /// # Panics
    /// After 2^64 mints — unreachable in a real session; a panic is
    /// honest where silently reusing an epoch would reopen the
    /// stale-receipt collision the epoch exists to close.
    pub fn mint(&mut self) -> AttachEpoch {
        self.next = self
            .next
            .checked_add(1)
            .expect("AttachEpochAllocator exhausted (2^64 mints)");
        AttachEpoch(self.next)
    }
}

/// Headless projection engine for one [`ViewInstanceId`].
///
/// Owns the spec tree, per-control [`InteractionDispatcher`], and
/// per-control [`CommandBinding`]s. `mount`/`update`/`unmount`
/// reconcile by [`ComponentKey`]; `dispatch_input` routes inputs to
/// controls and emits intents through the bindings. A `generation`
/// guards every mutation — stale generations are rejected with
/// [`ViewError::StaleGeneration`] and never mutate the current tree.
pub struct ViewHost {
    view_id: ViewInstanceId,
    /// Runtime-instance epoch for this host — fixed at construction
    /// and part of every binding's command-id namespace. Two hosts
    /// created for the same logical id (drop → recreate, layout
    /// restore) carry distinct epochs, so a receipt minted against
    /// one can never resolve inside the other.
    attach_epoch: AttachEpoch,
    generation: u64,
    /// Mount epoch — bumped on every fresh `mount` after `unmount`.
    /// It is part of each binding's command-id namespace so ids
    /// minted under a dead epoch can never collide with (and be
    /// answered as) ids minted under the live one.
    mount_epoch: u64,
    spec: Option<ViewSpec>,
    mounted: HashMap<ComponentKey, Mounted>,
    dispatcher: InteractionDispatcher,
    bindings: HashMap<ControlId, CommandBinding>,
    /// `command_id` → owning control. Receipts arrive through the IPC
    /// envelope carrying only the minted id; this index routes them
    /// back. Entries drop on terminal receipts and control unregister.
    command_index: HashMap<CommandId, ControlId>,
    ports: HashMap<ControlId, ValuePorts>,
    events: VecDeque<ViewEvent>,
}

impl ViewHost {
    /// Creates an empty host for `view_id` (detached, generation 0)
    /// carrying `attach_epoch` as its runtime-instance identity.
    /// Mint the epoch from the session's [`AttachEpochAllocator`] —
    /// two hosts for one logical id must never share an epoch.
    #[must_use]
    pub fn new(view_id: ViewInstanceId, attach_epoch: AttachEpoch) -> Self {
        Self {
            view_id,
            attach_epoch,
            generation: 0,
            mount_epoch: 0,
            spec: None,
            mounted: HashMap::new(),
            dispatcher: InteractionDispatcher::new(),
            bindings: HashMap::new(),
            command_index: HashMap::new(),
            ports: HashMap::new(),
            events: VecDeque::new(),
        }
    }

    /// The view this host projects.
    #[must_use]
    pub fn view_id(&self) -> ViewInstanceId {
        self.view_id
    }

    /// The current generation. Callers must pass this back to
    /// `update`/`dispatch_input`; adapters read it after `mount`/
    /// `detach` bumps it (`detach` also returns it; `attach`
    /// returns `()` — read the getter).
    #[must_use]
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// The current spec, if mounted.
    #[must_use]
    pub fn spec(&self) -> Option<&ViewSpec> {
        self.spec.as_ref()
    }

    /// Backend-neutral a11y tree of a mounted component. `None` when the
    /// key is not mounted or its kind defines no a11y tree.
    #[must_use]
    pub fn a11y_node(&self, key: &ComponentKey) -> Option<A11yNode> {
        self.mounted.get(key)?.spec.a11y_node()
    }

    /// Currently mounted component keys.
    #[must_use]
    pub fn mounted_keys(&self) -> Vec<ComponentKey> {
        let mut keys: Vec<ComponentKey> = self.mounted.keys().cloned().collect();
        keys.sort();
        keys
    }

    /// Supplies function ports for a control before or after mount —
    /// `ValuePorts` are local-only and never part of the spec.
    pub fn register_ports(&mut self, control_id: ControlId, ports: ValuePorts) {
        self.ports.insert(control_id, ports);
    }

    fn check_gen(&self, generation: u64) -> Result<(), ViewError> {
        if generation != self.generation {
            return Err(ViewError::StaleGeneration {
                expected: self.generation,
                got: generation,
            });
        }
        Ok(())
    }

    /// Validate a spec tree. Returns the flattened `(key, node)` list
    /// when valid.
    fn validate_spec(spec: &ViewSpec) -> Result<Vec<(&ComponentKey, &ComponentSpec)>, ViewError> {
        fn walk<'a>(
            node: &'a ComponentSpec,
            flat: &mut Vec<(&'a ComponentKey, &'a ComponentSpec)>,
        ) {
            flat.push((node.key(), node));
            match node {
                ComponentSpec::Panel(p) => {
                    for c in &p.children {
                        walk(c, flat);
                    }
                }
                ComponentSpec::Tabs(t) => {
                    for tab in &t.tabs {
                        flat.push((&tab.key, tab.child.as_ref()));
                        walk(tab.child.as_ref(), flat);
                    }
                }
                _ => {}
            }
        }
        let mut flat: Vec<(&ComponentKey, &ComponentSpec)> = Vec::new();
        walk(&spec.root, &mut flat);

        let mut seen = std::collections::HashSet::new();
        for (key, node) in &flat {
            if !seen.insert(*key) {
                return Err(ViewError::InvalidSpec {
                    detail: format!("duplicate component key {key}"),
                });
            }
            Self::check_node(node)?;
        }
        Ok(flat)
    }

    /// Resolved visualization payload bounds (`Waveform`, `Meter`, `TimeAxis`, `Spectrogram`).
    fn check_viz(node: &ComponentSpec) -> Result<(), ViewError> {
        match node {
            ComponentSpec::Waveform(w) => {
                if w.channels == 0 || w.channel_bins.len() != w.channels {
                    return Err(ViewError::InvalidSpec {
                        detail: format!(
                            "waveform {} channels {} != channel_bins {}",
                            w.key,
                            w.channels,
                            w.channel_bins.len()
                        ),
                    });
                }
                let total: usize = w.channel_bins.iter().map(Vec::len).sum();
                if total > MAX_WAVEFORM_BINS {
                    return Err(ViewError::InvalidSpec {
                        detail: format!(
                            "waveform {} carries {total} bins (> {MAX_WAVEFORM_BINS})",
                            w.key
                        ),
                    });
                }
                if w.viewport.end < w.viewport.start {
                    return Err(ViewError::InvalidSpec {
                        detail: format!("waveform {} viewport end < start", w.key),
                    });
                }
            }
            ComponentSpec::Meter(m) => {
                if m.channels == 0 || m.channel_levels.len() != m.channels {
                    return Err(ViewError::InvalidSpec {
                        detail: format!(
                            "meter {} channels {} != channel_levels {}",
                            m.key,
                            m.channels,
                            m.channel_levels.len()
                        ),
                    });
                }
                let total: usize = m.channel_levels.iter().map(Vec::len).sum();
                if total > MAX_METER_BINS {
                    return Err(ViewError::InvalidSpec {
                        detail: format!(
                            "meter {} carries {total} bins (> {MAX_METER_BINS})",
                            m.key
                        ),
                    });
                }
                if m.viewport.end < m.viewport.start {
                    return Err(ViewError::InvalidSpec {
                        detail: format!("meter {} viewport end < start", m.key),
                    });
                }
            }
            ComponentSpec::TimeAxis(t) => {
                if t.ticks.len() > MAX_TIME_AXIS_TICKS {
                    return Err(ViewError::InvalidSpec {
                        detail: format!(
                            "time_axis {} carries {} ticks (> {MAX_TIME_AXIS_TICKS})",
                            t.key,
                            t.ticks.len()
                        ),
                    });
                }
                if t.viewport.end < t.viewport.start {
                    return Err(ViewError::InvalidSpec {
                        detail: format!("time_axis {} viewport end < start", t.key),
                    });
                }
                if t.ticks.iter().any(|tick| !tick.at.is_finite()) {
                    return Err(ViewError::InvalidSpec {
                        detail: format!("time_axis {} has a non-finite tick", t.key),
                    });
                }
            }
            ComponentSpec::Spectrogram(s) => Self::check_spectrogram(s)?,
            _ => {}
        }
        Ok(())
    }

    fn check_spectrogram(s: &SpectrogramSpec) -> Result<(), ViewError> {
        if s.rows == 0 || s.columns.iter().any(|c| c.magnitudes.len() != s.rows) {
            return Err(ViewError::InvalidSpec {
                detail: format!(
                    "spectrogram {} rows {} must be >= 1 and match every column",
                    s.key, s.rows
                ),
            });
        }
        let cells = s.columns.len().saturating_mul(s.rows);
        if cells > MAX_SPECTROGRAM_CELLS {
            return Err(ViewError::InvalidSpec {
                detail: format!(
                    "spectrogram {} carries {cells} cells (> {MAX_SPECTROGRAM_CELLS})",
                    s.key
                ),
            });
        }
        if s.viewport.end < s.viewport.start {
            return Err(ViewError::InvalidSpec {
                detail: format!("spectrogram {} viewport end < start", s.key),
            });
        }
        let r = &s.magnitude_range;
        if !(r.start.is_finite() && r.end.is_finite() && r.start < r.end) {
            return Err(ViewError::InvalidSpec {
                detail: format!(
                    "spectrogram {} magnitude_range must be finite and start < end",
                    s.key
                ),
            });
        }
        Ok(())
    }

    fn push_event(&mut self, e: ViewEvent) {
        if self.events.len() >= MAX_VIEW_EVENTS {
            // Coalescible events evict first; non-droppable records
            // are never lost under this bound.
            let droppable = self.events.iter().position(|ev| match ev {
                ViewEvent::Gesture { event, .. } => event.is_coalescible(),
                ViewEvent::Intent { intent, .. } => crate::behavior::is_droppable(intent),
                _ => false,
            });
            match droppable {
                Some(pos) => {
                    self.events.remove(pos);
                }
                None => {
                    self.events.pop_front();
                }
            }
        }
        self.events.push_back(e);
    }

    /// Reconcile outcome for one `(key, node)` pair.
    fn reconcile_action(mounted: Option<&Mounted>, node: &ComponentSpec) -> ReconcileAction {
        match mounted {
            None => ReconcileAction::Mount,
            Some(m) if m.kind != node.kind() => ReconcileAction::KindSwap,
            Some(m) if m.spec == *node => ReconcileAction::Unchanged,
            Some(m) => {
                // Same kind, changed props → updated; identity
                // preserved unless the control contract changed.
                let contract_changed = match (m.spec.control(), node.control()) {
                    (Some(a), Some(b)) => {
                        a.descriptor != b.descriptor
                            || a.detach_policy != b.detach_policy
                            || a.unsupported != b.unsupported
                            || a.control_id != b.control_id
                    }
                    (None, None) => false,
                    _ => true,
                };
                ReconcileAction::Updated { contract_changed }
            }
        }
    }

    /// Resolves the mounted key owning `control_id`; falls back to a
    /// key synthesized from the id when the control is already gone.
    fn key_for_control(&self, control_id: &str) -> ComponentKey {
        self.mounted
            .iter()
            .find(|(_, m)| {
                m.control
                    .as_ref()
                    .is_some_and(|c| c.control_id == control_id)
            })
            .map_or_else(|| ComponentKey(control_id.to_string()), |(k, _)| k.clone())
    }

    /// Per-node structural checks: kind payload bounds and descriptor rules.
    fn check_node(node: &ComponentSpec) -> Result<(), ViewError> {
        Self::check_viz(node)?;
        match node {
            ComponentSpec::Tabs(t) => {
                if t.tabs.is_empty() {
                    return Err(ViewError::InvalidSpec {
                        detail: format!("tabs {} has no tabs", t.key),
                    });
                }
                if t.active >= t.tabs.len() {
                    return Err(ViewError::InvalidSpec {
                        detail: format!(
                            "tabs {} active {} out of range {}",
                            t.key,
                            t.active,
                            t.tabs.len()
                        ),
                    });
                }
            }
            ComponentSpec::Toggle(c) => {
                let d = &c.descriptor;
                let binary = d.min.total_cmp(&0.0).is_eq()
                    && d.max.total_cmp(&1.0).is_eq()
                    && d.step == Some(1.0)
                    && matches!(
                        d.mapping,
                        None | Some(crate::behavior::ValueMappingSpec::Linear)
                    );
                if !binary {
                    return Err(ViewError::InvalidSpec {
                        detail: format!(
                            "toggle {} descriptor must be the fixed binary range \
                                 {{min: 0, max: 1, step: 1}} with linear mapping",
                            c.key
                        ),
                    });
                }
            }
            ComponentSpec::NativeSlot(n) => {
                if let Some([w, h]) = n.min_size
                    && (!w.is_finite() || !h.is_finite() || w < 0.0 || h < 0.0)
                {
                    return Err(ViewError::InvalidSpec {
                        detail: format!("native_slot {} min_size invalid", n.key),
                    });
                }
            }
            ComponentSpec::Table(t) => {
                if t.rows.len() > MAX_TABLE_ROWS || t.columns.len() > MAX_TABLE_COLUMNS {
                    return Err(ViewError::InvalidSpec {
                        detail: format!(
                            "table {} is {}x{} (> {MAX_TABLE_ROWS} rows or > {MAX_TABLE_COLUMNS} columns)",
                            t.key,
                            t.rows.len(),
                            t.columns.len()
                        ),
                    });
                }
                if t.rows.iter().any(|row| row.len() != t.columns.len()) {
                    return Err(ViewError::InvalidSpec {
                        detail: format!(
                            "table {} row cell count != columns {}",
                            t.key,
                            t.columns.len()
                        ),
                    });
                }
                let non_finite = t
                    .rows
                    .iter()
                    .flatten()
                    .flatten()
                    .any(|v| matches!(v, CellValue::Number(n) if !n.is_finite()));
                if non_finite {
                    return Err(ViewError::InvalidSpec {
                        detail: format!("table {} has a non-finite cell", t.key),
                    });
                }
            }
            _ => {}
        }
        if let Some(c) = node.control() {
            let issues = validate_descriptor(&c.descriptor);
            if !issues.is_empty() {
                let detail = issues
                    .iter()
                    .map(|i| i.detail.clone())
                    .collect::<Vec<_>>()
                    .join("; ");
                return Err(ViewError::InvalidSpec {
                    detail: format!("control {} descriptor: {detail}", c.key),
                });
            }
        }
        Ok(())
    }

    fn register_control(&mut self, spec: &ControlSpec) {
        let control_id = spec
            .control_id
            .clone()
            .unwrap_or_else(|| spec.key.0.clone());
        let options = InteractionControlOptions {
            control_id: control_id.clone(),
            descriptor: Some(spec.descriptor.clone()),
            ports: self.ports.get(&control_id).cloned(),
            target: spec.target.clone(),
            origin_view: Some(format!("view-{}", self.view_id.get())),
            unsupported: spec.unsupported,
            detach_policy: Some(spec.detach_policy),
            terminated_gesture_history: None,
        };
        let events = self.dispatcher.register(options);
        for event in events {
            self.emit_gesture(&spec.key, event);
        }
        let namespace = self.binding_namespace(&control_id);
        self.bindings
            .entry(control_id.clone())
            .or_insert_with(|| CommandBinding::new(namespace));
        // A binding that survived re-registration (descriptor change)
        // still holds the last accepted projection — re-apply it to
        // the fresh control so the controlled display restores the
        // last observed truth through the new descriptor's mapper
        // instead of sitting at the default until the next push.
        let observed = self
            .bindings
            .get(&control_id)
            .and_then(|b| b.state().last_observed)
            .and_then(|p| crate::behavior::projection_value_input(&p.value));
        if let Some(input) = observed
            && let Some(control) = self.dispatcher.control_mut(&control_id)
        {
            control.apply_external(input);
        }
    }

    /// Command-id namespace for `control_id` under this host's
    /// attach epoch and the current mount epoch. `unmount` drops
    /// the bindings wholesale and dropping the host drops the whole
    /// runtime instance, so the two epoch segments keep minted ids
    /// distinct from both dead mounts and dead host instances —
    /// stale receipts then miss the `command_index` instead of
    /// resolving onto a recycled id.
    fn binding_namespace(&self, control_id: &str) -> String {
        format!(
            "view-{}:a{}:m{}:{}",
            self.view_id.get(),
            self.attach_epoch,
            self.mount_epoch,
            control_id
        )
    }

    fn unregister_control(&mut self, meta: &ControlMeta) {
        let events = self.dispatcher.unregister(&meta.control_id);
        for event in events {
            self.emit_gesture(&meta.spec.key, event);
        }
        self.command_index
            .retain(|_, owner| owner != &meta.control_id);
    }

    /// Routes one emitted interaction event through the intent
    /// pipeline: recorded as `Gesture`, translated to a bound
    /// `Intent` when it is a gesture-boundary event. The binding key
    /// is the event's own `control_id` (authoritative) — the mounted
    /// meta only supplies the fallback target.
    fn emit_gesture(&mut self, key: &ComponentKey, event: InteractionEvent) {
        let control_id = event.meta().control_id.clone();
        let fallback = self.mounted.get(key).and_then(|m| {
            m.control
                .as_ref()
                .filter(|c| c.control_id == control_id)
                .and_then(|c| c.spec.target.clone())
        });
        if let Some(intent) = intent_from_gesture(&event, fallback.as_ref()) {
            let namespace = self.binding_namespace(&control_id);
            let binding = self
                .bindings
                .entry(control_id.clone())
                .or_insert_with(|| CommandBinding::new(namespace));
            let (command_id, expect_receipt) = binding.dispatch(intent.clone(), None);
            self.command_index.insert(command_id.clone(), control_id);
            self.push_event(ViewEvent::Intent {
                intent,
                command_id,
                expect_receipt,
            });
        }
        self.push_event(ViewEvent::Gesture {
            key: key.clone(),
            event,
        });
    }

    /// Mounts a spec on an empty host. Returns the ops emitted.
    ///
    /// # Errors
    /// [`ViewError::InvalidSpec`] when the spec fails validation.
    pub fn mount(&mut self, spec: ViewSpec) -> Result<Vec<ProjectionOp>, ViewError> {
        let flat = Self::validate_spec(&spec)?;
        if self.spec.is_some() {
            let generation = self.generation;
            return self.update(spec, generation);
        }
        self.generation += 1;
        self.mount_epoch += 1;
        let mut ops = Vec::new();
        for (key, node) in &flat {
            ops.push(ProjectionOp::Mounted {
                key: (*key).clone(),
            });
            if let Some(c) = node.control() {
                self.register_control(c);
            }
            self.mounted.insert(
                (*key).clone(),
                Mounted {
                    kind: node.kind(),
                    spec: (*node).clone(),
                    control: node.control().map(|c| ControlMeta {
                        control_id: c.control_id.clone().unwrap_or_else(|| c.key.0.clone()),
                        spec: c.clone(),
                    }),
                },
            );
        }
        self.spec = Some(spec);
        for op in &ops {
            self.push_event(ViewEvent::Op(op.clone()));
        }
        Ok(ops)
    }

    /// Reconciles a new spec against the mounted tree by key:
    /// new keys `Mounted`, changed same-key props `Updated`
    /// (identity preserved — in-flight gestures keep running when
    /// the descriptor is unchanged), missing keys `Removed`
    /// (gestures end per detach policy), and kind swaps become
    /// `Removed`+`Mounted`.
    ///
    /// # Errors
    /// [`ViewError::StaleGeneration`] when `generation` is stale;
    /// [`ViewError::NotMounted`] when the host has no mounted tree
    /// (`update` is not a remount entry — go through
    /// [`ViewHost::mount`]); [`ViewError::InvalidSpec`] when the new
    /// spec fails validation. On error the mounted tree is untouched.
    pub fn update(
        &mut self,
        spec: ViewSpec,
        generation: u64,
    ) -> Result<Vec<ProjectionOp>, ViewError> {
        self.check_gen(generation)?;
        // `update` reconciles a mounted tree only — an unmounted host
        // must remount through `mount`, which bumps the namespace
        // epoch. Otherwise a current-gen caller could resurrect the
        // dead mount epoch and re-mint its command ids, letting stale
        // receipts correlate onto the new bindings.
        if self.spec.is_none() {
            return Err(ViewError::NotMounted);
        }
        let flat = Self::validate_spec(&spec)?;
        let mut ops = Vec::new();
        let next_keys: std::collections::HashSet<ComponentKey> =
            flat.iter().map(|(k, _)| (*k).clone()).collect();

        // Removed keys first — detach semantics must not be confused
        // with an update on the surviving tree.
        // `mounted` is a HashMap: sort so the op order is independent of
        // the hash seed (same spec transition → same ops, every run).
        let mut removed: Vec<ComponentKey> = self
            .mounted
            .keys()
            .filter(|k| !next_keys.contains(*k))
            .cloned()
            .collect();
        removed.sort();
        for key in removed {
            if let Some(m) = self.mounted.remove(&key)
                && let Some(meta) = &m.control
            {
                self.unregister_control(meta);
            }
            ops.push(ProjectionOp::Removed { key });
        }

        for &(key, node) in &flat {
            match Self::reconcile_action(self.mounted.get(key), node) {
                ReconcileAction::Unchanged => {}
                ReconcileAction::Updated { contract_changed } => {
                    if contract_changed {
                        // unregister+register: detach semantics.
                        if let Some(m) = self.mounted.get(key)
                            && let Some(meta) = &m.control
                        {
                            let meta = meta.clone();
                            self.unregister_control(&meta);
                        }
                        if let Some(c) = node.control() {
                            self.register_control(c);
                        }
                    }
                    ops.push(ProjectionOp::Updated { key: key.clone() });
                    if let Some(m) = self.mounted.get_mut(key) {
                        m.spec = node.clone();
                        m.control = node.control().map(|c| ControlMeta {
                            control_id: c.control_id.clone().unwrap_or_else(|| c.key.0.clone()),
                            spec: c.clone(),
                        });
                    }
                }
                ReconcileAction::Mount | ReconcileAction::KindSwap => {
                    if let Some(m) = self.mounted.remove(key) {
                        if let Some(meta) = &m.control {
                            self.unregister_control(meta);
                        }
                        ops.push(ProjectionOp::Removed { key: key.clone() });
                    }
                    ops.push(ProjectionOp::Mounted { key: key.clone() });
                    if let Some(c) = node.control() {
                        self.register_control(c);
                    }
                    self.mounted.insert(
                        key.clone(),
                        Mounted {
                            kind: node.kind(),
                            spec: node.clone(),
                            control: node.control().map(|c| ControlMeta {
                                control_id: c.control_id.clone().unwrap_or_else(|| c.key.0.clone()),
                                spec: c.clone(),
                            }),
                        },
                    );
                }
            }
        }
        self.spec = Some(spec);
        for op in &ops {
            self.push_event(ViewEvent::Op(op.clone()));
        }
        Ok(ops)
    }

    /// Unmounts the whole tree: every key is `Unmounted`, all
    /// gestures end per their detach policy, and bindings detach.
    pub fn unmount(&mut self) -> Vec<ProjectionOp> {
        let events = self
            .dispatcher
            .detach_all(crate::behavior::CancelReason::UNMOUNT);
        let keys = self.mounted_keys();
        for event in events {
            // Attribute the event to its control's key.
            let key = self.key_for_control(&event.meta().control_id);
            self.emit_gesture(&key, event);
        }
        self.mounted.clear();
        self.spec = None;
        self.dispatcher = InteractionDispatcher::new();
        // Unmount is terminal for this host epoch: stale bindings are
        // dropped so a remount starts from a clean generation.
        self.bindings.clear();
        self.command_index.clear();
        let ops: Vec<ProjectionOp> = keys
            .into_iter()
            .map(|key| ProjectionOp::Unmounted { key })
            .collect();
        for op in &ops {
            self.push_event(ViewEvent::Op(op.clone()));
        }
        ops
    }

    /// Detaches the view (dock transfer / window hide): every in-flight
    /// gesture ends per its detach policy, bindings go `stale`, and the
    /// generation bumps so late inputs are rejected. The tree stays
    /// mounted — reattach continues from the same spec.
    pub fn detach(&mut self, reason: &str) -> u64 {
        let events = self.dispatcher.detach_all(reason);
        for event in events {
            let key = self.key_for_control(&event.meta().control_id);
            self.emit_gesture(&key, event);
        }
        for b in self.bindings.values_mut() {
            b.detach();
        }
        // Detached bindings absorb late replies; the index is dropped
        // so stale ids never reach a re-attached epoch.
        self.command_index.clear();
        self.generation += 1;
        self.generation
    }

    /// Re-attaches a detached view (same `ViewInstanceId`, new epoch
    /// already minted by `detach`). Bindings accept replies again.
    pub fn attach(&mut self) {
        for b in self.bindings.values_mut() {
            b.attach();
        }
    }

    /// Dispatch an input to a mounted control. Returns the emitted
    /// interaction events (also recorded in the event stream along
    /// with any minted intents).
    ///
    /// # Errors
    /// [`ViewError::StaleGeneration`] for inputs minted under a dead
    /// epoch; [`ViewError::UnknownComponent`] for unmounted keys.
    pub fn dispatch_input(
        &mut self,
        generation: u64,
        key: &ComponentKey,
        input: ControlInput,
    ) -> Result<Vec<InteractionEvent>, ViewError> {
        self.check_gen(generation)?;
        let Some(m) = self.mounted.get(key) else {
            return Err(ViewError::UnknownComponent { key: key.clone() });
        };
        let Some(meta) = m.control.clone() else {
            // Non-control components accept no inputs.
            return Ok(Vec::new());
        };
        let events = self.dispatcher.dispatch(&meta.control_id, input);
        for event in &events {
            self.emit_gesture(key, event.clone());
        }
        Ok(events)
    }

    /// Runtime availability override for one control (e.g. a view-level
    /// disabled state). Cancels in-flight gestures on unavailable.
    ///
    /// # Errors
    /// [`ViewError::UnknownComponent`] for unmounted/non-control keys.
    pub fn set_availability(
        &mut self,
        key: &ComponentKey,
        flags: AvailabilityFlags,
    ) -> Result<Vec<InteractionEvent>, ViewError> {
        let Some(m) = self.mounted.get(key) else {
            return Err(ViewError::UnknownComponent { key: key.clone() });
        };
        let Some(meta) = m.control.clone() else {
            return Err(ViewError::UnknownComponent { key: key.clone() });
        };
        let events = self
            .dispatcher
            .control_mut(&meta.control_id)
            .map(|c| c.set_availability(flags))
            .unwrap_or_default();
        for event in &events {
            self.emit_gesture(key, event.clone());
        }
        Ok(events)
    }

    /// Current display snapshot for a mounted component.
    #[must_use]
    pub fn observe(&self, key: &ComponentKey) -> Option<ComponentSnapshot> {
        let m = self.mounted.get(key)?;
        let control = m
            .control
            .as_ref()
            .and_then(|c| self.dispatcher.control(&c.control_id));
        let binding = m
            .control
            .as_ref()
            .and_then(|c| self.bindings.get(&c.control_id));
        Some(ComponentSnapshot {
            display: control.map(crate::behavior::InteractionControl::value),
            committed: control.map(crate::behavior::InteractionControl::committed_value),
            phase: binding.map_or(CommandPhase::Idle, CommandBinding::phase),
            observed: binding.and_then(|b| b.state().last_observed),
            mounted: true,
        })
    }

    /// The per-control command binding, when one exists.
    #[must_use]
    pub fn binding(&self, control_id: &str) -> Option<&CommandBinding> {
        self.bindings.get(control_id)
    }

    /// Mutable access to a per-control binding (receipts/projections).
    pub fn binding_mut(&mut self, control_id: &str) -> Option<&mut CommandBinding> {
        self.bindings.get_mut(control_id)
    }

    /// Routes a consumer receipt to the binding that minted the
    /// command. The IPC envelope correlates by `command_id` alone, so
    /// the dispatch-time index resolves the owning control. Returns
    /// `false` when no index entry or binding exists for the id
    /// (unknown, or stale after detach/unregister). A receipt that
    /// *routes* but fails the binding's generation/dedup guards is
    /// absorbed inside `on_receipt` and still returns `true`.
    pub fn apply_receipt(&mut self, receipt: &CommandReceipt) -> bool {
        let Some(owner) = self.command_index.get(&receipt.command_id).cloned() else {
            return false;
        };
        let Some(binding) = self.bindings.get_mut(&owner) else {
            return false;
        };
        binding.on_receipt(receipt);
        if matches!(
            receipt.status,
            ReceiptStatus::Rejected | ReceiptStatus::Unsupported
        ) {
            self.command_index.remove(&receipt.command_id);
        }
        true
    }

    /// Routes an authoritative state projection to `control_id`'s
    /// binding. The wire payload is opaque JSON — adapters decode the
    /// consumer's projection envelope into this call. Returns `false`
    /// when no binding exists for `control_id` (unmounted or
    /// never-registered). Projections correlate by `control_id`
    /// alone — they carry no epoch-scoped id, so a fresh binding
    /// (e.g. after remount) accepts any *first* revision; a delayed
    /// older-revision projection can land once and self-heals on the
    /// next newer one.
    ///
    /// An accepted projection also drives the controlled display:
    /// [`crate::behavior::projection_value_input`] decodes the
    /// payload into an absolute [`crate::behavior::ValueInput`] and
    /// [`crate::behavior::InteractionControl::apply_external`] writes
    /// it through the control's mapper — staged while a gesture is
    /// active (preview wins), committed immediately while idle, and
    /// silent (no events, no intents) in both cases. The three display
    /// states stay distinct: `display`/`committed` are the controlled
    /// display, receipts are `applied` progress, and `observed` is
    /// the authoritative [`StateProjection`] itself.
    pub fn apply_projection(
        &mut self,
        control_id: &str,
        projection: &StateProjection<serde_json::Value>,
    ) -> bool {
        let accepted = self
            .bindings
            .get_mut(control_id)
            .is_some_and(|b| b.on_projection(projection));
        if accepted
            && let Some(input) = crate::behavior::projection_value_input(&projection.value)
            && let Some(control) = self.dispatcher.control_mut(control_id)
        {
            control.apply_external(input);
        }
        accepted
    }

    /// Drains the recorded event stream.
    pub fn drain_events(&mut self) -> Vec<ViewEvent> {
        self.events.drain(..).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behavior::{InputSource, ValueInput};

    fn view_id() -> ViewInstanceId {
        ViewInstanceId::new(7).expect("nonzero")
    }

    fn fader(key: &str) -> ComponentSpec {
        ComponentSpec::Fader(ControlSpec {
            key: ComponentKey::from(key),
            control_id: None,
            target: Some(format!("mixer.{key}")),
            descriptor: ValueDescriptor {
                default_value: Some(50.0),
                ..ValueDescriptor::ranged(0.0, 100.0)
            },
            orientation: Orientation::Vertical,
            detach_policy: DetachPolicy::Cancel,
            unsupported: false,
            visual: ComponentVisualState::default(),
            a11y: A11ySpec {
                role: Some(A11yRole::Slider),
                label: Some(key.to_string()),
                ..Default::default()
            },
        })
    }

    fn panel(children: Vec<ComponentSpec>) -> ViewSpec {
        ViewSpec {
            root: ComponentSpec::Panel(PanelSpec {
                key: ComponentKey::from("root"),
                title: Some("root".to_string()),
                children,
                visual: ComponentVisualState::default(),
                a11y: A11ySpec {
                    role: Some(A11yRole::Group),
                    ..Default::default()
                },
            }),
        }
    }

    #[test]
    fn mount_update_unmount_ops() {
        let mut host = ViewHost::new(view_id(), AttachEpoch::new(1));
        let ops = host
            .mount(panel(vec![fader("a"), fader("b")]))
            .expect("mount");
        let kinds: Vec<&'static str> = ops
            .iter()
            .map(|op| match op {
                ProjectionOp::Mounted { .. } => "mounted",
                _ => "other",
            })
            .collect();
        assert_eq!(kinds, vec!["mounted", "mounted", "mounted"]);
        // Same-key prop change → Updated, identity preserved.
        let mut spec = panel(vec![fader("a"), fader("b")]);
        if let ComponentSpec::Panel(p) = &mut spec.root {
            p.title = Some("renamed".to_string());
        }
        let ops = host.update(spec, host.generation()).expect("update");
        assert!(matches!(ops[0], ProjectionOp::Updated { .. }));
        // Removed key.
        let ops = host
            .update(panel(vec![fader("a")]), host.generation())
            .expect("update");
        assert!(
            ops.iter()
                .any(|op| matches!(op, ProjectionOp::Removed { key } if key.0 == "b"))
        );
        let ops = host.unmount();
        assert!(
            ops.iter()
                .all(|op| matches!(op, ProjectionOp::Unmounted { .. }))
        );
        assert_eq!(host.mounted_keys(), [] as [ComponentKey; 0]);
    }

    #[test]
    fn stale_generation_rejected() {
        let mut host = ViewHost::new(view_id(), AttachEpoch::new(1));
        host.mount(panel(vec![fader("a")])).expect("mount");
        let stale = host.generation() + 99;
        let err = host
            .dispatch_input(
                stale,
                &ComponentKey::from("a"),
                ControlInput::Begin {
                    gesture_id: "g".to_string(),
                    source: InputSource::from(InputSource::POINTER),
                    origin_view: None,
                    target: None,
                    at: None,
                    modifiers: None,
                    seq: None,
                },
            )
            .unwrap_err();
        assert!(matches!(err, ViewError::StaleGeneration { .. }));
    }

    #[test]
    fn gesture_intent_flow() {
        let mut host = ViewHost::new(view_id(), AttachEpoch::new(1));
        host.mount(panel(vec![fader("a")])).expect("mount");
        let generation = host.generation();
        host.dispatch_input(
            generation,
            &ComponentKey::from("a"),
            ControlInput::Begin {
                gesture_id: "g1".to_string(),
                source: InputSource::from(InputSource::POINTER),
                origin_view: None,
                target: None,
                at: None,
                modifiers: None,
                seq: None,
            },
        )
        .expect("begin");
        host.dispatch_input(
            generation,
            &ComponentKey::from("a"),
            ControlInput::Preview {
                gesture_id: "g1".to_string(),
                value: ValueInput::Normalized(0.7),
                modifiers: None,
                seq: None,
            },
        )
        .expect("preview");
        let events = host
            .dispatch_input(
                generation,
                &ComponentKey::from("a"),
                ControlInput::Commit {
                    gesture_id: "g1".to_string(),
                    value: None,
                    modifiers: None,
                    seq: None,
                },
            )
            .expect("commit");
        assert!(matches!(events[0], InteractionEvent::Commit { .. }));
        let stream = host.drain_events();
        let intents: Vec<&CommandIntent> = stream
            .iter()
            .filter_map(|e| match e {
                ViewEvent::Intent { intent, .. } => Some(intent),
                _ => None,
            })
            .collect();
        assert_eq!(intents.len(), 3); // begin + preview + commit
        assert!(matches!(intents[2], CommandIntent::Commit { .. }));
    }

    #[test]
    fn remove_cancels_gesture() {
        let mut host = ViewHost::new(view_id(), AttachEpoch::new(1));
        host.mount(panel(vec![fader("a"), fader("b")]))
            .expect("mount");
        let generation = host.generation();
        host.dispatch_input(
            generation,
            &ComponentKey::from("a"),
            ControlInput::Begin {
                gesture_id: "g1".to_string(),
                source: InputSource::from(InputSource::POINTER),
                origin_view: None,
                target: None,
                at: None,
                modifiers: None,
                seq: None,
            },
        )
        .expect("begin");
        host.update(panel(vec![fader("b")]), generation)
            .expect("update");
        let stream = host.drain_events();
        // The removed control's gesture must be safely cancelled.
        assert!(stream.iter().any(|e| matches!(
            e,
            ViewEvent::Gesture {
                event: InteractionEvent::Cancel { .. },
                ..
            }
        )));
    }

    fn commit_intent_id(host: &mut ViewHost, key: &str, gesture: &str) -> CommandId {
        host.dispatch_input(
            host.generation(),
            &ComponentKey::from(key),
            ControlInput::Begin {
                gesture_id: gesture.to_string(),
                source: InputSource::from(InputSource::POINTER),
                origin_view: None,
                target: None,
                at: None,
                modifiers: None,
                seq: None,
            },
        )
        .expect("begin");
        host.dispatch_input(
            host.generation(),
            &ComponentKey::from(key),
            ControlInput::Commit {
                gesture_id: gesture.to_string(),
                value: None,
                modifiers: None,
                seq: None,
            },
        )
        .expect("commit");
        host.drain_events()
            .into_iter()
            .filter_map(|e| match e {
                ViewEvent::Intent {
                    intent: CommandIntent::Commit { .. },
                    command_id,
                    ..
                } => Some(command_id),
                _ => None,
            })
            .next_back()
            .expect("commit intent minted")
    }

    fn receipt(command_id: CommandId, status: ReceiptStatus) -> CommandReceipt {
        CommandReceipt {
            command_id,
            status,
            revision: Some(crate::behavior::Revision::Num(1)),
            detail: None,
        }
    }

    #[test]
    fn remount_absorbs_stale_receipts() {
        // Same stable key + ViewInstance across unmount/remount: the
        // new mount epoch must mint distinct command ids so a
        // pre-unmount receipt can never advance the new epoch's head.
        let mut host = ViewHost::new(view_id(), AttachEpoch::new(1));
        host.mount(panel(vec![fader("a")])).expect("mount");
        let old_commit = commit_intent_id(&mut host, "a", "g1");
        host.unmount();
        host.mount(panel(vec![fader("a")])).expect("remount");
        let new_commit = commit_intent_id(&mut host, "a", "g2");

        assert_ne!(old_commit, new_commit);
        // The pre-unmount receipt is stale: index miss → absorbed,
        // the new head stays pending rather than being faked applied.
        assert!(!host.apply_receipt(&receipt(old_commit.clone(), ReceiptStatus::Applied)));
        assert_eq!(
            host.binding("a").expect("binding").phase(),
            CommandPhase::Pending
        );
        assert_eq!(
            host.binding("a").expect("binding").state().last_receipt,
            None
        );

        // The receipt minted under the new epoch still settles normally.
        assert!(host.apply_receipt(&receipt(new_commit, ReceiptStatus::Applied)));
        assert_eq!(
            host.binding("a").expect("binding").phase(),
            CommandPhase::Applied
        );
    }

    #[test]
    fn update_reregister_keeps_binding_epoch() {
        // Removed + re-added through `update` is NOT a remount: the
        // binding survives, its counter keeps minting unique ids in
        // the same epoch, and receipts for the re-added control's
        // fresh commands still settle. The epoch boundary is
        // `unmount`, not per-registration.
        let mut host = ViewHost::new(view_id(), AttachEpoch::new(1));
        host.mount(panel(vec![fader("a"), fader("b")]))
            .expect("mount");
        let first_commit = commit_intent_id(&mut host, "a", "g1");
        let generation = host.generation();
        host.update(panel(vec![fader("b")]), generation)
            .expect("remove a");
        host.update(panel(vec![fader("a"), fader("b")]), generation)
            .expect("re-add a");
        let second_commit = commit_intent_id(&mut host, "a", "g2");

        assert_ne!(first_commit, second_commit);
        // Same epoch → namespace segment stays `m1`; the counter
        // continues (begin `:3`, commit `:4`) rather than restarting.
        assert!(second_commit.ends_with(":4"), "got {second_commit}");
        // Pre-removal receipt was index-cleared at unregister → stale.
        assert!(!host.apply_receipt(&receipt(first_commit, ReceiptStatus::Applied)));
        // The re-registered control's own receipt settles normally.
        assert!(host.apply_receipt(&receipt(second_commit, ReceiptStatus::Applied)));
        assert_eq!(
            host.binding("a").expect("binding").phase(),
            CommandPhase::Applied
        );
    }

    #[test]
    fn fresh_host_same_logical_id_absorbs_stale_receipts() {
        // Drop → recreate a ViewHost for the same logical
        // ViewInstanceId (layout restore / move): the runtime
        // instances are distinct epochs, so a receipt minted against
        // the dead host can never resolve inside the new one.
        let mut epochs = AttachEpochAllocator::new();
        let mut first = ViewHost::new(view_id(), epochs.mint());
        first.mount(panel(vec![fader("a")])).expect("mount");
        let dead_commit = commit_intent_id(&mut first, "a", "g1");
        drop(first);

        let mut second = ViewHost::new(view_id(), epochs.mint());
        second.mount(panel(vec![fader("a")])).expect("mount");
        let live_commit = commit_intent_id(&mut second, "a", "g2");

        assert_ne!(dead_commit, live_commit);
        assert!(!second.apply_receipt(&receipt(dead_commit, ReceiptStatus::Applied)));
        assert_eq!(
            second.binding("a").expect("binding").phase(),
            CommandPhase::Pending
        );
        assert!(second.apply_receipt(&receipt(live_commit, ReceiptStatus::Applied)));
        assert_eq!(
            second.binding("a").expect("binding").phase(),
            CommandPhase::Applied
        );
    }

    #[test]
    fn attach_epoch_allocator_mints_monotonic_epochs() {
        let mut epochs = AttachEpochAllocator::new();
        let a = epochs.mint();
        let b = epochs.mint();
        let c = epochs.mint();
        assert_eq!((a.get(), b.get(), c.get()), (1, 2, 3));
    }

    #[test]
    fn update_on_unmounted_host_cannot_remount_dead_epoch() {
        // `update` is not a remount entry — an unmounted host has no
        // tree to reconcile, so the caller must go through `mount`
        // (the epoch boundary). Without the guard a current-gen
        // `update` silently remounted under the dead mount epoch,
        // re-minting the same command ids so stale receipts
        // correlated onto live bindings again.
        let mut host = ViewHost::new(view_id(), AttachEpoch::new(1));
        host.mount(panel(vec![fader("a")])).expect("mount");
        let dead_commit = commit_intent_id(&mut host, "a", "g1");
        host.unmount();

        // A caller holding the *current* generation still cannot
        // resurrect the dead namespace through `update`.
        let err = host
            .update(panel(vec![fader("a")]), host.generation())
            .expect_err("update on an unmounted host must fail");
        assert!(matches!(err, ViewError::NotMounted));

        // A virgin host cannot `update`-mount under epoch 0 either —
        // the first mount must go through `mount` so the namespace
        // epoch is established explicitly.
        let mut virgin = ViewHost::new(view_id(), AttachEpoch::new(2));
        let err = virgin
            .update(panel(vec![fader("a")]), virgin.generation())
            .expect_err("update on a never-mounted host must fail");
        assert!(matches!(err, ViewError::NotMounted));

        // The only remount path is `mount`: bumped epoch, fresh
        // namespace — the dead mount's receipt still cannot resolve.
        host.mount(panel(vec![fader("a")])).expect("remount");
        let live_commit = commit_intent_id(&mut host, "a", "g2");
        assert_ne!(dead_commit, live_commit);
        assert!(!host.apply_receipt(&receipt(dead_commit, ReceiptStatus::Applied)));
        assert!(host.apply_receipt(&receipt(live_commit, ReceiptStatus::Applied)));
    }

    fn projection(value: serde_json::Value, revision: u64) -> StateProjection<serde_json::Value> {
        StateProjection {
            revision: crate::behavior::Revision::Num(revision),
            value,
            provenance: None,
        }
    }

    fn display_plain(host: &ViewHost, key: &str) -> f64 {
        host.observe(&ComponentKey::from(key))
            .and_then(|s| s.display)
            .expect("display")
            .plain
    }

    #[test]
    fn projection_updates_controlled_display_when_idle() {
        let mut host = ViewHost::new(view_id(), AttachEpoch::new(1));
        host.mount(panel(vec![fader("a")])).expect("mount");
        let key = ComponentKey::from("a");
        assert_eq!(display_plain(&host, "a"), 50.0);
        // Idle external nonzero update: observed lands AND the
        // controlled display follows — the renderer's
        // `snap.display.normalized` now paints the truth.
        assert!(host.apply_projection("a", &projection(serde_json::json!({"plain": 80.0}), 1)));
        let snap = host.observe(&key).expect("snapshot");
        assert!((snap.display.expect("display").plain - 80.0).abs() < 1e-9);
        assert!((snap.committed.expect("committed").plain - 80.0).abs() < 1e-9);
        assert!(snap.observed.is_some());
        assert_eq!(snap.phase, CommandPhase::Observed);
        // The write is silent — no gesture events, no intents (the
        // controlled-write loop stays closed).
        assert!(
            !host
                .drain_events()
                .iter()
                .any(|e| matches!(e, ViewEvent::Intent { .. } | ViewEvent::Gesture { .. }))
        );
    }

    #[test]
    fn projection_defers_to_gesture_preview() {
        let mut host = ViewHost::new(view_id(), AttachEpoch::new(1));
        host.mount(panel(vec![fader("a")])).expect("mount");
        let generation = host.generation();
        let key = ComponentKey::from("a");
        host.dispatch_input(
            generation,
            &key,
            ControlInput::Begin {
                gesture_id: "g1".to_string(),
                source: InputSource::from(InputSource::POINTER),
                origin_view: None,
                target: None,
                at: None,
                modifiers: None,
                seq: None,
            },
        )
        .expect("begin");
        host.dispatch_input(
            generation,
            &key,
            ControlInput::Preview {
                gesture_id: "g1".to_string(),
                value: ValueInput::Normalized(0.7),
                modifiers: None,
                seq: None,
            },
        )
        .expect("preview");
        // An authoritative write mid-gesture is accepted into
        // `observed` but the preview keeps winning the display.
        assert!(host.apply_projection("a", &projection(serde_json::json!({"plain": 30.0}), 1)));
        assert!((display_plain(&host, "a") - 70.0).abs() < 1e-9);
        let snap = host.observe(&key).expect("snapshot");
        assert!(snap.observed.is_some());
        // Commit: the staged truth lands as the committed display —
        // the last observed state wins once the gesture ends.
        host.dispatch_input(
            generation,
            &key,
            ControlInput::Commit {
                gesture_id: "g1".to_string(),
                value: None,
                modifiers: None,
                seq: None,
            },
        )
        .expect("commit");
        assert!((display_plain(&host, "a") - 30.0).abs() < 1e-9);
    }

    #[test]
    fn projection_lands_after_gesture_cancel() {
        let mut host = ViewHost::new(view_id(), AttachEpoch::new(1));
        host.mount(panel(vec![fader("a")])).expect("mount");
        let generation = host.generation();
        let key = ComponentKey::from("a");
        host.dispatch_input(
            generation,
            &key,
            ControlInput::Begin {
                gesture_id: "g1".to_string(),
                source: InputSource::from(InputSource::POINTER),
                origin_view: None,
                target: None,
                at: None,
                modifiers: None,
                seq: None,
            },
        )
        .expect("begin");
        host.dispatch_input(
            generation,
            &key,
            ControlInput::Cancel {
                gesture_id: "g1".to_string(),
                reason: None,
                seq: None,
            },
        )
        .expect("cancel");
        // Cancel restores the committed value; with the gesture over,
        // the next projection applies immediately while idle.
        assert!(host.apply_projection("a", &projection(serde_json::json!({"plain": 30.0}), 1)));
        assert!((display_plain(&host, "a") - 30.0).abs() < 1e-9);
    }

    #[test]
    fn projection_staged_during_cancelled_gesture_lands() {
        let mut host = ViewHost::new(view_id(), AttachEpoch::new(1));
        host.mount(panel(vec![fader("a")])).expect("mount");
        let generation = host.generation();
        let key = ComponentKey::from("a");
        host.dispatch_input(
            generation,
            &key,
            ControlInput::Begin {
                gesture_id: "g1".to_string(),
                source: InputSource::from(InputSource::POINTER),
                origin_view: None,
                target: None,
                at: None,
                modifiers: None,
                seq: None,
            },
        )
        .expect("begin");
        host.dispatch_input(
            generation,
            &key,
            ControlInput::Preview {
                gesture_id: "g1".to_string(),
                value: ValueInput::Normalized(0.7),
                modifiers: None,
                seq: None,
            },
        )
        .expect("preview");
        assert!(host.apply_projection("a", &projection(serde_json::json!({"plain": 30.0}), 1)));
        assert!((display_plain(&host, "a") - 70.0).abs() < 1e-9);
        host.dispatch_input(
            generation,
            &key,
            ControlInput::Cancel {
                gesture_id: "g1".to_string(),
                reason: None,
                seq: None,
            },
        )
        .expect("cancel");
        // The staged truth lands on cancel — the display shows the
        // last observed value, not the aborted preview nor the stale
        // pre-gesture committed.
        assert!((display_plain(&host, "a") - 30.0).abs() < 1e-9);
    }

    #[test]
    fn stale_projection_leaves_display_untouched() {
        let mut host = ViewHost::new(view_id(), AttachEpoch::new(1));
        host.mount(panel(vec![fader("a")])).expect("mount");
        assert!(host.apply_projection("a", &projection(serde_json::json!({"plain": 80.0}), 2)));
        assert!((display_plain(&host, "a") - 80.0).abs() < 1e-9);
        // Older and equal revisions are dropped before touching the
        // display — a delayed answer never overwrites a newer truth.
        assert!(!host.apply_projection("a", &projection(serde_json::json!({"plain": 10.0}), 1)));
        assert!(!host.apply_projection("a", &projection(serde_json::json!({"plain": 90.0}), 2)));
        assert!((display_plain(&host, "a") - 80.0).abs() < 1e-9);
        assert!(host.apply_projection("a", &projection(serde_json::json!({"plain": 90.0}), 3)));
        assert!((display_plain(&host, "a") - 90.0).abs() < 1e-9);
    }

    #[test]
    fn undecodable_projection_records_observed_only() {
        let mut host = ViewHost::new(view_id(), AttachEpoch::new(1));
        host.mount(panel(vec![fader("a")])).expect("mount");
        let key = ComponentKey::from("a");
        // A non-control-value payload is still authoritative truth:
        // the binding records `observed`, but the display is not
        // guessed at — no unconditional `f64` interpretation.
        assert!(host.apply_projection("a", &projection(serde_json::json!("loudness"), 1)));
        let snap = host.observe(&key).expect("snapshot");
        assert!(snap.observed.is_some());
        assert_eq!(snap.phase, CommandPhase::Observed);
        assert!((display_plain(&host, "a") - 50.0).abs() < 1e-9);
    }

    #[test]
    fn detached_binding_absorbs_projection_until_reattach() {
        let mut host = ViewHost::new(view_id(), AttachEpoch::new(1));
        host.mount(panel(vec![fader("a")])).expect("mount");
        assert!(host.apply_projection("a", &projection(serde_json::json!({"plain": 80.0}), 1)));
        host.detach("dock-transfer");
        // Detached: the projection is absorbed — no observed write,
        // no display write.
        assert!(!host.apply_projection("a", &projection(serde_json::json!({"plain": 90.0}), 2)));
        assert!((display_plain(&host, "a") - 80.0).abs() < 1e-9);
        host.attach();
        // Reattach: the next advancing projection lands normally.
        assert!(host.apply_projection("a", &projection(serde_json::json!({"plain": 95.0}), 3)));
        assert!((display_plain(&host, "a") - 95.0).abs() < 1e-9);
    }

    #[test]
    fn reregister_restores_last_observed_display() {
        let mut host = ViewHost::new(view_id(), AttachEpoch::new(1));
        host.mount(panel(vec![fader("a"), fader("b")]))
            .expect("mount");
        assert!(host.apply_projection("a", &projection(serde_json::json!({"plain": 80.0}), 1)));
        assert!((display_plain(&host, "a") - 80.0).abs() < 1e-9);
        let generation = host.generation();
        // Remove + re-add through `update`: the binding survives with
        // its last observed projection, and the fresh control
        // restores the display through the (unchanged) mapper.
        host.update(panel(vec![fader("b")]), generation)
            .expect("remove a");
        host.update(panel(vec![fader("a"), fader("b")]), generation)
            .expect("re-add a");
        assert!((display_plain(&host, "a") - 80.0).abs() < 1e-9);
    }

    fn toggle(key: &str) -> ComponentSpec {
        ComponentSpec::Toggle(ControlSpec {
            key: ComponentKey::from(key),
            control_id: None,
            target: Some(format!("mix.{key}.on")),
            descriptor: ValueDescriptor {
                default_value: Some(0.0),
                step: Some(1.0),
                ..ValueDescriptor::ranged(0.0, 1.0)
            },
            orientation: Orientation::Horizontal,
            detach_policy: DetachPolicy::Cancel,
            unsupported: false,
            visual: ComponentVisualState::default(),
            a11y: A11ySpec {
                role: Some(A11yRole::Switch),
                label: Some(key.to_string()),
                ..Default::default()
            },
        })
    }

    fn meter(key: &str) -> ComponentSpec {
        ComponentSpec::Meter(MeterSpec {
            key: ComponentKey::from(key),
            channels: 1,
            channel_levels: vec![vec![MeterBin {
                start: 0,
                end: 480,
                peak: 0.8,
                rms: 0.5,
                count: 480,
            }]],
            viewport: 0..480,
            visual: ComponentVisualState::default(),
            a11y: A11ySpec {
                role: Some(A11yRole::Img),
                label: Some(key.to_string()),
                ..Default::default()
            },
        })
    }

    fn time_axis(key: &str) -> ComponentSpec {
        ComponentSpec::TimeAxis(TimeAxisSpec {
            key: ComponentKey::from(key),
            viewport: 0..480,
            ticks: vec![
                AxisTick {
                    at: 0.0,
                    label: "0:00".into(),
                    major: true,
                },
                AxisTick {
                    at: 240.0,
                    label: "1 bar".into(),
                    major: false,
                },
                AxisTick {
                    at: 479.5,
                    label: "end".into(),
                    major: true,
                },
            ],
            visual: ComponentVisualState::default(),
            a11y: A11ySpec {
                role: Some(A11yRole::Img),
                label: Some(key.to_string()),
                ..Default::default()
            },
        })
    }

    #[test]
    fn new_kinds_mount() {
        let mut host = ViewHost::new(view_id(), AttachEpoch::new(1));
        let knob = ComponentSpec::Knob(ControlSpec {
            key: ComponentKey::from("cutoff"),
            control_id: None,
            target: Some("filter.cutoff".into()),
            descriptor: ValueDescriptor {
                default_value: Some(0.5),
                ..ValueDescriptor::ranged(0.0, 1.0)
            },
            orientation: Orientation::Vertical,
            detach_policy: DetachPolicy::Cancel,
            unsupported: false,
            visual: ComponentVisualState::default(),
            a11y: A11ySpec {
                role: Some(A11yRole::Slider),
                label: Some("cutoff".into()),
                ..Default::default()
            },
        });
        host.mount(panel(vec![
            knob,
            toggle("gate"),
            meter("lvl"),
            time_axis("ruler"),
        ]))
        .expect("mount");
        let keys = host.mounted_keys();
        assert_eq!(keys.len(), 5);
        // Every mounted key observes a snapshot — controls carry a
        // display, viz kinds carry none but stay observable.
        for key in &keys {
            assert!(host.observe(key).is_some(), "{key} observable");
        }
        // The spec tree round-trips the new kinds with stable keys.
        let root = host.spec().expect("spec").root.clone();
        if let ComponentSpec::Panel(p) = root {
            let kinds: Vec<&'static str> = p.children.iter().map(ComponentSpec::kind).collect();
            assert_eq!(kinds, vec!["knob", "toggle", "meter", "time_axis"]);
        } else {
            panic!("root must be panel");
        }
    }

    #[test]
    fn toggle_requires_binary_descriptor() {
        let mut host = ViewHost::new(view_id(), AttachEpoch::new(1));
        let bad = ComponentSpec::Toggle(ControlSpec {
            key: ComponentKey::from("bad"),
            control_id: None,
            target: None,
            descriptor: ValueDescriptor::ranged(0.0, 10.0),
            orientation: Orientation::Horizontal,
            detach_policy: DetachPolicy::Cancel,
            unsupported: false,
            visual: ComponentVisualState::default(),
            a11y: A11ySpec::default(),
        });
        let err = host
            .mount(panel(vec![bad]))
            .expect_err("reject non-binary toggle");
        assert!(matches!(err, ViewError::InvalidSpec { .. }));
    }

    #[test]
    fn meter_and_time_axis_reject_unbounded_payloads() {
        let mut host = ViewHost::new(view_id(), AttachEpoch::new(1));
        let too_many_ticks = ComponentSpec::TimeAxis(TimeAxisSpec {
            key: ComponentKey::from("ruler"),
            viewport: 0..1,
            ticks: (0..=MAX_TIME_AXIS_TICKS)
                .map(|_| AxisTick {
                    at: 0.0,
                    label: String::new(),
                    major: false,
                })
                .collect(),
            visual: ComponentVisualState::default(),
            a11y: A11ySpec::default(),
        });
        assert!(matches!(
            host.mount(panel(vec![too_many_ticks])),
            Err(ViewError::InvalidSpec { .. })
        ));
        let nan_tick = ComponentSpec::TimeAxis(TimeAxisSpec {
            key: ComponentKey::from("ruler"),
            viewport: 0..1,
            ticks: vec![AxisTick {
                at: f64::NAN,
                label: "x".into(),
                major: true,
            }],
            visual: ComponentVisualState::default(),
            a11y: A11ySpec::default(),
        });
        assert!(matches!(
            host.mount(panel(vec![nan_tick])),
            Err(ViewError::InvalidSpec { .. })
        ));
        // Meter channel mismatch.
        let mut m = meter("lvl");
        if let ComponentSpec::Meter(spec) = &mut m {
            spec.channel_levels.push(vec![]);
        }
        assert!(matches!(
            host.mount(panel(vec![m])),
            Err(ViewError::InvalidSpec { .. })
        ));
    }

    fn label(key: &str, text: &str) -> ComponentSpec {
        ComponentSpec::Label(LabelSpec {
            key: ComponentKey::from(key),
            text: text.into(),
            visual: ComponentVisualState::default(),
            a11y: A11ySpec::default(),
        })
    }

    fn table(key: &str, rows: Vec<Vec<Option<CellValue>>>) -> ComponentSpec {
        ComponentSpec::Table(TableSpec {
            key: ComponentKey::from(key),
            columns: vec!["stage".into(), "state".into(), "ms".into()],
            rows,
            visual: ComponentVisualState::default(),
            a11y: A11ySpec {
                label: Some(key.into()),
                ..A11ySpec::default()
            },
        })
    }

    fn spectrogram(key: &str, rows: usize, columns: Vec<SpectrogramColumn>) -> ComponentSpec {
        ComponentSpec::Spectrogram(SpectrogramSpec {
            key: ComponentKey::from(key),
            viewport: 0..200,
            rows,
            magnitude_range: 0.0..1.0,
            columns,
            visual: ComponentVisualState::default(),
            a11y: A11ySpec {
                role: Some(A11yRole::Img),
                label: Some(key.to_string()),
                ..Default::default()
            },
        })
    }

    fn cell_texts(row: &A11yNode) -> Vec<Option<String>> {
        row.children
            .iter()
            .map(|c| c.value.as_ref().map(ToString::to_string))
            .collect()
    }

    #[test]
    fn table_keeps_row_and_column_order_and_missing_cells() {
        let mut host = ViewHost::new(view_id(), AttachEpoch::new(1));
        let rows = vec![
            vec![
                Some(CellValue::Text("decode".into())),
                None,
                Some(CellValue::Number(12.5)),
            ],
            vec![
                None,
                Some(CellValue::Text("running".into())),
                Some(CellValue::Number(3.0)),
            ],
        ];
        host.mount(panel(vec![table("events", rows)]))
            .expect("mount");
        let node = host
            .a11y_node(&ComponentKey::from("events"))
            .expect("table a11y");
        assert_eq!(node.role, A11yRole::Table);
        assert_eq!(node.name.as_deref(), Some("events"));
        let [header, first, second] = node.children.as_slice() else {
            panic!("header row plus two data rows");
        };
        assert!(
            header
                .children
                .iter()
                .all(|c| c.role == A11yRole::ColumnHeader)
        );
        assert_eq!(
            cell_texts(header),
            vec![
                Some("stage".into()),
                Some("state".into()),
                Some("ms".into())
            ]
        );
        assert_eq!(
            cell_texts(first),
            vec![Some("decode".into()), None, Some("12.5".into())]
        );
        assert_eq!(
            cell_texts(second),
            vec![None, Some("running".into()), Some("3".into())]
        );
        assert!(first.children.iter().all(|c| c.role == A11yRole::Cell));
    }

    #[test]
    fn table_rejects_bad_arity_non_finite_and_oversized_payloads() {
        let mut host = ViewHost::new(view_id(), AttachEpoch::new(1));
        let short_row = table("t", vec![vec![None, None]]);
        assert!(matches!(
            host.mount(panel(vec![short_row])),
            Err(ViewError::InvalidSpec { .. })
        ));
        let nan_cell = table(
            "t",
            vec![vec![None, None, Some(CellValue::Number(f64::NAN))]],
        );
        assert!(matches!(
            host.mount(panel(vec![nan_cell])),
            Err(ViewError::InvalidSpec { .. })
        ));
        let too_many_rows = table(
            "t",
            (0..=MAX_TABLE_ROWS)
                .map(|_| vec![None, None, None])
                .collect(),
        );
        assert!(matches!(
            host.mount(panel(vec![too_many_rows])),
            Err(ViewError::InvalidSpec { .. })
        ));
    }

    #[test]
    fn label_a11y_defaults_to_text_role_and_keeps_text() {
        let node = label("status", "Running").a11y_node().expect("label a11y");
        assert_eq!(node.role, A11yRole::Text);
        assert_eq!(node.value, Some(CellValue::Text("Running".into())));
        assert_eq!(label("status", "x").kind(), "label");
        assert_eq!(table("events", vec![]).kind(), "table");
    }

    #[test]
    fn mounted_label_and_table_observe_and_reconcile_by_key() {
        let mut host = ViewHost::new(view_id(), AttachEpoch::new(1));
        host.mount(panel(vec![
            label("status", "Running"),
            table("events", vec![]),
        ]))
        .expect("mount");
        assert!(host.observe(&ComponentKey::from("events")).is_some());
        assert!(host.a11y_node(&ComponentKey::from("status")).is_some());
        assert!(host.a11y_node(&ComponentKey::from("missing")).is_none());
        let generation = host.generation();
        let ops = host
            .update(panel(vec![label("status", "Done")]), generation)
            .expect("reconcile");
        assert!(ops.contains(&ProjectionOp::Removed {
            key: ComponentKey::from("events"),
        }));
    }

    #[test]
    fn spectrogram_keeps_gap_cells_and_rejects_unbounded_payloads() {
        let column = |start, end, count, magnitudes: Vec<f64>| SpectrogramColumn {
            start,
            end,
            count,
            magnitudes,
        };
        let mut host = ViewHost::new(view_id(), AttachEpoch::new(1));
        host.mount(panel(vec![spectrogram(
            "spec",
            2,
            vec![
                column(0, 100, 100, vec![f64::NAN, 0.5]),
                column(100, 200, 0, vec![0.0, 0.0]),
            ],
        )]))
        .expect("NaN magnitude and count==0 column are valid payload");
        let ComponentSpec::Panel(p) = host.spec().expect("spec").root.clone() else {
            panic!("root must be panel");
        };
        let ComponentSpec::Spectrogram(s) = &p.children[0] else {
            panic!("spectrogram child");
        };
        assert!(s.columns[0].magnitudes[0].is_nan(), "NaN gap is retained");
        assert_eq!(s.columns[1].count, 0, "count==0 column is retained");

        let bad_rows = spectrogram("spec", 2, vec![column(0, 1, 1, vec![0.5])]);
        assert!(matches!(
            host.mount(panel(vec![bad_rows])),
            Err(ViewError::InvalidSpec { .. })
        ));
        let too_many_cells = spectrogram(
            "spec",
            1,
            (0..=MAX_SPECTROGRAM_CELLS)
                .map(|i| column(i as u64, i as u64 + 1, 1, vec![0.0]))
                .collect(),
        );
        assert!(matches!(
            host.mount(panel(vec![too_many_cells])),
            Err(ViewError::InvalidSpec { .. })
        ));
        let mut inverted = spectrogram("spec", 1, vec![]);
        if let ComponentSpec::Spectrogram(s) = &mut inverted {
            s.magnitude_range = 1.0..1.0;
        }
        assert!(matches!(
            host.mount(panel(vec![inverted])),
            Err(ViewError::InvalidSpec { .. })
        ));
    }
}
