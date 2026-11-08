//! Frontend bridge — the JS surface the foundation injects into the
//! webview (`with_initialization_script`) plus the Rust-side push
//! helper.
//!
//! **Legacy (Issue #465):** WebView/JS transport only — the
//! driver-neutral path delivers typed [`crate::ipc::Intent`]s via
//! [`crate::driver::WindowLifecycle::Action`] and does not use this
//! bridge. Kept for the `WebView` adapter until Issue #465 P2.
//!
//! `window.__rvf` exposes:
//!
//! - `call(op, params) -> Promise<data>` — sends an `rvf.ipc.v1`
//!   intent; resolves on the first `result.ok` for it (for a
//!   job-spawning intent that is the ack, `accepted`), rejects with an
//!   `Error` carrying `.code` on failure.
//! - `onEvent(cb) -> unsubscribe` — receives every non-result event
//!   (`job`, `notify`, `lifecycle`, `error`), and the terminal `result`
//!   (receipt) of a job-spawning intent whose `call` already settled on
//!   the ack (Issue #456).
//! - `dataUrl(name, offset?, len?) -> string` — builds the `rvf://data`
//!   pull URL for a registered [`crate::data::DataSource`].
//! - `dispatch(obj)` — internal entry point invoked from Rust via
//!   `evaluate_script` (see [`dispatch_script`]).
//!
//! The bridge is framework-agnostic plain JS — Solid apps wrap it in a
//! thin typed client on their side.

use crate::ipc::Event;

/// Initialization script installed before any page script runs.
pub const BRIDGE_JS: &str = r"(() => {
  if (window.__rvf) return;
  const pending = new Map();
  const listeners = new Set();
  let seq = 0;
  window.__rvf = {
    version: 1,
    call(op, params) {
      const id = 'i-' + (++seq);
      return new Promise((res, rej) => {
        pending.set(id, { res, rej });
        try {
          window.ipc.postMessage(
            JSON.stringify({ v: 1, kind: 'intent', id, op, params: params || {} }));
        } catch (e) { pending.delete(id); rej(e); }
      });
    },
    onEvent(cb) { listeners.add(cb); return () => { listeners.delete(cb); }; },
    dataUrl(name, offset, len) {
      const q = new URLSearchParams({ offset: String(offset || 0) });
      if (len != null) q.set('len', String(len));
      const path = String(name).split('/').map(encodeURIComponent).join('/');
      return 'rvf://data/' + path + '?' + q;
    },
    dispatch(m) {
      if (m && m.kind === 'result' && pending.has(m.id)) {
        const p = pending.get(m.id); pending.delete(m.id);
        if (m.ok) { p.res(m.data); }
        else {
          const e = new Error((m.error && m.error.message) || 'rvf error');
          e.code = m.error && m.error.code;
          p.rej(e);
        }
        return;
      }
      for (const cb of Array.from(listeners)) {
        try { cb(m); } catch (e) { console.error(e); }
      }
    },
  };
})();";

/// Builds the `evaluate_script` body that delivers `event` to
/// `window.__rvf`.
///
/// The event JSON is double-encoded (`JSON.parse(<js string literal>)`)
/// so any quoting/escaping inside payloads — quotes, `</script>`,
/// U+2028/2029 — survives the JS string-literal hop intact. Returns a
/// no-op-safe expression when `__rvf` was not installed (page not
/// loaded yet); the caller drops such events by design.
#[must_use]
pub fn dispatch_script(event: &Event) -> String {
    let json = event.to_json();
    let literal = serde_json::to_string(&json).unwrap_or_else(|_| "\"{}\"".to_string());
    format!("window.__rvf && window.__rvf.dispatch(JSON.parse({literal}))")
}
