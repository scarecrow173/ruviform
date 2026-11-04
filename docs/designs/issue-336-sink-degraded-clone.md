---
id: "DESIGN-elUvrfyRcsMrmrESYqsxDn"
type: "design"
status: "approved"
title: "Issue #336 SinkDegraded Clone 設計"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/336"
    - "docs/specs/issue-336-sink-degraded-clone.md"
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
    - "docs/adr/issue-336-sink-degraded-clone-derive.md"
    - "docs/designs/foundation-observability-minimal-api.md"
    - "docs/specs/issue-276-generic-logging-adapter.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-336-sink-degraded-clone.md"
    - "docs/adr/issue-336-sink-degraded-clone-derive.md"
  derived-by:
    - "docs/plans/issue-336-sink-degraded-clone.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata: {}
---

# Issue #336 SinkDegraded Clone 設計

## Context

`SinkDegraded`（`crates/ruviform-observability/src/logging/build.rs`）は
`BestEffort` sink 初期化失敗を Host へ通知する純データ record:

```rust
#[derive(Debug)]
pub struct SinkDegraded {
    pub sink_index: usize,
    pub kind: SinkKind,
    pub reason: String,
}
```

Synvelis SYN-OBS-01 の統合で、Consumer Host が health/degraded 状態を
スナップショットとして複製保持したい場面で `Clone` 不在が借用経路の
工夫を強いることが判明した（Issue #336）。

## 関連型の Clone 有無（確認結果）

health/degraded 分類型の `Clone` 有無を `src/logging/` 全型で確認した結果:

| 型 | 所在 | Clone | 備考 |
| --- | --- | --- | --- |
| `SinkDegraded` | `logging/build.rs` | **なし（本 Issue で追加）** | 全フィールド複製可能 |
| `SinkState` | `logging/health.rs` | `Clone, Copy` | `#[non_exhaustive]` enum |
| `SinkKind` | `logging/health.rs` | `Clone, Copy` | `#[non_exhaustive]` enum |
| `SinkHealthSnapshot` | `logging/health.rs` | `Clone` | snapshot record |
| `HealthSnapshot` | `logging/health.rs` | `Clone` | snapshot record |
| `LoggingHealth` | `logging/health.rs` | `Clone` | cloneable handle（Arc 共有） |
| `LoggingBuildResult` | `logging/build.rs` | なし（据え置き） | `guards` が shutdown 排他所有 |
| `LoggingLayersResult` | `logging/build.rs` | なし（据え置き） | 同上 + `layers` 複製不可 |
| `SinkFailure` / `LoggingBuildError` | `logging/error.rs` | なし（据え置き） | error 型・要求外 |

`SinkDegraded` のみ非 Clone という非一貫状態であり、観測 snapshot 系の
型分類では `HealthSnapshot` 系と同じ「値として複製できる record」が自然。

## 設計判断

1. `SinkDegraded` に `Clone` のみ追加する（`#[derive(Debug, Clone)]`）。
   `PartialEq` / `Eq` / `Hash` / `serde` は要求外のため付与しない —
   「必要最小限の derive」方針（ADR 参照）。将来必要になれば additive に
   追加可能。
2. `LoggingBuildResult` / `LoggingLayersResult` は非 Clone を維持する。
   `guards: Vec<WriterGuard>` は各 sink writer thread の shutdown handle を
   排他所有しており、複製すると同一 writer に対する shutdown 責務が
   二重化して契約を破る。`layers`（`Box<dyn Layer>`）も複製不可。
   Host が degraded record を保持したい場合は `result.degraded.clone()`
   で `Vec<SinkDegraded>` を複製する — 本変更でそれが可能になる。
3. `SinkFailure` / `LoggingBuildError` 等の error 型は Issue 範囲外で
   変更しない（失敗時のみ発生する所有権移転経路であり snapshot 用途がない）。

## 境界と互換性

- derive 追加は additive のみ。既存 API・挙動の変更なし。後方互換を壊さない。
- 公開 enum への影響なし（`SinkDegraded` は struct であり
  `#[non_exhaustive]` ポリシー対象外 — ADR `public-enum-non-exhaustive-policy`
  の対象はクレート外到達可能な公開 enum のみ）。
- RT 契約への影響なし（observability crate は非 RT 経路）。

## 検証

- `tests/logging.rs` に `result.degraded.clone()` の複製がフィールド値を
  保持することを示す最小テストを追加する（Clone 正当性の実証）。
- `mise run fmt` / `check` / `lint` / `test` が green。
