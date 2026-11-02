//! Standard-neutral parameter *write contract*: which update means a
//! parameter accepts ([`ParamWriteMeans`]), the typed rejection
//! returned when it does not ([`ParamWriteRejection`]), and the
//! validation helpers ([`check_param_write`],
//! [`check_param_write_for`]) plus the opt-in continuous-value
//! coalescing helper ([`coalesce_param_events_latest`]).
//!
//! ## Means versus flags
//!
//! A write means describes *how* an update reaches the plugin; the
//! [`ParamFlags`] on [`ParamInfo`] describe *whether* the parameter
//! accepts it. [`check_param_write`] combines both:
//!
//! - [`ParamWriteMeans::TimedEvent`] — a [`ParamEvent`] delivered
//!   inside an [`EventList`] at a `frame_offset` (the sample-accurate
//!   path). Requires [`ParamFlags::AUTOMATABLE`]; rejected by
//!   [`ParamFlags::READONLY`].
//! - [`ParamWriteMeans::BlockBoundary`] — a block-rate update applied
//!   at frame offset `0`. Does not require `AUTOMATABLE` (no
//!   automation guarantee is implied); still rejected by `READONLY`.
//! - [`ParamWriteMeans::Immediate`] — an out-of-band immediate write.
//!   The neutral contract has no immediate-write verb, so this is
//!   always [`ParamWriteRejection::UnsupportedMeans`].
//! - [`ParamWriteMeans::PerNote`] — per-note/per-voice targeting.
//!   [`EventKind`] carries no note identity, so this is always
//!   [`ParamWriteRejection::UnsupportedMeans`] regardless of
//!   [`ParamFlags::MODULATABLE`] (that flag only reports that the
//!   underlying standard *advertises* per-note modulation).
//!
//! `STEPPED` is deliberately *not* checked here: the quantization
//! rule (step count / legal values) is not carried by [`ParamInfo`],
//! so the neutral layer cannot apply it — a consumer that knows the
//! rule must quantize before writing and must reject continuous
//! requests it cannot quantize.
//!
//! [`ParamEvent`]: crate::ParamEvent
//! [`EventList`]: crate::EventList
//! [`EventKind`]: crate::EventKind
//! [`ParamFlags`]: crate::ParamFlags
//! [`ParamInfo`]: crate::ParamInfo

use ruviform_media_core::MediaTime;

use crate::event::ParamEvent;
use crate::param::{ParamFlags, ParamId, ParamInfo};

/// How a parameter update reaches the plugin.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy — new means (e.g. per-note targeting once [`EventKind`]
/// gains note identity) may be added without breaking downstream
/// `match` arms.
///
/// [`EventKind`]: crate::EventKind
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ParamWriteMeans {
    /// A [`ParamEvent`] delivered inside an [`EventList`] at a
    /// `frame_offset` — the sample-accurate path.
    ///
    /// [`ParamEvent`]: crate::ParamEvent
    /// [`EventList`]: crate::EventList
    TimedEvent,
    /// A block-rate update applied at frame offset `0` — the degraded
    /// precision path for targets that do not accept timed events.
    BlockBoundary,
    /// An out-of-band immediate write. The neutral contract has no
    /// immediate-write verb; always rejected as
    /// [`ParamWriteRejection::UnsupportedMeans`].
    Immediate,
    /// Per-note / per-voice targeted update. [`EventKind`] carries no
    /// note identity; always rejected as
    /// [`ParamWriteRejection::UnsupportedMeans`].
    ///
    /// [`EventKind`]: crate::EventKind
    PerNote,
}

/// Why a parameter write was rejected — a defined rejection, never a
/// silent misapplication.
///
/// The enum is `#[non_exhaustive]` per the unified public-enum API
/// policy.
///
/// **RT contract:** RT-suitable — `Copy` value type.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ParamWriteRejection {
    /// [`ParamFlags::READONLY`] is set — the host may not write this
    /// parameter at all.
    ReadOnly,
    /// [`ParamFlags::AUTOMATABLE`] is clear — the parameter does not
    /// accept timed (automation) updates.
    NotAutomatable,
    /// The requested [`ParamWriteMeans`] is not part of the neutral
    /// contract (e.g. immediate writes, per-note targeting).
    UnsupportedMeans,
    /// The requested precision (e.g. sample-accurate) cannot be
    /// honored for this target — the caller must degrade or refuse.
    /// Reserved for capability composition; [`check_param_write`]
    /// itself does not produce it.
    UnsupportedPrecision,
    /// No [`ParamInfo`] with the requested [`ParamId`] exists — the
    /// binding targets a parameter the plugin does not enumerate.
    UnknownParam,
    /// The supplied value was `NaN`/`±inf`. Normally unreachable
    /// because [`ParamValue::new`](crate::ParamValue::new) is the
    /// single authority rejecting non-finite input; retained for
    /// callers translating external, unvalidated representations.
    NonFiniteValue,
}

impl core::fmt::Display for ParamWriteRejection {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let text = match self {
            Self::ReadOnly => "param_readonly",
            Self::NotAutomatable => "param_not_automatable",
            Self::UnsupportedMeans => "param_unsupported_means",
            Self::UnsupportedPrecision => "param_unsupported_precision",
            Self::UnknownParam => "param_unknown",
            Self::NonFiniteValue => "param_non_finite",
        };
        f.write_str(text)
    }
}

/// Checks whether `info` accepts a write via `means`.
///
/// Read-only is checked after the means itself is known to be
/// supported, so `Immediate`/`PerNote` report `UnsupportedMeans`
/// rather than `ReadOnly` even on a read-only parameter — the means
/// is the more fundamental rejection.
///
/// **RT contract:** RT-suitable — pure flag test, allocation-free.
///
/// # Errors
///
/// [`ParamWriteRejection`] per the module-level means × flags matrix.
///
/// # Examples
///
/// ```
/// use ruviform_plugin_core::{
///     ParamFlags, ParamId, ParamInfo, ParamWriteMeans, check_param_write,
/// };
///
/// const GAIN: ParamInfo = ParamInfo {
///     id: ParamId(0),
///     name: "Gain",
///     unit: "dB",
///     min_value: -60.0,
///     max_value: 12.0,
///     default_value: 0.0,
///     flags: ParamFlags::AUTOMATABLE,
/// };
/// assert!(check_param_write(&GAIN, ParamWriteMeans::TimedEvent).is_ok());
/// assert!(check_param_write(&GAIN, ParamWriteMeans::PerNote).is_err());
/// ```
pub fn check_param_write(
    info: &ParamInfo,
    means: ParamWriteMeans,
) -> Result<(), ParamWriteRejection> {
    match means {
        ParamWriteMeans::TimedEvent => {
            if info.flags.contains(ParamFlags::READONLY) {
                Err(ParamWriteRejection::ReadOnly)
            } else if !info.flags.contains(ParamFlags::AUTOMATABLE) {
                Err(ParamWriteRejection::NotAutomatable)
            } else {
                Ok(())
            }
        }
        ParamWriteMeans::BlockBoundary => {
            if info.flags.contains(ParamFlags::READONLY) {
                Err(ParamWriteRejection::ReadOnly)
            } else {
                Ok(())
            }
        }
        ParamWriteMeans::Immediate | ParamWriteMeans::PerNote => {
            Err(ParamWriteRejection::UnsupportedMeans)
        }
    }
}

/// Checks whether the parameter `id` in `params` accepts a write via
/// `means` — [`ParamWriteRejection::UnknownParam`] when `params` does
/// not enumerate `id`.
///
/// **RT contract:** RT-suitable — bounded `O(n)` scan, allocation-free.
///
/// # Errors
///
/// [`ParamWriteRejection::UnknownParam`] when `id` is absent; otherwise
/// the [`check_param_write`] result for the matching [`ParamInfo`].
///
/// # Examples
///
/// ```
/// use ruviform_plugin_core::{
///     ParamFlags, ParamId, ParamInfo, ParamWriteMeans, check_param_write_for,
/// };
///
/// const PARAMS: [ParamInfo; 1] = [ParamInfo {
///     id: ParamId(0),
///     name: "Gain",
///     unit: "",
///     min_value: 0.0,
///     max_value: 1.0,
///     default_value: 0.0,
///     flags: ParamFlags::AUTOMATABLE,
/// }];
/// assert!(check_param_write_for(&PARAMS, ParamId(0), ParamWriteMeans::TimedEvent).is_ok());
/// assert!(check_param_write_for(&PARAMS, ParamId(9), ParamWriteMeans::TimedEvent).is_err());
/// ```
pub fn check_param_write_for(
    params: &[ParamInfo],
    id: ParamId,
    means: ParamWriteMeans,
) -> Result<(), ParamWriteRejection> {
    match params.iter().find(|info| info.id == id) {
        Some(info) => check_param_write(info, means),
        None => Err(ParamWriteRejection::UnknownParam),
    }
}

/// A parameter event with its absolute media position — the shape a
/// consumer assembles before lowering into a `TransportPlan`.
///
/// **RT contract:** non-RT build-side type — plan construction is a
/// non-RT operation by design.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct TimedParamEvent {
    /// Absolute media position the event is due at.
    pub position: MediaTime,
    /// The parameter change payload.
    pub event: ParamEvent,
}

/// Opt-in capacity coalescing: keeps only the latest value of each
/// *contiguous* run of [`ParamEvent`]s that share both `position` and
/// `param_id`.
///
/// Deliberately conservative — events are only merged when the run is
/// contiguous (a different `param_id` between two events for the same
/// target blocks merging) and every merged event is a `ParamValue`
/// change, so no irreversible event and no other parameter is ever
/// folded in. The default path does *not* coalesce: capacity overflow
/// is a defined fault; this helper exists for consumers that
/// explicitly opt into latest-wins compression of continuous
/// automation.
///
/// The input is stable-sorted by `position` in place first, so
/// same-position groups keep their original relative order and the
/// surviving representative is the last one the caller appended.
///
/// **RT contract:** non-RT — allocates scratch internally; call on
/// the plan-build side only.
///
/// # Examples
///
/// ```
/// use ruviform_media_core::{MediaTime, TimeBase};
/// use ruviform_plugin_core::{
///     ParamEvent, ParamId, ParamValue, TimedParamEvent, coalesce_param_events_latest,
/// };
///
/// let pos = MediaTime::new(64, TimeBase::COMMON_90K);
/// let mut events = vec![
///     TimedParamEvent { position: pos, event: ParamEvent { param_id: ParamId(1), value: ParamValue::new(0.3).unwrap() } },
///     TimedParamEvent { position: pos, event: ParamEvent { param_id: ParamId(1), value: ParamValue::new(0.8).unwrap() } },
/// ];
/// coalesce_param_events_latest(&mut events);
/// assert_eq!(events.len(), 1);
/// assert_eq!(events[0].event.value.get(), 0.8);
/// ```
pub fn coalesce_param_events_latest(events: &mut Vec<TimedParamEvent>) {
    events.sort_by_key(|entry| entry.position);
    let mut write = 0usize;
    for read in 0..events.len() {
        let current = events[read];
        let is_last_of_run = events.get(read + 1).is_none_or(|next| {
            next.position != current.position || next.event.param_id != current.event.param_id
        });
        if is_last_of_run {
            events[write] = current;
            write += 1;
        }
    }
    events.truncate(write);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::param::ParamValue;

    const fn param(id: u32, flags: ParamFlags) -> ParamInfo {
        ParamInfo {
            id: ParamId(id),
            name: "p",
            unit: "",
            min_value: 0.0,
            max_value: 1.0,
            default_value: 0.0,
            flags,
        }
    }

    #[test]
    fn timed_event_requires_automatable_and_writable() {
        let ok = param(0, ParamFlags::AUTOMATABLE);
        assert_eq!(check_param_write(&ok, ParamWriteMeans::TimedEvent), Ok(()));

        let plain = param(1, ParamFlags::NONE);
        assert_eq!(
            check_param_write(&plain, ParamWriteMeans::TimedEvent),
            Err(ParamWriteRejection::NotAutomatable),
        );

        let readonly = param(2, ParamFlags::AUTOMATABLE.union(ParamFlags::READONLY));
        assert_eq!(
            check_param_write(&readonly, ParamWriteMeans::TimedEvent),
            Err(ParamWriteRejection::ReadOnly),
        );
    }

    #[test]
    fn block_boundary_allows_non_automatable_but_not_readonly() {
        let plain = param(1, ParamFlags::NONE);
        assert_eq!(
            check_param_write(&plain, ParamWriteMeans::BlockBoundary),
            Ok(()),
        );
        let readonly = param(2, ParamFlags::READONLY);
        assert_eq!(
            check_param_write(&readonly, ParamWriteMeans::BlockBoundary),
            Err(ParamWriteRejection::ReadOnly),
        );
    }

    #[test]
    fn immediate_and_per_note_are_unsupported_means() {
        let ok = param(0, ParamFlags::AUTOMATABLE.union(ParamFlags::MODULATABLE));
        for means in [ParamWriteMeans::Immediate, ParamWriteMeans::PerNote] {
            assert_eq!(
                check_param_write(&ok, means),
                Err(ParamWriteRejection::UnsupportedMeans),
            );
        }
    }

    #[test]
    fn check_for_unknown_param_rejected() {
        let params = [param(0, ParamFlags::AUTOMATABLE)];
        assert_eq!(
            check_param_write_for(&params, ParamId(7), ParamWriteMeans::TimedEvent),
            Err(ParamWriteRejection::UnknownParam),
        );
        assert_eq!(
            check_param_write_for(&params, ParamId(0), ParamWriteMeans::TimedEvent),
            Ok(()),
        );
    }

    #[test]
    fn coalesce_keeps_latest_of_contiguous_run() {
        use ruviform_media_core::TimeBase;
        let pos = |t| MediaTime::new(t, TimeBase::COMMON_90K);
        let ev = |id, v| TimedParamEvent {
            position: pos(64),
            event: ParamEvent {
                param_id: ParamId(id),
                value: ParamValue::new(v).unwrap(),
            },
        };
        let mut events = vec![
            TimedParamEvent {
                position: pos(96),
                ..ev(1, 0.1)
            },
            ev(1, 0.3),
            ev(2, 0.9),
            ev(1, 0.8),
        ];
        coalesce_param_events_latest(&mut events);
        // Sorted by position: [1@64=.3, 2@64=.9, 1@64=.8, 1@96=.1]
        // The two id=1 events at pos 64 are NOT contiguous (id=2 in
        // between) so both survive; only contiguous runs collapse.
        assert_eq!(events.len(), 4);
        assert_eq!(events[0].event.value.get(), 0.3);
        assert_eq!(events[2].event.value.get(), 0.8);
        assert_eq!(events[3].position, pos(96));
    }

    #[test]
    fn coalesce_collapses_contiguous_same_param_run() {
        use ruviform_media_core::TimeBase;
        let pos = MediaTime::new(64, TimeBase::COMMON_90K);
        let ev = |id, v| TimedParamEvent {
            position: pos,
            event: ParamEvent {
                param_id: ParamId(id),
                value: ParamValue::new(v).unwrap(),
            },
        };
        let mut events = vec![ev(1, 0.1), ev(1, 0.5), ev(1, 0.9), ev(2, 0.4)];
        coalesce_param_events_latest(&mut events);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event.param_id, ParamId(1));
        assert_eq!(events[0].event.value.get(), 0.9);
        assert_eq!(events[1].event.param_id, ParamId(2));
    }
}
