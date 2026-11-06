//! Registry — the collector-internal source table, key claims and
//! retained-removed tombstones behind the registry mutex.
//!
//! Key-claim rules (per the Issue #289 design):
//!
//! - A `&'static str` metric key is claimed by its live owner
//!   (`Active`/`Retiring`/`FinalDrain`); re-registering a live key is
//!   `KeyConflict` — including while the old owner is still draining.
//! - The claim record **persists for the collector's lifetime** even
//!   after `Removed` and retained-tombstone eviction (bounded by the
//!   compile-time `'static` key space). A removed key re-registers
//!   with a fresh [`SourceId`] and `generation + 1`; re-registering
//!   with a *different* [`MetricKind`] is `InvalidDescriptor` —
//!   "metric type never changes under the same key" holds
//!   permanently, not just while a tombstone survives.
//! - The reader-cell claim is kept **only while the registration is
//!   live**: the registry itself holds the reader `Arc`, so the cell
//!   address cannot be recycled while claimed — a `SourceReused`
//!   check by raw pointer can never ABA. Once removed, the same cell
//!   may register again as a *new series* with a fresh baseline.

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use crate::metrics::Measurement;
use crate::source::{
    AggregationPolicy, MetricDescriptor, MetricKind, MetricReader, MetricSource, ObservationScope,
    RegistrationError, RegistrationLease, SourceGeneration, SourceId, SourceIdentity,
    SourceLifecycle, SourceState,
};

use super::config::CollectorConfig;

/// Kind-specific aggregation state of a live series.
#[derive(Debug)]
pub(crate) enum SeriesState {
    /// Counter: previous raw read and the sum of all reported deltas.
    Counter { prev_raw: u64, cumulative: u64 },
    /// Gauge: last raw read (baseline at registration) plus whether a
    /// committed observation exists yet — the first poll reports
    /// `previous: None`.
    Gauge { last_raw: u64, observed_once: bool },
    /// Histogram: previous per-bucket counts.
    Histogram { prev_buckets: Box<[u64]> },
}

/// One live (`Active`/`Retiring`/`FinalDrain`) registration.
#[derive(Debug)]
pub(crate) struct SourceEntry {
    /// Registrar-minted identity.
    pub identity: SourceIdentity,
    /// Immutable descriptor declared at registration.
    pub descriptor: MetricDescriptor,
    /// Declared observation scope.
    pub scope: ObservationScope,
    /// The reader cell — owned by the registry for the registration's
    /// lifetime so a dropped lease cannot erase observations.
    pub reader: MetricReader,
    /// Shared lifecycle cell — the registry drives
    /// `Retiring → FinalDrain → Removed` on it.
    pub state: Arc<SourceState>,
    /// The last poll this source was observed — the rate anchor
    /// (registration instant until the first poll).
    pub last_observed_at: Instant,
    /// The last poll whose raw value differed from its predecessor.
    pub last_change_at: Instant,
    /// The latest committed measurement — `None` until the first
    /// poll commits one (a never-polled source has no snapshot entry:
    /// the collector does not fabricate observations).
    pub last: Option<Measurement>,
    /// Kind-specific aggregation state.
    pub series: SeriesState,
}

/// The persistent record behind a metric key.
#[derive(Debug)]
struct KeyClaim {
    /// The currently-owning (or last-owning) registration.
    owner: SourceId,
    /// The kind this key is pinned to — permanently.
    kind: MetricKind,
    /// The generation the last registration of this key received.
    generation: u64,
}

/// The registry table — lives inside the collector's `Mutex<State>`.
#[derive(Debug, Default)]
pub(crate) struct Registry {
    /// Live registrations, keyed by `SourceId` (sorted iteration —
    /// deterministic batch order and round-robin fairness).
    pub entries: BTreeMap<SourceId, SourceEntry>,
    /// Persistent key claims — outlive `Removed` and tombstone
    /// eviction (bounded by the compile-time `'static` key space).
    claims: HashMap<&'static str, KeyClaim>,
    /// Live reader-cell claims — the cell `Arc` is held by the
    /// matching entry, so the raw pointer cannot be recycled while
    /// claimed.
    cell_claims: HashMap<usize, SourceId>,
    /// `Removed` series tombstones retained for snapshots —
    /// `Unavailable` measurements evicted oldest-first when the
    /// series budget needs room.
    pub retained: BTreeMap<SourceId, Measurement>,
    /// Eviction order over `retained`.
    retained_order: VecDeque<SourceId>,
}

impl Registry {
    /// Registers `source` — validates the descriptor, enforces the
    /// key/cell claims and capacity, mints the identity, establishes
    /// the baseline, and returns the domain's lease together with the
    /// number of retained tombstones evicted to reclaim the series
    /// budget (for health accounting).
    ///
    /// Caller holds the registry lock; the collector-state atomic was
    /// checked before entry (and re-checked under the lock).
    pub(crate) fn register(
        &mut self,
        source: &MetricSource,
        config: &CollectorConfig,
        next_id: &AtomicU64,
    ) -> Result<(RegistrationLease, usize), RegistrationError> {
        let descriptor = source.descriptor().clone();
        let scope = source.scope();
        let reader = source.reader().clone();

        validate_descriptor(&descriptor, &reader)?;

        let key = descriptor.key;
        if let Some(claim) = self.claims.get(&key) {
            if claim.kind != descriptor.kind {
                return Err(RegistrationError::InvalidDescriptor {
                    reason: "metric kind changed for key",
                });
            }
            if self.entries.contains_key(&claim.owner) {
                return Err(RegistrationError::KeyConflict { key });
            }
        }

        let cell = reader.shared_cell_ptr();
        if self.cell_claims.contains_key(&cell) {
            return Err(RegistrationError::SourceReused);
        }

        if self.entries.len() >= config.max_sources {
            return Err(RegistrationError::CapacityExceeded {
                limit: config.max_sources,
            });
        }

        // Reclaim retained-series capacity for the incoming entry.
        let mut evicted = 0usize;
        while self.entries.len() + self.retained.len() >= config.max_series
            && self.evict_oldest_retained().is_some()
        {
            evicted += 1;
        }

        let raw_id = next_id.fetch_add(1, Ordering::Relaxed) + 1;
        let id = SourceId::new(raw_id).expect("minted ids are non-zero");
        let generation = self.claims.get(&key).map_or(0, |c| c.generation + 1);
        let identity = SourceIdentity::new(id, SourceGeneration::new(generation));
        self.claims.insert(
            key,
            KeyClaim {
                owner: id,
                kind: descriptor.kind,
                generation,
            },
        );

        let now = Instant::now();
        let series = match &reader {
            MetricReader::Counter(r) => SeriesState::Counter {
                prev_raw: r.read(),
                cumulative: 0,
            },
            MetricReader::Gauge(r) => SeriesState::Gauge {
                last_raw: r.read(),
                observed_once: false,
            },
            MetricReader::Histogram(r) => SeriesState::Histogram {
                prev_buckets: r.snapshot(),
            },
        };

        let state = Arc::new(SourceState::new());
        let entry = SourceEntry {
            identity,
            descriptor,
            scope,
            reader,
            state: Arc::clone(&state),
            last_observed_at: now,
            last_change_at: now,
            last: None,
            series,
        };
        self.entries.insert(id, entry);
        self.cell_claims.insert(cell, id);
        Ok((RegistrationLease::new(identity, state), evicted))
    }

    /// Removes a live registration after its `mark_removed` — releases
    /// the reader-cell claim and returns the entry for tombstoning.
    pub(crate) fn remove_entry(&mut self, id: SourceId) -> Option<SourceEntry> {
        let entry = self.entries.remove(&id)?;
        self.cell_claims.remove(&entry.reader.shared_cell_ptr());
        Some(entry)
    }

    /// Retains a `Removed` tombstone for snapshots, then evicts the
    /// oldest retained tombstones until the series budget fits.
    /// Returns the number of tombstones evicted.
    pub(crate) fn retain_tombstone(
        &mut self,
        id: SourceId,
        tombstone: Measurement,
        config: &CollectorConfig,
    ) -> usize {
        self.retained.insert(id, tombstone);
        self.retained_order.push_back(id);
        let mut evicted = 0;
        while self.entries.len() + self.retained.len() > config.max_series {
            if self.evict_oldest_retained().is_none() {
                break;
            }
            evicted += 1;
        }
        evicted
    }

    /// Evicts the oldest retained tombstone, if any.
    fn evict_oldest_retained(&mut self) -> Option<()> {
        let id = self.retained_order.pop_front()?;
        self.retained.remove(&id).map(|_| ())
    }

    /// Counts of live sources by coarse state for health.
    pub(crate) fn live_counts(&self) -> (usize, usize) {
        let mut active = 0;
        let mut retiring = 0;
        for entry in self.entries.values() {
            match entry.state.lifecycle() {
                SourceLifecycle::Active => active += 1,
                _ => retiring += 1,
            }
        }
        (active, retiring)
    }
}

/// Descriptor validation at the registration boundary — everything
/// rejectable is a typed `InvalidDescriptor`.
fn validate_descriptor(
    descriptor: &MetricDescriptor,
    reader: &MetricReader,
) -> Result<(), RegistrationError> {
    if descriptor.key.is_empty() {
        return Err(RegistrationError::InvalidDescriptor {
            reason: "metric key must be non-empty",
        });
    }
    match descriptor.kind {
        MetricKind::Counter => {
            if descriptor.aggregation != AggregationPolicy::Cumulative {
                return Err(RegistrationError::InvalidDescriptor {
                    reason: "counter requires Cumulative aggregation",
                });
            }
            if descriptor.buckets.is_some() {
                return Err(RegistrationError::InvalidDescriptor {
                    reason: "buckets are only meaningful for histograms",
                });
            }
        }
        MetricKind::Gauge => {
            if descriptor.aggregation != AggregationPolicy::LatestValue {
                return Err(RegistrationError::InvalidDescriptor {
                    reason: "gauge requires LatestValue aggregation",
                });
            }
            if descriptor.buckets.is_some() {
                return Err(RegistrationError::InvalidDescriptor {
                    reason: "buckets are only meaningful for histograms",
                });
            }
        }
        MetricKind::Histogram => {
            if descriptor.aggregation != AggregationPolicy::BucketCounts {
                return Err(RegistrationError::InvalidDescriptor {
                    reason: "histogram requires BucketCounts aggregation",
                });
            }
            let Some(boundaries) = descriptor.buckets else {
                return Err(RegistrationError::InvalidDescriptor {
                    reason: "histogram requires bucket boundaries",
                });
            };
            if boundaries.is_empty() {
                return Err(RegistrationError::InvalidDescriptor {
                    reason: "histogram boundaries must be non-empty",
                });
            }
            if boundaries.windows(2).any(|w| w[0] >= w[1]) {
                return Err(RegistrationError::InvalidDescriptor {
                    reason: "histogram boundaries must be strictly ascending",
                });
            }
            match reader {
                MetricReader::Histogram(r) => {
                    // The descriptor's declared boundaries must match
                    // the reader's real boundaries *by value* — equal
                    // bucket counts alone would silently mislabel
                    // every bucket.
                    if r.boundaries() != boundaries {
                        return Err(RegistrationError::InvalidDescriptor {
                            reason: "descriptor boundaries do not match the histogram reader",
                        });
                    }
                }
                _ => {
                    return Err(RegistrationError::InvalidDescriptor {
                        reason: "histogram descriptor requires a histogram reader",
                    });
                }
            }
        }
    }
    Ok(())
}
