//! Application-stable parameter targets: [`TargetId`] ↔
//! plugin-local [`ParamId`] bindings with a [`BindingGeneration`]
//! that changes on every rebind.
//!
//! `ParamId` is plugin-local — it is only meaningful for the plugin
//! instance that enumerated it, and it may change across plugin
//! updates or state restores. Consumers therefore address parameters
//! by an application-stable [`TargetId`] and let [`ParamTargetTable`]
//! resolve it to a [`ParamBinding`]. Resolution is explicit:
//!
//! - [`TargetResolution::Resolved`] — a live binding.
//! - [`TargetResolution::Unresolved`] — no binding recorded; any
//!   write request for it is a defined rejection, never a silent
//!   fallback to a wrong parameter.
//! - [`TargetResolution::Defunct`] — a binding exists but the target
//!   failed revalidation (the parameter vanished from enumeration,
//!   or the binding was explicitly invalidated after a plugin
//!   change). Stale state-restore candidates surface here instead of
//!   misapplying values.
//!
//! Generation pinning: consumers that open a control transaction
//! (e.g. a gesture) capture the binding's [`BindingGeneration`] and
//! later re-check it with [`ParamTargetTable::resolve_pin`] — a
//! `None` result means the target was rebound in between and the
//! pinned write must be refused.
//!
//! **RT contract:** non-RT only — the table lives on the control
//! side; the RT path only consumes the resolved [`ParamBinding`].

use std::collections::BTreeMap;

use ruviform_graph::NodeId;
use ruviform_plugin_core::{ParamId, ParamInfo, ParamWriteRejection};

/// Application-stable parameter target identifier.
///
/// The value's allocation policy is owned by the consumer —
/// Ruviform only requires stability within a session/persisted
/// project. `Copy` value type.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TargetId(pub u64);

/// Monotonic binding revision: bumped on every (re)bind so a pinned
/// write can detect that the target moved underneath it.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BindingGeneration(pub u64);

impl BindingGeneration {
    /// The next generation value (wrapping at `u64::MAX`).
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn next(self) -> Self {
        Self(self.0.wrapping_add(1))
    }
}

/// A resolved binding from an application [`TargetId`] to a
/// plugin-local `(node, param)` pair.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ParamBinding {
    /// The application-stable target.
    pub target: TargetId,
    /// The graph node hosting the plugin.
    pub node: NodeId,
    /// The plugin-local parameter id.
    pub param: ParamId,
    /// Revision of this binding — bumped on rebind.
    pub generation: BindingGeneration,
}

/// The outcome of resolving a [`TargetId`].
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum TargetResolution {
    /// A live binding.
    Resolved(ParamBinding),
    /// No binding is recorded for the target.
    Unresolved,
    /// A binding exists but failed revalidation — the parameter is
    /// no longer enumerated or the binding was invalidated. Writes
    /// must be refused, not redirected.
    Defunct,
}

/// Non-RT binding table: `TargetId` → `ParamBinding` with generation
/// tracking and revalidation against the current parameter
/// enumeration.
///
/// # Examples
///
/// ```
/// use ruviform_audio_plugin_bridge::{ParamTargetTable, TargetId, TargetResolution};
/// use ruviform_graph::{GraphBuilder, NodeDef, NodeKind};
/// use ruviform_plugin_core::{ParamFlags, ParamId, ParamInfo};
///
/// const PARAMS: [ParamInfo; 1] = [ParamInfo {
///     id: ParamId(3),
///     name: "Gain",
///     unit: "",
///     min_value: 0.0,
///     max_value: 1.0,
///     default_value: 0.0,
///     flags: ParamFlags::AUTOMATABLE,
/// }];
/// let mut builder = GraphBuilder::new();
/// let (node, _) = builder.add_node(&NodeDef {
///     kind: NodeKind::Processor,
///     ports: Vec::new(),
/// });
/// let mut table = ParamTargetTable::new();
/// let binding = table.bind(TargetId(1), node, ParamId(3), &PARAMS).unwrap();
/// assert!(matches!(
///     table.resolve(TargetId(1)),
///     TargetResolution::Resolved(_)
/// ));
/// assert!(matches!(
///     table.resolve(TargetId(9)),
///     TargetResolution::Unresolved
/// ));
/// assert!(table.bind(TargetId(2), node, ParamId(9), &PARAMS).is_err());
/// # let _ = binding;
/// ```
#[derive(Debug, Default)]
pub struct ParamTargetTable {
    bindings: BTreeMap<TargetId, ParamBinding>,
    defunct: BTreeMap<TargetId, ParamBinding>,
    next_generation: u64,
}

impl ParamTargetTable {
    /// An empty table.
    ///
    /// **RT contract:** non-RT — allocation-free constructor.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            bindings: BTreeMap::new(),
            defunct: BTreeMap::new(),
            next_generation: 0,
        }
    }

    /// Binds `target` to `(node, param)` — bumps the binding
    /// generation even when the mapping is unchanged so pinned
    /// writers always observe the rebind.
    ///
    /// # Errors
    ///
    /// [`ParamWriteRejection::UnknownParam`] when `params` does not
    /// enumerate `param` — a binding to a parameter the plugin does
    /// not expose is refused up front.
    pub fn bind(
        &mut self,
        target: TargetId,
        node: NodeId,
        param: ParamId,
        params: &[ParamInfo],
    ) -> Result<ParamBinding, ParamWriteRejection> {
        if !params.iter().any(|info| info.id == param) {
            return Err(ParamWriteRejection::UnknownParam);
        }
        self.next_generation = self.next_generation.wrapping_add(1);
        let binding = ParamBinding {
            target,
            node,
            param,
            generation: BindingGeneration(self.next_generation),
        };
        self.bindings.insert(target, binding);
        self.defunct.remove(&target);
        Ok(binding)
    }

    /// Removes the binding; `true` when one existed.
    ///
    /// **RT contract:** non-RT — map operation.
    pub fn unbind(&mut self, target: TargetId) -> bool {
        self.defunct.remove(&target);
        self.bindings.remove(&target).is_some()
    }

    /// Resolves `target` against the recorded bindings.
    ///
    /// **RT contract:** non-RT — map lookup.
    #[must_use]
    pub fn resolve(&self, target: TargetId) -> TargetResolution {
        if self.bindings.contains_key(&target) {
            TargetResolution::Resolved(self.bindings[&target])
        } else if self.defunct.contains_key(&target) {
            TargetResolution::Defunct
        } else {
            TargetResolution::Unresolved
        }
    }

    /// Resolves `target` only when its live generation still equals
    /// `pinned` — `None` means the target was rebound after the pin
    /// was taken and the write must be refused (defined rejection),
    /// not applied to the moved binding.
    ///
    /// **RT contract:** non-RT — map lookup.
    #[must_use]
    pub fn resolve_pin(&self, target: TargetId, pinned: BindingGeneration) -> Option<ParamBinding> {
        match self.resolve(target) {
            TargetResolution::Resolved(binding) if binding.generation == pinned => Some(binding),
            _ => None,
        }
    }

    /// Revalidates the binding for `target` against the plugin's
    /// current enumeration: a parameter that vanished flips the
    /// binding to [`TargetResolution::Defunct`] (and returns `false`)
    /// so later resolves refuse writes instead of misapplying them.
    ///
    /// **RT contract:** non-RT — map operation + bounded scan.
    pub fn revalidate(&mut self, target: TargetId, node: NodeId, params: &[ParamInfo]) -> bool {
        let Some(binding) = self.bindings.get(&target).copied() else {
            return false;
        };
        if binding.node == node && params.iter().any(|info| info.id == binding.param) {
            return true;
        }
        self.bindings.remove(&target);
        self.defunct.insert(target, binding);
        false
    }

    /// Marks every binding that points at `node` defunct — the node
    /// was detached or its plugin replaced.
    ///
    /// **RT contract:** non-RT — map operation.
    pub fn invalidate_node(&mut self, node: NodeId) {
        let moved: Vec<TargetId> = self
            .bindings
            .values()
            .filter_map(|binding| (binding.node == node).then_some(binding.target))
            .collect();
        for target in moved {
            if let Some(binding) = self.bindings.remove(&target) {
                self.defunct.insert(target, binding);
            }
        }
    }

    /// Finds the target bound to `(node, param)`, if any — used when
    /// translating delivered [`ParamEvent`]s back into application
    /// targets for observability.
    ///
    /// [`ParamEvent`]: ruviform_plugin_core::ParamEvent
    ///
    /// **RT contract:** non-RT — bounded `O(n)` scan.
    #[must_use]
    pub fn target_for(&self, node: NodeId, param: ParamId) -> Option<TargetId> {
        self.bindings
            .values()
            .find(|binding| binding.node == node && binding.param == param)
            .map(|binding| binding.target)
    }
}
