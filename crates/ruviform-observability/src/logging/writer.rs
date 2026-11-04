//! Bounded non-blocking writer path — one background worker per sink.
//!
//! Chosen over `tracing_appender::non_blocking` because the acceptance
//! criteria require *distinguished* observability of queue-full drops,
//! `WriteZero` partial writes, other write/rotation failures, flush
//! failures, and channel disconnects — none of which `non_blocking`
//! exposes. See ADR
//! `issue-276-logging-adapter-bounded-writer-ownership-and-build-time-sink-resilienc`.
//!
//! One formatted event becomes one [`WorkerMsg::Event`] on a
//! [`std::sync::mpsc::sync_channel`] bounded by `queue_capacity`. The
//! emitting path (`EventWriter::drop`) uses `try_send` only — it never
//! blocks; full queues and disconnected channels drop the event into the
//! respective health counter. Shutdown is an explicit in-band
//! [`WorkerMsg::Shutdown`] sent by [`WriterGuard`]: the worker writes all
//! messages queued *before* it (FIFO drain), counts messages queued
//! *after* it as `channel_disconnected`, performs a final `flush`, and
//! exits. It also exits when every sender is dropped without a shutdown
//! (e.g. everything released at once).
//!
//! The worker keeps going after individual write failures — a later
//! message may succeed once a transient error clears (e.g. after
//! rotation). If it panics, the channel disconnects and the emitting side
//! observes `channel_disconnected` + `Failed`.
//!
//! The logger's own failures are counted in [`SinkMetrics`] only — they
//! are never written back into the logging pipeline (no recursion).

use std::io::{self, Write};
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, SyncSender, TrySendError, sync_channel};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use super::health::{SinkMetrics, SinkState};

/// How long a [`WriterGuard`] waits for a sink's worker to drain, flush,
/// and exit during shutdown before abandoning it.
///
/// On timeout the worker is **not** force-killed (threads cannot be safely
/// killed in-process): it is detached and `shutdown_abandoned` is recorded
/// in the health snapshot. Any messages still queued at that point, or any
/// queued at process crash/abort, are lost.
pub const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

/// Messages on a sink's bounded channel.
#[derive(Debug)]
enum WorkerMsg {
    /// One fully formatted event record.
    Event(Vec<u8>),
    /// In-band shutdown marker: the worker drains every `Event` queued
    /// before this marker, counts any queued after it as
    /// `channel_disconnected`, flushes once, and exits.
    Shutdown,
}

/// Sending side of a sink's bounded queue — owned by the sink's
/// `MakeWriter` closure and cloned into each `EventWriter` it produces.
#[derive(Debug)]
pub(crate) struct BoundedSender {
    tx: SyncSender<WorkerMsg>,
    metrics: Arc<SinkMetrics>,
}

impl BoundedSender {
    /// Returns a fresh per-event writer that buffers the formatted record
    /// and enqueues it on drop.
    pub(crate) fn event_writer(&self) -> EventWriter {
        EventWriter {
            tx: self.tx.clone(),
            metrics: Arc::clone(&self.metrics),
            buf: Vec::new(),
        }
    }
}

/// Per-event `io::Write` adaptor handed to `fmt` layers via `MakeWriter`.
///
/// The `fmt` machinery may issue several `write` calls for one record;
/// buffering until `Drop` keeps each event a single queue message so
/// `events_written` and the drop counters count events, not fragments.
#[derive(Debug)]
pub(crate) struct EventWriter {
    tx: SyncSender<WorkerMsg>,
    metrics: Arc<SinkMetrics>,
    buf: Vec<u8>,
}

impl Write for EventWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buf.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Drop for EventWriter {
    fn drop(&mut self) {
        if self.buf.is_empty() {
            return;
        }
        let message = WorkerMsg::Event(std::mem::take(&mut self.buf));
        match self.tx.try_send(message) {
            Ok(()) => {}
            Err(TrySendError::Full(_)) => {
                self.metrics
                    .dropped_queue_full
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
            Err(TrySendError::Disconnected(_)) => {
                self.metrics
                    .channel_disconnected
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                self.metrics.set_state(SinkState::Failed);
            }
        }
    }
}

/// Shutdown handle for one sink's background writer.
///
/// Dropping it sends an in-band shutdown marker (bounded by the timeout
/// budget — a permanently full queue skips straight to the wait), then
/// waits up to [`SHUTDOWN_TIMEOUT`] for the worker to drain, flush, and
/// exit, and joins it. If the wait times out the worker is abandoned
/// (never force-killed) and `shutdown_abandoned` is recorded in the health
/// snapshot. Any messages still queued at that point, or any queued at
/// process crash/abort, are lost.
///
/// Keep the guards alive for as long as the corresponding `Dispatch` /
/// layers may be used: once a guard drops, later events for that sink are
/// dropped as `channel_disconnected`.
#[derive(Debug)]
#[must_use]
pub struct WriterGuard {
    tx: Option<SyncSender<WorkerMsg>>,
    done_rx: Option<Receiver<()>>,
    join: Option<JoinHandle<()>>,
    metrics: Arc<SinkMetrics>,
    shutdown_timeout: Duration,
}

impl Drop for WriterGuard {
    fn drop(&mut self) {
        let deadline = Instant::now() + self.shutdown_timeout;
        if let Some(tx) = &self.tx {
            // Signal shutdown — retry while the queue stays full, inside
            // the same deadline that bounds the wait below.
            loop {
                match tx.try_send(WorkerMsg::Shutdown) {
                    Err(TrySendError::Full(_)) if Instant::now() < deadline => {
                        thread::yield_now();
                    }
                    // Sent, the receiver is already gone, or the deadline
                    // passed with a still-full queue — stop signalling.
                    _ => break,
                }
            }
        }
        // Releasing our sender lets the worker also exit via the plain
        // all-senders-dropped path when the shutdown marker never landed.
        drop(self.tx.take());
        let Some(done_rx) = self.done_rx.take() else {
            return;
        };
        let remaining = deadline.saturating_duration_since(Instant::now());
        match done_rx.recv_timeout(remaining) {
            // Worker drained, flushed, and signalled done — or its done
            // sender dropped because it already exited (e.g. panicked);
            // either way `join` returns promptly.
            Ok(()) | Err(RecvTimeoutError::Disconnected) => {
                if let Some(join) = self.join.take() {
                    let _ = join.join();
                }
                self.metrics.set_state(SinkState::Failed);
            }
            Err(RecvTimeoutError::Timeout) => {
                self.metrics.mark_shutdown_abandoned();
                self.metrics.set_state(SinkState::Failed);
            }
        }
    }
}

/// Spawns the bounded writer path for one sink.
///
/// Returns the sender used to build `EventWriter`s and the [`WriterGuard`]
/// that performs shutdown. `inner` is the final destination
/// (`RollingFileAppender`, `std::io::Stderr`, or a test double).
pub(crate) fn spawn(
    inner: Box<dyn Write + Send>,
    queue_capacity: NonZeroUsize,
    metrics: Arc<SinkMetrics>,
) -> (BoundedSender, WriterGuard) {
    spawn_with_timeout(inner, queue_capacity, metrics, SHUTDOWN_TIMEOUT)
}

fn spawn_with_timeout(
    inner: Box<dyn Write + Send>,
    queue_capacity: NonZeroUsize,
    metrics: Arc<SinkMetrics>,
    shutdown_timeout: Duration,
) -> (BoundedSender, WriterGuard) {
    let (tx, rx) = sync_channel::<WorkerMsg>(queue_capacity.get());
    let (done_tx, done_rx) = std::sync::mpsc::channel::<()>();
    let worker_metrics = Arc::clone(&metrics);
    let join = thread::Builder::new()
        .name(format!("ruviform-log-sink-{}", metrics.sink_index))
        .spawn(move || worker_loop(&rx, inner, &worker_metrics, &done_tx))
        .expect("failed to spawn ruviform logging sink worker thread");
    (
        BoundedSender {
            tx: tx.clone(),
            metrics: metrics.clone(),
        },
        WriterGuard {
            tx: Some(tx),
            done_rx: Some(done_rx),
            join: Some(join),
            metrics,
            shutdown_timeout,
        },
    )
}

/// Worker: relay each queued `Event` through `write_all`, classify the
/// result; on `Shutdown`, count and discard any stragglers queued behind
/// it, then flush once and exit; on all-senders-dropped, flush once and
/// exit. Failures are counted — never written to the log pipeline.
fn worker_loop(
    rx: &Receiver<WorkerMsg>,
    mut inner: Box<dyn Write + Send>,
    metrics: &Arc<SinkMetrics>,
    done_tx: &Sender<()>,
) {
    use std::sync::atomic::Ordering::Relaxed;
    'outer: while let Ok(message) = rx.recv() {
        match message {
            WorkerMsg::Event(bytes) => match inner.write_all(&bytes) {
                Ok(()) => {
                    metrics.events_written.fetch_add(1, Relaxed);
                }
                Err(error) if error.kind() == io::ErrorKind::WriteZero => {
                    metrics.partial_writes.fetch_add(1, Relaxed);
                }
                Err(_) => {
                    metrics.write_failures.fetch_add(1, Relaxed);
                }
            },
            WorkerMsg::Shutdown => {
                // Events accepted into the queue *behind* the shutdown
                // marker are dropped — count them so the loss is visible.
                while let Ok(straggler) = rx.try_recv() {
                    if let WorkerMsg::Event(_) = straggler {
                        metrics.channel_disconnected.fetch_add(1, Relaxed);
                    }
                }
                break 'outer;
            }
        }
    }
    // Drain complete (Shutdown received, or every sender dropped): final
    // flush, mark the sink no longer writing, and signal done.
    if inner.flush().is_err() {
        metrics.flush_failures.fetch_add(1, Relaxed);
    }
    metrics.set_state(SinkState::Failed);
    let _ = done_tx.send(());
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::num::NonZeroUsize;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicU64, Ordering};

    use crate::logging::health::SinkKind;

    /// Writer that records into a shared buffer.
    struct SharedBufWriter(Arc<Mutex<Vec<u8>>>);

    impl Write for SharedBufWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    /// Writer that fails every write with a generic io error.
    struct AlwaysErr;

    impl Write for AlwaysErr {
        fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
            Err(io::Error::other("boom"))
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    /// Writer that reports `Ok(0)` — surfaces as `WriteZero` in `write_all`.
    struct WriteZeroWriter;

    impl Write for WriteZeroWriter {
        fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
            Ok(0)
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    /// Writer that panics — kills the worker, disconnecting the channel.
    struct PanicWriter;

    impl Write for PanicWriter {
        fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
            panic!("writer crashed");
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    /// Writer that succeeds on write but fails the final flush.
    struct FlushFailWriter {
        written: Arc<AtomicU64>,
    }

    impl Write for FlushFailWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.written.fetch_add(1, Ordering::Relaxed);
            Ok(buf.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::other("flush failed"))
        }
    }

    /// Writer that stalls in `write` far past the test's short timeout.
    struct StallWriter;

    impl Write for StallWriter {
        fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
            thread::sleep(Duration::from_secs(60));
            Ok(0)
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    /// Buffers `bytes` into an `EventWriter` without invoking its `Write`
    /// impl — the helper keeps each call a single-`write` send.
    fn event(sender: &BoundedSender, bytes: &[u8]) -> EventWriter {
        let mut writer = sender.event_writer();
        writer.buf.extend_from_slice(bytes);
        writer
    }

    fn metrics() -> Arc<SinkMetrics> {
        Arc::new(SinkMetrics::new(0, SinkKind::RollingFile))
    }

    fn load(counter: &AtomicU64) -> u64 {
        counter.load(Ordering::Relaxed)
    }

    #[test]
    fn queue_full_drops_are_counted() {
        let metrics = metrics();
        // Never consume: a hand-built channel that stays full.
        let (tx, rx) = sync_channel::<WorkerMsg>(1);
        let sender = BoundedSender {
            tx,
            metrics: Arc::clone(&metrics),
        };
        drop(event(&sender, b"one"));
        drop(event(&sender, b"two"));
        assert_eq!(load(&metrics.dropped_queue_full), 1);
        assert_eq!(load(&metrics.channel_disconnected), 0);
        // The first message is still queued for the receiver.
        match rx.try_recv().unwrap() {
            WorkerMsg::Event(bytes) => assert_eq!(bytes, b"one".to_vec()),
            WorkerMsg::Shutdown => panic!("unexpected shutdown marker"),
        }
    }

    #[test]
    fn disconnect_before_first_send_marks_failed() {
        let metrics = metrics();
        let (tx, rx) = sync_channel::<WorkerMsg>(4);
        drop(rx);
        let sender = BoundedSender {
            tx,
            metrics: Arc::clone(&metrics),
        };
        drop(event(&sender, b"evt"));
        assert_eq!(load(&metrics.channel_disconnected), 1);
        assert_eq!(metrics.snapshot().state, SinkState::Failed);
    }

    #[test]
    fn writer_failures_are_counted_and_worker_continues() {
        let metrics = metrics();
        let (sender, guard) = spawn(
            Box::new(AlwaysErr),
            NonZeroUsize::new(8).unwrap(),
            Arc::clone(&metrics),
        );
        for _ in 0..3 {
            drop(event(&sender, b"x"));
        }
        drop(sender);
        drop(guard);
        assert_eq!(load(&metrics.write_failures), 3);
        assert_eq!(load(&metrics.partial_writes), 0);
        assert_eq!(metrics.snapshot().state, SinkState::Failed);
    }

    #[test]
    fn write_zero_is_counted_as_partial_write() {
        let metrics = metrics();
        let (sender, guard) = spawn(
            Box::new(WriteZeroWriter),
            NonZeroUsize::new(4).unwrap(),
            Arc::clone(&metrics),
        );
        drop(event(&sender, b"x"));
        drop(sender);
        drop(guard);
        assert_eq!(load(&metrics.partial_writes), 1);
        assert_eq!(load(&metrics.write_failures), 0);
    }

    #[test]
    fn flush_failure_is_counted_at_shutdown() {
        let metrics = metrics();
        let written = Arc::new(AtomicU64::new(0));
        let (sender, guard) = spawn(
            Box::new(FlushFailWriter {
                written: Arc::clone(&written),
            }),
            NonZeroUsize::new(4).unwrap(),
            Arc::clone(&metrics),
        );
        drop(event(&sender, b"x"));
        drop(sender);
        drop(guard);
        assert_eq!(load(&metrics.flush_failures), 1);
        assert_eq!(written.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn successful_writes_drain_and_join() {
        let metrics = metrics();
        let buf = Arc::new(Mutex::new(Vec::new()));
        let (sender, guard) = spawn(
            Box::new(SharedBufWriter(Arc::clone(&buf))),
            NonZeroUsize::new(4).unwrap(),
            Arc::clone(&metrics),
        );
        drop(event(&sender, b"hello,"));
        drop(event(&sender, b"world"));
        drop(sender);
        drop(guard);
        assert_eq!(load(&metrics.events_written), 2);
        assert_eq!(*buf.lock().unwrap(), b"hello,world".to_vec());
    }

    #[test]
    fn dead_worker_marks_sink_failed_on_next_send() {
        let metrics = metrics();
        let (sender, guard) = spawn(
            Box::new(PanicWriter),
            NonZeroUsize::new(4).unwrap(),
            Arc::clone(&metrics),
        );
        // Let the worker pick up the message and die on it.
        drop(event(&sender, b"kaboom"));
        thread::sleep(Duration::from_millis(200));
        drop(event(&sender, b"after"));
        assert!(load(&metrics.channel_disconnected) >= 1);
        drop(sender);
        drop(guard);
        assert_eq!(metrics.snapshot().state, SinkState::Failed);
    }

    #[test]
    fn shutdown_marker_drains_then_drops_stragglers() {
        let metrics = metrics();
        let buf = Arc::new(Mutex::new(Vec::new()));
        let (tx, rx) = sync_channel::<WorkerMsg>(8);
        let (done_tx, done_rx) = std::sync::mpsc::channel::<()>();
        let worker_metrics = Arc::clone(&metrics);
        let worker_buf = Arc::clone(&buf);
        let join = thread::spawn(move || {
            worker_loop(
                &rx,
                Box::new(SharedBufWriter(worker_buf)),
                &worker_metrics,
                &done_tx,
            );
        });
        tx.try_send(WorkerMsg::Event(b"before".to_vec())).unwrap();
        tx.try_send(WorkerMsg::Shutdown).unwrap();
        tx.try_send(WorkerMsg::Event(b"after".to_vec())).unwrap();
        drop(tx);
        join.join().unwrap();
        done_rx.try_recv().unwrap();
        assert_eq!(load(&metrics.channel_disconnected), 1);
        // Pre-shutdown event made it through; the straggler did not.
        assert_eq!(load(&metrics.events_written), 1);
        assert_eq!(*buf.lock().unwrap(), b"before".to_vec());
    }

    #[test]
    fn shutdown_timeout_marks_abandoned() {
        let metrics = metrics();
        let (sender, guard) = spawn_with_timeout(
            Box::new(StallWriter),
            NonZeroUsize::new(4).unwrap(),
            Arc::clone(&metrics),
            Duration::from_millis(50),
        );
        drop(event(&sender, b"stall"));
        drop(sender);
        // Worker is stuck in write for ~60s; the 50ms wait abandons it.
        let started = Instant::now();
        drop(guard);
        assert!(started.elapsed() < Duration::from_secs(5));
        let snapshot = metrics.snapshot();
        assert!(snapshot.shutdown_abandoned);
        assert_eq!(snapshot.state, SinkState::Failed);
    }
}
