//! Host-resolved parameter value observability: the
//! [`ParamDispatchLedger`] tracks each scheduled parameter update
//! through `Scheduled` → `Dispatched` → terminal (`Dropped` /
//! `Reported` / `Unavailable`) so a consumer can distinguish
//! *sent-planned*, *sent*, and *plugin-reported* values.
//!
//! ## Honesty contract
//!
//! - `Dispatched` is a **deterministic projection**: it is recomputed
//!   from the installed [`TransportPlan`], the emitted
//!   [`TransportSpan`] and the physical call window — it is *not* a
//!   plugin-side acknowledgment and does not prove the value reached
//!   DSP code.
//! - `Reported` is the value the plugin *claims*; no adapter
//!   currently provides a plugin→host reporting channel, so
//!   [`ParamDispatchLedger::mark_reporting_unavailable`] is the
//!   expected terminal stage (`Unavailable`) — the plugin-internal
//!   true value is never guaranteed.
//! - Every drop is attributed to a [`DropReason`]; nothing silently
//!   disappears.
//!
//! **RT contract:** non-RT only — the ledger allocates and lives on
//! the control side, fed by `install_plan`, span/call projection and
//! drained [`NodeReport`]s.

use std::collections::BTreeMap;

use ruviform_graph::NodeId;
use ruviform_media_core::{
    EngineFrame, MediaTime, TransportPlan, TransportSpan, event_engine_frame,
};
use ruviform_plugin_core::{EventKind, ParamValue};

use crate::control::NodeReport;
use crate::target::{ParamTargetTable, TargetId};

/// Where a scheduled parameter update stands.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum DispatchStage {
    /// In the installed plan, no physical call has covered its
    /// position yet — *sent-planned*.
    Scheduled,
    /// Projected onto a physical `process` call at `call_start` +
    /// `frame_offset` — *sent*. Projection, not acknowledgment (see
    /// module docs).
    Dispatched {
        /// Absolute engine frame the physical call starts at.
        call_start: u64,
        /// Offset of the event inside that call.
        frame_offset: u32,
    },
    /// Dropped on the way — see [`DropReason`].
    Dropped {
        /// Why the update did not reach the plugin.
        reason: DropReason,
    },
    /// A plugin-reported value exists (payload lives in
    /// [`ParamValueObservation::value`]).
    Reported,
    /// No plugin→host reporting channel exists — the observable
    /// trail ends at `Dispatched`; *not* an error.
    Unavailable,
}

/// Why a scheduled update was dropped — a defined failure, never a
/// silent one.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum DropReason {
    /// The per-call event scratch could not hold the window —
    /// [`NodeReport::EventScratchOverflow`].
    EventScratchOverflow,
    /// `PluginEndpoint::process` returned an error — payload is the
    /// report's coarse error category.
    EndpointError {
        /// Coarse error category from [`NodeReport::EndpointError`].
        kind: u8,
    },
    /// The span could not be partitioned into legal physical calls —
    /// [`NodeReport::UndeliverableSpan`].
    UndeliverableSpan,
    /// A scheduler fault (stale generation, undeliverable apply, …).
    ScheduleFault,
    /// A newer plan replaced this one before delivery.
    PlanReplaced,
    /// The plan never installed — the [`crate::PlanVault`] was full.
    VaultExceeded,
}

/// One observable update: which target, which stage, which value.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ParamValueObservation {
    /// The application target.
    pub target: TargetId,
    /// Current stage.
    pub stage: DispatchStage,
    /// The value in flight: host-resolved for `Scheduled`/`Dispatched`,
    /// plugin-reported for `Reported`, `None` for `Unavailable`.
    pub value: Option<ParamValue>,
    /// Ledger revision (bumped per `record_plan`).
    pub revision: u64,
    /// Scheduled absolute position.
    pub position: MediaTime,
}

#[derive(Copy, Clone, Debug)]
struct Entry {
    target: TargetId,
    event_index: usize,
    position: MediaTime,
    value: ParamValue,
    source_sequence: u64,
    stage: DispatchStage,
    revision: u64,
}

/// Non-RT observability ledger for parameter updates delivered via
/// `TransportPlan<EventKind>` — see module docs for the honesty
/// contract of each stage.
///
/// # Examples
///
/// ```
/// use ruviform_audio_plugin_bridge::{DispatchStage, ParamDispatchLedger};
///
/// let ledger = ParamDispatchLedger::new();
/// assert_eq!(ledger.len(), 0);
/// # let _ = DispatchStage::Scheduled;
/// ```
#[derive(Debug, Default)]
pub struct ParamDispatchLedger {
    entries: Vec<Entry>,
    revision: u64,
    last_call: Option<(u64, u32)>,
}

impl ParamDispatchLedger {
    /// An empty ledger.
    ///
    /// **RT contract:** non-RT — constructor.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
            revision: 0,
            last_call: None,
        }
    }

    /// Number of tracked updates.
    ///
    /// **RT contract:** non-RT — `const` accessor.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the ledger is empty.
    ///
    /// **RT contract:** non-RT — `const` accessor.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Records every `ParamValue` event in `plan` whose `ParamId`
    /// resolves to a live binding for `node` in `table` — entries
    /// start as [`DispatchStage::Scheduled`]. Events for unbound
    /// parameters are *not* tracked (they were never addressable
    /// through this table).
    ///
    /// Returns the number of recorded entries.
    ///
    /// **RT contract:** non-RT — allocates per recorded event.
    pub fn record_plan(
        &mut self,
        plan: &TransportPlan<EventKind>,
        table: &ParamTargetTable,
        node: NodeId,
    ) -> usize {
        self.revision = self.revision.wrapping_add(1);
        let revision = self.revision;
        let mut recorded = 0usize;
        for (event_index, event) in plan.events.iter().enumerate() {
            if let EventKind::ParamValue(param_event) = event.payload
                && let Some(target) = table.target_for(node, param_event.param_id)
            {
                self.entries.push(Entry {
                    target,
                    event_index,
                    position: event.position,
                    value: param_event.value,
                    source_sequence: event.source_sequence,
                    stage: DispatchStage::Scheduled,
                    revision,
                });
                recorded += 1;
            }
        }
        recorded
    }

    /// Marks previously `Scheduled` entries as `PlanReplaced` when a
    /// newer plan is installed — old-plan events that never reached a
    /// call do not silently vanish.
    ///
    /// **RT contract:** non-RT — bounded scan.
    pub fn mark_plan_replaced(&mut self) {
        for entry in &mut self.entries {
            if matches!(entry.stage, DispatchStage::Scheduled) {
                entry.stage = DispatchStage::Dropped {
                    reason: DropReason::PlanReplaced,
                };
            }
        }
    }

    /// Projects `Scheduled` entries onto the physical call
    /// `[call_start, call_start + call_frames)` of `span`: every entry
    /// whose [`event_engine_frame`] lands inside becomes
    /// [`DispatchStage::Dispatched`] with its in-call offset. Entries
    /// outside this call keep `Scheduled` — a later call picks them
    /// up; entries whose generation does not match the span get
    /// `None` from `event_engine_frame` and are left for
    /// [`Self::apply_report`] to attribute.
    ///
    /// Returns how many entries transitioned to `Dispatched`.
    ///
    /// **RT contract:** non-RT — bounded scans, no allocation.
    pub fn project_call(
        &mut self,
        plan: &TransportPlan<EventKind>,
        span: &TransportSpan,
        call_start: EngineFrame,
        call_frames: u32,
    ) -> usize {
        let mut dispatched = 0usize;
        for entry in &mut self.entries {
            if !matches!(entry.stage, DispatchStage::Scheduled) {
                continue;
            }
            let Some(event) = plan.events.get(entry.event_index) else {
                continue;
            };
            let Some(frame) = event_engine_frame(span, event) else {
                continue;
            };
            let Some(offset) = frame.checked_offset(call_start) else {
                continue;
            };
            if offset < u64::from(call_frames) {
                entry.stage = DispatchStage::Dispatched {
                    call_start: call_start.frame(),
                    frame_offset: u32::try_from(offset).unwrap_or(u32::MAX),
                };
                dispatched += 1;
            }
        }
        self.last_call = Some((call_start.frame(), call_frames));
        dispatched
    }

    /// Maps a drained [`NodeReport`] onto entries it can be
    /// attributed to — returns how many entries moved to `Dropped`.
    ///
    /// Attribution is deliberately conservative:
    /// [`NodeReport::UndeliverableSpan`] drops entries dispatched into
    /// calls inside `[start, end)`; scratch overflow and endpoint
    /// errors drop the entries dispatched into the *most recently
    /// projected call*; a scheduler `Fault` drops every still-pending
    /// entry (`Scheduled`/`Dispatched`) as `ScheduleFault`. Receipts
    /// affect no entries.
    ///
    /// **RT contract:** non-RT — bounded scans, no allocation.
    pub fn apply_report(&mut self, report: &NodeReport) -> usize {
        match report {
            NodeReport::UndeliverableSpan { start, end } => {
                let mut dropped = 0usize;
                for entry in &mut self.entries {
                    if let DispatchStage::Dispatched { call_start, .. } = entry.stage
                        && call_start >= *start
                        && call_start < *end
                    {
                        entry.stage = DispatchStage::Dropped {
                            reason: DropReason::UndeliverableSpan,
                        };
                        dropped += 1;
                    }
                }
                dropped
            }
            NodeReport::EventScratchOverflow | NodeReport::EndpointError { .. } => {
                let reason = match report {
                    NodeReport::EventScratchOverflow => DropReason::EventScratchOverflow,
                    NodeReport::EndpointError { kind } => DropReason::EndpointError { kind: *kind },
                    _ => unreachable!(),
                };
                let mut dropped = 0usize;
                if let Some((last_start, _)) = self.last_call {
                    for entry in &mut self.entries {
                        if matches!(
                            entry.stage,
                            DispatchStage::Dispatched { call_start, .. } if call_start == last_start
                        ) {
                            entry.stage = DispatchStage::Dropped { reason };
                            dropped += 1;
                        }
                    }
                }
                dropped
            }
            NodeReport::Fault(_) => {
                let mut dropped = 0usize;
                for entry in &mut self.entries {
                    if matches!(
                        entry.stage,
                        DispatchStage::Scheduled | DispatchStage::Dispatched { .. }
                    ) {
                        entry.stage = DispatchStage::Dropped {
                            reason: DropReason::ScheduleFault,
                        };
                        dropped += 1;
                    }
                }
                dropped
            }
            NodeReport::VaultExceeded => {
                let mut dropped = 0usize;
                for entry in &mut self.entries {
                    if matches!(entry.stage, DispatchStage::Scheduled) {
                        entry.stage = DispatchStage::Dropped {
                            reason: DropReason::VaultExceeded,
                        };
                        dropped += 1;
                    }
                }
                dropped
            }
            NodeReport::Receipt(_) => 0,
        }
    }

    /// Records a plugin-reported `value` for `target` — moves the
    /// latest non-terminal entry to [`DispatchStage::Reported`].
    /// Returns `false` when no eligible entry exists.
    ///
    /// **RT contract:** non-RT — bounded scan.
    pub fn record_reported(&mut self, target: TargetId, value: ParamValue) -> bool {
        let Some(entry) = self.entries.iter_mut().rev().find(|entry| {
            entry.target == target
                && matches!(
                    entry.stage,
                    DispatchStage::Scheduled | DispatchStage::Dispatched { .. }
                )
        }) else {
            return false;
        };
        entry.stage = DispatchStage::Reported;
        entry.value = value;
        true
    }

    /// Declares that no plugin→host reporting channel exists: every
    /// still-pending (`Scheduled`/`Dispatched`) entry moves to
    /// [`DispatchStage::Unavailable`] — the honest terminal stage, not
    /// an error.
    ///
    /// **RT contract:** non-RT — bounded scan.
    pub fn mark_reporting_unavailable(&mut self) {
        for entry in &mut self.entries {
            if matches!(
                entry.stage,
                DispatchStage::Scheduled | DispatchStage::Dispatched { .. }
            ) {
                entry.stage = DispatchStage::Unavailable;
            }
        }
    }

    /// The latest observation for `target` — by `(revision,
    /// source_sequence)` order.
    ///
    /// **RT contract:** non-RT — bounded scan.
    #[must_use]
    pub fn query(&self, target: TargetId) -> Option<ParamValueObservation> {
        self.entries
            .iter()
            .filter(|entry| entry.target == target)
            .max_by_key(|entry| (entry.revision, entry.source_sequence))
            .map(|entry| ParamValueObservation {
                target: entry.target,
                stage: entry.stage,
                value: match entry.stage {
                    DispatchStage::Unavailable => None,
                    _ => Some(entry.value),
                },
                revision: entry.revision,
                position: entry.position,
            })
    }

    /// All current observations, grouped per target with the latest
    /// entry winning — one row per target.
    ///
    /// **RT contract:** non-RT — allocates the returned map.
    #[must_use]
    pub fn observations(&self) -> BTreeMap<TargetId, ParamValueObservation> {
        let mut out = BTreeMap::new();
        for entry in &self.entries {
            out.insert(
                entry.target,
                ParamValueObservation {
                    target: entry.target,
                    stage: entry.stage,
                    value: match entry.stage {
                        DispatchStage::Unavailable => None,
                        _ => Some(entry.value),
                    },
                    revision: entry.revision,
                    position: entry.position,
                },
            );
        }
        out
    }
}
