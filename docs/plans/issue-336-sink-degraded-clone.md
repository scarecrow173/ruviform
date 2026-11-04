---
id: "PLAN-psvzdxaW4MHzk7mhctaYxG"
type: "plan"
status: "approved"
test-spec-skip: "derive 追加のみの additive 変更 — test-spec は作らず、clone 正当性を tests/logging.rs の sink_degraded_record_is_cloneable で直接検証する"
title: "Issue #336 SinkDegraded Clone 実装計画"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/336"
    - "docs/specs/issue-336-sink-degraded-clone.md"
    - "docs/designs/issue-336-sink-degraded-clone.md"
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
    - "docs/specs/issue-336-sink-degraded-clone.md"
    - "docs/designs/issue-336-sink-degraded-clone.md"
  implemented-by:
    - "docs/tasks/issue-336-t1-sink-degraded-clone.md"
  depends-on:
    - "docs/designs/issue-336-sink-degraded-clone.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/issue-336-sink-degraded-clone-derive.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-336-sink-degraded-clone.md"
    - "docs/designs/issue-336-sink-degraded-clone.md"
  derived-by:
    - "docs/tasks/issue-336-t1-sink-degraded-clone.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata: {}
---

# Issue #336 SinkDegraded Clone 実装計画

## Scope split

| Task | 内容 | 依存 |
| --- | --- | --- |
| T1 | `SinkDegraded` へ `Clone` derive 追加 + Clone 正当性テスト + 検証 | なし |

単一タスクの小粒変更のため T1 のみで構成する。

## File map

- `crates/ruviform-observability/src/logging/build.rs`: `SinkDegraded` の
  `#[derive(Debug)]` を `#[derive(Debug, Clone)]` に変更（1 行）。
- `crates/ruviform-observability/tests/logging.rs`: BestEffort degraded 系
  テストに、`degraded` record の `clone()` がフィールド値を保持する
  アサーションを追加。

## 依存グラフ

T1 のみ（依存なし）。

## Risk register

| リスク | 影響 | 対策 |
| --- | --- | --- |
| `Clone` 以外の trait まで追加して Issue 範囲を超える | API 表面の意図しない拡大 | `Clone` のみ追加。比較系 trait は付与しない |
| `LoggingBuildResult` まで Clone 化したくなる誘惑 | shutdown 所有権契約の破壊 | design の非 Clone 判断どおり据え置き |

## Rollback

`#[derive(Debug, Clone)]` を `#[derive(Debug)]` に戻し、追加テストを除去
すれば完全に元に戻る（additive 変更のため下位互換リスクなし）。

## Verification matrix

| 検証 | 手段 | 基準 |
| --- | --- | --- |
| Clone 正当性 | `tests/logging.rs` の clone アサーション | clone 複製が sink_index/kind/reason を保持 |
| 静的検証 | `mise run fmt` / `check` / `lint` | 全緑 |
| 回帰 | `mise run test` + `cargo test -p ruviform-observability --features logging` | 全緑 |

（derive 追加のみの additive 変更のため test-spec は作成しない —
`test-spec-skip` 記録どおり、clone 正当性を直接テストで検証する）
