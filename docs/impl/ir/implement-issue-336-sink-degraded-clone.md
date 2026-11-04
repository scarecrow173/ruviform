---
id: "IMPL-yMfm4VQypUJDoAtkXVsGna"
type: "impl"
status: "completed"
title: "SinkDegraded Clone derive 追加 (Issue #336)"
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
  implements:
    - "docs/tasks/issue-336-t1-sink-degraded-clone.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-336-t1-sink-degraded-clone.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-336-sink-degraded-clone.md"
    - "docs/adr/issue-336-sink-degraded-clone-derive.md"
    - "docs/designs/issue-336-sink-degraded-clone.md"
    - "docs/plans/issue-336-sink-degraded-clone.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# SinkDegraded Clone derive 追加 (Issue #336)

## Summary

`crates/ruviform-observability/src/logging/build.rs` の `SinkDegraded` に
`Clone` を derive(`#[derive(Debug)]` → `#[derive(Debug, Clone)]`)し、
`tests/logging.rs` に clone 複製がフィールド値を保持する
`sink_degraded_record_is_cloneable` テストを追加した。付与した trait は
ADR 決定どおり `Clone` のみ(比較系・直列化 trait は要求外)。

## Context

- 実施タスク: docs/tasks/issue-336-t1-sink-degraded-clone.md
- 上流: spec(要求・AC)・ADR(`Clone` のみ付与の決定)・design(関連
  health/degraded 型の Clone 有無確認結果・非 Clone 据え置き境界)
- 動機: Synvelis SYN-OBS-01 統合で Host が health/degraded 状態を
  スナップショット複製保持したい場面で借用経路の workaround を要した。

## Implementation

### Changes

- `src/logging/build.rs` — `SinkDegraded` の derive に `Clone` を追加し、
  Clone 付与理由(Host のスナップショット保持・health 型群との一貫)を
  doc comment に記した。
- `tests/logging.rs` — E4c として `sink_degraded_record_is_cloneable` を
  追加: BestEffort sink 失敗で degraded record を生成し、
  `result.degraded.clone()` の複製が `sink_index` / `kind` / `reason` を
  保持することを検証。

### Decisions During Implementation

- `LoggingBuildResult` / `LoggingLayersResult` は非 Clone 維持:
  `guards: Vec<WriterGuard>` が sink writer shutdown handle を排他所有し
  複製不能(design の境界判断どおり)。Host は `result.degraded.clone()` で
  `Vec<SinkDegraded>` を複製する。
- `PartialEq` / `Eq` / `Hash` / `serde` は付与しない(「必要最小限の
  derive」方針)。将来必要になれば additive に追加可能。
- `SinkFailure` / `LoggingBuildError` 等の error 型は Issue 範囲外で
  変更しない。

## Related Experiments

### Adopted

なし(実験ログは作成していない)。

### Rejected

なし。

## Validation

| 検証 | 実行 | 結果 |
| --- | --- | --- |
| `mise run fmt` (`cargo fmt --all`) | 実行 | Pass — diff なし |
| `mise run check` (`cargo check --workspace --all-targets`) | 実行 | Pass |
| `mise run lint` (`cargo clippy --workspace --all-targets -- -D warnings`) | 実行 | Pass |
| `mise run test` (`cargo test --workspace`) | 実行 | Pass — 全 test target 緑 |
| `cargo test -p ruviform-observability --features logging` | 実行 | Pass — 28 pass(新規 `sink_degraded_record_is_cloneable` 含む) + doctest 5 |
| `cargo clippy -p ruviform-observability --all-targets --features logging -- -D warnings` | 実行 | Pass |

`logging` feature は既定 off・CI lane なしのため feature 有効検証は
ローカル実施(#334/#335 と同じ構成) — canonical = mise/CI ではない補助証拠。

## Risks

- 残リスクなし。derive 追加は additive で後方互換を壊さない。
- Rollback: `#[derive(Debug, Clone)]` を `#[derive(Debug)]` に戻し追加
  テストを除去すれば完全に元に戻る。

## Follow-ups

- `logging` feature の CI lane 不在は本 Issue 範囲外(#276 系の既存構成)。
  feature 有効の自動検証を CI に載せるかは別途検討事項。
