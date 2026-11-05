---
id: "PLAN-m7f29Op1o9KCQ037OSq6i8"
type: "plan"
status: "approved"
title: "Implement Issue #280 OBS-05 RT record bridge・record vocabulary"
created: "2026-11-05"
updated: "2026-11-05"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-280-rt-record-bridge-clock-correlation.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-280-rt-record-bridge-clock-correlation.md"
    - "docs/designs/issue-280-rt-record-bridge-clock-correlation.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-280-rt-record-bridge-tests.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/289"
    - "https://github.com/scarecrow173/ruviform/issues/278"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/289"
    - "https://github.com/scarecrow173/ruviform/issues/278"
  deferred-by: []
---

# Implement Issue #280 OBS-05 RT record bridge Implementation Plan

## Goal

design `DESIGN-fhvRxnPJKvnhRAlg7TbeKm` どおりに `ruviform-observability` へ
`record`（ClockContext・Correlation・EventRecord）と `rt_bridge`
（bridge/RtRecorder/RecordSink/ReceivedRecord）を実装し、spec AC-1〜AC-8 を
`mise` 検証と `rt_` 契約試験で証明する。

## Scope

1 本の実装ストリーム（record 語彙 → rt_bridge 輸送 → README/crate doc/統合試験）。
実 Collector・Registry・`tracing` export・`ruviform-rt` 側変更・MPMC・
外部依存追加は含まない。

## Upstream Documents

- `docs/specs/issue-280-rt-record-bridge-clock-correlation.md`（AC-1〜AC-8）
- Issue #280（完了条件の正本）・親 Issue #275（統合設計正本）

## Design Inputs

- `docs/designs/issue-280-rt-record-bridge-clock-correlation.md`

## File Map

- `crates/ruviform-observability/src/record/mod.rs` — re-export
- `crates/ruviform-observability/src/record/clock.rs` — `ClockContext`
- `crates/ruviform-observability/src/record/correlation.rs` — `CorrelationToken`/
  `CorrelationLink`/`CorrelationBook`/`CorrelationStatus`/`UnlinkedReason`
- `crates/ruviform-observability/src/record/event.rs` — `EventId`/`RecordHeader`/
  `EventRecord`
- `crates/ruviform-observability/src/rt_bridge.rs` — `bridge`/`RtRecorder`/
  `RecordSink`/`ReceivedRecord`（private `Envelope`）
- `crates/ruviform-observability/src/lib.rs` — `pub mod record`/`pub mod rt_bridge`
  追加・crate doc・依存方針節
- `crates/ruviform-observability/Cargo.toml` — `ruviform-rt`・`ruviform-media-core`
  path 依存追加（`dev-dependencies` の `assert_no_alloc` は維持）
- `crates/ruviform-observability/README.md` — 責務・RT 契約表・依存方針差分
- `crates/README.md` — `ruviform-observability` 行の更新

## Implementation Sequence

- [ ] T-1: `record` モジュール（clock/correlation/event）+ `Cargo.toml` に
  `ruviform-media-core` 依存追加 + `rt_`/golden 試験
- [ ] T-2: `rt_bridge` モジュール + `Cargo.toml` に `ruviform-rt` 依存追加 +
  FIFO/drop/lag/quiescence/OBS-02 寿命統合試験 + README/crates/README/lib.rs

## Dependencies and Risks

- `ruviform-rt`・`ruviform-media-core` の公開 API は安定利用を前提（変更しない）。
- OBS-02 AC-6 の「他 ruviform クレートへ依存しない」を本 Issue 限りで
  「葉 primitive への一方向依存のみ許可」へ改訂する点は設計・README に明記済み。
- #289（OBS-07）・#278（OBS-03）の実装は後続 Issue。本実装は
  `drain`/`drain_final`/`ReceivedRecord`/`RecordHeader`/`CounterReader` を
  それらがそのまま消費できる形に留める。
- 他 worker が `ruviform-observability`/`ruviform-rt`/`ruviform-media-core` を
  編集中の可能性 → 最終検証前に `git merge origin/main` で統合する。

## Task Breakdown

- `docs/tasks/p280-t1-record-vocabulary-clock-correlation-event.md`
- `docs/tasks/p280-t2-rt-bridge-transport-drop-loss-lifecycle.md`

## Verification

- [ ] `mise run fmt-check` / `mise run check` / `mise run lint` / `mise run test` /
  `mise run test-rt` が全緑
- [ ] `rt_` 契約試験が no-alloc・bounded latency を pin
- [ ] `git merge origin/main` 後の最終検証（共有ファイル競合の統合確認）

## Open Questions / Gaps

- なし（Issue 本文・#275・#101 政策・OBS-02 契約との整合は設計で確定済み）。
