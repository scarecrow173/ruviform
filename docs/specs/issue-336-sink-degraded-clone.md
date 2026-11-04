---
id: "SPEC-AJKrUcOGJ76vbIWIZ065rr"
type: "spec"
status: "approved"
title: "Issue #336 SinkDegraded への Clone derive 追加"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/336"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-observability/src/logging/build.rs"
      - type: "file"
        path: "crates/ruviform-observability/tests/logging.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "docs/plans/issue-336-sink-degraded-clone.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/foundation-observability-minimal-api.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-276-generic-logging-adapter.md"
  derived-by:
    - "docs/adr/issue-336-sink-degraded-clone-derive.md"
    - "docs/designs/issue-336-sink-degraded-clone.md"
    - "docs/plans/issue-336-sink-degraded-clone.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata: {}
---

# Issue #336 SinkDegraded への Clone derive 追加

## Problem

`ruviform-observability` の logging sink degraded 状態型 `SinkDegraded` が
`Clone` を derive していない。Synvelis #81 (SYN-OBS-01) の統合で、Consumer
Host が health/degraded 状態をスナップショットとして複製保持する際に
借用経路の工夫を強いられている（Synvelis 側は現状 workaround 済み・緊急度低）。

## Diagnosis（変更前に診断する）

`SinkDegraded` は `BestEffort` sink の初期化失敗を通知する純データ record
（`sink_index: usize` / `kind: SinkKind` / `reason: String`）であり、所有権
リソースを含まない。全フィールドが `Clone` 可能（`SinkKind` は `Copy`）。
同一分類の health/degraded 型 — `SinkState`・`SinkKind`・`SinkHealthSnapshot`・
`HealthSnapshot`・`LoggingHealth` — はすべて既に `Clone` を derive しており、
`SinkDegraded` だけが非 Clone という非一貫状態にある。非 Clone とする設計意図は
コード・docs ともに記録がなく、derive 漏れと判断できる。

## Requirements

1. `SinkDegraded` に `Clone` を derive する（必要最小限の derive のみ追加し、
   `PartialEq` 等の要求外 trait は付与しない）。
2. 関連する health/degraded 分類型の `Clone` 有無を確認し、一貫性を確認する
   （既存の Clone 型へは変更を加えない）。
3. `LoggingBuildResult` / `LoggingLayersResult` には `Clone` を付けない —
   `guards: Vec<WriterGuard>` が sink writer の shutdown handle を排他所有
   しており複製不能であり、この判断を design で明示する。
4. 後方互換性を壊さない（derive 追加は additive）。

## Acceptance Criteria

- AC1: `SinkDegraded` が `Clone` を実装し、`result.degraded.clone()` で
  フィールド値が保持された複製を取得できることをテストで示す。
- AC2: health/degraded 分類型の Clone 有無の確認結果と、`LoggingBuildResult`
  系を非 Clone に据え置く理由が design 文書に記録される。
- AC3: `mise run fmt` / `check` / `lint` / `test` が green。

## Non-goals

- `Clone` 以外の trait derive（`PartialEq`・`Eq`・`Hash`・`serde` 等）。
- `LoggingBuildResult` / `LoggingLayersResult` / `LoggingBuildError` 等の
  Clone 化。`WriterGuard` は shutdown 所有権を持つため複製不可。
- Synvelis 側 workaround の撤去（Consumer 側の責務・別 Issue）。
