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
    /// Non-interactive visualization (Waveform).
    Img,
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
    /// Resolved-waveform visualization.
    Waveform(WaveformSpec),
    /// Native-surface region — layout bounds feed the slot port.
    #[serde(rename = "native_slot")]
    NativeSlot(NativeSlotSpec),
}

impl ComponentSpec {
    /// The component's stable key.
    #[must_use]
    pub fn key(&self) -> &ComponentKey {
        match self {
            Self::Panel(s) => &s.key,
            Self::Tabs(s) => &s.key,
            Self::Fader(s) | Self::Seek(s) => &s.key,
            Self::Waveform(s) => &s.key,
            Self::NativeSlot(s) => &s.key,
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
            Self::Waveform(_) => "waveform",
            Self::NativeSlot(_) => "native_slot",
        }
    }

    /// Control spec for continuous controls (`Fader`/`Seek`).
    #[must_use]
    pub fn control(&self) -> Option<&ControlSpec> {
        match self {
            Self::Fader(s) | Self::Seek(s) => Some(s),
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
    generation: u64,
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
    /// Creates an empty host for `view_id` (detached, generation 0).
    #[must_use]
    pub fn new(view_id: ViewInstanceId) -> Self {
        Self {
            view_id,
            generation: 0,
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
    /// `update`/`dispatch_input`; adapters receive it from the last
    /// `mount`/`detach`/`attach` result.
    #[must_use]
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// The current spec, if mounted.
    #[must_use]
    pub fn spec(&self) -> Option<&ViewSpec> {
        self.spec.as_ref()
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
                ComponentSpec::NativeSlot(n) => {
                    if let Some([w, h]) = n.min_size
                        && (!w.is_finite() || !h.is_finite() || w < 0.0 || h < 0.0)
                    {
                        return Err(ViewError::InvalidSpec {
                            detail: format!("native_slot {} min_size invalid", n.key),
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
        }
        Ok(flat)
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
        self.bindings.entry(control_id.clone()).or_insert_with(|| {
            CommandBinding::new(format!("view-{}:{}", self.view_id.get(), control_id))
        });
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
            let binding = self.bindings.entry(control_id.clone()).or_insert_with(|| {
                CommandBinding::new(format!("view-{}:{}", self.view_id.get(), control_id))
            });
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
    /// [`ViewError::InvalidSpec`] when the new spec fails validation.
    /// On error the mounted tree is untouched.
    pub fn update(
        &mut self,
        spec: ViewSpec,
        generation: u64,
    ) -> Result<Vec<ProjectionOp>, ViewError> {
        self.check_gen(generation)?;
        let flat = Self::validate_spec(&spec)?;
        let mut ops = Vec::new();
        let next_keys: std::collections::HashSet<ComponentKey> =
            flat.iter().map(|(k, _)| (*k).clone()).collect();

        // Removed keys first — detach semantics must not be confused
        // with an update on the surviving tree.
        let removed: Vec<ComponentKey> = self
            .mounted
            .keys()
            .filter(|k| !next_keys.contains(*k))
            .cloned()
            .collect();
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
        let phase = m
            .control
            .as_ref()
            .and_then(|c| self.bindings.get(&c.control_id))
            .map_or(CommandPhase::Idle, CommandBinding::phase);
        Some(ComponentSnapshot {
            display: control.map(crate::behavior::InteractionControl::value),
            committed: control.map(crate::behavior::InteractionControl::committed_value),
            phase,
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
    /// `false` for unknown or stale ids (late replies after detach or
    /// unregister are absorbed without effect).
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
    /// when no binding exists (unmounted control, stale view epoch).
    pub fn apply_projection(
        &mut self,
        control_id: &str,
        projection: &StateProjection<serde_json::Value>,
    ) -> bool {
        self.bindings
            .get_mut(control_id)
            .is_some_and(|b| b.on_projection(projection))
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
        let mut host = ViewHost::new(view_id());
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
        let mut host = ViewHost::new(view_id());
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
        let mut host = ViewHost::new(view_id());
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
        let mut host = ViewHost::new(view_id());
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
}
