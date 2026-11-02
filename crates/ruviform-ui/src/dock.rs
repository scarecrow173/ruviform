//! Dock tree model, cross-window view move contract, window-close
//! policy vocabulary, and layout persistence (Issue #182).
//!
//! A `DockWorkspace` attaches at most one [`DockTree`] to each
//! registered [`WindowId`]. A tree is a recursive [`DockNode`]:
//! `Split(axis, ratio, children)` arranges two or more children along
//! an axis; `Tabs(views, active)` is a leaf holding one or more
//! logical [`ViewInstanceId`]s. Views are consumer-issued logical
//! identities — independent of any OS window or DOM entity — whose
//! **owner is at most one place across every window's tree**; a
//! duplicate requires a new id. Views not docked anywhere live in the
//! workspace's `detached` pool (still exactly one logical identity).
//!
//! **Move contract:** moving a view between trees is a two-phase
//! handoff — [`DockWorkspace::begin_move`] marks the view in-transfer
//! (all further edits rejected as [`DockError::ViewBusy`]) while the
//! destination window/webview is prepared, then
//! [`DockWorkspace::commit_move`] switches ownership atomically.
//! Failure rolls back to the source dock — the view never left it.
//! [`DockWorkspace::move_view`] is the one-shot form used by
//! menu-driven moves and in-window drops (no drag machinery needed —
//! the explicit menu-move fallback path). View *state* save/restore
//! is the consumer's job at the `MoveCommitted` event; DOM transfer
//! across webviews is never guaranteed.
//!
//! **Persistence:** [`LayoutSnapshot`] is a versioned save schema for
//! window geometry (physical pixels, virtual-desktop coordinates),
//! monitor signatures (never persistent display ids), and dock trees.
//! [`LayoutSnapshot::restore`] validates structure and corrects
//! off-screen/missing-monitor geometry; failures fall back to a safe
//! empty layout via [`LayoutSnapshot::restore_or_default`].
//!
//! **Window-close policy:** what happens to docked views when a
//! window closes is the consumer's explicit choice —
//! [`DockWorkspace::remove_window`] requires a
//! [`WindowCloseDisposition`]. Nothing here ever cancels jobs or
//! closes a project.
//!
//! **RT contract:** non-RT only — produced and consumed on the UI
//! thread.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::num::NonZeroU64;

use serde::{Deserialize, Serialize};

use crate::window::{MonitorInfo, WindowId};

// ---------------------------------------------------------------------------
// Identity
// ---------------------------------------------------------------------------

/// Logical view identifier, issued by the consumer.
///
/// Unlike [`WindowId`] (host-issued, session-scoped), a
/// `ViewInstanceId` is minted by the consumer and survives moves,
/// detach/reattach, and layout save/restore — it names a logical
/// view, not an OS window or DOM entity. At most one owner (a dock
/// leaf or the detached pool) may hold a given id at any time;
/// duplication requires minting a new id.
///
/// **RT contract:** non-RT only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ViewInstanceId(NonZeroU64);

impl ViewInstanceId {
    /// Wraps a raw value as a [`ViewInstanceId`]; `0` is rejected.
    #[must_use]
    pub const fn new(raw: u64) -> Option<Self> {
        match NonZeroU64::new(raw) {
            Some(v) => Some(Self(v)),
            None => None,
        }
    }

    /// The raw nonzero value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

impl std::fmt::Display for ViewInstanceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "v{}", self.0)
    }
}

// ---------------------------------------------------------------------------
// Model
// ---------------------------------------------------------------------------

/// Split axis: `Horizontal` places children side by side (left/right),
/// `Vertical` stacks them (top/bottom).
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Axis {
    /// Children arranged left to right.
    Horizontal,
    /// Children arranged top to bottom.
    Vertical,
}

/// Recursive dock-tree node.
///
/// `#[non_exhaustive]` prevents downstream construction — nodes must
/// be built through [`DockNode::tabs`]/[`DockNode::split`], which
/// enforce the structural invariants by rejection:
///
/// - `Tabs` holds at least one view; `active` is clamped into range.
/// - `Split` holds at least two children; `ratio` must be finite and
///   inside `(0, 1)` — it is the axis-fraction occupied by
///   `children[0]`, the remaining `1 - ratio` shared equally by the
///   other children.
///
/// Structural edits (view removal, restore of untrusted data) may
/// still produce empty leaves or under-populated splits;
/// [`DockTree::normalize`] repairs those (empty `Tabs` removed,
/// `<2`-child splits collapsed, `active` clamped). Invalid ratios
/// are always rejected, never repaired.
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum DockNode {
    /// A split container: `children` (≥ 2) arranged along `axis`,
    /// `ratio` being `children[0]`'s share of the split extent.
    Split {
        /// Layout direction.
        axis: Axis,
        /// Fraction of the split extent occupied by `children[0]`;
        /// finite, inside `(0, 1)`.
        ratio: f64,
        /// At least two child nodes.
        children: Vec<DockNode>,
    },
    /// A tab leaf: `views` (≥ 1) with `active` selecting the visible
    /// tab (clamped to the last index when out of range).
    Tabs {
        /// Docked view ids — at least one, no duplicates.
        views: Vec<ViewInstanceId>,
        /// Index of the active tab; clamped to `views.len() - 1`.
        #[serde(default)]
        active: usize,
    },
}

impl DockNode {
    /// Builds a `Tabs` leaf. `active` is clamped to the last index.
    ///
    /// # Errors
    /// [`DockError::EmptyTabs`] when `views` is empty.
    pub fn tabs(views: Vec<ViewInstanceId>, active: usize) -> Result<Self, DockError> {
        if views.is_empty() {
            return Err(DockError::EmptyTabs);
        }
        Ok(Self::Tabs {
            active: active.min(views.len() - 1),
            views,
        })
    }

    /// Builds a `Split` node.
    ///
    /// # Errors
    /// [`DockError::InvalidChildren`] when fewer than two children;
    /// [`DockError::InvalidRatio`] when `ratio` is not finite or
    /// outside `(0, 1)`.
    pub fn split(axis: Axis, ratio: f64, children: Vec<DockNode>) -> Result<Self, DockError> {
        if children.len() < 2 {
            return Err(DockError::InvalidChildren {
                children: children.len(),
            });
        }
        if !ratio.is_finite() || ratio <= 0.0 || ratio >= 1.0 {
            return Err(DockError::InvalidRatio { ratio });
        }
        Ok(Self::Split {
            axis,
            ratio,
            children,
        })
    }

    /// Whether this node is a `Tabs` leaf.
    #[must_use]
    pub const fn is_leaf(&self) -> bool {
        matches!(self, Self::Tabs { .. })
    }

    /// Iterates all view ids in this subtree (depth-first).
    pub fn views(&self) -> impl Iterator<Item = ViewInstanceId> + '_ {
        self.iter_seed()
    }

    /// Repairs this subtree in place: removes empty `Tabs` leaves,
    /// collapses splits with fewer than two children (one-child
    /// splits are replaced by their child), and clamps `active`.
    /// Returns `false` when this node holds no views and its parent
    /// must drop it.
    fn normalize(&mut self) -> bool {
        match self {
            Self::Tabs { views, active } => {
                if views.is_empty() {
                    return false;
                }
                *active = (*active).min(views.len() - 1);
                true
            }
            Self::Split { children, .. } => {
                children.retain_mut(DockNode::normalize);
                match children.len() {
                    0 => false,
                    1 => {
                        *self = children.pop().expect("len checked");
                        true
                    }
                    _ => true,
                }
            }
        }
    }

    /// Checks this subtree's invariants: finite `(0, 1)` ratios,
    /// non-empty tabs with in-range `active`, splits with ≥ 2
    /// children, and no view id appearing twice.
    ///
    /// # Errors
    /// The first violated invariant as a [`DockError`].
    fn validate(&self, seen: &mut HashSet<ViewInstanceId>) -> Result<(), DockError> {
        match self {
            Self::Tabs { views, active } => {
                if views.is_empty() {
                    return Err(DockError::EmptyTabs);
                }
                if *active >= views.len() {
                    return Err(DockError::InvalidActive {
                        active: *active,
                        views: views.len(),
                    });
                }
                for view in views {
                    if !seen.insert(*view) {
                        return Err(DockError::DuplicateView {
                            view: *view,
                            owner: None,
                        });
                    }
                }
                Ok(())
            }
            Self::Split {
                axis: _,
                ratio,
                children,
            } => {
                if children.len() < 2 {
                    return Err(DockError::InvalidChildren {
                        children: children.len(),
                    });
                }
                if !ratio.is_finite() || *ratio <= 0.0 || *ratio >= 1.0 {
                    return Err(DockError::InvalidRatio { ratio: *ratio });
                }
                for child in children {
                    child.validate(seen)?;
                }
                Ok(())
            }
        }
    }

    /// Locates `view` in this subtree: returns the leaf path and the
    /// tab index inside that leaf.
    fn locate(&self, path: &mut DockPath, view: ViewInstanceId) -> Option<ViewLocation> {
        match self {
            Self::Tabs { views, .. } => {
                views
                    .iter()
                    .position(|v| *v == view)
                    .map(|tab| ViewLocation {
                        leaf: path.clone(),
                        tab,
                    })
            }
            Self::Split { children, .. } => {
                for (i, child) in children.iter().enumerate() {
                    path.0.push(i);
                    if let Some(loc) = child.locate(path, view) {
                        return Some(loc);
                    }
                    path.0.pop();
                }
                None
            }
        }
    }

    /// Returns the node at `path` (child indices from the root), or
    /// `None` when it does not resolve.
    #[must_use]
    pub fn node(&self, path: &DockPath) -> Option<&DockNode> {
        let mut node = self;
        for &index in &path.0 {
            match node {
                Self::Split { children, .. } => node = children.get(index)?,
                Self::Tabs { .. } => return None,
            }
        }
        Some(node)
    }

    /// Mutable counterpart of [`node`](Self::node).
    fn node_mut(&mut self, path: &DockPath) -> Option<&mut DockNode> {
        let mut node = self;
        for &index in &path.0 {
            match node {
                Self::Split { children, .. } => node = children.get_mut(index)?,
                Self::Tabs { .. } => return None,
            }
        }
        Some(node)
    }
}

struct ViewsIter<'a> {
    /// Node worklist — splits expand their children in order.
    stack: Vec<&'a DockNode>,
    /// Cursor over the current leaf's view list.
    current: std::slice::Iter<'a, ViewInstanceId>,
}

impl Iterator for ViewsIter<'_> {
    type Item = ViewInstanceId;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(&view) = self.current.next() {
                return Some(view);
            }
            // Advance to the next leaf carrying views.
            loop {
                match self.stack.pop()? {
                    DockNode::Tabs { views, .. } => {
                        self.current = views.iter();
                        break;
                    }
                    DockNode::Split { children, .. } => {
                        self.stack.extend(children.iter().rev());
                    }
                }
            }
        }
    }
}

impl DockNode {
    fn iter_seed(&self) -> ViewsIter<'_> {
        ViewsIter {
            stack: vec![self],
            current: [].iter(),
        }
    }
}

/// Path from the tree root to a node — a sequence of child indices.
/// `DockPath::root()` addresses the root itself. Paths are
/// invalidated by structural edits; re-acquire via
/// [`DockTree::locate`] after changes.
///
/// **RT contract:** non-RT only.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DockPath(Vec<usize>);

impl DockPath {
    /// The root path (empty index list).
    #[must_use]
    pub fn root() -> Self {
        Self(Vec::new())
    }

    /// The path extended by one child index.
    #[must_use]
    pub fn child(&self, index: usize) -> Self {
        let mut p = self.0.clone();
        p.push(index);
        Self(p)
    }

    /// Number of edges below the root.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether this is the root path.
    #[must_use]
    pub fn is_root(&self) -> bool {
        self.0.is_empty()
    }

    /// Whether the path carries no child indices (same as
    /// [`is_root`](Self::is_root)).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The raw child-index sequence.
    #[must_use]
    pub fn as_slice(&self) -> &[usize] {
        &self.0
    }
}

/// Where a view sits inside a tree: the leaf path and the tab index
/// within that leaf.
///
/// **RT contract:** non-RT only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewLocation {
    /// Path of the `Tabs` leaf containing the view.
    pub leaf: DockPath,
    /// Tab index of the view inside that leaf.
    pub tab: usize,
}

/// A normalized, validated dock tree — at least one view leaf.
///
/// A window with no views simply has no `DockTree` (the workspace
/// slot is `None`), so a `DockTree` is never empty.
///
/// **RT contract:** non-RT only.
#[derive(Debug, Clone, PartialEq)]
pub struct DockTree {
    root: DockNode,
}

impl DockTree {
    /// Builds a tree from `root` — normalizes then validates it.
    ///
    /// # Errors
    /// [`DockError::EmptyTabs`] when the normalized root carries no
    /// views; the first violated invariant otherwise.
    pub fn new(mut root: DockNode) -> Result<Self, DockError> {
        if !root.normalize() {
            return Err(DockError::EmptyTabs);
        }
        let mut seen = HashSet::new();
        root.validate(&mut seen)?;
        Ok(Self { root })
    }

    /// The root node.
    #[must_use]
    pub const fn root(&self) -> &DockNode {
        &self.root
    }

    /// Iterates all view ids in the tree.
    pub fn views(&self) -> impl Iterator<Item = ViewInstanceId> + '_ {
        self.root.views()
    }

    /// Number of view ids in the tree.
    #[must_use]
    pub fn view_count(&self) -> usize {
        self.views().count()
    }

    /// Locates `view` (leaf path + tab index) or `None`.
    #[must_use]
    pub fn locate(&self, view: ViewInstanceId) -> Option<ViewLocation> {
        self.root.locate(&mut DockPath::root(), view)
    }

    /// The node at `path`, or `None` when it does not resolve.
    #[must_use]
    pub fn node(&self, path: &DockPath) -> Option<&DockNode> {
        self.root.node(path)
    }

    /// Re-validates the tree (used after untrusted deserialization —
    /// constructors keep fresh trees valid already).
    ///
    /// # Errors
    /// The first violated invariant as a [`DockError`].
    pub fn validate(&self) -> Result<(), DockError> {
        self.root.validate(&mut HashSet::new())
    }

    /// Re-normalizes the tree in place (post structural edits);
    /// returns `false` when the tree lost its last view and the
    /// owning window must drop it.
    fn normalize(&mut self) -> bool {
        self.root.normalize()
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors returned by dock model, workspace, and persistence
/// operations.
///
/// **RT contract:** non-RT only — surfaced on the UI thread.
#[non_exhaustive]
#[derive(Debug)]
pub enum DockError {
    /// The window is not registered in the workspace.
    UnknownWindow {
        /// The missing window.
        window: WindowId,
    },
    /// The view is neither docked nor in the detached pool.
    UnknownView {
        /// The missing view.
        view: ViewInstanceId,
    },
    /// The view already has an owner — duplication requires a new
    /// view id. `owner` is `Some(w)` when another window's tree owns
    /// it, `None` for a duplicate inside one tree or the detached
    /// pool.
    DuplicateView {
        /// The conflicting view.
        view: ViewInstanceId,
        /// Current owning window, when any.
        owner: Option<WindowId>,
    },
    /// A `Tabs` leaf must hold at least one view.
    EmptyTabs,
    /// A `Split` must hold at least two children.
    InvalidChildren {
        /// The offending child count.
        children: usize,
    },
    /// Split ratio must be finite and inside `(0, 1)`.
    InvalidRatio {
        /// The offending ratio.
        ratio: f64,
    },
    /// `active` must be a valid tab index.
    InvalidActive {
        /// The offending index.
        active: usize,
        /// Leaf view count.
        views: usize,
    },
    /// A [`DockPath`] did not resolve inside the tree.
    BadPath,
    /// A drop/attach target is invalid (unregistered window, missing
    /// anchor view, self-anchor).
    InvalidTarget {
        /// Human-readable reason.
        reason: String,
    },
    /// The view is in transfer — edits are rejected until the move
    /// commits or aborts.
    ViewBusy {
        /// The in-transfer view.
        view: ViewInstanceId,
    },
    /// The token does not identify a pending move.
    NotInTransfer,
    /// The move destination disappeared between begin and commit —
    /// the view stayed at its source dock (rollback).
    TargetUnavailable {
        /// The lost destination window.
        window: WindowId,
    },
    /// A persisted layout failed validation.
    InvalidSnapshot {
        /// Human-readable reason.
        reason: String,
    },
    /// Snapshot schema version is not supported.
    UnsupportedSchema {
        /// The schema value found in the data.
        found: u32,
    },
}

impl std::fmt::Display for DockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownWindow { window } => write!(f, "unknown window {window}"),
            Self::UnknownView { view } => write!(f, "unknown view {view}"),
            Self::DuplicateView { view, owner } => match owner {
                Some(w) => write!(f, "view {view} already owned by window {w}"),
                None => write!(f, "view {view} already present"),
            },
            Self::EmptyTabs => write!(f, "tabs leaf must hold at least one view"),
            Self::InvalidChildren { children } => {
                write!(f, "split needs at least two children, got {children}")
            }
            Self::InvalidRatio { ratio } => {
                write!(
                    f,
                    "split ratio must be finite and inside (0, 1), got {ratio}"
                )
            }
            Self::InvalidActive { active, views } => {
                write!(f, "active index {active} out of range for {views} views")
            }
            Self::BadPath => write!(f, "dock path does not resolve"),
            Self::InvalidTarget { reason } => write!(f, "invalid dock target: {reason}"),
            Self::ViewBusy { view } => write!(f, "view {view} is in transfer"),
            Self::NotInTransfer => write!(f, "move token is not pending"),
            Self::TargetUnavailable { window } => {
                write!(f, "move destination window {window} is gone")
            }
            Self::InvalidSnapshot { reason } => write!(f, "invalid layout snapshot: {reason}"),
            Self::UnsupportedSchema { found } => {
                write!(f, "unsupported layout schema version {found}")
            }
        }
    }
}

impl std::error::Error for DockError {}

// ---------------------------------------------------------------------------
// Workspace
// ---------------------------------------------------------------------------

/// Where a view lands when docked — the shared placement vocabulary
/// for drag drops and menu-driven moves.
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropZone {
    /// Split the anchor leaf horizontally; the view lands left.
    Left,
    /// Split the anchor leaf horizontally; the view lands right.
    Right,
    /// Split the anchor leaf vertically; the view lands above.
    Top,
    /// Split the anchor leaf vertically; the view lands below.
    Bottom,
    /// Append the view as a tab of the anchor leaf (and activate it).
    Tab,
}

/// Target of an attach/move: an existing tree zone (relative to the
/// leaf containing `anchor`) or a window's root level.
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockTarget {
    /// Dock at the window's root level: no tree → a new `Tabs` tree;
    /// `Tabs` root → a new tab; `Split` root → a new trailing child.
    Window(WindowId),
    /// Dock relative to the leaf containing `anchor` inside
    /// `window`'s tree.
    Zone {
        /// The window whose tree receives the view.
        window: WindowId,
        /// A view inside the leaf to split/tab into.
        anchor: ViewInstanceId,
        /// The drop zone relative to that leaf.
        zone: DropZone,
    },
}

/// What happens to a closing window's docked views — always the
/// consumer's explicit choice, never a default.
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowCloseDisposition {
    /// Keep the views in the workspace's detached pool — reattachable
    /// later; nothing is lost.
    DetachViews,
    /// Re-dock every view into `target`'s tree as root-level tabs
    /// (layout structure is not carried over).
    RelocateTo {
        /// The window receiving the views.
        target: WindowId,
    },
    /// Remove the views from the workspace entirely — the consumer
    /// has already saved their state (for example, application exit).
    ReleaseViews,
}

/// Token identifying one in-flight view move.
///
/// **RT contract:** non-RT only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MoveToken(u64);

/// Result of a committed move: where the view came from (`None` =
/// detached pool) and where it landed.
///
/// **RT contract:** non-RT only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoveReport {
    /// The moved view.
    pub view: ViewInstanceId,
    /// The source window, or `None` when it came from the detached pool.
    pub from: Option<WindowId>,
    /// The destination window.
    pub to: WindowId,
}

/// Workspace notifications drained via [`DockWorkspace::drain_events`].
/// `TreeChanged` marks the window for re-render; `MoveCommitted` is
/// the consumer's view-state handoff point (save at source, rebuild
/// at destination — DOM transfer is never guaranteed).
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DockEvent {
    /// A window's tree changed structure.
    TreeChanged {
        /// The affected window.
        window: WindowId,
    },
    /// A view entered transfer — its edits are locked until
    /// commit/abort.
    MovePrepared {
        /// The in-transfer view.
        view: ViewInstanceId,
    },
    /// Ownership switched atomically — restore view state at `to`.
    MoveCommitted {
        /// The moved view.
        view: ViewInstanceId,
        /// Source window (`None` = detached pool).
        from: Option<WindowId>,
        /// Destination window.
        to: WindowId,
    },
    /// A transfer rolled back; the view stayed at its source dock.
    MoveAborted {
        /// The view whose move was aborted.
        view: ViewInstanceId,
    },
    /// A window was removed; `views` lists what it held.
    WindowRemoved {
        /// The removed window.
        window: WindowId,
        /// Views the window's tree carried.
        views: Vec<ViewInstanceId>,
    },
}

#[derive(Debug)]
struct PendingMove {
    view: ViewInstanceId,
    target: DockTarget,
}

/// The single source of truth for every window's dock state.
///
/// Pure model — it never calls the host; consumers inject window
/// lifecycle (`register_window`/`remove_window`) and drive moves
/// around their own webview preparation. Invariants: ≤ 1 tree per
/// window, ≤ 1 owner per view across all trees plus the detached
/// pool, no edits on an in-transfer view.
///
/// **RT contract:** non-RT only — driven on the UI thread.
#[derive(Debug, Default)]
pub struct DockWorkspace {
    /// Registered windows; `None` = open window with no dock tree.
    windows: BTreeMap<WindowId, Option<DockTree>>,
    /// Views owned by the workspace but not docked anywhere.
    detached: BTreeSet<ViewInstanceId>,
    /// In-flight moves, keyed by token id.
    pending: HashMap<u64, PendingMove>,
    next_move: u64,
    events: VecDeque<DockEvent>,
}

impl DockWorkspace {
    /// An empty workspace.
    #[must_use]
    pub fn new() -> Self {
        Self {
            windows: BTreeMap::new(),
            detached: BTreeSet::new(),
            pending: HashMap::new(),
            next_move: 1,
            events: VecDeque::new(),
        }
    }

    fn emit(&mut self, event: DockEvent) {
        self.events.push_back(event);
    }

    /// Drains pending [`DockEvent`]s.
    pub fn drain_events(&mut self) -> Vec<DockEvent> {
        self.events.drain(..).collect()
    }

    // -- window management -------------------------------------------------

    /// Registers a window (idempotent) — initially with no tree.
    pub fn register_window(&mut self, window: WindowId) {
        self.windows.entry(window).or_insert(None);
    }

    /// Whether `window` is registered.
    #[must_use]
    pub fn is_registered(&self, window: WindowId) -> bool {
        self.windows.contains_key(&window)
    }

    /// Registered windows in id order.
    #[must_use]
    pub fn windows(&self) -> Vec<WindowId> {
        self.windows.keys().copied().collect()
    }

    /// The window's tree, or `None` when the window is empty or
    /// unregistered.
    #[must_use]
    pub fn tree(&self, window: WindowId) -> Option<&DockTree> {
        self.windows.get(&window).and_then(Option::as_ref)
    }

    /// Removes `window` and disposes of its docked views per
    /// `disposition` — the consumer's explicit choice (there is no
    /// default). Pending moves involving the window are aborted
    /// first. Returns the views the window carried.
    ///
    /// # Errors
    /// [`DockError::UnknownWindow`] when `window` is not registered;
    /// [`DockError::InvalidTarget`] when `RelocateTo`'s target is the
    /// same window or unregistered.
    pub fn remove_window(
        &mut self,
        window: WindowId,
        disposition: WindowCloseDisposition,
    ) -> Result<Vec<ViewInstanceId>, DockError> {
        if !self.windows.contains_key(&window) {
            return Err(DockError::UnknownWindow { window });
        }
        let relocate_target = match disposition {
            WindowCloseDisposition::RelocateTo { target } => {
                if target == window || !self.windows.contains_key(&target) {
                    return Err(DockError::InvalidTarget {
                        reason: format!("cannot relocate views to window {target}"),
                    });
                }
                Some(target)
            }
            _ => None,
        };
        // Abort every pending move whose view lives in this window —
        // the disposition below disposes of them, so a stale transfer
        // lock must not linger. Moves merely *targeting* this window
        // are left alone: their commit discovers the window gone and
        // rolls back as `TargetUnavailable` (the view stayed docked).
        let stale: Vec<u64> = self
            .pending
            .iter()
            .filter(|(_, p)| self.owner_of(p.view) == Some(window))
            .map(|(id, _)| *id)
            .collect();
        for id in stale {
            if let Some(p) = self.pending.remove(&id) {
                self.emit(DockEvent::MoveAborted { view: p.view });
            }
        }
        let views: Vec<ViewInstanceId> = self
            .windows
            .remove(&window)
            .and_then(|slot| slot.map(|t| t.views().collect()))
            .unwrap_or_default();
        match relocate_target {
            Some(target) => {
                for view in &views {
                    self.attach_to_window(target, *view);
                }
                self.emit(DockEvent::TreeChanged { window: target });
            }
            None => match disposition {
                WindowCloseDisposition::DetachViews => {
                    self.detached.extend(views.iter().copied());
                }
                WindowCloseDisposition::ReleaseViews
                | WindowCloseDisposition::RelocateTo { .. } => {}
            },
        }
        self.emit(DockEvent::WindowRemoved {
            window,
            views: views.clone(),
        });
        Ok(views)
    }

    // -- view ownership ----------------------------------------------------

    /// The window owning `view`, or `None` when detached/unknown.
    #[must_use]
    pub fn owner_of(&self, view: ViewInstanceId) -> Option<WindowId> {
        self.windows
            .iter()
            .find(|(_, slot)| slot.as_ref().is_some_and(|t| t.locate(view).is_some()))
            .map(|(w, _)| *w)
    }

    /// Whether `view` sits in the detached pool.
    #[must_use]
    pub fn is_detached(&self, view: ViewInstanceId) -> bool {
        self.detached.contains(&view)
    }

    /// Detached-pool views in id order.
    #[must_use]
    pub fn detached_views(&self) -> Vec<ViewInstanceId> {
        self.detached.iter().copied().collect()
    }

    /// Every view id the workspace knows (docked + detached).
    #[must_use]
    pub fn all_views(&self) -> Vec<ViewInstanceId> {
        let mut views: Vec<ViewInstanceId> = self
            .windows
            .values()
            .filter_map(Option::as_ref)
            .flat_map(DockTree::views)
            .chain(self.detached.iter().copied())
            .collect();
        views.sort();
        views
    }

    /// Adds a view to the detached pool (initial registration or
    /// snapshot restore).
    ///
    /// # Errors
    /// [`DockError::DuplicateView`] when the view is already owned or
    /// detached.
    pub fn add_detached(&mut self, view: ViewInstanceId) -> Result<(), DockError> {
        self.assert_free(view)?;
        self.detached.insert(view);
        Ok(())
    }

    /// Rejects views that already have an owner (any tree or the
    /// detached pool).
    fn assert_free(&self, view: ViewInstanceId) -> Result<(), DockError> {
        if let Some(owner) = self.owner_of(view) {
            return Err(DockError::DuplicateView {
                view,
                owner: Some(owner),
            });
        }
        if self.detached.contains(&view) {
            return Err(DockError::DuplicateView { view, owner: None });
        }
        Ok(())
    }

    /// Attaches a whole tree to a registered, tree-less window —
    /// used for initial layout and snapshot restore.
    ///
    /// # Errors
    /// [`DockError::UnknownWindow`] for unregistered windows;
    /// [`DockError::InvalidTarget`] when the window already has a
    /// tree; [`DockError::DuplicateView`] when any of the tree's
    /// views is owned elsewhere or detached.
    pub fn attach_tree(&mut self, window: WindowId, tree: DockTree) -> Result<(), DockError> {
        match self.windows.get(&window) {
            None => return Err(DockError::UnknownWindow { window }),
            Some(Some(_)) => {
                return Err(DockError::InvalidTarget {
                    reason: format!("window {window} already has a dock tree"),
                });
            }
            Some(None) => {}
        }
        for view in tree.views() {
            self.assert_free(view)?;
        }
        if let Some(slot) = self.windows.get_mut(&window) {
            *slot = Some(tree);
        }
        self.emit(DockEvent::TreeChanged { window });
        Ok(())
    }

    /// Attaches a free (new or detached) view at `target`.
    ///
    /// # Errors
    /// [`DockError::ViewBusy`] when the view is in transfer;
    /// [`DockError::DuplicateView`] when it is already docked;
    /// [`DockError::InvalidTarget`]/[`DockError::UnknownWindow`] for a
    /// bad target.
    pub fn attach_view(
        &mut self,
        view: ViewInstanceId,
        target: DockTarget,
    ) -> Result<(), DockError> {
        self.assert_not_busy(view)?;
        if let Some(owner) = self.owner_of(view) {
            return Err(DockError::DuplicateView {
                view,
                owner: Some(owner),
            });
        }
        // Validate the target before touching ownership — a failed
        // attach leaves a detached view detached (insert-boundary
        // rollback).
        self.check_target(&target, Some(view))?;
        self.detached.remove(&view);
        self.insert_at(target, view)
    }

    /// Detaches `view` into the detached pool (kept — reattachable).
    ///
    /// # Errors
    /// [`DockError::ViewBusy`] when in transfer;
    /// [`DockError::UnknownView`] when the view is not docked.
    pub fn detach_view(&mut self, view: ViewInstanceId) -> Result<(), DockError> {
        self.assert_not_busy(view)?;
        let owner = self.owner_of(view).ok_or(DockError::UnknownView { view })?;
        self.remove_view_from(owner, view);
        self.detached.insert(view);
        Ok(())
    }

    /// Removes `view` from the workspace entirely (dock or pool).
    ///
    /// # Errors
    /// [`DockError::ViewBusy`] when in transfer;
    /// [`DockError::UnknownView`] when the view is unknown.
    pub fn remove_view(&mut self, view: ViewInstanceId) -> Result<(), DockError> {
        self.assert_not_busy(view)?;
        match self.owner_of(view) {
            Some(owner) => self.remove_view_from(owner, view),
            None if self.detached.remove(&view) => {}
            None => return Err(DockError::UnknownView { view }),
        }
        Ok(())
    }

    // -- operation vocabulary ----------------------------------------------

    /// Sets the ratio of the `Split` node at `path` in `window`'s
    /// tree — the splitter-drag commit.
    ///
    /// # Errors
    /// [`DockError::UnknownWindow`], [`DockError::BadPath`] when the
    /// path does not resolve to a `Split`, [`DockError::InvalidRatio`].
    pub fn set_ratio(
        &mut self,
        window: WindowId,
        path: &DockPath,
        ratio: f64,
    ) -> Result<(), DockError> {
        if !ratio.is_finite() || ratio <= 0.0 || ratio >= 1.0 {
            return Err(DockError::InvalidRatio { ratio });
        }
        let tree = self.tree_mut(window)?;
        match tree.root.node_mut(path) {
            Some(DockNode::Split { ratio: r, .. }) => {
                *r = ratio;
                self.emit(DockEvent::TreeChanged { window });
                Ok(())
            }
            _ => Err(DockError::BadPath),
        }
    }

    /// Moves `view`'s tab to `index` inside its leaf (clamped).
    ///
    /// # Errors
    /// [`DockError::ViewBusy`], [`DockError::UnknownView`].
    pub fn move_tab(&mut self, view: ViewInstanceId, index: usize) -> Result<(), DockError> {
        self.assert_not_busy(view)?;
        let owner = self.owner_of(view).ok_or(DockError::UnknownView { view })?;
        let tree = self.tree_mut(owner)?;
        let loc = tree.locate(view).ok_or(DockError::UnknownView { view })?;
        let Some(DockNode::Tabs { views, active }) = tree.root.node_mut(&loc.leaf) else {
            return Err(DockError::BadPath);
        };
        let index = index.min(views.len() - 1);
        if index != loc.tab {
            let v = views.remove(loc.tab);
            views.insert(index, v);
            if *active == loc.tab {
                *active = index;
            } else if loc.tab < *active && index >= *active {
                *active -= 1;
            } else if loc.tab > *active && index <= *active {
                *active += 1;
            }
        }
        self.emit(DockEvent::TreeChanged { window: owner });
        Ok(())
    }

    /// Activates `view`'s tab in its leaf.
    ///
    /// # Errors
    /// [`DockError::ViewBusy`], [`DockError::UnknownView`].
    pub fn activate_view(&mut self, view: ViewInstanceId) -> Result<(), DockError> {
        self.assert_not_busy(view)?;
        let owner = self.owner_of(view).ok_or(DockError::UnknownView { view })?;
        let tree = self.tree_mut(owner)?;
        let loc = tree.locate(view).ok_or(DockError::UnknownView { view })?;
        let Some(DockNode::Tabs { active, .. }) = tree.root.node_mut(&loc.leaf) else {
            return Err(DockError::BadPath);
        };
        *active = loc.tab;
        self.emit(DockEvent::TreeChanged { window: owner });
        Ok(())
    }

    // -- move contract ------------------------------------------------------

    /// Begins a two-phase move: validates the view and `target`, then
    /// marks the view in-transfer — its structure is untouched and
    /// every further edit on it is rejected as [`DockError::ViewBusy`]
    /// until commit/abort (no dual-edit during transfer).
    ///
    /// Callers prepare the destination between begin and commit: for
    /// a new window, `WindowHandle::create_window` +
    /// [`register_window`](Self::register_window) must already have
    /// happened so `target` resolves, while the webview's `Ready`
    /// gates [`commit_move`](Self::commit_move).
    ///
    /// # Errors
    /// [`DockError::ViewBusy`] when already in transfer;
    /// [`DockError::UnknownView`] / [`DockError::UnknownWindow`] /
    /// [`DockError::InvalidTarget`] for bad inputs.
    pub fn begin_move(
        &mut self,
        view: ViewInstanceId,
        target: DockTarget,
    ) -> Result<MoveToken, DockError> {
        self.assert_not_busy(view)?;
        if self.owner_of(view).is_none() && !self.detached.contains(&view) {
            return Err(DockError::UnknownView { view });
        }
        self.check_target(&target, Some(view))?;
        let token = MoveToken(self.next_move);
        self.next_move += 1;
        self.pending.insert(token.0, PendingMove { view, target });
        self.emit(DockEvent::MovePrepared { view });
        Ok(token)
    }

    /// Commits a pending move — atomic detach + attach + normalize.
    /// When the destination disappeared since `begin_move`, the move
    /// rolls back automatically: the pending entry clears, a
    /// [`DockEvent::MoveAborted`] is emitted, the view stays at its
    /// source dock, and [`DockError::TargetUnavailable`] is returned.
    ///
    /// # Errors
    /// [`DockError::NotInTransfer`] for an unknown token;
    /// [`DockError::TargetUnavailable`] / [`DockError::InvalidTarget`]
    /// when the destination is gone or no longer resolvable.
    pub fn commit_move(&mut self, token: MoveToken) -> Result<MoveReport, DockError> {
        let Some(pending) = self.pending.get(&token.0) else {
            return Err(DockError::NotInTransfer);
        };
        let view = pending.view;
        let target = pending.target;
        if let Err(e) = self.check_target(&target, Some(view)) {
            self.pending.remove(&token.0);
            self.emit(DockEvent::MoveAborted { view });
            return Err(match e {
                DockError::UnknownWindow { window } => DockError::TargetUnavailable { window },
                other => other,
            });
        }
        self.pending.remove(&token.0);
        // Detach from source (dock → normalize; or the pool), then
        // attach at the target — the ownership switch is atomic.
        let from = self.owner_of(view);
        if let Some(owner) = from {
            self.remove_view_from(owner, view);
        } else {
            self.detached.remove(&view);
        }
        let to = target.window();
        self.insert_at(target, view)?;
        self.emit(DockEvent::MoveCommitted { view, from, to });
        Ok(MoveReport { view, from, to })
    }

    /// Aborts a pending move — the view stays at its source dock;
    /// nothing changed structurally.
    ///
    /// # Errors
    /// [`DockError::NotInTransfer`] for an unknown token.
    pub fn abort_move(&mut self, token: MoveToken) -> Result<(), DockError> {
        let Some(pending) = self.pending.remove(&token.0) else {
            return Err(DockError::NotInTransfer);
        };
        self.emit(DockEvent::MoveAborted { view: pending.view });
        Ok(())
    }

    /// One-shot move = `begin_move` + `commit_move` — the menu-move
    /// and in-window-drop path: identical semantics, no drag
    /// machinery required.
    ///
    /// # Errors
    /// As [`begin_move`](Self::begin_move)/[`commit_move`](Self::commit_move).
    pub fn move_view(
        &mut self,
        view: ViewInstanceId,
        target: DockTarget,
    ) -> Result<MoveReport, DockError> {
        let token = self.begin_move(view, target)?;
        self.commit_move(token)
    }

    /// Whether `view` is in transfer (edit-locked).
    #[must_use]
    pub fn is_transferring(&self, view: ViewInstanceId) -> bool {
        self.pending.values().any(|p| p.view == view)
    }

    // -- internals -----------------------------------------------------------

    fn tree_mut(&mut self, window: WindowId) -> Result<&mut DockTree, DockError> {
        self.windows
            .get_mut(&window)
            .ok_or(DockError::UnknownWindow { window })?
            .as_mut()
            .ok_or(DockError::InvalidTarget {
                reason: format!("window {window} has no dock tree"),
            })
    }

    fn assert_not_busy(&self, view: ViewInstanceId) -> Result<(), DockError> {
        if self.is_transferring(view) {
            return Err(DockError::ViewBusy { view });
        }
        Ok(())
    }

    /// Validates `target`: its window must be registered and a
    /// `Zone`'s anchor view must live in that window's tree (and not
    /// be the moved view itself).
    fn check_target(
        &self,
        target: &DockTarget,
        moving: Option<ViewInstanceId>,
    ) -> Result<(), DockError> {
        match target {
            DockTarget::Window(window) => {
                if !self.windows.contains_key(window) {
                    return Err(DockError::UnknownWindow { window: *window });
                }
            }
            DockTarget::Zone {
                window,
                anchor,
                zone: _,
            } => {
                let tree = self
                    .windows
                    .get(window)
                    .ok_or(DockError::UnknownWindow { window: *window })?
                    .as_ref()
                    .ok_or_else(|| DockError::InvalidTarget {
                        reason: format!("window {window} has no dock tree"),
                    })?;
                if Some(*anchor) == moving {
                    return Err(DockError::InvalidTarget {
                        reason: format!("view {anchor} cannot be its own drop anchor"),
                    });
                }
                if tree.locate(*anchor).is_none() {
                    return Err(DockError::InvalidTarget {
                        reason: format!("anchor view {anchor} is not docked in window {window}"),
                    });
                }
            }
        }
        Ok(())
    }

    /// Removes `view` from `window`'s tree and normalizes; drops the
    /// tree entirely when it lost its last view.
    fn remove_view_from(&mut self, window: WindowId, view: ViewInstanceId) {
        let Some(slot) = self.windows.get_mut(&window) else {
            return;
        };
        let Some(tree) = slot.as_mut() else { return };
        tree.root.remove_view(view);
        if !tree.normalize() {
            *slot = None;
        }
        self.emit(DockEvent::TreeChanged { window });
    }

    /// Attaches `view` at `target` (already validated).
    fn insert_at(&mut self, target: DockTarget, view: ViewInstanceId) -> Result<(), DockError> {
        let window = target.window();
        match target {
            DockTarget::Window(_) => self.attach_to_window(window, view),
            DockTarget::Zone { anchor, zone, .. } => {
                self.insert_at_zone(window, anchor, zone, view)?;
            }
        }
        self.emit(DockEvent::TreeChanged { window });
        Ok(())
    }

    /// Root-level attach: no tree → new `Tabs` tree; `Tabs` root →
    /// new tab (activated); `Split` root → trailing `Tabs` child.
    fn attach_to_window(&mut self, window: WindowId, view: ViewInstanceId) {
        let Some(slot) = self.windows.get_mut(&window) else {
            return;
        };
        match slot.as_mut() {
            None => {
                *slot = Some(DockTree {
                    root: DockNode::Tabs {
                        views: vec![view],
                        active: 0,
                    },
                });
            }
            Some(tree) => match &mut tree.root {
                DockNode::Tabs { views, active } => {
                    views.push(view);
                    *active = views.len() - 1;
                }
                DockNode::Split { children, .. } => {
                    children.push(DockNode::Tabs {
                        views: vec![view],
                        active: 0,
                    });
                }
            },
        }
    }

    /// Zone attach: replaces the anchor leaf with a 2-child split, or
    /// appends `view` to it for `DropZone::Tab`.
    fn insert_at_zone(
        &mut self,
        window: WindowId,
        anchor: ViewInstanceId,
        zone: DropZone,
        view: ViewInstanceId,
    ) -> Result<(), DockError> {
        let tree = self.tree_mut(window)?;
        let Some(loc) = tree.locate(anchor) else {
            return Err(DockError::InvalidTarget {
                reason: format!("anchor view {anchor} is not docked in window {window}"),
            });
        };
        let Some(leaf_node) = tree.root.node_mut(&loc.leaf) else {
            return Err(DockError::BadPath);
        };
        let old_leaf = std::mem::replace(
            leaf_node,
            DockNode::Tabs {
                views: vec![],
                active: 0,
            },
        );
        *leaf_node = match zone {
            DropZone::Tab => {
                let DockNode::Tabs { mut views, .. } = old_leaf else {
                    return Err(DockError::BadPath);
                };
                views.push(view);
                DockNode::Tabs {
                    active: views.len() - 1,
                    views,
                }
            }
            DropZone::Left | DropZone::Right | DropZone::Top | DropZone::Bottom => {
                let new_leaf = DockNode::Tabs {
                    views: vec![view],
                    active: 0,
                };
                let axis = match zone {
                    DropZone::Left | DropZone::Right => Axis::Horizontal,
                    _ => Axis::Vertical,
                };
                let children = match zone {
                    DropZone::Left | DropZone::Top => vec![new_leaf, old_leaf],
                    _ => vec![old_leaf, new_leaf],
                };
                DockNode::Split {
                    axis,
                    ratio: 0.5,
                    children,
                }
            }
        };
        Ok(())
    }
}

impl DockTarget {
    /// The window receiving the view.
    #[must_use]
    pub const fn window(&self) -> WindowId {
        match self {
            Self::Window(w) | Self::Zone { window: w, .. } => *w,
        }
    }
}

impl DockNode {
    /// Removes `view` from this subtree (leaf tab lists only; empty
    /// leaves are left for `normalize` to drop).
    fn remove_view(&mut self, view: ViewInstanceId) -> bool {
        match self {
            Self::Tabs { views, active } => {
                if let Some(i) = views.iter().position(|v| *v == view) {
                    views.remove(i);
                    if *active > i {
                        *active -= 1;
                    } else if *active == i && !views.is_empty() {
                        *active = (*active).min(views.len() - 1);
                    }
                    true
                } else {
                    false
                }
            }
            Self::Split { children, .. } => children.iter_mut().any(|c| c.remove_view(view)),
        }
    }
}

// ---------------------------------------------------------------------------
// Persistence
// ---------------------------------------------------------------------------

/// Current layout snapshot schema version.
pub const LAYOUT_SCHEMA_VERSION: u32 = 1;

/// Versioned persisted layout: window geometry (physical pixels,
/// virtual-desktop coordinates), monitor signatures, dock trees, and
/// the detached pool.
///
/// Monitor identity is a *signature* ([`MonitorHint`]: name + bounds +
/// scale factor) — never a persistent display id, since those change
/// between sessions. Restore matches signatures; unmatched or
/// off-screen geometry is re-centered onto the best available
/// monitor.
///
/// **RT contract:** non-RT only.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LayoutSnapshot {
    /// Schema version — must equal [`LAYOUT_SCHEMA_VERSION`].
    pub schema: u32,
    /// One entry per captured window (capture-order).
    #[serde(default)]
    pub windows: Vec<WindowSnapshot>,
    /// Detached-pool view ids.
    #[serde(default)]
    pub detached: Vec<u64>,
}

/// One captured window: geometry + monitor signature + dock tree.
///
/// **RT contract:** non-RT only.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowSnapshot {
    /// Left edge, virtual-desktop coordinates (physical px).
    pub x: i32,
    /// Top edge, virtual-desktop coordinates (physical px).
    pub y: i32,
    /// Width in physical pixels.
    pub width: u32,
    /// Height in physical pixels.
    pub height: u32,
    /// Signature of the monitor hosting the window at capture time.
    #[serde(default)]
    pub monitor: Option<MonitorHint>,
    /// The window's dock tree (root node), or `None` when empty.
    #[serde(default)]
    pub tree: Option<DockNode>,
}

/// A monitor signature for layout restore — matched by value, not
/// by `id` (display ids are not assumed persistent).
///
/// **RT contract:** non-RT only.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MonitorHint {
    /// Display name, when the platform reported one.
    #[serde(default)]
    pub name: Option<String>,
    /// Monitor bounds width (physical px).
    pub width: u32,
    /// Monitor bounds height (physical px).
    pub height: u32,
    /// `HiDPI` scale factor.
    pub scale_factor: f64,
}

impl From<&MonitorInfo> for MonitorHint {
    fn from(m: &MonitorInfo) -> Self {
        Self {
            name: m.name.clone(),
            width: m.width,
            height: m.height,
            scale_factor: m.scale_factor,
        }
    }
}

/// Current window geometry — capture input for
/// [`LayoutSnapshot::capture`].
///
/// **RT contract:** non-RT only.
#[derive(Debug, Clone)]
pub struct WindowGeometry {
    /// Left edge, virtual-desktop coordinates (physical px).
    pub x: i32,
    /// Top edge, virtual-desktop coordinates (physical px).
    pub y: i32,
    /// Width in physical pixels.
    pub width: u32,
    /// Height in physical pixels.
    pub height: u32,
    /// Signature of the monitor currently hosting the window.
    pub monitor: Option<MonitorHint>,
}

/// Validated, restore-ready layout.
///
/// **RT contract:** non-RT only.
#[derive(Debug, Clone, Default)]
pub struct RestoredLayout {
    /// Per-window placements, snapshot order — pair with the
    /// windows the consumer (re)creates in the same order.
    pub placements: Vec<WindowPlacement>,
    /// Views to repopulate the detached pool.
    pub detached: Vec<ViewInstanceId>,
}

/// One validated placement: corrected geometry + validated tree.
///
/// **RT contract:** non-RT only.
#[derive(Debug, Clone)]
pub struct WindowPlacement {
    /// Corrected left edge (physical px).
    pub x: i32,
    /// Corrected top edge (physical px).
    pub y: i32,
    /// Corrected width (physical px).
    pub width: u32,
    /// Corrected height (physical px).
    pub height: u32,
    /// Index into the monitor list passed to
    /// [`LayoutSnapshot::restore`], when placement resolved to one.
    pub monitor: Option<usize>,
    /// Validated dock tree, when the window had one.
    pub tree: Option<DockTree>,
}

impl LayoutSnapshot {
    /// Captures the current workspace + geometry: `windows` and
    /// `geometry` are parallel (same order, same length) — consumers
    /// obtain geometry from `WindowHandle::windows`.
    ///
    /// # Errors
    /// [`DockError::InvalidSnapshot`] when the two lists disagree in
    /// length.
    pub fn capture(
        windows: &[WindowId],
        geometry: &[WindowGeometry],
        workspace: &DockWorkspace,
    ) -> Result<Self, DockError> {
        if windows.len() != geometry.len() {
            return Err(DockError::InvalidSnapshot {
                reason: format!(
                    "windows ({}) and geometry ({}) length mismatch",
                    windows.len(),
                    geometry.len()
                ),
            });
        }
        Ok(Self {
            schema: LAYOUT_SCHEMA_VERSION,
            windows: windows
                .iter()
                .zip(geometry)
                .map(|(w, g)| WindowSnapshot {
                    x: g.x,
                    y: g.y,
                    width: g.width,
                    height: g.height,
                    monitor: g.monitor.clone(),
                    tree: workspace.tree(*w).map(|t| t.root().clone()),
                })
                .collect(),
            detached: workspace.detached.iter().map(|v| v.get()).collect(),
        })
    }

    /// Deserializes a snapshot (syntax only — call
    /// [`restore`](Self::restore) for validation).
    ///
    /// # Errors
    /// [`DockError::InvalidSnapshot`] on malformed JSON.
    pub fn from_json(json: &str) -> Result<Self, DockError> {
        serde_json::from_str(json).map_err(|e| DockError::InvalidSnapshot {
            reason: format!("malformed layout JSON: {e}"),
        })
    }

    /// Serializes to JSON.
    ///
    /// # Errors
    /// [`DockError::InvalidSnapshot`] on serialization failure.
    pub fn to_json(&self) -> Result<String, DockError> {
        serde_json::to_string(self).map_err(|e| DockError::InvalidSnapshot {
            reason: format!("cannot serialize layout: {e}"),
        })
    }

    /// Validates and restores: schema check, geometry sanity,
    /// per-window normalize+validate, global view-ownership
    /// dedup, and off-screen/missing-monitor correction.
    ///
    /// A window whose tree holds no views after normalization is
    /// treated as tree-less (empty window), matching live semantics;
    /// a structurally invalid tree fails the whole snapshot.
    ///
    /// # Errors
    /// [`DockError::UnsupportedSchema`], [`DockError::InvalidSnapshot`].
    pub fn restore(&self, monitors: &[MonitorInfo]) -> Result<RestoredLayout, DockError> {
        if self.schema != LAYOUT_SCHEMA_VERSION {
            return Err(DockError::UnsupportedSchema { found: self.schema });
        }
        let mut seen = HashSet::new();
        let mut placements = Vec::with_capacity(self.windows.len());
        for (i, w) in self.windows.iter().enumerate() {
            if w.width == 0 || w.height == 0 {
                return Err(DockError::InvalidSnapshot {
                    reason: format!("window[{i}] has zero-size geometry"),
                });
            }
            let tree = match &w.tree {
                None => None,
                Some(node) => {
                    let mut node = node.clone();
                    // A fully-empty tree means "window had no views" —
                    // normalize to `None` rather than failing.
                    if node.normalize() {
                        node.validate(&mut seen)
                            .map_err(|e| DockError::InvalidSnapshot {
                                reason: format!("window[{i}] tree invalid: {e}"),
                            })?;
                        Some(DockTree { root: node })
                    } else {
                        None
                    }
                }
            };
            let (x, y, width, height, monitor) =
                correct_bounds(w.x, w.y, w.width, w.height, w.monitor.as_ref(), monitors);
            placements.push(WindowPlacement {
                x,
                y,
                width,
                height,
                monitor,
                tree,
            });
        }
        let mut detached = Vec::with_capacity(self.detached.len());
        for raw in &self.detached {
            let view = ViewInstanceId::new(*raw).ok_or_else(|| DockError::InvalidSnapshot {
                reason: "detached view id 0 is invalid".to_string(),
            })?;
            if !seen.insert(view) {
                return Err(DockError::InvalidSnapshot {
                    reason: format!("detached view {view} is also docked"),
                });
            }
            detached.push(view);
        }
        Ok(RestoredLayout {
            placements,
            detached,
        })
    }

    /// `restore`, or the safe empty layout when the snapshot is
    /// unusable — callers get a working layout either way.
    #[must_use]
    pub fn restore_or_default(&self, monitors: &[MonitorInfo]) -> RestoredLayout {
        self.restore(monitors).unwrap_or_default()
    }
}

/// Applies a restored [`WindowPlacement::tree`] to `window`
/// (registered, tree-less) — equivalent to
/// [`DockWorkspace::attach_tree`], kept as a named step for restore
/// flows.
impl DockWorkspace {
    /// Attaches a restored tree to a registered window.
    ///
    /// # Errors
    /// As [`DockWorkspace::attach_tree`].
    pub fn apply_restored_tree(
        &mut self,
        window: WindowId,
        tree: DockTree,
    ) -> Result<(), DockError> {
        self.attach_tree(window, tree)
    }
}

/// Corrects captured geometry against the live monitor list: the
/// window stays only when its center lands inside a monitor; named
/// signature match wins re-targeting, otherwise the first monitor;
/// no monitors → geometry passed through with `None`.
fn correct_bounds(
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    hint: Option<&MonitorHint>,
    monitors: &[MonitorInfo],
) -> (i32, i32, u32, u32, Option<usize>) {
    let cx = i64::from(x) + i64::from(width) / 2;
    let cy = i64::from(y) + i64::from(height) / 2;
    for (i, m) in monitors.iter().enumerate() {
        let (mx, my, mw, mh) = (
            i64::from(m.x),
            i64::from(m.y),
            i64::from(m.width),
            i64::from(m.height),
        );
        if cx >= mx && cx < mx + mw && cy >= my && cy < my + mh {
            return (x, y, width, height, Some(i));
        }
    }
    // Off-screen or missing monitor: re-center on the signature
    // match, else the primary (first) monitor.
    let target = hint.and_then(|h| {
        monitors.iter().enumerate().find(|(_, m)| {
            m.name == h.name
                && m.width == h.width
                && m.height == h.height
                && (m.scale_factor - h.scale_factor).abs() < f64::EPSILON
        })
    });
    let Some((i, m)) = target.or_else(|| monitors.iter().enumerate().next()) else {
        return (x, y, width, height, None);
    };
    let nw = width.min(m.width);
    let nh = height.min(m.height);
    let nx = m.x + (m.width.saturating_sub(nw) / 2).cast_signed();
    let ny = m.y + (m.height.saturating_sub(nh) / 2).cast_signed();
    (nx, ny, nw, nh, Some(i))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vid(raw: u64) -> ViewInstanceId {
        ViewInstanceId::new(raw).expect("nonzero")
    }

    fn leaf(views: &[u64]) -> DockNode {
        DockNode::tabs(views.iter().map(|v| vid(*v)).collect(), 0).expect("tabs")
    }

    #[test]
    fn view_instance_id_contract() {
        assert!(ViewInstanceId::new(0).is_none());
        let v = vid(7);
        assert_eq!(v.get(), 7);
        assert_eq!(v.to_string(), "v7");
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(json, "7");
        assert_eq!(serde_json::from_str::<ViewInstanceId>(&json).unwrap(), v);
    }

    #[test]
    fn tabs_reject_empty_views() {
        assert!(matches!(
            DockNode::tabs(Vec::new(), 0),
            Err(DockError::EmptyTabs)
        ));
        // Active is clamped into range.
        let t = DockNode::tabs(vec![vid(1), vid(2)], 9).unwrap();
        match t {
            DockNode::Tabs { active, .. } => assert_eq!(active, 1),
            _ => panic!("expected tabs"),
        }
    }

    #[test]
    fn split_rejects_bad_children_and_ratio() {
        assert!(matches!(
            DockNode::split(Axis::Horizontal, 0.5, vec![leaf(&[1])]),
            Err(DockError::InvalidChildren { children: 1 })
        ));
        for bad in [0.0, 1.0, f64::NAN, f64::INFINITY, -0.2] {
            assert!(
                matches!(
                    DockNode::split(Axis::Vertical, bad, vec![leaf(&[1]), leaf(&[2])]),
                    Err(DockError::InvalidRatio { .. })
                ),
                "ratio {bad} must be rejected"
            );
        }
        assert!(DockNode::split(Axis::Vertical, 0.5, vec![leaf(&[1]), leaf(&[2])]).is_ok());
    }

    #[test]
    fn tree_normalize_removes_empty_leaves_and_collapses_splits() {
        // Split( H, [Tabs[1], Split(V, [Tabs[], Tabs[2]])] )
        //   → normalize removes the empty leaf, collapses inner split
        //     to Tabs[2], outer split keeps two children.
        let raw = DockNode::Split {
            axis: Axis::Horizontal,
            ratio: 0.5,
            children: vec![
                leaf(&[1]),
                DockNode::Split {
                    axis: Axis::Vertical,
                    ratio: 0.3,
                    children: vec![
                        DockNode::Tabs {
                            views: vec![],
                            active: 0,
                        },
                        leaf(&[2]),
                    ],
                },
            ],
        };
        let tree = DockTree::new(raw).expect("normalize+validate");
        assert_eq!(tree.view_count(), 2);
        // Inner split collapsed → root split's second child is Tabs[2].
        match tree.root().node(&DockPath::root().child(1)).unwrap() {
            DockNode::Tabs { views, .. } => assert_eq!(views, &vec![vid(2)]),
            _ => panic!("expected collapsed tabs"),
        }
    }

    #[test]
    fn tree_normalize_collapses_single_child_root() {
        // Split(V, [Tabs[1], Split(H, [Tabs[]])]) → Tabs[1].
        let raw = DockNode::Split {
            axis: Axis::Vertical,
            ratio: 0.8,
            children: vec![
                leaf(&[1]),
                DockNode::Split {
                    axis: Axis::Horizontal,
                    ratio: 0.5,
                    children: vec![DockNode::Tabs {
                        views: vec![],
                        active: 0,
                    }],
                },
            ],
        };
        let tree = DockTree::new(raw).expect("normalize+validate");
        match tree.root() {
            DockNode::Tabs { views, .. } => assert_eq!(views, &vec![vid(1)]),
            _ => panic!("expected root collapsed to tabs"),
        }
    }

    #[test]
    fn tree_new_rejects_all_empty() {
        let raw = DockNode::Tabs {
            views: vec![],
            active: 0,
        };
        assert!(matches!(DockTree::new(raw), Err(DockError::EmptyTabs)));
    }

    #[test]
    fn tree_validate_rejects_duplicate_views_and_bad_ratio() {
        // Duplicate view across two leaves.
        let dup = DockNode::Split {
            axis: Axis::Horizontal,
            ratio: 0.5,
            children: vec![leaf(&[1, 2]), leaf(&[2, 3])],
        };
        assert!(matches!(
            DockTree::new(dup),
            Err(DockError::DuplicateView { .. })
        ));

        // Serde-restored node bypasses constructors → ratio caught by
        // validate (normalization does not repair ratios).
        let bad = DockNode::Split {
            axis: Axis::Horizontal,
            ratio: 1.7,
            children: vec![leaf(&[1]), leaf(&[2])],
        };
        assert!(matches!(
            DockTree::new(bad),
            Err(DockError::InvalidRatio { .. })
        ));
    }

    #[test]
    fn locate_and_node_address_nodes() {
        let tree = DockTree::new(
            DockNode::split(Axis::Horizontal, 0.4, vec![leaf(&[1, 2]), leaf(&[3])]).unwrap(),
        )
        .unwrap();
        let loc = tree.locate(vid(2)).expect("found");
        assert_eq!(loc.leaf.as_slice(), &[0]);
        assert_eq!(loc.tab, 1);
        assert!(tree.locate(vid(99)).is_none());
        assert!(matches!(
            tree.node(&DockPath::root().child(1)),
            Some(DockNode::Tabs { .. })
        ));
        // A path into a leaf resolves no further.
        assert!(tree.node(&DockPath::root().child(0).child(0)).is_none());
    }

    #[test]
    fn docknode_serde_roundtrip() {
        let node = DockNode::split(Axis::Vertical, 0.25, vec![leaf(&[1, 2]), leaf(&[3])]).unwrap();
        let json = serde_json::to_value(&node).unwrap();
        assert_eq!(json["type"], "split");
        assert_eq!(json["axis"], "vertical");
        assert_eq!(json["children"][0]["type"], "tabs");
        assert_eq!(json["children"][0]["views"], serde_json::json!([1, 2]));
        let back: DockNode = serde_json::from_value(json).unwrap();
        assert_eq!(back, node);
    }

    // -- workspace tests ------------------------------------------------------

    fn wid(raw: u64) -> WindowId {
        WindowId::new(raw).expect("nonzero")
    }

    fn workspace_with(windows: &[u64]) -> DockWorkspace {
        let mut ws = DockWorkspace::new();
        for w in windows {
            ws.register_window(wid(*w));
        }
        ws
    }

    fn attach(ws: &mut DockWorkspace, window: u64, views: &[u64]) {
        for (i, v) in views.iter().enumerate() {
            ws.attach_view(
                vid(*v),
                if i == 0 {
                    DockTarget::Window(wid(window))
                } else {
                    DockTarget::Zone {
                        window: wid(window),
                        anchor: vid(views[0]),
                        zone: DropZone::Tab,
                    }
                },
            )
            .expect("attach");
        }
    }

    #[test]
    fn attach_rejects_duplicate_and_unknown() {
        let mut ws = workspace_with(&[1, 2]);
        attach(&mut ws, 1, &[1, 2]);

        // Same view into another window → DuplicateView.
        assert!(matches!(
            ws.attach_view(vid(1), DockTarget::Window(wid(2))),
            Err(DockError::DuplicateView {
                owner: Some(w),
                ..
            }) if w == wid(1)
        ));
        // Same view into the detached pool → DuplicateView.
        assert!(matches!(
            ws.add_detached(vid(2)),
            Err(DockError::DuplicateView { .. })
        ));
        // Unknown window → UnknownWindow.
        assert!(matches!(
            ws.attach_view(vid(9), DockTarget::Window(wid(99))),
            Err(DockError::UnknownWindow { .. })
        ));
        // Fresh view → ok; owner reported.
        ws.attach_view(vid(3), DockTarget::Window(wid(2))).unwrap();
        assert_eq!(ws.owner_of(vid(3)), Some(wid(2)));
        assert_eq!(ws.all_views(), vec![vid(1), vid(2), vid(3)]);
    }

    #[test]
    fn detach_and_reattach_preserve_view_id() {
        let mut ws = workspace_with(&[1, 2]);
        attach(&mut ws, 1, &[1, 2, 3]);

        ws.detach_view(vid(2)).unwrap();
        assert!(ws.is_detached(vid(2)));
        assert!(ws.owner_of(vid(2)).is_none());
        assert_eq!(
            ws.tree(wid(1)).unwrap().views().collect::<Vec<_>>(),
            vec![vid(1), vid(3)]
        );

        // Reattach to another window — same logical id.
        ws.attach_view(vid(2), DockTarget::Window(wid(2))).unwrap();
        assert_eq!(ws.owner_of(vid(2)), Some(wid(2)));
        assert!(!ws.is_detached(vid(2)));

        // Double-detach → UnknownView.
        ws.detach_view(vid(2)).unwrap();
        assert!(matches!(
            ws.detach_view(vid(2)),
            Err(DockError::UnknownView { .. })
        ));
        // remove_view purges from the pool.
        ws.remove_view(vid(2)).unwrap();
        assert!(matches!(
            ws.detach_view(vid(2)),
            Err(DockError::UnknownView { .. })
        ));
    }

    #[test]
    fn move_view_across_windows() {
        let mut ws = workspace_with(&[1, 2]);
        attach(&mut ws, 1, &[1, 2, 3]);
        attach(&mut ws, 2, &[4]);

        let report = ws
            .move_view(vid(2), DockTarget::Window(wid(2)))
            .expect("move");
        assert_eq!(report.from, Some(wid(1)));
        assert_eq!(report.to, wid(2));
        assert_eq!(ws.owner_of(vid(2)), Some(wid(2)));
        assert_eq!(
            ws.tree(wid(1)).unwrap().views().collect::<Vec<_>>(),
            vec![vid(1), vid(3)]
        );
        assert_eq!(
            ws.tree(wid(2)).unwrap().views().collect::<Vec<_>>(),
            vec![vid(4), vid(2)]
        );

        // Events: MovePrepared + TreeChanged×2 + MoveCommitted.
        let events = ws.drain_events();
        assert!(events.contains(&DockEvent::MovePrepared { view: vid(2) }));
        assert!(events.iter().any(
            |e| matches!(e, DockEvent::MoveCommitted { view, from: Some(f), to }
                    if *view == vid(2) && *f == wid(1) && *to == wid(2))
        ));
    }

    #[test]
    fn move_view_zones_split_the_anchor_leaf() {
        let mut ws = workspace_with(&[1]);
        attach(&mut ws, 1, &[1, 2]);

        ws.move_view(
            vid(2),
            DockTarget::Zone {
                window: wid(1),
                anchor: vid(1),
                zone: DropZone::Right,
            },
        )
        .unwrap();
        match ws.tree(wid(1)).unwrap().root() {
            DockNode::Split {
                axis,
                ratio,
                children,
            } => {
                assert_eq!(*axis, Axis::Horizontal);
                assert!((*ratio - 0.5).abs() < f64::EPSILON);
                assert_eq!(children.len(), 2);
            }
            _ => panic!("expected split root"),
        }

        // Zone move onto self → InvalidTarget.
        assert!(matches!(
            ws.move_view(
                vid(2),
                DockTarget::Zone {
                    window: wid(1),
                    anchor: vid(2),
                    zone: DropZone::Tab,
                }
            ),
            Err(DockError::InvalidTarget { .. })
        ));
        // Zone move onto a missing anchor → InvalidTarget.
        assert!(matches!(
            ws.move_view(
                vid(2),
                DockTarget::Zone {
                    window: wid(1),
                    anchor: vid(99),
                    zone: DropZone::Tab,
                }
            ),
            Err(DockError::InvalidTarget { .. })
        ));
    }

    #[test]
    fn move_last_view_drops_the_tree() {
        let mut ws = workspace_with(&[1, 2]);
        attach(&mut ws, 1, &[1]);
        ws.move_view(vid(1), DockTarget::Window(wid(2))).unwrap();
        assert!(ws.tree(wid(1)).is_none());
        assert!(ws.is_registered(wid(1)));
    }

    #[test]
    fn two_phase_move_rolls_back_when_target_gone() {
        let mut ws = workspace_with(&[1, 2]);
        attach(&mut ws, 1, &[1, 2]);

        // Begin → view locked.
        let token = ws
            .begin_move(vid(1), DockTarget::Window(wid(2)))
            .expect("begin");
        assert!(ws.is_transferring(vid(1)));
        // Edits on the in-transfer view are rejected — no dual-edit.
        assert!(matches!(
            ws.activate_view(vid(1)),
            Err(DockError::ViewBusy { .. })
        ));
        assert!(matches!(
            ws.detach_view(vid(1)),
            Err(DockError::ViewBusy { .. })
        ));
        assert!(matches!(
            ws.begin_move(vid(1), DockTarget::Window(wid(1))),
            Err(DockError::ViewBusy { .. })
        ));

        // Destination destroyed mid-transfer → commit rolls back.
        ws.remove_window(wid(2), WindowCloseDisposition::ReleaseViews)
            .unwrap();
        assert!(matches!(
            ws.commit_move(token),
            Err(DockError::TargetUnavailable { window }) if window == wid(2)
        ));
        // View stayed at its source dock; lock cleared.
        assert_eq!(ws.owner_of(vid(1)), Some(wid(1)));
        assert!(!ws.is_transferring(vid(1)));
        let events = ws.drain_events();
        assert!(events.contains(&DockEvent::MoveAborted { view: vid(1) }));
    }

    #[test]
    fn abort_move_restores_source_and_clears_lock() {
        let mut ws = workspace_with(&[1, 2]);
        attach(&mut ws, 1, &[1, 2]);
        let token = ws.begin_move(vid(2), DockTarget::Window(wid(2))).unwrap();
        ws.abort_move(token).unwrap();
        assert_eq!(ws.owner_of(vid(2)), Some(wid(1)));
        assert!(!ws.is_transferring(vid(2)));
        // Stale token → NotInTransfer.
        assert!(matches!(
            ws.commit_move(token),
            Err(DockError::NotInTransfer)
        ));
    }

    #[test]
    fn remove_window_dispositions_are_consumer_choice() {
        // DetachViews → detached pool.
        let mut ws = workspace_with(&[1, 2]);
        attach(&mut ws, 1, &[1, 2]);
        attach(&mut ws, 2, &[3]);
        let views = ws
            .remove_window(wid(1), WindowCloseDisposition::DetachViews)
            .unwrap();
        assert_eq!(views, vec![vid(1), vid(2)]);
        assert!(ws.is_detached(vid(1)) && ws.is_detached(vid(2)));

        // RelocateTo → target tree gains the views.
        let views = ws.remove_window(
            wid(2),
            WindowCloseDisposition::RelocateTo { target: wid(3) },
        );
        // w3 not registered → InvalidTarget.
        assert!(matches!(views, Err(DockError::InvalidTarget { .. })));
        ws.register_window(wid(3));
        ws.remove_window(
            wid(2),
            WindowCloseDisposition::RelocateTo { target: wid(3) },
        )
        .unwrap();
        assert_eq!(ws.owner_of(vid(3)), Some(wid(3)));

        // ReleaseViews → workspace forgets them.
        ws.remove_window(wid(3), WindowCloseDisposition::ReleaseViews)
            .unwrap();
        assert!(matches!(
            ws.detach_view(vid(3)),
            Err(DockError::UnknownView { .. })
        ));
        // Unknown window → UnknownWindow.
        assert!(matches!(
            ws.remove_window(wid(99), WindowCloseDisposition::DetachViews),
            Err(DockError::UnknownWindow { .. })
        ));
    }

    #[test]
    fn set_ratio_move_tab_activate() {
        let mut ws = workspace_with(&[1]);
        attach(&mut ws, 1, &[1, 2, 3]);
        // Split [1,2,3] tab leaf: move_tab reorders + tracks active.
        ws.activate_view(vid(1)).unwrap();
        ws.move_tab(vid(3), 0).unwrap();
        match ws.tree(wid(1)).unwrap().root() {
            DockNode::Tabs { views, active } => {
                assert_eq!(views, &vec![vid(3), vid(1), vid(2)]);
                assert_eq!(*active, 1); // vid(1) still active
            }
            _ => panic!("expected tabs"),
        }

        // set_ratio on a split path.
        ws.move_view(
            vid(3),
            DockTarget::Zone {
                window: wid(1),
                anchor: vid(1),
                zone: DropZone::Top,
            },
        )
        .unwrap();
        ws.set_ratio(wid(1), &DockPath::root(), 0.7).unwrap();
        match ws.tree(wid(1)).unwrap().root() {
            DockNode::Split { ratio, .. } => assert!((*ratio - 0.7).abs() < f64::EPSILON),
            _ => panic!("expected split"),
        }
        assert!(matches!(
            ws.set_ratio(wid(1), &DockPath::root(), 2.0),
            Err(DockError::InvalidRatio { .. })
        ));
        // Path into a leaf is not a split.
        assert!(matches!(
            ws.set_ratio(wid(1), &DockPath::root().child(0), 0.5),
            Err(DockError::BadPath)
        ));
    }

    #[test]
    fn attach_tree_rejects_occupied_and_conflicting() {
        let mut ws = workspace_with(&[1, 2]);
        attach(&mut ws, 1, &[1]);
        // Window already has a tree → InvalidTarget.
        assert!(matches!(
            ws.attach_tree(wid(1), DockTree::new(leaf(&[2])).unwrap()),
            Err(DockError::InvalidTarget { .. })
        ));
        // Tree view conflicts with another window → DuplicateView.
        assert!(matches!(
            ws.attach_tree(wid(2), DockTree::new(leaf(&[1])).unwrap()),
            Err(DockError::DuplicateView { .. })
        ));
        // Clean attach.
        ws.attach_tree(wid(2), DockTree::new(leaf(&[2, 3])).unwrap())
            .unwrap();
        assert_eq!(ws.tree(wid(2)).unwrap().view_count(), 2);
    }

    // -- persistence tests ------------------------------------------------------

    fn monitor(name: &str, w: u32, h: u32, x: i32, y: i32, sf: f64) -> MonitorInfo {
        MonitorInfo {
            name: Some(name.to_string()),
            width: w,
            height: h,
            x,
            y,
            scale_factor: sf,
        }
    }

    fn geometry(x: i32, y: i32, w: u32, h: u32) -> WindowGeometry {
        WindowGeometry {
            x,
            y,
            width: w,
            height: h,
            monitor: None,
        }
    }

    #[test]
    fn snapshot_roundtrip_preserves_layout() {
        let mut ws = workspace_with(&[1, 2]);
        attach(&mut ws, 1, &[1, 2]);
        ws.move_view(
            vid(2),
            DockTarget::Zone {
                window: wid(1),
                anchor: vid(1),
                zone: DropZone::Bottom,
            },
        )
        .unwrap();
        attach(&mut ws, 2, &[3]);
        ws.add_detached(vid(9)).unwrap();

        let wins = ws.windows();
        let geom = vec![geometry(10, 20, 800, 600), geometry(900, 20, 640, 480)];
        let snap = LayoutSnapshot::capture(&wins, &geom, &ws).unwrap();
        let json = snap.to_json().unwrap();
        let back = LayoutSnapshot::from_json(&json).unwrap();

        let mons = vec![monitor("main", 1920, 1080, 0, 0, 1.0)];
        let restored = back.restore(&mons).unwrap();
        assert_eq!(restored.placements.len(), 2);
        assert_eq!(
            (restored.placements[0].x, restored.placements[0].y),
            (10, 20)
        );
        assert_eq!(restored.detached, vec![vid(9)]);

        // Replay into a fresh workspace → identical dock structure.
        let mut ws2 = workspace_with(&[10, 20]);
        ws2.apply_restored_tree(wid(10), restored.placements[0].tree.clone().unwrap())
            .unwrap();
        ws2.apply_restored_tree(wid(20), restored.placements[1].tree.clone().unwrap())
            .unwrap();
        for v in restored.detached {
            ws2.add_detached(v).unwrap();
        }
        assert_eq!(ws2.owner_of(vid(1)), Some(wid(10)));
        assert_eq!(ws2.owner_of(vid(2)), Some(wid(10)));
        assert_eq!(ws2.owner_of(vid(3)), Some(wid(20)));
        assert!(ws2.is_detached(vid(9)));
        // Split structure survived.
        assert!(matches!(
            ws2.tree(wid(10)).unwrap().root(),
            DockNode::Split {
                axis: Axis::Vertical,
                ..
            }
        ));
    }

    #[test]
    fn restore_rejects_bad_schema_and_structural_violations() {
        let mons = vec![monitor("main", 1920, 1080, 0, 0, 1.0)];

        // Wrong schema → UnsupportedSchema.
        let mut snap = LayoutSnapshot {
            schema: 99,
            ..LayoutSnapshot::default()
        };
        assert!(matches!(
            snap.restore(&mons),
            Err(DockError::UnsupportedSchema { found: 99 })
        ));

        // Duplicate view across two windows' trees → InvalidSnapshot.
        snap.schema = LAYOUT_SCHEMA_VERSION;
        snap.windows = vec![
            WindowSnapshot {
                x: 0,
                y: 0,
                width: 100,
                height: 100,
                monitor: None,
                tree: Some(leaf(&[1, 2])),
            },
            WindowSnapshot {
                x: 0,
                y: 0,
                width: 100,
                height: 100,
                monitor: None,
                tree: Some(leaf(&[2])),
            },
        ];
        assert!(matches!(
            snap.restore(&mons),
            Err(DockError::InvalidSnapshot { .. })
        ));

        // Invalid ratio in a tree → InvalidSnapshot.
        snap.windows[1].tree = Some(DockNode::Split {
            axis: Axis::Horizontal,
            ratio: 0.0,
            children: vec![leaf(&[7]), leaf(&[8])],
        });
        snap.windows[0].tree = Some(leaf(&[1, 2]));
        assert!(matches!(
            snap.restore(&mons),
            Err(DockError::InvalidSnapshot { .. })
        ));

        // Zero-size geometry → InvalidSnapshot.
        snap.windows[1].tree = Some(leaf(&[2]));
        snap.windows[1].width = 0;
        assert!(matches!(
            snap.restore(&mons),
            Err(DockError::InvalidSnapshot { .. })
        ));

        // Malformed JSON → InvalidSnapshot.
        assert!(matches!(
            LayoutSnapshot::from_json("{not json"),
            Err(DockError::InvalidSnapshot { .. })
        ));

        // restore_or_default never fails → safe empty layout.
        let bad = LayoutSnapshot::from_json("{not json").unwrap_err();
        let _ = bad;
        let restored = LayoutSnapshot {
            schema: 42,
            ..LayoutSnapshot::default()
        }
        .restore_or_default(&mons);
        assert!(restored.placements.is_empty() && restored.detached.is_empty());
    }

    #[test]
    fn restore_corrects_offscreen_and_missing_monitor() {
        let mons = vec![
            monitor("left", 1920, 1080, 0, 0, 1.0),
            monitor("right", 2560, 1440, 1920, 0, 1.5),
        ];
        // Center inside monitor 1 → geometry kept, monitor resolved.
        assert_eq!(
            correct_bounds(2000, 100, 800, 600, None, &mons),
            (2000, 100, 800, 600, Some(1))
        );

        // Off-screen + matching signature → re-centered on that monitor.
        let hint = MonitorHint {
            name: Some("right".to_string()),
            width: 2560,
            height: 1440,
            scale_factor: 1.5,
        };
        let (px, py, pw, ph, pm) = correct_bounds(-5000, -4000, 800, 600, Some(&hint), &mons);
        assert_eq!(pm, Some(1));
        let cx = i64::from(px) + i64::from(pw) / 2;
        let cy = i64::from(py) + i64::from(ph) / 2;
        assert!((1920..1920 + 2560).contains(&cx) && (0..1440).contains(&cy));

        // Signature gone → falls back to the first monitor, clamped.
        let (_, _, pw, ph, pm) = correct_bounds(
            99999,
            99999,
            3000,
            2000,
            Some(&MonitorHint {
                name: Some("gone".to_string()),
                width: 3840,
                height: 2160,
                scale_factor: 2.0,
            }),
            &mons,
        );
        assert_eq!(pm, Some(0));
        assert!(pw <= 1920 && ph <= 1080);

        // No monitors → passthrough with None.
        assert_eq!(correct_bounds(1, 2, 3, 4, None, &[]), (1, 2, 3, 4, None));
    }

    #[test]
    fn restore_normalizes_dirty_trees() {
        let mons = vec![monitor("main", 1920, 1080, 0, 0, 1.0)];
        let snap = LayoutSnapshot {
            schema: LAYOUT_SCHEMA_VERSION,
            windows: vec![WindowSnapshot {
                x: 0,
                y: 0,
                width: 100,
                height: 100,
                monitor: None,
                // Single-child split + empty leaf → collapses to
                // Tabs[1]; out-of-range active clamps.
                tree: Some(DockNode::Split {
                    axis: Axis::Horizontal,
                    ratio: 0.5,
                    children: vec![DockNode::Tabs {
                        views: vec![vid(1)],
                        active: 42,
                    }],
                }),
            }],
            detached: vec![],
        };
        let restored = snap.restore(&mons).unwrap();
        let tree = restored.placements[0].tree.as_ref().unwrap();
        match tree.root() {
            DockNode::Tabs { views, active } => {
                assert_eq!(views, &vec![vid(1)]);
                assert_eq!(*active, 0);
            }
            _ => panic!("expected collapsed tabs root"),
        }
    }
}
