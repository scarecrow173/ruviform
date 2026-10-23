//! Offline execution driver: chunk-driven sequential and parallel driving of
//! [`Processor`]/[`RtProcessor`] kernels with progress, cancellation and
//! bounded-queue backpressure (Issue #24).
//!
//! The driver is the offline `RtContext` minting site allowed by the design
//! baseline's issuance rule (a non-RT driver minting a "bounded-execution
//! context"). The minted context carries no RT constraints — it only lets the
//! same `Processor` contract be driven.
//!
//! ## Determinism guarantee
//!
//! For the same input chunks and the same `Setup`, `run` and `run_parallel`
//! return identical output: chunk order and content equal the sequential
//! result regardless of `workers`. The guarantee holds under the caller-side
//! precondition that each chunk is processed independently — an `Rt` carrying
//! state across chunks (e.g. splitting a continuous biquad stream) is outside
//! the deterministic scope. Scheduling order and chunk→worker assignment are
//! nondeterministic; the output is not.
//!
//! **RT contract:** every API in this module is `non-RT only` — calling them
//! from an RT contract section is a contract violation.

use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::sync_channel;
use std::sync::{Arc, PoisonError};
use std::thread;

use ruviform_rt::{Processor, RtContext, RtProcessor};

/// Cloneable cancellation flag checked at every chunk boundary.
///
/// **RT contract:** non-RT only.
#[derive(Debug, Clone, Default)]
pub struct CancellationToken {
    inner: Arc<AtomicBool>,
}

impl CancellationToken {
    /// Creates a fresh, unset token.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Requests cancellation. Subsequent `is_cancelled` on any clone
    /// observes it.
    pub fn cancel(&self) {
        self.inner.store(true, Ordering::Relaxed);
    }

    /// Whether cancellation was requested.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.inner.load(Ordering::Relaxed)
    }
}

/// Shared progress counters updated by the driver per completed chunk.
///
/// **RT contract:** non-RT only.
#[derive(Debug, Clone, Default)]
pub struct Progress {
    completed: Arc<AtomicU64>,
    total: Arc<AtomicU64>,
}

impl Progress {
    /// Creates a zeroed counter pair; the driver sets `total` when a run
    /// starts.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Chunks finished so far.
    #[must_use]
    pub fn completed(&self) -> u64 {
        self.completed.load(Ordering::Relaxed)
    }

    /// Total chunks scheduled by the current run.
    #[must_use]
    pub fn total(&self) -> u64 {
        self.total.load(Ordering::Relaxed)
    }

    /// `completed / total`; `0.0` before a run sets the total.
    #[expect(
        clippy::cast_precision_loss,
        reason = "chunk counts are far below the f64 exact-integer range"
    )]
    #[must_use]
    pub fn fraction(&self) -> f64 {
        let total = self.total();
        if total == 0 {
            0.0
        } else {
            self.completed() as f64 / total as f64
        }
    }

    fn reset(&self, total: u64) {
        self.completed.store(0, Ordering::Relaxed);
        self.total.store(total, Ordering::Relaxed);
    }

    fn bump(&self) {
        self.completed.fetch_add(1, Ordering::Relaxed);
    }
}

/// Driver configuration.
///
/// **RT contract:** non-RT only.
#[derive(Debug, Clone, Copy)]
pub struct OfflineConfig {
    /// Fixed worker count for [`OfflineDriver::run_parallel`]. `1` is the
    /// sequential-equivalent case; [`OfflineDriver::run`] ignores this field.
    pub workers: usize,
    /// Bounded channel capacity on both the input and output side.
    /// `0` selects `workers * 2` (at least 2).
    pub queue_capacity: usize,
}

impl Default for OfflineConfig {
    fn default() -> Self {
        Self {
            workers: 1,
            queue_capacity: 0,
        }
    }
}

/// Aggregated run failure. The first failure wins.
///
/// **RT contract:** non-RT only.
#[non_exhaustive]
#[derive(Debug)]
pub enum OfflineError {
    /// `CancellationToken::cancel` was observed at a chunk boundary; partial
    /// results are discarded.
    Cancelled,
    /// `Processor::prepare` failed (worker index for parallel runs).
    Prepare {
        /// Worker index that failed (`0` for the sequential driver).
        worker: usize,
        /// Rendered `prepare` error.
        message: String,
    },
    /// A worker thread panicked while processing.
    WorkerPanicked,
}

impl core::fmt::Display for OfflineError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Cancelled => f.write_str("offline run cancelled"),
            Self::Prepare { worker, message } => {
                write!(f, "prepare failed on worker {worker}: {message}")
            }
            Self::WorkerPanicked => f.write_str("offline worker panicked"),
        }
    }
}

impl std::error::Error for OfflineError {}

type IoOf<P> = <<P as Processor>::Rt as RtProcessor>::Io;

/// Stateless offline driver. All methods take every input explicitly.
///
/// **RT contract:** non-RT only.
pub struct OfflineDriver;

impl OfflineDriver {
    /// Sequential drive: prepares one `Rt`, then `process`es each chunk with
    /// a freshly minted `RtContext`. Cancellation is checked at chunk
    /// boundaries; progress is bumped per completed chunk.
    ///
    /// This is the determinism reference path — `run_parallel` must produce
    /// identical output.
    pub fn run<P>(
        processor: &mut P,
        setup: P::Setup,
        chunks: Vec<<P::Rt as RtProcessor>::Io>,
        cancel: &CancellationToken,
        progress: &Progress,
    ) -> Result<Vec<<P::Rt as RtProcessor>::Io>, OfflineError>
    where
        P: Processor,
        P::Error: std::fmt::Display,
    {
        progress.reset(chunks.len() as u64);
        let mut rt = processor
            .prepare(setup)
            .map_err(|e| OfflineError::Prepare {
                worker: 0,
                message: e.to_string(),
            })?;
        let mut out = Vec::with_capacity(chunks.len());
        for mut io in chunks {
            if cancel.is_cancelled() {
                return Err(OfflineError::Cancelled);
            }
            rt.process(&mut io, &RtContext::mint());
            progress.bump();
            out.push(io);
        }
        Ok(out)
    }

    /// Parallel drive: `config.workers` worker threads spawned per call
    /// via `std::thread::scope`, each with its own `Rt` prepared from
    /// `setup.clone()`, pull `(seq, chunk)` from a bounded channel;
    /// results are reassembled in input order.
    ///
    /// Bounded channels on both sides provide backpressure: the producer
    /// loop blocks once `queue_capacity` chunks are in flight. Requires
    /// `Send`/`Clone` bounds that `run` does not.
    ///
    /// A panic inside `process` or `prepare` surfaces as
    /// [`OfflineError::WorkerPanicked`] / [`OfflineError::Prepare`].
    pub fn run_parallel<P>(
        config: OfflineConfig,
        processor: &P,
        setup: P::Setup,
        chunks: Vec<<P::Rt as RtProcessor>::Io>,
        cancel: &CancellationToken,
        progress: &Progress,
    ) -> Result<Vec<<P::Rt as RtProcessor>::Io>, OfflineError>
    where
        // `prepare` takes `&mut self`, so each worker needs its own
        // processor instance — cloned up-front (scoped threads move the
        // clones in; `P: Send` covers that).
        P: Processor + Clone + Send + Sync,
        P::Setup: Clone + Send,
        P::Error: std::fmt::Display,
        P::Rt: Send,
        <P::Rt as RtProcessor>::Io: Send,
    {
        let workers = config.workers.max(1);
        if workers == 1 {
            return Self::run(&mut processor.clone(), setup, chunks, cancel, progress);
        }
        let cap = if config.queue_capacity == 0 {
            (workers * 2).max(2)
        } else {
            config.queue_capacity
        };
        progress.reset(chunks.len() as u64);

        // One processor instance and one Setup per worker (`prepare` is
        // `&mut self`).
        let workers_parts: Vec<(P, P::Setup)> = (0..workers)
            .map(|_| (processor.clone(), setup.clone()))
            .collect();
        let (in_tx, in_rx) = sync_channel::<(usize, IoOf<P>)>(cap);
        let (out_tx, out_rx) = sync_channel::<(usize, IoOf<P>)>(cap);
        let (err_tx, err_rx) = sync_channel::<OfflineError>(workers);
        let in_rx = std::sync::Mutex::new(in_rx);
        // Shared flag so the producer stops feeding once a worker reports
        // prepare failure (it cannot observe `err_rx` — owned by the
        // collecting thread).
        let failed = Arc::new(AtomicBool::new(false));

        let body = AssertUnwindSafe(|| {
            thread::scope(|scope| {
                for (worker, (mut wproc, wsetup)) in workers_parts.into_iter().enumerate() {
                    let in_rx = &in_rx;
                    let out_tx = out_tx.clone();
                    let err_tx = err_tx.clone();
                    let cancel = cancel.clone();
                    let progress = progress.clone();
                    let failed = failed.clone();
                    scope.spawn(move || {
                        // Invariant: every worker drains the input channel
                        // until the producer drops `in_tx`. Failed or
                        // cancelled workers keep draining (discarding chunks
                        // unprocessed) — otherwise a producer blocked in
                        // `in_tx.send` would deadlock.
                        let mut rt = match wproc.prepare(wsetup) {
                            Ok(rt) => Some(rt),
                            Err(e) => {
                                failed.store(true, Ordering::Relaxed);
                                let _ = err_tx.send(OfflineError::Prepare {
                                    worker,
                                    message: e.to_string(),
                                });
                                None
                            }
                        };
                        while let Ok((seq, mut io)) = {
                            let rx = in_rx.lock().unwrap_or_else(PoisonError::into_inner);
                            rx.recv()
                        } {
                            if cancel.is_cancelled() || failed.load(Ordering::Relaxed) {
                                continue;
                            }
                            // Contain the panic inside this worker and drop
                            // the (possibly corrupt) `Rt`; `failed` tells the
                            // producer to stop feeding.
                            let panicked = if let Some(rt_ref) = rt.as_mut() {
                                catch_unwind(AssertUnwindSafe(|| {
                                    rt_ref.process(&mut io, &RtContext::mint());
                                }))
                                .is_err()
                            } else {
                                continue;
                            };
                            if panicked {
                                failed.store(true, Ordering::Relaxed);
                                rt = None;
                                let _ = err_tx.send(OfflineError::WorkerPanicked);
                                continue;
                            }
                            // Out channel closed means the caller already
                            // failed — keep draining, discard the result.
                            if out_tx.send((seq, io)).is_err() {
                                rt = None;
                            }
                            progress.bump();
                        }
                    });
                }
                drop(out_tx);

                // Producer runs on its own thread: while it blocks on the
                // bounded input channel, this thread concurrently drains the
                // output channel — otherwise a full output channel would
                // deadlock the pipeline (workers stuck in `out_tx.send`
                // never free input capacity).
                let producer = scope.spawn(move || {
                    for (seq, io) in chunks.into_iter().enumerate() {
                        if cancel.is_cancelled() || failed.load(Ordering::Relaxed) {
                            break;
                        }
                        if in_tx.send((seq, io)).is_err() {
                            break;
                        }
                    }
                    drop(in_tx);
                });

                // Always collect until every worker exits (out channel
                // closes) — early return would strand workers blocked in
                // `out_tx.send` and deadlock `thread::scope`'s join.
                let mut results = BTreeMap::new();
                while let Ok((seq, io)) = out_rx.recv() {
                    results.insert(seq, io);
                }
                if producer.join().is_err() {
                    return Err(OfflineError::WorkerPanicked);
                }
                if let Ok(e) = err_rx.try_recv() {
                    return Err(e);
                }
                if cancel.is_cancelled() {
                    return Err(OfflineError::Cancelled);
                }
                Ok(results.into_values().collect::<Vec<_>>())
            })
        });
        match catch_unwind(body) {
            Err(_) => Err(OfflineError::WorkerPanicked),
            Ok(res) => res,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Plus-one kernel over `Vec<f32>` chunks.
    struct PlusOne;
    impl RtProcessor for PlusOne {
        type Io = Vec<f32>;
        fn process(&mut self, io: &mut Self::Io, _ctx: &RtContext) {
            for x in io.iter_mut() {
                *x += 1.0;
            }
        }
    }

    #[derive(Debug)]
    struct PrepareErr;
    impl std::fmt::Display for PrepareErr {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("prepare rejected")
        }
    }
    impl std::error::Error for PrepareErr {}

    #[derive(Clone)]
    struct AddStage {
        fail_prepare: bool,
    }
    impl Processor for AddStage {
        type Setup = f32;
        type Rt = PlusOne;
        type Error = PrepareErr;
        fn prepare(&mut self, _setup: f32) -> Result<PlusOne, PrepareErr> {
            if self.fail_prepare {
                Err(PrepareErr)
            } else {
                Ok(PlusOne)
            }
        }
    }

    /// `process` panics — worker panic mapping.
    #[derive(Clone)]
    struct PanicStage;
    impl Processor for PanicStage {
        type Setup = ();
        type Rt = PanicRt;
        type Error = std::convert::Infallible;
        fn prepare(&mut self, (): ()) -> Result<PanicRt, std::convert::Infallible> {
            Ok(PanicRt)
        }
    }
    struct PanicRt;
    impl RtProcessor for PanicRt {
        type Io = Vec<f32>;
        fn process(&mut self, _: &mut Vec<f32>, _: &RtContext) {
            panic!("boom");
        }
    }

    #[expect(
        clippy::cast_precision_loss,
        reason = "fixture indices are far below the f32 exact-integer range"
    )]
    fn chunks(n: usize) -> Vec<Vec<f32>> {
        (0..n).map(|i| vec![i as f32]).collect()
    }

    #[expect(
        clippy::cast_precision_loss,
        reason = "fixture indices are far below the f32 exact-integer range"
    )]
    #[test]
    fn offline_run_sequential_matches_input_order() {
        let mut stage = AddStage {
            fail_prepare: false,
        };
        let cancel = CancellationToken::new();
        let progress = Progress::new();
        let out = OfflineDriver::run(&mut stage, 0.0, chunks(64), &cancel, &progress).unwrap();
        assert_eq!(out.len(), 64);
        for (i, v) in out.iter().enumerate() {
            assert_eq!(v[0], i as f32 + 1.0);
        }
    }

    #[test]
    fn offline_run_parallel_deterministic_output() {
        // Output must equal the sequential reference for every worker count.
        let mut stage = AddStage {
            fail_prepare: false,
        };
        let cancel = CancellationToken::new();
        let progress = Progress::new();
        let want = OfflineDriver::run(&mut stage, 0.0, chunks(256), &cancel, &progress).unwrap();
        for workers in [1usize, 2, 4, 8] {
            let stage = AddStage {
                fail_prepare: false,
            };
            let cancel = CancellationToken::new();
            let progress = Progress::new();
            let cfg = OfflineConfig {
                workers,
                queue_capacity: 0,
            };
            let got =
                OfflineDriver::run_parallel(cfg, &stage, 0.0, chunks(256), &cancel, &progress)
                    .unwrap();
            assert_eq!(got, want, "workers={workers}");
        }
    }

    #[test]
    fn offline_run_cancels_at_chunk_boundary() {
        let cancel = CancellationToken::new();
        cancel.cancel();
        let progress = Progress::new();
        let stage = AddStage {
            fail_prepare: false,
        };
        let cfg = OfflineConfig {
            workers: 4,
            queue_capacity: 2,
        };
        let err = OfflineDriver::run_parallel(cfg, &stage, 0.0, chunks(64), &cancel, &progress)
            .unwrap_err();
        assert!(matches!(err, OfflineError::Cancelled));

        let mut stage = AddStage {
            fail_prepare: false,
        };
        let err = OfflineDriver::run(&mut stage, 0.0, chunks(64), &cancel, &progress).unwrap_err();
        assert!(matches!(err, OfflineError::Cancelled));
    }

    #[test]
    fn offline_run_reports_progress() {
        let mut stage = AddStage {
            fail_prepare: false,
        };
        let cancel = CancellationToken::new();
        let progress = Progress::new();
        OfflineDriver::run(&mut stage, 0.0, chunks(37), &cancel, &progress).unwrap();
        assert_eq!(progress.completed(), 37);
        assert_eq!(progress.total(), 37);
        assert!((progress.fraction() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn offline_run_aggregates_prepare_failure() {
        let stage = AddStage { fail_prepare: true };
        let cancel = CancellationToken::new();
        let progress = Progress::new();
        let cfg = OfflineConfig {
            workers: 3,
            queue_capacity: 2,
        };
        let err = OfflineDriver::run_parallel(cfg, &stage, 0.0, chunks(8), &cancel, &progress)
            .unwrap_err();
        assert!(matches!(err, OfflineError::Prepare { .. }));
    }

    #[test]
    fn offline_run_maps_worker_panic() {
        let stage = PanicStage;
        let cancel = CancellationToken::new();
        let progress = Progress::new();
        let cfg = OfflineConfig {
            workers: 2,
            queue_capacity: 2,
        };
        let err = OfflineDriver::run_parallel(cfg, &stage, (), chunks(8), &cancel, &progress)
            .unwrap_err();
        assert!(matches!(err, OfflineError::WorkerPanicked));
    }
}
