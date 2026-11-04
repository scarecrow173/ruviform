---
id: "ADR-jD7nl5tHt9wQ3G5Xm5YSyC"
type: "adr"
status: "accepted"
title: "Issue #336 SinkDegraded Clone 方針 — snapshot record は Clone 可能に揃える"
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
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-336-sink-degraded-clone.md"
    - "docs/designs/foundation-observability-minimal-api.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-336-sink-degraded-clone.md"
  derived-by:
    - "docs/designs/issue-336-sink-degraded-clone.md"
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

# Issue #336 SinkDegraded Clone 方針 — snapshot record は Clone 可能に揃える

## Context and Problem Statement

`SinkDegraded`（`crates/ruviform-observability/src/logging/build.rs`）は
`BestEffort` sink 初期化失敗を Host へ通知する純データ record で、`Clone` を
derive していない。Consumer Host（Synvelis SYN-OBS-01）が health/degraded
状態をスナップショット保持したい場面で借用経路を強いられる。一方、同一分類の
health/degraded 型（`SinkState`・`SinkKind`・`SinkHealthSnapshot`・
`HealthSnapshot`・`LoggingHealth`）はすべて `Clone` 済みで、`SinkDegraded`
のみ非 Clone という非一貫状態。

## Decision Drivers

- 観測用 snapshot 型は値として複製できるのが自然（`HealthSnapshot` 系と同じ
  利用形態）。
- `SinkDegraded` の全フィールド（`usize` / `SinkKind`(`Copy`) / `String`）は
  所有権リソースを含まず複製安全。
- 公開 enum 方針（`#[non_exhaustive]`）と同列の API 利便性ポリシーとの一貫性。
- derive 追加は additive で後方互換を壊さない。

## Considered Options

- `SinkDegraded` に `Clone` のみ derive する。
- `Clone` に加えて `PartialEq` / `Eq` 等の比較系 trait も併せて derive する。
- `Clone` を付けず、非 Clone の設計理由を doc/docs に明記する。

## Decision Outcome

`SinkDegraded` に `Clone` のみ derive する（Option 1 を採用）。

理由:

- 非 Clone とすべき設計上の根拠（排他所有・単一インスタンス契約・内部不変量の
  複製耐性問題）が存在しない。非 Clone のまま記録する Option 3 は、理由が
  「derive 漏れ」にすぎず文書化する不変条件がないため不適。
- `PartialEq` / `Eq` は現行の要求（スナップショット複製）に不要であり、
  「必要最小限の derive」の Issue 方針どおり見送る。将来必要になった時点で
  additive に追加できる。

## Consequences

- `result.degraded.clone()` で Host が degraded record を値として保持できる。
  Synvelis 側 workaround は任意のタイミングで撤去可能（強制ではない）。
- `LoggingBuildResult` / `LoggingLayersResult` は引き続き非 Clone —
  `guards: Vec<WriterGuard>` が sink writer の shutdown handle を排他所有
  しており複製は shutdown 契約を破る。この判断は design 文書で明示する。
