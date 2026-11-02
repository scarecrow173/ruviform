---
id: "ADR-s16uWrHwN9qD8cKo517PU7"
type: "adr"
status: "accepted"
title: "Issue #181 Multi-Window — contract placement, close-decision separation, and IPC routing"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/181"
  changes:
    added:
      - "crates/ruviform-ui/src/window.rs"
    modified:
      - "crates/ruviform-ui/src/event.rs"
      - "crates/ruviform-ui/src/runtime.rs"
      - "crates/ruviform-ui/src/service.rs"
      - "crates/ruviform-ui/src/lib.rs"
      - "crates/ruviform-ui-webview/src/host.rs"
      - "crates/ruviform-ui-webview/src/lib.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/adr/ui-architecture-webview-hybrid.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
    - "docs/adr/realtime-contract-and-verification-policy.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-181-multi-window.md"
    - "docs/discovery/issue-181-multi-window-inventory.md"
    - "docs/designs/issue-156-gui-host-foundation.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-181-multi-window-inventory.md"
    - "docs/specs/issue-181-multi-window.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/156"
    - "https://github.com/scarecrow173/ruviform/issues/192"
    - "https://github.com/scarecrow173/ruviform/issues/193"
    - "https://github.com/scarecrow173/synvelis/issues/21"
    - "https://github.com/scarecrow173/synvelis/issues/22"
  defers: []
  deferred-by: []
metadata:
  gate: "M7"
  issue: 181
---

# Issue #181 Multi-Window — contract placement, close-decision separation, and IPC routing

## Context and Problem Statement

Issue #181 requires a generic multi-window foundation: one native event loop managing multiple Windows/WebViews, per-window event origin, separate CloseRequested/Destroy, and IPC responses routed to the correct window. The inventory (`docs/discovery/issue-181-multi-window-inventory.md`) shows the current contract is single-window at all three layers — `HostEvent` carries no origin, `UiRuntime` returns destination-less events, and `host::run` owns exactly one window where `CloseRequested` exits the loop and cancels every job. The decisions to make:

1. Where window identity and the event envelope live, and whether `HostEvent` itself gains a window field.
2. How IPC responses identify their destination window — and whether `rvf.ipc.v1` must change on the wire.
3. How the close request/decision/destroy lifecycle is modeled (approve/veto/defer, default policy, app-exit ownership).
4. How job events route in a multi-window world, and how window/job lifetimes are separated without breaking the single-window baseline (AudioLens gate).
5. Where the multi-window host implementation lives relative to the existing `host::run`.
6. How unsafe paths (create failure, WebView not-ready, window loss, dead-destination delivery) behave.

## Decision Drivers

* Additive-where-possible compatibility: `HostEvent`/`UiRuntime::handle`/`host::run` are the AudioLens single-window baseline (#219 consumer classification; issue's additional gate) — their signatures and semantics must not silently change.
* `ruviform-ui` stays the backend-free contract crate: no wry/tao/WebView2/WebKitGTK types in the public surface; `#[non_exhaustive]` public enums; no `unsafe`.
* CloseRequested semantics differ fundamentally between one implicit window and N explicit windows: a sub-window close must never imply app exit or job cancellation — the app-exit/last-window policy belongs to the consumer.
* `rvf.ipc.v1` `id` is a per-webview local correlation key (JS `i-<seq>`); two webviews can emit identical ids — the routing authority must be host-side, not on the wire.
* WebView2's user-data folder is a singleton lock and wry's `WebContext` is the natural per-session scope — one shared `WebContext` for all windows, `data_dir` as session config.
* UI-thread discipline: window operations happen on the host's UI thread; no synchronous I/O or long work inside event callbacks; RT never touches UI.

## Considered Options

* Option A: **Additive envelope** — keep `HostEvent` windowless; add `WindowId` + a `WindowEvent` envelope (`Windowed{window,event}` + loop-global `Tick`), a `RoutedEvent{window: Option<WindowId>, event}` return type, a `WindowHandle` port trait, and parallel `UiRuntime` entry points (`handle_window`/`drain_jobs_routed`/`cancel_window_jobs`). Single-window `handle`/`run` untouched.
* Option B: **Origin field inside `HostEvent`** — add `window: WindowId` to every `HostEvent` variant (or restructure as `{window, payload}`) and change `handle` to return routed events everywhere. One event path, but it is a breaking change to every variant's construction and silently rewrites the single-window baseline.
* Option C: **Per-window runtime instances** — one `UiRuntime` per window, each with its own job registry; cross-window events prohibited. Simple isolation, but duplicates services/jobs and cannot express app-scoped long-running jobs whose events return to a specific window.
* Option D: **Wire-format window id** — put a `window`/`w` field into the `rvf.ipc.v1` envelope so the webview self-identifies. Requires frontend/bridge participation and a schema bump; the webview does not know its host-side id at inject time without extra wiring.
* Option E: **Extend `host::run` in place** — grow `HostConfig`/`run` to a window list and make `HostHandle` multi-window. Breaks the single-window signature and mixes the two policies.

## Decision Outcome

Chosen option: **Option A** (additive envelope), with Option D explicitly rejected — **`rvf.ipc.v1` wire format stays unchanged**.

### Window identity and event envelope

* `WindowId(NonZeroU64)` in `ruviform-ui::window`: nonzero, unique per host session, monotonically issued by the adapter, never reused after destroy. It is a host-side routing token — serializable for consumer bookkeeping but never emitted into `rvf.ipc.v1` payloads.
* `WindowEvent` enum: `Windowed{window, event: HostEvent}` for per-window events + `Tick` for the loop-global wakeup. `HostEvent` gains two additive variants — `Moved{x,y}` (physical position) and `Destroyed` (terminal teardown: approved close, programmatic destroy, or abnormal backend loss). `HostEvent::Tick` remains only for the legacy single-window path.
* `RoutedEvent{window: Option<WindowId>, event: Event}`: runtime output carrying an explicit destination. `Some(w)` = deliver to window `w`; `None` = unrouted (headless/legacy origin or windowless job) — the consumer picks the sink.

### IPC routing — no wire change

* `rvf.ipc.v1` stays `v:1` with identical envelope, codes, and correlation rules. Call ids remain per-webview unique, which is *sufficient*: a response is always routed back to the webview that issued the intent, so collisions across webviews are resolved by host-side routing. The bridge script is unchanged.
* `UiRuntime::handle_window(window, ev)` tags every emitted event with `Some(window)`; `dispatch_op`/`handle` keep producing `Vec<Event>` for the single-window/headless path. Job records store `owner: Option<WindowId>` — the intent's origin — used *only* for event routing; job lifetime is independent of the window.
* `UiRuntime::drain_jobs_routed()` is the multi-window tick path: job events are routed to their owner (`Some`) or `None` when ownerless.
* `ServiceContext` gains `origin: Option<WindowId>` (additive field) so a service can observe which window issued an op.

### Close-request lifecycle

* `CloseDecision{Approve, Veto, Defer}` answers a `CloseRequested`: `Approve` destroys the window and emits `Destroyed`; `Veto` cancels and the window lives normally; `Defer` keeps the window open marked close-pending — the consumer must later call `destroy_window` or approve a subsequent request. An unanswered `CloseRequested` defaults to `Approve` for that window only (never exits the loop).
* `WindowHandle` port: `eval` (fire-and-forget; dead destinations are silently dropped), `resize`, `focus`, `create_window -> Result<WindowId>`, `request_close` (routes through the veto-able flow), `respond_close`, `destroy_window` (immediate), `monitors`, `windows`, `exit` (loop-level app exit — consumer policy).
* **Single-window baseline preserved**: `UiRuntime::handle(CloseRequested)` still cancels all jobs and emits `closing`; `host::run` still exits on close. The multi-window `handle_window(CloseRequested)` emits the `closing` lifecycle event to that window but never cancels jobs; `cancel_window_jobs(window)` is the opt-in vocabulary for window-scoped job cleanup.
* App-exit/last-window policy is consumer-owned: the loop keeps running with zero windows; `WindowHandle::exit()` is the only loop-level stop.

### Host implementation

* `ruviform-ui-webview` gains `SessionConfig` + `run_multi`: one tao `EventLoop`, a `WindowId` registry over `{tao id → entry}`, one shared `WebContext` (single UDF — WebView2 singleton), dynamic creation via `EventLoopWindowTarget`, bounded per-window pre-`Ready` eval buffers (drop-oldest), and `Destroyed` terminal emission on any teardown. `host::run` is untouched.
* Fallback rules: `create_window` failure returns `Err` leaving no window; `eval` to unknown/destroyed/not-ready destinations never panics (not-ready queues bounded); fallible ops on dead windows return `UiError`; OS/backend teardown terminates as `Destroyed`.

## Consequences

* Good, because the single-window baseline is structurally untouched — AudioLens regression risk is zero at the contract level.
* Good, because `rvf.ipc.v1` needs no schema bump and the bridge/frontend stays unchanged; duplicated webview-side ids are handled entirely host-side.
* Good, because fake-host tests can exercise the full contract (event separation, close veto/defer, dead destinations, duplicate ids) headless.
* Bad, because two parallel runtime entry points (`handle`/`handle_window`) exist — mitigated by documenting them as disjoint modes; mixing is a usage error.
* Bad, because job events to a destroyed window are silently dropped at delivery — an accepted PoC limitation; observability hooks are deferred.
* Bad, because `Defer` vs `Veto` differ only in bookkeeping — both leave the window alive; the distinction documents consumer intent for future pending-close UX.
