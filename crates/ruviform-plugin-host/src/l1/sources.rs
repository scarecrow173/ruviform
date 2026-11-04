//! L1 audio counter source registration — Issue #279 (OBS-04).
//!
//! The four diagnostic counters [`super::AudioCounters`] already owns
//! for an `InstKind::Audio` registry entry become Observation Sources
//! through the OBS-02 (`Issue #277`)
//! [`ObservationRegistrar`]/[`RegistrationLease`] contract: the domain
//! declares descriptors and semantics at registration and consumers
//! (the Issue #289 collector, Issue #278's tracing export) own
//! conversion, deltas and severity — this module performs none of
//! those.
//!
//! # Boundary
//!
//! - Registration and lease lifecycle are **non-RT only** — they run
//!   on the mint path after the `Creating → Live` flip and on the
//!   facade's `destroy` path. An RT writer never sees a registrar
//!   handle, registry lock, or lease: the leg keeps only its
//!   pre-created [`ruviform_observability::Counter`] writers.
//! - A registration failure is a typed degradation, not a mint
//!   failure: the offending counter's slot records
//!   [`AudioSourceSlot::Refused`] and plugin processing continues —
//!   the registrar's error never decides whether an audio callback
//!   may run.
//! - Source epoch equals counter-cell epoch: a mint registers a
//!   fresh source identity and `destroy` retires it; leg swaps and
//!   endpoint checkouts share the cell and never re-register.

use std::fmt;
use std::time::Duration;

use ruviform_observability::source::{
    AggregationPolicy, Confidentiality, MetricDescriptor, MetricKind, MetricReader, MetricSource,
    ObservationRegistrar, ObservationScope, ObservationUnit, RegistrationError, RegistrationLease,
    ReportingHint, ReportingImpact, RetiredSource, SourceSemantics, SourceTemperature,
};

use super::AudioCounterReaders;

/// The registered counters' metric key namespace — `l1.` scoped
/// keys bound at compile time so the domain's series cardinality is
/// statically bounded (the collector's runtime limits stay its own).
const ORIGIN: &str = "ruviform-plugin-host::l1";

/// Domain-declared reporting hint for the three fault counters —
/// `dropped_submits`, `results_failed_observed` and
/// `results_dropped` are zero in correct operation, so an increase
/// is the report-worthy event. `min_interval` is a declared hint,
/// never a scheduling guarantee.
const FAULT_REPORTING: ReportingHint = ReportingHint::OnIncrease {
    min_delta: 1,
    min_interval: Duration::from_secs(1),
    impact: ReportingImpact::High,
};

/// The four domain-declared descriptors, in
/// [`AudioCounterReaders`] field order. Each carries the Issue #121
/// meaning table verbatim so a consumer never has to guess it.
///
/// `passthrough_submits` is deliberately `SnapshotOnly`: its count
/// includes the warmup blocks every live leg substitutes, so an
/// unconditional increase report would fault a correct warmup. The
/// post-warmup abnormal case is covered by the two fault counters'
/// `OnIncrease` declarations (Issue #279's "別条件で判定").
fn descriptors() -> [MetricDescriptor; 4] {
    [
        MetricDescriptor {
            kind: MetricKind::Counter,
            key: "l1.dropped_submits",
            unit: ObservationUnit::Count,
            reporting: FAULT_REPORTING,
            aggregation: AggregationPolicy::Cumulative,
            buckets: None,
            semantics: SourceSemantics {
                meaning: "submit could not be handed to the driver (busy slot or full \
                          channel); at most +1 per block; never an XRUN or other event",
                temperature: SourceTemperature::Warm,
                warmup: None,
                expected_range: None,
                origin: ORIGIN,
                confidentiality: Confidentiality::Internal,
            },
        },
        MetricDescriptor {
            kind: MetricKind::Counter,
            key: "l1.passthrough_submits",
            unit: ObservationUnit::Count,
            reporting: ReportingHint::SnapshotOnly,
            aggregation: AggregationPolicy::Cumulative,
            buckets: None,
            semantics: SourceSemantics {
                meaning: "blocks where substitute output actually ran — includes warmup \
                          blocks, so an increase is not unconditionally abnormal",
                temperature: SourceTemperature::Warm,
                warmup: None,
                expected_range: None,
                origin: ORIGIN,
                confidentiality: Confidentiality::Internal,
            },
        },
        MetricDescriptor {
            kind: MetricKind::Counter,
            key: "l1.results_failed_observed",
            unit: ObservationUnit::Count,
            reporting: FAULT_REPORTING,
            aggregation: AggregationPolicy::Cumulative,
            buckets: None,
            semantics: SourceSemantics {
                meaning: "RESULT_FAILED results the RT drain actually observed popping \
                          the result channel — not the count of failures generated",
                temperature: SourceTemperature::Warm,
                warmup: None,
                expected_range: None,
                origin: ORIGIN,
                confidentiality: Confidentiality::Internal,
            },
        },
        MetricDescriptor {
            kind: MetricKind::Counter,
            key: "l1.results_dropped",
            unit: ObservationUnit::Count,
            reporting: FAULT_REPORTING,
            aggregation: AggregationPolicy::Cumulative,
            buckets: None,
            semantics: SourceSemantics {
                meaning: "reader-side result enqueue failures — not every form of \
                          result loss",
                temperature: SourceTemperature::Warm,
                warmup: None,
                expected_range: None,
                origin: ORIGIN,
                confidentiality: Confidentiality::Internal,
            },
        },
    ]
}

/// One counter's registration outcome — the per-source view of a
/// four-source mint. `pub(crate)` crate-internal enum: outside the
/// `#[non_exhaustive]` ADR scope.
pub(crate) enum AudioSourceSlot {
    /// The registrar accepted the source — the lease is the
    /// domain's lifecycle handle until [`AudioSourceLeases::retire`].
    Registered(RegistrationLease),
    /// The registrar refused with a typed error — a degradation of
    /// this source only; plugin processing continues.
    Refused(RegistrationError),
    /// The lease was retired through [`AudioSourceLeases::retire`] —
    /// its ticket lives in [`AudioSourceLeases::retired`].
    Retired,
    /// No registrar was attached at mint — intentionally
    /// unregistered, not a failure.
    Unattached,
}

impl fmt::Debug for AudioSourceSlot {
    /// The `Refused` payload is *the* evidence of a typed
    /// degradation — it is part of `Debug` (and of `slots()`'s test
    /// surface), never dropped silently.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Registered(lease) => f
                .debug_tuple("Registered")
                .field(&lease.identity())
                .finish(),
            Self::Refused(error) => f.debug_tuple("Refused").field(error).finish(),
            Self::Retired => f.write_str("Retired"),
            Self::Unattached => f.write_str("Unattached"),
        }
    }
}

/// The four source registrations one mint produced plus the
/// [`RetiredSource`] tickets its `retire` returned — the facade's
/// `source_leases` field. Kept on `L1Instance` (not `InstShared`):
/// the leases are the domain's lifecycle handles and live exactly as
/// long as the facade does.
///
/// **RT contract:** non-RT only — registration, `retire` and `Drop`
/// may allocate and lock.
#[derive(Debug)]
pub(crate) struct AudioSourceLeases {
    /// Per-counter outcomes, in [`AudioCounterReaders`] field order.
    slots: [AudioSourceSlot; 4],
    /// The tickets `retire` collected — kept so `FinalDrain →
    /// Removed` progress stays observable (tests; Issue #293's
    /// integration surface later).
    retired: Vec<RetiredSource>,
}

impl AudioSourceLeases {
    /// The mint-boundary constructor: `Some` registrar registers
    /// the four `readers` ([`Self::register`]); `None` leaves every
    /// slot [`AudioSourceSlot::Unattached`] ([`Self::unattached`]).
    pub(crate) fn mint(
        registrar: Option<&dyn ObservationRegistrar>,
        readers: AudioCounterReaders,
    ) -> Self {
        registrar.map_or_else(Self::unattached, |r| Self::register(readers, r))
    }

    /// Registers all four counters with `registrar` — one
    /// `register` call per counter so a per-source typed refusal
    /// degrades only that slot and never rolls back its siblings
    /// (per-source degradation, Issue #279). **Non-RT only.**
    pub(crate) fn register(
        readers: AudioCounterReaders,
        registrar: &dyn ObservationRegistrar,
    ) -> Self {
        let [dropped, passthrough, failed_observed, results_dropped] = descriptors();
        let pairs = [
            (dropped, readers.dropped_submits),
            (passthrough, readers.passthrough_submits),
            (failed_observed, readers.results_failed_observed),
            (results_dropped, readers.results_dropped),
        ];
        let slots = pairs.map(|(descriptor, reader)| {
            let source = MetricSource::new(
                descriptor,
                ObservationScope::Instance,
                MetricReader::Counter(reader),
            )
            .expect("a Counter reader always pairs with kind Counter");
            match registrar.register(source) {
                Ok(lease) => AudioSourceSlot::Registered(lease),
                Err(error) => AudioSourceSlot::Refused(error),
            }
        });
        Self {
            slots,
            retired: Vec::new(),
        }
    }

    /// The all-`Unattached` bundle a registrar-less mint carries —
    /// the pre-OBS-04 default: nothing was ever registered, so
    /// `retire` is a no-op.
    pub(crate) fn unattached() -> Self {
        Self {
            slots: [
                AudioSourceSlot::Unattached,
                AudioSourceSlot::Unattached,
                AudioSourceSlot::Unattached,
                AudioSourceSlot::Unattached,
            ],
            retired: Vec::new(),
        }
    }

    /// Declares writer quiescence on every `Registered` lease —
    /// `Active → Retiring` with the returned ticket parked in
    /// `retired` for drain observation. Call only after every
    /// owner-side writer has stopped (post-`remove_instance` on the
    /// `destroy` path); the collector owns `FinalDrain → Removed`
    /// from there. **Non-RT only.**
    pub(crate) fn retire(&mut self) {
        for slot in &mut self.slots {
            if !matches!(slot, AudioSourceSlot::Registered(_)) {
                continue;
            }
            let AudioSourceSlot::Registered(lease) =
                std::mem::replace(slot, AudioSourceSlot::Retired)
            else {
                unreachable!("checked above");
            };
            self.retired.push(lease.retire());
        }
    }

    /// `true` when every slot is `Unattached` — the pre-OBS-04 /
    /// registrar-less shape.
    #[cfg(test)]
    pub(crate) fn is_unattached(&self) -> bool {
        self.slots
            .iter()
            .all(|slot| matches!(slot, AudioSourceSlot::Unattached))
    }

    /// The slot states, in counter order — test observation only.
    #[cfg(test)]
    pub(crate) fn slots(&self) -> &[AudioSourceSlot; 4] {
        &self.slots
    }

    /// How many slots still hold a `Registered` lease — test
    /// observation only.
    #[cfg(test)]
    pub(crate) fn registered_count(&self) -> usize {
        self.slots
            .iter()
            .filter(|slot| matches!(slot, AudioSourceSlot::Registered(_)))
            .count()
    }

    /// The `Registered` slots' identities, in counter order — test
    /// observation only.
    #[cfg(test)]
    pub(crate) fn registered_identities(
        &self,
    ) -> Vec<ruviform_observability::source::SourceIdentity> {
        self.slots
            .iter()
            .filter_map(|slot| match slot {
                AudioSourceSlot::Registered(lease) => Some(lease.identity()),
                _ => None,
            })
            .collect()
    }

    /// The retired tickets' lifecycle states — test observation of
    /// `Retiring → FinalDrain → Removed` progress.
    #[cfg(test)]
    pub(crate) fn retired_lifecycles(
        &self,
    ) -> Vec<ruviform_observability::source::SourceLifecycle> {
        self.retired.iter().map(RetiredSource::lifecycle).collect()
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    //! A deterministic in-crate [`ObservationRegistrar`] — records
    //! every accepted source, mints sequential identities and serves
    //! injected per-call errors, so the mint/destroy wiring tests can
    //! observe registration without an Issue #289 collector.

    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Arc, Mutex, MutexGuard};

    use ruviform_observability::source::{
        MetricDescriptor, MetricReader, MetricSource, ObservationRegistrar, ObservationScope,
        RegistrationError, RegistrationLease, SourceGeneration, SourceId, SourceIdentity,
        SourceState,
    };

    /// Minimal in-memory registrar in the OBS-02 test-double shape —
    /// keeps the accepted sources (their reader cells included) so a
    /// test can read the counter through the registry's own copy and
    /// drive `FinalDrain → Removed` on the shared `SourceState`.
    #[derive(Debug)]
    pub(crate) struct RecordingRegistrar {
        next: AtomicU64,
        generation: AtomicU64,
        /// The capacity `register` enforces —
        /// `CapacityExceeded{limit}` once `sources.len()` reaches it.
        limit: usize,
        /// Accepted `(source, shared state)` pairs in registration
        /// order.
        sources: Mutex<Vec<(MetricSource, Arc<SourceState>)>>,
        /// Per-`register`-call injected errors, consumed FIFO — an
        /// empty queue means "accept".
        errors: Mutex<VecDeque<RegistrationError>>,
    }

    impl RecordingRegistrar {
        /// An accepting registrar with the given source capacity —
        /// `register` beyond `limit` refuses
        /// `CapacityExceeded{limit}`.
        pub(crate) fn accepting(limit: usize) -> Self {
            Self {
                next: AtomicU64::new(0),
                generation: AtomicU64::new(0),
                limit,
                sources: Mutex::new(Vec::new()),
                errors: Mutex::new(VecDeque::new()),
            }
        }

        /// Queues `error` for the next `register` call.
        pub(crate) fn fail_next(&self, error: RegistrationError) {
            self.errors().push_back(error);
        }

        /// The accepted sources' descriptors, in registration order.
        pub(crate) fn descriptors(&self) -> Vec<MetricDescriptor> {
            self.sources()
                .iter()
                .map(|(source, _)| source.descriptor().clone())
                .collect()
        }

        /// The accepted sources' scopes, in registration order.
        pub(crate) fn scopes(&self) -> Vec<ObservationScope> {
            self.sources()
                .iter()
                .map(|(source, _)| source.scope())
                .collect()
        }

        /// How many sources the registrar accepted.
        pub(crate) fn accepted(&self) -> usize {
            self.sources().len()
        }

        /// The accepted sources' shared lifecycle states — the
        /// collector-side handle `begin_final_drain`/`mark_removed`
        /// is driven through.
        #[cfg(test)]
        pub(crate) fn states(&self) -> Vec<Arc<SourceState>> {
            self.sources()
                .iter()
                .map(|(_, state)| Arc::clone(state))
                .collect()
        }

        /// Reads the counter value through the registry's own reader
        /// copy — proof the registered reader shares the domain's
        /// cell.
        #[cfg(test)]
        pub(crate) fn counter_values(&self) -> Vec<u64> {
            self.sources()
                .iter()
                .map(|(source, _)| match source.reader() {
                    MetricReader::Counter(reader) => reader.read(),
                    _ => 0,
                })
                .collect()
        }

        /// The generation this registrar stamps on new identities —
        /// bumped to emulate a fresh collector generation across
        /// re-mints.
        pub(crate) fn set_generation(&self, generation: u64) {
            self.generation.store(generation, Ordering::Relaxed);
        }

        fn sources(&self) -> MutexGuard<'_, Vec<(MetricSource, Arc<SourceState>)>> {
            self.sources
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
        }

        fn errors(&self) -> MutexGuard<'_, VecDeque<RegistrationError>> {
            self.errors
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
        }
    }

    impl ObservationRegistrar for RecordingRegistrar {
        fn register(&self, source: MetricSource) -> Result<RegistrationLease, RegistrationError> {
            if let Some(error) = self.errors().pop_front() {
                return Err(error);
            }
            let mut sources = self.sources();
            if sources.len() >= self.limit {
                return Err(RegistrationError::CapacityExceeded { limit: self.limit });
            }
            let raw = self.next.fetch_add(1, Ordering::Relaxed) + 1;
            let identity = SourceIdentity::new(
                SourceId::new(raw).unwrap_or_else(|| unreachable!("raw >= 1")),
                SourceGeneration::new(self.generation.load(Ordering::Relaxed)),
            );
            let state = Arc::new(SourceState::new());
            sources.push((source, Arc::clone(&state)));
            Ok(RegistrationLease::new(identity, state))
        }
    }
}

#[cfg(test)]
mod tests {
    use ruviform_observability::counter;
    use ruviform_observability::source::{
        AggregationPolicy, MetricKind, ObservationScope, ObservationUnit, RegistrationError,
        ReportingHint, SourceLifecycle,
    };

    use super::super::AudioCounterReaders;
    use super::test_support::RecordingRegistrar;
    use super::{AudioSourceLeases, AudioSourceSlot};

    /// Four fresh counter readers — the same shape `create_instance`
    /// clones out of `InstShared::audio_counter_readers`, one writer
    /// kept per cell so a test can push observable increments.
    fn fresh_readers() -> (AudioCounterReaders, [ruviform_observability::Counter; 4]) {
        let (dropped_w, dropped_r) = counter();
        let (passthrough_w, passthrough_r) = counter();
        let (failed_w, failed_r) = counter();
        let (results_dropped_w, results_dropped_r) = counter();
        (
            AudioCounterReaders {
                dropped_submits: dropped_r,
                passthrough_submits: passthrough_r,
                results_failed_observed: failed_r,
                results_dropped: results_dropped_r,
            },
            [dropped_w, passthrough_w, failed_w, results_dropped_w],
        )
    }

    #[test]
    fn register_accepts_four_counter_sources_with_instance_scope() {
        let registrar = RecordingRegistrar::accepting(8);
        let (readers, _writers) = fresh_readers();
        let leases = AudioSourceLeases::register(readers, &registrar);
        assert_eq!(registrar.accepted(), 4);
        assert_eq!(leases.registered_identities().len(), 4);
        // Keys, kinds, units and aggregation pin the Issue #121 /
        // OBS-04 domain declaration; identity minting stays the
        // registrar's.
        let descriptors = registrar.descriptors();
        let keys: Vec<&str> = descriptors.iter().map(|d| d.key).collect();
        assert_eq!(
            keys,
            [
                "l1.dropped_submits",
                "l1.passthrough_submits",
                "l1.results_failed_observed",
                "l1.results_dropped",
            ]
        );
        for descriptor in &descriptors {
            assert_eq!(descriptor.kind, MetricKind::Counter);
            assert_eq!(descriptor.unit, ObservationUnit::Count);
            assert_eq!(descriptor.aggregation, AggregationPolicy::Cumulative);
        }
        assert!(matches!(
            descriptors[1].reporting,
            ReportingHint::SnapshotOnly
        ));
        for scope in registrar.scopes() {
            assert_eq!(scope, ObservationScope::Instance);
        }
        // Same-domain cells: the registrar's reader copy reads the
        // writer's increments.
        let (readers, writers) = fresh_readers();
        let registrar = RecordingRegistrar::accepting(8);
        let _leases = AudioSourceLeases::register(readers, &registrar);
        writers[0].increment();
        writers[3].increment();
        writers[3].increment();
        assert_eq!(registrar.counter_values(), [1, 0, 0, 2]);
    }

    #[test]
    fn unattached_slots_never_touch_the_registrar() {
        let registrar = RecordingRegistrar::accepting(8);
        let mut leases = AudioSourceLeases::unattached();
        assert!(leases.is_unattached());
        assert_eq!(leases.registered_identities().len(), 0);
        leases.retire();
        assert_eq!(registrar.accepted(), 0);
        assert_eq!(leases.retired_lifecycles(), []);
    }

    #[test]
    fn refused_slots_degrade_only_their_own_counter() {
        let registrar = RecordingRegistrar::accepting(8);
        registrar.fail_next(RegistrationError::CapacityExceeded { limit: 3 });
        registrar.fail_next(RegistrationError::KeyConflict {
            key: "l1.dropped_submits",
        });
        let (readers, _writers) = fresh_readers();
        let mut leases = AudioSourceLeases::register(readers, &registrar);
        let slots = leases.slots();
        assert!(matches!(
            slots[0],
            AudioSourceSlot::Refused(RegistrationError::CapacityExceeded { limit: 3 })
        ));
        assert!(matches!(
            slots[1],
            AudioSourceSlot::Refused(RegistrationError::KeyConflict { .. })
        ));
        assert!(matches!(slots[2], AudioSourceSlot::Registered(_)));
        assert!(matches!(slots[3], AudioSourceSlot::Registered(_)));
        // Two refused, two registered — only the registered pair
        // retires; refused slots carry no lease to retire.
        leases.retire();
        assert_eq!(leases.retired_lifecycles(), [SourceLifecycle::Retiring; 2]);
    }

    #[test]
    fn retire_turns_registered_leases_into_tickets() {
        let registrar = RecordingRegistrar::accepting(8);
        let (readers, _writers) = fresh_readers();
        let mut leases = AudioSourceLeases::register(readers, &registrar);
        assert_eq!(leases.registered_identities().len(), 4);
        leases.retire();
        assert_eq!(leases.retired_lifecycles(), [SourceLifecycle::Retiring; 4]);
        // Retire is idempotent on already-Retired slots.
        leases.retire();
        assert_eq!(leases.retired_lifecycles().len(), 4);
        // The collector drives FinalDrain → Removed on its own
        // state clones — observable through the parked tickets.
        for state in registrar.states() {
            state.begin_final_drain().unwrap();
        }
        assert_eq!(
            leases.retired_lifecycles(),
            [SourceLifecycle::FinalDrain; 4]
        );
        for state in registrar.states() {
            state.mark_removed().unwrap();
        }
        assert_eq!(leases.retired_lifecycles(), [SourceLifecycle::Removed; 4]);
    }

    #[test]
    fn source_identities_distinguish_same_key_across_mints() {
        // Two mints of the same four keys on one registrar — the
        // Issue #279 "同名 key・異 Instance" case: series are told
        // apart by `SourceId`, never merged by key.
        let registrar = RecordingRegistrar::accepting(8);
        let (readers_a, _wa) = fresh_readers();
        let (readers_b, _wb) = fresh_readers();
        let leases_a = AudioSourceLeases::register(readers_a, &registrar);
        let leases_b = AudioSourceLeases::register(readers_b, &registrar);
        let ids_a = leases_a.registered_identities();
        let ids_b = leases_b.registered_identities();
        assert_eq!(ids_a.len(), 4);
        assert_eq!(ids_b.len(), 4);
        for (a, b) in ids_a.iter().zip(ids_b.iter()) {
            assert_ne!(a.id(), b.id());
        }
        // A fresh collector generation separates series even where
        // ids coincide (the registrar's stamping, not the domain's).
        registrar.set_generation(7);
        let (readers_c, _wc) = fresh_readers();
        let leases_c = AudioSourceLeases::register(readers_c, &registrar);
        for c in leases_c.registered_identities() {
            assert_eq!(c.generation().get(), 7);
        }
    }

    #[test]
    fn dropped_lease_abandons_observably() {
        // A lease dropped without `retire` marks `abandoned` on the
        // shared state — the Issue #279 "黙って消えない" contract.
        let registrar = RecordingRegistrar::accepting(8);
        let (readers, _writers) = fresh_readers();
        let leases = AudioSourceLeases::register(readers, &registrar);
        assert_eq!(leases.registered_identities().len(), 4);
        drop(leases);
        for state in registrar.states() {
            assert!(state.is_abandoned());
            assert_eq!(state.lifecycle(), SourceLifecycle::Retiring);
        }
    }

    #[test]
    fn capacity_exceeded_refuses_additional_sources() {
        // A collector with a tight `max_sources` — a typed refusal,
        // never a silent merge.
        let registrar = RecordingRegistrar::accepting(2);
        let (readers, _writers) = fresh_readers();
        let leases = AudioSourceLeases::register(readers, &registrar);
        let refused = leases
            .slots()
            .iter()
            .filter(|slot| matches!(slot, AudioSourceSlot::Refused(_)))
            .count();
        assert_eq!(registrar.accepted(), 2);
        assert_eq!(refused, 2);
    }
}
