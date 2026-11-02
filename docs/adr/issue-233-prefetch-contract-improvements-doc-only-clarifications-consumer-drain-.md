---
id: "ADR-034Qoirxpj2Nt7TtjWeDgk"
type: "adr"
status: "accepted"
title: "Issue #233 prefetch contract improvements — doc-only clarifications, consumer drain assist + seek discard flag, fused reopen-at-position"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/233"
    - "docs/discovery/issue-233-media-reader-prefetch-contract-improvements.md"
    - "docs/specs/issue-233-media-reader-prefetch-contract-improvements.md"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-media-reader/src/prefetch.rs"
      - type: "file"
        path: "crates/ruviform-media-reader/README.md"
      - type: "file"
        path: "crates/ruviform-media-reader/tests/prefetch.rs"
      - type: "file"
        path: "crates/ruviform-media-reader/tests/rt_fill.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-233-media-reader-prefetch-contract-improvements.md"
    - "docs/adr/issue-197-media-reader-boundary-additions.md"
    - "docs/adr/realtime-contract-and-verification-policy.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-197-media-reader-prefetch-rt-consumer.md"
    - "docs/impl/ir/issue-197-media-reader-implementation-record.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-233-media-reader-prefetch-contract-improvements.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: []
    consulted: []
    informed: []
---

# Issue #233 prefetch contract improvements — doc-only clarifications, consumer drain assist + seek discard flag, fused reopen-at-position

## Context and Problem Statement

Issue #233 records four improvement candidates observed when Synvelis E2E-L1 exercised the merged #197 `ruviform-media-reader` Prefetch/Seek/Pump contract. Two are contract-text gaps (`fill`'s `expected_first` reads like an engine-frame expectation but is a stream-position assertion; `pump()`'s ≤1-item-per-call bound and `Backpressured` retry semantics live only in tests). Two are feature gaps: `seek` leaves already-sequenced items ahead of the new `Boundary` to play through under the old generation, so sample-exact re-anchor requires an output-producing quiesce-drain→seek choreography; and `reopen` always re-anchors at frame 0, forcing a reopen→seek two-step for mid-stream source replacement. Every change must be additive on the #197 contract, keep RT paths allocation-free/non-blocking/bounded, and stay media-generic (no Synvelis semantics).

## Decision Drivers

* Additivity: no signature, type, or observable-default-behavior change to `fill`/`seek`/`reopen`/`pump`/`PrefetchItem`/`PrefetchTelemetry`.
* Fix at the layer that owns the invariant: stale play-through is owned by the consumer's sequencing rules and the producer's pending queue — the drain/discard mechanism belongs there, not in callers.
* RT safety: any consumer-side mechanism must be bounded, allocation-free, lock-free, non-blocking (pool-size == free-capacity makes block return infallible).
* Minimal coherent change: private `PrefetchCmd` fields are free; public `PrefetchItem`/`PrefetchTelemetry` changes are not needed by any chosen shape and are avoided.
* Contract honesty: dropped spans and landed positions stay distinguishable in reports/telemetry — no silent loss beyond reported counters.

## Considered Options

* Option A (per candidate): doc-only contract rewrites for candidates 1 and 3; additive `PrefetchConsumer::discard_pending` drain assist + `PrefetchHandle` seek variant carrying `discard_pending` → producer `pending` purge for candidate 2; additive `PrefetchHandle::reopen_at` fusing reopen+seek via `PrefetchCmd::Reopen.first: Option<u64>` for candidate 4.
* Option B (candidate 2 alternative): shared requested-generation atomic or a `PrefetchItem::Discard` marker so the consumer stale-drops items pushed before the `Boundary` — true zero-play-through.
* Option C (candidate 2 alternative): producer-side `discard_pending` flag only — `pending.clear()` on seek, no consumer drain.
* Option D (candidate 4 alternative): keep the reopen→seek two-step; document the choreography instead of adding `reopen_at`.

## Decision Outcome

Chosen option: "Option A", because it resolves every candidate at the layer owning the invariant with the smallest additive surface: doc-only where the behavior is already correct, one bounded consumer drain plus an optional producer pending purge where sequencing semantics are the gap, and one private command field where the two-step is the gap. Option B is declined — it requires new shared state or a public marker variant and changes the adoption model for marginal gain over the bounded drain; recorded as a deferred design concern. Option C alone is declined — `pending` holds only markers, so it cannot touch the ring backlog that constitutes audible play-through; it ships only paired with the drain assist. Option D is declined — the issue lists reopen-at-position as a feature candidate and the fused command is a small additive method, not a contract change.

### Consequences

* Good, because sample-exact re-anchor becomes `seek` (+ optional `discard_pending` variant) → `discard_pending()` → resume `fill`, a bounded RT-safe path with no output-producing drain; stale `Fault`/`End` markers can be suppressed at the source when requested.
* Good, because `reopen_at` reports the honest post-seek landing through the existing `Boundary`/`SeekReceipt` landed semantics — no new marker type, no consumer change.
* Good, because candidates 1 and 3 make the contract readable without tests while changing zero behavior.
* Bad, because the drain assist cannot remove items pushed to the ring *before* it runs that arrive *after* it completes (no pre-Boundary generation visibility) — callers still converge on `Boundary`/telemetry, same as today; documented explicitly.
* Bad, because `reopen_at` cannot pre-check `seekable` on the replacement source (the `Fact` is per-source, known only after `open` runs) — post-reopen seek failure reports via the `Fault{Unseekable}` + `Boundary` path instead of an immediate handle-side error; documented.

### Confirmation

Contract tests cover drain bound/adoption, stale-marker suppression under the discard flag, reopen-at-position landed reporting and its failure path; `assert_no_alloc` RT coverage for `discard_pending`; `mise run fmt-check`/`check`/`lint`/`test` green; the full doc chain routes to `complete` under `doc-driven-dev-graph`.

## Implementation Plan

* Affected paths: `crates/ruviform-media-reader/src/prefetch.rs` (doc text, `PrefetchCmd::{Seek,Reopen}` fields, producer `exec_cmd` pending purge + post-reopen seek, `PrefetchConsumer::discard_pending`, `DiscardReport`, `PrefetchHandle::{seek variant, reopen_at}`), `crates/ruviform-media-reader/README.md`, `crates/ruviform-media-reader/tests/{prefetch,rt_fill}.rs`, `crates/ruviform-media-reader/src/lib.rs` re-exports.
* Patterns to follow: `FillReport`/`FaultCounts` explicit report structs; `Boundary{first_frame: reader.position().0}` honest-landing pattern from `Seek`; `#[non_exhaustive]` on new public types; `**RT contract:**` labels; `&RtContext` gating for stream-consuming RT methods.
* Patterns to avoid: signature changes on existing methods; new `PrefetchItem` variants; shared mutable state beyond the existing telemetry `Arc`; parameter/field renames.
* Dependencies or configuration: none — no new crates, features, or tools.

## Verification

* [ ] `mise run test` — new contract tests for discard/drain/reopen_at pass alongside all pre-existing tests unmodified.
* [ ] `mise run lint` / `mise run check` / `mise run fmt-check` green.
* [ ] RT lane (`rt_`-prefixed test) proves `discard_pending` performs no allocation.
* [ ] Doc audit (`audit_briefing.js`, `audit_adr.js`) and graph exit-audit pass.

## Pros and Cons of the Options

### Option A — per-candidate doc-only / drain assist + flag / fused reopen

* Good, because it fixes each candidate at the owning layer with additive surface only; the drain loop bound is ring capacity + 1 (RT-provable); `reopen` delegates through one shared command path.
* Bad, because residual pre-Boundary play-through remains possible if `discard_pending` runs before the producer enqueues the `Boundary` — inherent to generation-via-Boundary adoption; mitigated by the documented convergence rule and the `boundary` report field.

### Option B — requested-generation cell / Discard marker

* Good, because the consumer could stale-drop items pushed before `Boundary` arrival — true zero-play-through.
* Bad, because it adds shared mutable state or a public `PrefetchItem` variant and changes the generation-adoption model — disproportionate machinery for a low-priority additive improvement; the bounded drain already covers the observed need.

### Option C — producer pending purge only

* Good, because it is a two-line producer change.
* Bad, because `pending` holds only markers — the audio-relevant backlog lives in the data ring the producer cannot touch; alone it does not fix play-through.

### Option D — document the reopen→seek two-step

* Good, because zero code change.
* Bad, because it keeps a two-command, two-generation choreography with an intermediate frame-0 boundary for an operation that is one concept (replace source at position) — the fused method is a small additive surface.

## More Information

- Issue: https://github.com/scarecrow173/ruviform/issues/233; provenance: Synvelis #24 E2E-L1 (`synvelis/docs/impl/ir/l1-e2e-local-wav-verified-playback.md` Known Limitations / Follow-ups).
- Upstream: `docs/specs/issue-233-media-reader-prefetch-contract-improvements.md`, `docs/discovery/issue-233-media-reader-prefetch-contract-improvements.md`.
- Refined contract: `docs/specs/issue-197-media-reader-prefetch-rt-consumer.md`, `docs/adr/issue-197-media-reader-boundary-additions.md`.
