---
id: "ADR-034Qx7RMuPSyPr6zOUFaYe"
type: "adr"
status: "accepted"
title: "Issue 244 prefetch contract warts: boundary-first fault emission + dead-generation sequence + End commit-once"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/244"
    - "docs/discovery/issue-244-media-reader-prefetch-contract-warts.md"
    - "docs/specs/issue-244-media-reader-prefetch-contract-warts.md"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-media-reader/src/prefetch.rs"
      - type: "file"
        path: "crates/ruviform-media-reader/README.md"
      - type: "file"
        path: "crates/ruviform-media-reader/tests/prefetch.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-244-media-reader-prefetch-contract-warts.md"
    - "docs/adr/issue-197-media-reader-boundary-additions.md"
    - "docs/adr/issue-233-prefetch-contract-improvements-doc-only-clarifications-consumer-drain-.md"
    - "docs/adr/realtime-contract-and-verification-policy.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-197-media-reader-prefetch-rt-consumer.md"
    - "docs/impl/ir/issue-197-media-reader-implementation-record.md"
    - "docs/impl/ir/issue-233-media-reader-prefetch-contract-improvements.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-244-media-reader-prefetch-contract-warts.md"
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

# Issue 244 prefetch contract warts: boundary-first fault emission + dead-generation sequence + End commit-once

## Context and Problem Statement

Issue #244 tracks the four residual warts the #233 review-protocol confirmed and deferred as #197 pre-existing contract semantics: (a) `seek`/`reopen_at` failure paths emit `Fault{gen=N}` *before* `Boundary{gen=N}`, so the consumer — which adopts a generation only at `Boundary` — stale-drops the fault invisibly instead of counting it; (b) the `reopen`/`reopen_at` failure arm emits no `Boundary` at all (`Fault` + `stream_ended`→`End` only), so the burned generation `N` is never adopted and the consumer starves in underrun at `N-1`; (c) `end_emitted` is set only on the direct-push path, so an `End` requeued through `pending` under backpressure is synthesized twice; (d) the `discard_pending` fault arm and `End`-then-`Boundary` sequencing lack coverage for want of a deterministic fault fixture.

The issue's explicit constraint: the two P2 items change *observable* #197 contract behavior, so they must not land silently — either they land as an unconditional contract amendment with consumer analysis + migration note, or as an opt-in/additive shape; this ADR records that decision.

## Decision Drivers

* The #197 intent (scope-4): every non-content span distinguishable — faults are *meant* to be counted; a fault that silently stale-drops defeats the classification the contract exists for.
* The boundary-driven adoption model is the invariant being restored, not redesigned — consumer `step`/`discard_pending` logic stays untouched.
* Issue requirement: no silent observable-behavior change; the handling must be explicit in the design record.
* Minimal coherent change: both P2 warts are one invariant violation — failure-path markers precede (or lack) the generation's `Boundary`; correcting emission order fixes both at the owning layer.
* An opt-in flag would preserve a wart with no legitimate consumer and double the contract's observable orderings — disproportionate machinery per the #233 ADR's additivity reasoning.
* Crates are `publish = false`, version `0.1.0` — there is no SemVer surface; "breaking" means observable runtime semantics, governed by the Issue #219 consumer-impact classification.

## Considered Options

* Option A — unconditional boundary-first correction: `Seek`/`Reopen{first:Some}` failure arms emit `Boundary` then `Fault`; `Reopen` `Err` arm emits `Boundary{gen,0}` → `Fault{gen}` → `End{gen}` (dead-generation sequence); `end_emitted` commit-once; seeded-fault coverage.
* Option B — opt-in flag (config field or per-command) selecting boundary-first emission.
* Option C — consumer-side: count pre-boundary generation-mismatched faults as real faults instead of stale drops; no producer change (does not address the missing-`Boundary` starvation at all).
* Option D — status quo: keep the warts documented; fix coverage only.

## Decision Outcome

Chosen option: "Option A", because both P2 warts are the same invariant violation — the producer must emit exactly one `Boundary` as the *first* item of every generation, with all subsequent generation-tagged items following it. Restoring that invariant at the emission layer makes fault observability work the way the #197 scope-4 contract always intended, gives a failed reopen a well-formed terminal generation (`Boundary` → `Fault` → `End`: adopted, counted, `eof`-reached instead of starved), and needs no new API. Option B is declined — two observable orderings permanently complicate the contract for a behavior no consumer should want; the wart exists because of emission order, not because silent faults are useful. Option C is declined — it conflates stale detection with fault reporting and cannot express the dead-generation fix. Option D is declined — the issue exists to fix these; they are confirmed real defects.

### Migration note (deliberate observable-behavior change)

This amendment changes observable contract *behavior*, not signatures — no type, method, field, or enum variant changes; consumer code compiles and runs unchanged. Observable deltas:

| Path | Before | After |
| --- | --- | --- |
| `seek`/`seek_discard_pending` rejected | `Fault` stale-dropped: `stale_drops` +1, `fault_events`/`faults.unseekable` unchanged | `fault_events` +1, `fill` bumps `faults.unseekable`; `stale_drops` unchanged by the fault |
| `reopen_at` post-reopen seek failure | same stale-drop shape | same correction — fault counted post-boundary, re-anchor at landed position unchanged |
| `reopen`/`reopen_at` open/reopen failure | `Fault`+`End` stale-dropped; consumer starves at gen `N-1` in underrun; `telemetry().generation()` stays `N-1` | `Boundary`→`Fault`→`End`: consumer adopts `N` (`position`→0), fault counted, `fill` reports `eof`; telemetry converges |
| `End` under backpressure | second `End{gen}` could be synthesized after the requeued copy | exactly one `End` per generation |

Consumer impact (Issue #219 classification): **Synvelis — `no impact`**: its quiesce-drain→seek workaround remains valid; failed seeks now *additionally* surface in `faults.unseekable`/`fault_events` instead of counting as `stale_drops` — strictly more diagnostic signal; no code change required. **AudioLens — `unknown`**: prefetch adoption is not verifiable from this repo; the delta is enumerated above for the reviewer gate (any AudioLens code asserting `stale_drops` on failure paths or depending on post-reopen-failure starvation would observe different counters; neither pattern is a reasonable contract use).

### Consequences

* Good, because fault observability now holds on the paths consumers need most — failed seeks/reopens are counted and classified, and a dead generation terminates observably (`eof`) instead of starving.
* Good, because the invariant "every generation begins with its `Boundary`" is simple, total across all arms, and removes the consumer's need to reason about pre-adoption items.
* Good, because `stale_drops` regains its intended meaning — only genuinely stale items count there.
* Bad, because observable counters shift for failure paths (`fault_events`/`faults` up, `stale_drops` down) — a deliberate, documented amendment; consumers asserting the old accounting would see different values (none known; enumerated in the migration note).
* Bad, because a failed reopen now *adopts* the dead generation — consumers that inferred "reopen failed" from `generation` staying low lose that indirect signal; replaced by strictly better direct signals (`Fault` + `eof`).

### Confirmation

Contract tests pin every amended path under the seeded-fault fixture (boundary-first fault observability on `seek`/`reopen_at`, generation adoption + `eof` on reopen failure, single-`End` under ring-full requeue, `discard_pending` fault arm and `End`-then-`Boundary` drain); `rd02_reopen_at_seek_failure_faults_then_reanchors` is updated to assert the corrected observable; `mise run fmt-check`/`check`/`lint`/`test` green; review-protocol validates the diff and this contract decision; migration note duplicated in the impl record and PR consumer-impact section.

## Implementation Plan

* Affected paths: `crates/ruviform-media-reader/src/prefetch.rs` (`exec_cmd` `Seek`/`Reopen` arm reorder, `Reopen` `Err` arm `Boundary`, `pump` `end_emitted` commit-once; module Contract + method rustdoc), `crates/ruviform-media-reader/README.md`, `crates/ruviform-media-reader/tests/prefetch.rs` (seeded-fault fixture + coverage), `docs/impl/ir/issue-244-*.md`.
* Patterns to follow: `Boundary{first_frame: reader.position().0}` honest-landing; `Fault{frames:0}` point events; `stream_ended` → lazy `End` synthesis for terminal generations; test-only fixtures colocated in `tests/prefetch.rs` (mirrors existing `NoSeek` wrappers).
* Patterns to avoid: new public API/flags, `PrefetchItem` variants, consumer-side logic edits, changes to `seek`/`reopen` signatures or success paths.
* Dependencies or configuration: none — no new crates, features, or tools.

## Verification

* [ ] `mise run test` — new contract tests pass; pre-existing suite green with only the `rd02_reopen_at_seek_failure` semantic update.
* [ ] `mise run lint` / `check` / `fmt-check` green.
* [ ] `mise run test-rt` — RT lane unaffected (no consumer logic changes).
* [ ] review-protocol verdict recorded in the impl record; doc audit + graph exit-audit pass.

## Pros and Cons of the Options

### Option A — unconditional boundary-first correction

* Good, because it fixes both P2 warts at the layer owning the invariant with the smallest coherent change (reorder + one extra `Boundary`); restores #197's own observability intent; dead generations become well-formed terminal sequences.
* Bad, because failure-path observable counters change — non-additive semantics requiring the migration note this ADR carries; `stale_drops`-on-fault assertions (one in-repo test) must be updated.

### Option B — opt-in flag

* Good, because default behavior is untouched.
* Bad, because it preserves a confirmed wart permanently, doubles the contract's observable orderings, and adds API surface nobody should select — the flag's "off" behavior is strictly worse diagnostics.

### Option C — consumer-side fault counting

* Good, because no producer change.
* Bad, because it conflates stale detection with fault reporting, cannot fix generation starvation (no `Boundary` is emitted to adopt), and violates boundary-driven adoption — the wrong layer entirely.

### Option D — status quo

* Good, because zero churn.
* Bad, because confirmed defects stay: failed seeks/reopens remain invisible to fault classification and failed reopens keep starving consumers — the issue exists to resolve exactly this.

## More Information

- Issue: https://github.com/scarecrow173/ruviform/issues/244 — residual warts; the #233 impl record Follow-ups section is their canonical description.
- Amended contract: `docs/specs/issue-197-media-reader-prefetch-rt-consumer.md`, `docs/adr/issue-197-media-reader-boundary-additions.md` — this ADR amends *failure-path emission order* only; all other #197/#233 semantics stand.
- Upstream: `docs/discovery/issue-244-media-reader-prefetch-contract-warts.md`, `docs/specs/issue-244-media-reader-prefetch-contract-warts.md`.
