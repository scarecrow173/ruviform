---
id: "ADR-034Qkwzty3hORnIr3aDMO7"
type: "adr"
status: "proposed"
title: "Parameter control events contract placement and deferral of generic Control Graph crate"
created: "2026-11-02"
updated: "2026-11-02"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/186"
    - "https://github.com/scarecrow173/synvelis/issues/6"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/adr/transport-span-scheduler-applied-receipt.md"
    - "docs/adr/configurable-ppqn-resolution-independent-musical-time.md"
    - "docs/adr/synvelis-driven-generic-execution-boundaries.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/issue-186-param-control-events-inventory.md"
    - "docs/specs/issue-186-param-control-events.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-186-param-control-events-inventory.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/185"
    - "https://github.com/scarecrow173/ruviform/issues/188"
    - "https://github.com/scarecrow173/ruviform/issues/193"
    - "https://github.com/scarecrow173/ruviform/issues/196"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: ["synvelis"]
---

# Parameter control events contract placement and deferral of generic Control Graph crate

## Context and Problem Statement

Issue #186 requires a generic, standards-neutral parameter-control contract: per-parameter capability discovery, ParamId↔TargetId mapping, timed delivery, defined rejection, and scheduled/dispatched/reported observability. The inventory (discovery doc) shows the timed delivery skeleton already exists — `TransportPlan<EventKind>` + `TransportScheduler` + `PluginNode` + `EventList` (#190/#196) — while the surrounding contract types do not. Two placement questions must be decided: (1) where the new contract types live across the layer boundary (plugin-core vs bridge vs a new crate), and (2) whether a generic Control Graph crate (Source→Transform→Target DAG evaluation) should be implemented now for Synvelis Automation/Modulation.

## Decision Drivers

* Layering rule: neutral contracts in foundations (plugin-core/media-core), consumer-facing machinery in bridge, adapter-specific details in adapter crates (`docs/adr/synvelis-driven-generic-execution-boundaries.md`).
* #190 spec already defines Timed Control Delivery on `TransportPlan<EventKind>` and explicitly defers param-level capability/rejection/coalescing to #186.
* `NodeEvent` (typed runtime) lacks timing/payload expressiveness; #196 design rejected it for plan/command transport.
* Synvelis is the only consumer of Control evaluation, and its default additive-composition policy is product-owned — a generic DAG engine now would be premature abstraction (YAGNI).
* RT constraint: no-alloc/no-block/no-OS-I/O with bounded queues and defined overflow behavior.
* No new crate without demonstrated need (issue constraint + architecture baseline).

## Considered Options

* Option A: Neutral contract types in `ruviform-plugin-core` + consumer machinery (binding, dispatch ledger) in `ruviform-audio-plugin-bridge`; timed delivery rides existing `TransportPlan<EventKind>` — **no new crate**.
* Option B: New `ruviform-control` crate owning capability/binding/evaluation/DAG — full generic Control Graph now.
* Option C: Everything in bridge only (no plugin-core changes) — keep plugin-core untouched.
* Option D: Use `NodeEvent` SPSC channel for immediate param writes instead of timed events.

## Decision Outcome

Chosen option: "Option A: neutral types in plugin-core + machinery in bridge, no new crate", because it places each responsibility at the layer that owns it, reuses the proven #190/#196 delivery path end-to-end, and defers the generic evaluation engine until a second consumer or Synvelis PoC demonstrates real need — while still delivering the vocabulary (Target/Host-resolved/Scheduled/Dispatched/Reported) that a future evaluation crate would consume.

### Consequences

* Good, because `ParamWriteRejection`/`ParamWriteMeans`/same-offset semantics become standards-neutral contracts usable by any host, and `TargetId` binding + dispatch ledger live in the consumer layer where app identity concerns belong.
* Good, because no new crate means no workspace manifest/lockfile churn and no premature API surface.
* Good, because timed delivery inherits generation gating, budgets, receipts, span partitioning, and vault/retire for free.
* Bad, because `EventKind::ParamValue` remains the only delivery vocabulary; per-note/MIDI events and plugin→host param reporting are deferred (#188 / follow-up).
* Bad, because the dispatch ledger's "Dispatched" stage is a deterministic projection (recomputed from plan + contract + reports), not a wire-level acknowledgment — honest but coarser than true plugin-side confirmation.

### Confirmation

Confirmed by: design doc's capability matrix + contracts; `rt_` tests proving bounded no-alloc dispatch; L0 mock-plugin E2E tests proving order/offset/value delivery; defined-rejection tests; dispatch-ledger queries distinguishing Scheduled/Dispatched/Reported/Unavailable; `verify-ci` green.

## Implementation Plan

* Affected paths:
  - `crates/ruviform-plugin-core/src/param_write.rs` (new): `ParamWriteMeans`, `ParamWriteRejection`, `check_param_write(&ParamInfo, means)`, same-offset semantics documentation; `coalesce_param_events_latest` opt-in helper.
  - `crates/ruviform-plugin-core/src/event.rs`: contract docs for same-offset same-param stable ordering + last-wins application semantics.
  - `crates/ruviform-audio-plugin-bridge/src/target.rs` (new): `TargetId`, `ParamBinding`, `BindingGeneration`, `TargetResolution`, `ParamTargetTable`.
  - `crates/ruviform-audio-plugin-bridge/src/dispatch.rs` (new): `ParamDispatchLedger` — Scheduled/Dispatched/Dropped/Reported/Unavailable stages fed by plan + deterministic projection + `NodeReport` faults.
  - `crates/ruviform-audio-plugin-bridge/src/cap.rs`: per-param precision fields on the honest matrix (event_precision: sample/block; per_note: Unsupported; reporting: Unavailable).
  - Tests: bridge `tests/param_delivery.rs`, `tests/param_observability.rs`, rt tests alongside existing pattern.
* Patterns to follow: `TransportPlan`/`install_plan` for delivery; `ParamValue::new` validation; `NodeReport` fault taxonomy; `assert_no_alloc` rt_ test pattern; `#[non_exhaustive]` public enums; doctest examples.
* Patterns to avoid: `NodeEvent` for param payload; silent clamping of rejection states; importing Synvelis ParameterTarget/Automation types; new wire verbs.
* Dependencies or configuration: no new crates; no new external deps; `Cargo.lock` regenerated by build if needed.

## Verification

* [ ] `mise run fmt-check` / `check` / `lint` / `test` / `test-rt` / `verify-ci` all green
* [ ] L0 E2E: multi-time multi-target ParamEvents delivered in order at expected frame offsets/values through `PluginNode`
* [ ] Defined rejection: READONLY, per-note/immediate means, unresolved target, non-Processing lifecycle each produce typed rejection/fault
* [ ] rt_: dispatch path alloc-free; plan vault keeps old plan during callback, retires non-RT
* [ ] Ledger query returns Scheduled→Dispatched→Reported/Unavailable with revision+time
* [ ] Architectural check: no Synvelis types, no new crate, no `unsafe`, public enums `#[non_exhaustive]`

## Pros and Cons of the Options

### Option A — plugin-core neutral types + bridge machinery (chosen)

* Good, because it follows the ownership layering already established by the boundaries ADR and #196.
* Good, because delivery reuses verified machinery (generation gating, budgets, receipts, vault) instead of building parallel plumbing.
* Good, because scope stays PoC-sized while still producing real acceptance evidence.
* Bad, because two crates gain public API that must be maintained compatibly going forward.

### Option B — new `ruviform-control` crate with DAG evaluation

* Good, because it would fully realize the generic Source→Transform→Target vision and isolate it cleanly.
* Bad, because only one prospective consumer exists and its combination policy is product-owned — the API would be designed against a hypothetical.
* Bad, because it directly conflicts with the issue constraint "新規 Control Graph クレートや汎用 Modulation Source 群の先行実装は本タスクの決定ではない".

### Option C — everything in bridge

* Good, because plugin-core stays untouched (smallest blast radius).
* Bad, because `ParamWriteRejection`/capability contract is standards-neutral and would become unreachable to non-bridge hosts; also mixes app-identity (TargetId) concerns with neutral param contracts in one crate boundary incorrectly.

### Option D — `NodeEvent` SPSC for param writes

* Good, because the channel already exists with generation gating and stale-discard.
* Bad, because it carries `opcode u16 + u32 + f32` — no ParamId width guarantee, no timing, f32 precision loss — and #196 design explicitly rejected this use; would silently degrade sample-accuracy.

## More Information

* Discovery: `docs/discovery/issue-186-param-control-events-inventory.md` (E1–E10 evidence).
* Upstream decisions: transport span scheduler ADR (generation identity spaces), configurable PPQN ADR (BeatPosition authority), boundaries ADR (Synvelis vs Ruviform ownership).
* Consumer: synvelis#6 Parameter Control Model (Reference Value → contributions → Host-resolved), synvelis#20/#22 (binding generation pinning, Timed Control generation matching).
