---
id: "DISC-034S312gtUiTestDiscAA11"
type: "discovery"
status: "resolved"
title: "Issue #312 test-only GUI host lifecycle / secure control adapter — inventory and option survey"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/312"
  changes: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-156-gui-host-foundation.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
  refines: []
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/ruviform/issues/312"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/310"
    - "https://github.com/scarecrow173/ruviform/issues/311"
    - "https://github.com/scarecrow173/ruviform/issues/313"
    - "https://github.com/scarecrow173/ruviform/issues/243"
    - "https://github.com/scarecrow173/ruviform/issues/156"
    - "https://github.com/scarecrow173/ruviform/issues/181"
  defers: []
  deferred-by: []
---

# Issue #312 test-only GUI host lifecycle / secure control adapter — inventory and option survey

## Purpose

Inventory the existing GUI host surface the Issue #312 test control port must attach to, and pin the option space for: where the test-only lifecycle/control port lives, how the driver↔app channel is transported and authenticated, and how observation/fixture/clock/fault semantics stay honest (no fake user ops, no fake full-clock control). Resolved input for the spec/ADR/design.

## Existing Surface Inventory

- `crates/ruviform-ui` (extension layer, Issue #156): backend-free host boundary. Reusable pieces already public:
  - `ipc`: `Intent`/`Event`/`ErrorBody`/`Envelope`/`parse_intent`/`valid_op`/`SCHEMA_VERSION`/`codes` — versioned envelope + stable error codes + size cap (`MAX_INTENT_BYTES`). The `Event` enum already covers `Result` (correlated), `Job`, `Notify {seq,data}` (uncorrelated push), `Lifecycle`, `Error`.
  - `event`: `HostEvent` (Ready/Message/Resized/Focused/ScaleFactorChanged/Moved/CloseRequested/Destroyed/Tick) + `HostHandle` (eval/resize/focus/request_close).
  - `window`: `WindowId`, `WindowEvent::{Windowed,Tick}`, `RoutedEvent`, `CloseDecision`, `WindowConfig`, `MonitorInfo`, `WindowHandle` (eval/resize/focus/create_window/request_close/respond_close/destroy_window/windows/monitors/exit). Delivery contract: eval to unknown/destroyed windows is a silent drop; not-ready windows buffer a bounded backlog.
  - `runtime`: `UiRuntime` — `handle`/`handle_window`/`dispatch_op`/`drain_jobs*`/`poll`, `sys.*` ops incl. `sys.capabilities` (schema/os/arch/ops/data limits/jobs.running), job registry with `CancellationToken`, `cancel_window_jobs`.
  - `service`: `Service`/`JobSpec`/`JobContext` — `JobSpec` workers already panic-safe (`catch_unwind` → `worker_failed`).
  - `data`: `DataRegistry`/`DataSource` bounded pull path.
  - `demo`: `DemoService` (`demo.echo/synthesize/sleep/fail/panic`) — ready-made dummy consumer service surface, shared GUI/CLI by construction (Issue #156 contract).
- `crates/ruviform-ui-webview` (adapter): `host::run` single-window and `session::run_multi` multi-window loops. The app supplies `on_event(WindowEvent, &mut dyn WindowHandle)` — every host event passes through consumer code, which is exactly where read-only observation and allowlisted fault injection can attach with **zero adapter changes**.
- `crates/ruviform-ui/tests/multi_window.rs`: a `FakeHost: WindowHandle` precedent (window map, ready/backlog, close-pending, delivered eval capture) — the shape the headless test host generalizes.
- `crates/ruviform-cli-sample` + `demo_cli` example: precedent for "same service, non-GUI entry" — the dummy consumer pattern already exists headless.
- mise/CI: `test` covers all workspace crates headless; `ui-verify` lane exists for webview-feature checks. No new CI lane required if the new crate stays headless-only.

## Boundary Constraints (Issue + architecture)

- WebView production host/API changes minimized — ideally none.
- Test infra must not be a required dep of production artifacts or RT.
- No second product Command Gateway; no OS-level input injection (RF-GT-04 / #313 owns click/drag/focus/DOM).
- No arbitrary shell/file/eval API on the control channel.
- Normal user ops vs test observation/fixture separation: test-side receipts must never be counted as user-operation E2E pass.
- Honest clock control: declare which clock domain(s) are controllable; never pretend OS events/GPU/wall-clock are controlled.

## Options Surveyed

### Where does the control port implementation live?

- **A. New crate `ruviform-ui-test`**, depending on `ruviform-ui` only (public API). Agent, driver, fake host, protocol all inside. Production crates untouched — isolation is structural (DAG), not flag-based.
- B. Feature-gated module inside `ruviform-ui` (`test-support` feature): smaller file count, but feature unification means any consumer enabling it transitively injects test surface into the foundation crate; release-artifact unreachability becomes a flag audit instead of a dependency fact.
- C. `#[cfg(test)]` code inside `ruviform-ui`: cannot serve external consumers' E2E apps or a spawned dummy consumer process.

Chosen: **A** — cleanest fail-closed story ("no dep ⇒ no control surface"), matches adapter-layer precedent, keeps `ruviform-ui` dep set unchanged.

### Transport

- **A. Child-process stdio framing (default)**: `Command` with piped stdin/stdout; length-prefixed JSON frames on stdout, logs stay on stderr. OS-inherited pipe permissions, no ports, works on Windows+unix+macOS with std only. Matches issue text "inherit したローカル Pipe".
- B. Loopback TCP (opt-in): parent `TcpListener` on `127.0.0.1:0`, child dials `RVF_TEST_ADDR`, `Hello` carries `RVF_TEST_SECRET` run-bound token; single accepted connection, loopback peer check, refused when secret unset. For attach-style runs or transports where pipes are awkward. std-only.
- C. Unix domain sockets: std on unix only; Windows needs a crate (named pipes). Adds platform split for no capability stdio doesn't already give.

Chosen: **A default + B opt-in**; C deferred (YAGNI — unix sockets can be added later behind the same `serve(Read,Write)` abstraction).

### Dispatch model

- **A. Agent as a pure dispatcher** (`handle_intent(&mut dyn TestIo) -> Vec<Event>`) + generic `serve(Read+Write)` frame loop. The app owns threading: headless binary runs `serve_stdio` on main; a windowed host app would queue frames and dispatch on event turns (tick cadence).
- B. Agent spawns its own threads: simpler call sites, but forces `Send` bounds onto `TestIo`/journal and hides the UI-thread affinity that real hosts require.

Chosen: **A** — honest about where host objects may run; `serve` is transport-agnostic so both stdio and TCP share one code path; headless path needs no threads at all.

### Observation capture point

- **A. Tee at the app's event boundary** (`Journal` records inbound `WindowEvent` + `JournalHandle` `WindowHandle`-decorator records outbound eval/host ops): works identically on `FakeHost` and real `run_multi` because both funnel through `on_event`. Read-only — the journal never alters traffic.
- B. Hooks inside `UiRuntime`/session: would require production changes for test-only needs.

Chosen: **A** — zero production surface change; receipts record kind/window/direction/size, not bulk payloads (bounded ring, redaction by construction).

### Clock control

- `HostTick` domain only: `test.clock.advance {domain:"host_tick", ticks:n}` emits `n` `WindowEvent::Tick`s — the drain point for job mailboxes. Declared scope: does **not** accelerate OS timers, wall clock, GPU/vsync, or webview-internal timers.
- Alternatives (virtual wall clock, time freezing): rejected — dishonest on a real host, and unnecessary for the drain semantics tests actually need.

### Fault injection allowlist

- `window_crash` → `destroy_window` (host-level loss; emits `Destroyed`, same terminal contract as backend loss).
- `ipc_disconnect` → suppress inbound `Message` events for a window until `fault.clear` (models dead frontend→host channel at the host boundary; the app sees nothing).
- `request_close` is *not* a fault — it maps to `WindowHandle::request_close` (production vocabulary already exists; used for close-veto scenarios).
- Rejected: arbitrary eval injection, DOM mutation, network faults, OS-signal simulation — out of boundary (RF-GT-04 / dishonest).

### Screenshot/trace

- `test.observe.screenshot` exists in the schema; both FakeHost and wry declare `screenshot:false` capability → `unavailable` error (honest NotRun). Real capture is OS/backend work owned by #313/#243.
- Trace = bounded journal export (`test.observe.receipts`); no always-on event store.

## Confirmed Mechanics

- `run_multi`/`host::run` need no changes: `WindowHandle::{create_window,request_close,respond_close,destroy_window,windows,monitors,exit}` plus `WindowEvent` delivery covers fixture arrange, close-veto, crash simulation, shutdown.
- `UiRuntime::dispatch_op("sys.capabilities")` gives schema/os/arch/ops/job count via public API — reused verbatim for the capabilities reply's runtime section.
- `CARGO_BIN_EXE_<name>` makes a `src/bin` dummy consumer spawnable from the crate's own integration tests — true cross-process stdio E2E under plain `cargo test`.
- No new external deps needed: serde/serde_json already used by `ruviform-ui`; frames, handshake, secret generation (`std::hash::RandomState` + time/pid seed) are std-only.

## Open Questions Resolved at Design

- Whether release-unreachability is asserted by a dedicated test: structural (dependency DAG + diff review) — recorded as verification, not runtime code.
- Whether `test.hello` re-handshake is allowed mid-session: fail-closed (one session per run; restart = driver respawns the process).
