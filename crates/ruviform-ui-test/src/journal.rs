//! Observation journal — a bounded receipt ring over host-boundary
//! traffic (Issue #312, neutral re-expression in Issue #313 slice S2).
//!
//! [`Journal`] records what crosses the `DriverEvent`/`WindowDriver`
//! boundary in both directions; [`JournalHandle`] is the
//! [`WindowDriver`] decorator that captures the app→host direction.
//! Receipts store kind/window/direction/byte-size only — message
//! bodies and event payloads are never copied into the journal, which
//! keeps the observation surface read-only and bounded by
//! construction.
//!
//! **RT contract:** non-RT only — UI-thread code.

use std::collections::VecDeque;

use ruviform_ui::{
    CloseDecision, DriverEvent, Event, LogicalSize, MonitorInfo, RoutedEvent, UiError,
    WindowCapabilities, WindowDriver, WindowId, WindowLifecycle, WindowSpec,
};

use crate::proto::{Direction, Receipt, ReceiptPage};

/// Default journal capacity (entries).
pub const DEFAULT_JOURNAL_CAP: usize = 1024;

/// Bounded monotonic receipt ring.
pub struct Journal {
    entries: VecDeque<Receipt>,
    next_seq: u64,
    cap: usize,
}

impl Default for Journal {
    fn default() -> Self {
        Self::new(DEFAULT_JOURNAL_CAP)
    }
}

impl Journal {
    /// Creates a journal retaining at most `cap` entries (oldest are
    /// dropped; reads report `truncated`).
    #[must_use]
    pub fn new(cap: usize) -> Self {
        Self {
            entries: VecDeque::new(),
            // Sequences start at 1: `entries_since` returns `seq >
            // since`, so a fresh cursor of 0 must observe every entry.
            next_seq: 1,
            cap: cap.max(1),
        }
    }

    /// Records one receipt; returns its sequence number.
    pub fn record(
        &mut self,
        direction: Direction,
        window: Option<WindowId>,
        kind: impl Into<String>,
        bytes: usize,
    ) -> u64 {
        let seq = self.next_seq;
        self.next_seq += 1;
        if self.entries.len() >= self.cap {
            self.entries.pop_front();
        }
        self.entries.push_back(Receipt {
            seq,
            direction,
            window: window.map(WindowId::get),
            kind: kind.into(),
            bytes: u32::try_from(bytes).unwrap_or(u32::MAX),
        });
        seq
    }

    /// Records an inbound [`DriverEvent`] (host→app delivery).
    pub fn record_in(&mut self, ev: &DriverEvent) -> u64 {
        match ev {
            DriverEvent::Windowed { window, event } => {
                let (kind, bytes) = lifecycle_kind(event);
                self.record(Direction::In, Some(*window), kind, bytes)
            }
            DriverEvent::Tick => self.record(Direction::In, None, "tick", 0),
            _ => self.record(Direction::In, None, "other", 0),
        }
    }

    /// Records an inbound payload suppressed by the `ipc_disconnect`
    /// fault — the drop is observable, never silent. `bytes` is the
    /// suppressed payload size the caller measured (raw body length
    /// or serialized intent length).
    pub fn record_suppressed(&mut self, window: WindowId, bytes: usize) -> u64 {
        self.record(Direction::In, Some(window), "suppressed", bytes)
    }

    /// Entries with `seq > since`, oldest first, at most `limit`.
    /// `truncated` reports that `since` fell below the retained floor
    /// (older receipts were dropped by the bounded ring).
    #[must_use]
    pub fn entries_since(&self, since: u64, limit: usize) -> ReceiptPage {
        let floor = self.entries.front().map_or(self.next_seq, |e| e.seq);
        let entries: Vec<Receipt> = self
            .entries
            .iter()
            .filter(|e| e.seq > since)
            .take(limit)
            .cloned()
            .collect();
        let next_since = entries.last().map_or(since, |e| e.seq);
        // Truncated only when a receipt the cursor could have seen was
        // actually evicted — dropped seqs are `..floor`, so the reader
        // loses entries iff `since + 1 < floor`. (`since = 0` on a
        // non-overflowed journal has floor 1 → not truncated.)
        ReceiptPage {
            entries,
            next_since,
            truncated: since + 1 < floor,
        }
    }

    /// High-water mark (the seq the next recorded entry will get).
    /// A reader starting at `since = 0` sees the whole journal.
    #[must_use]
    pub fn next_seq(&self) -> u64 {
        self.next_seq
    }
}

/// Maps a [`WindowLifecycle`] to its receipt kind + payload size.
/// `Action` reports the serialized intent length — the byte-size
/// equivalent of the legacy `Message` body's length.
fn lifecycle_kind(event: &WindowLifecycle) -> (&'static str, usize) {
    match event {
        WindowLifecycle::Created => ("created", 0),
        WindowLifecycle::Ready => ("ready", 0),
        WindowLifecycle::Action(intent) => (
            "message",
            serde_json::to_string(intent).map_or(0, |s| s.len()),
        ),
        WindowLifecycle::Resized { .. } => ("resized", 0),
        WindowLifecycle::Moved { .. } => ("moved", 0),
        WindowLifecycle::Focused(_) => ("focused", 0),
        WindowLifecycle::ScaleFactorChanged { .. } => ("scale_factor_changed", 0),
        WindowLifecycle::CloseRequested => ("close_requested", 0),
        WindowLifecycle::Destroyed => ("destroyed", 0),
        _ => ("other", 0),
    }
}

/// [`WindowDriver`] decorator recording app→host traffic into a
/// [`Journal`] before forwarding — the read-only tee that makes the
/// out direction observable on any host (fake or real).
///
/// Usage inside `on_event`: wrap the supplied driver,
/// `let mut jh = JournalHandle::new(drv, &mut journal);`, then call
/// driver ops on `jh`.
pub struct JournalHandle<'a> {
    inner: &'a mut dyn WindowDriver,
    journal: &'a mut Journal,
}

impl<'a> JournalHandle<'a> {
    /// Wraps `inner` so every call is journaled then forwarded.
    pub fn new(inner: &'a mut dyn WindowDriver, journal: &'a mut Journal) -> Self {
        Self { inner, journal }
    }
}

impl WindowDriver for JournalHandle<'_> {
    fn capabilities(&self) -> WindowCapabilities {
        self.inner.capabilities()
    }

    fn create_window(&mut self, spec: &WindowSpec) -> Result<WindowId, UiError> {
        self.journal
            .record(Direction::Out, None, "create_window", 0);
        self.inner.create_window(spec)
    }

    fn resize(&mut self, window: WindowId, size: LogicalSize) -> Result<(), UiError> {
        self.journal
            .record(Direction::Out, Some(window), "resize", 0);
        self.inner.resize(window, size)
    }

    fn focus(&mut self, window: WindowId, focused: bool) -> Result<(), UiError> {
        self.journal
            .record(Direction::Out, Some(window), "focus", 0);
        self.inner.focus(window, focused)
    }

    fn request_close(&mut self, window: WindowId) {
        self.journal
            .record(Direction::Out, Some(window), "request_close", 0);
        self.inner.request_close(window);
    }

    fn respond_close(&mut self, window: WindowId, decision: CloseDecision) -> Result<(), UiError> {
        self.journal
            .record(Direction::Out, Some(window), "respond_close", 0);
        self.inner.respond_close(window, decision)
    }

    fn destroy_window(&mut self, window: WindowId) -> Result<(), UiError> {
        self.journal
            .record(Direction::Out, Some(window), "destroy_window", 0);
        self.inner.destroy_window(window)
    }

    fn post_event(&mut self, window: WindowId, event: &Event) {
        let bytes = serde_json::to_string(event).map_or(0, |s| s.len());
        self.journal
            .record(Direction::Out, Some(window), "post_event", bytes);
        self.inner.post_event(window, event);
    }

    fn windows(&self) -> Vec<WindowId> {
        self.inner.windows()
    }

    fn monitors(&self) -> Vec<MonitorInfo> {
        self.inner.monitors()
    }

    fn exit(&mut self) {
        self.journal.record(Direction::Out, None, "exit", 0);
        self.inner.exit();
    }
}

/// Convenience: deliver one [`RoutedEvent`] the way neutral apps do —
/// `post_event` the typed payload to its destination window (unrouted
/// events are dropped, matching the legacy `eval` path).
pub fn deliver_routed(drv: &mut dyn WindowDriver, re: &RoutedEvent) {
    if let Some(window) = re.window {
        drv.post_event(window, &re.event);
    }
}
