---
id: "TASK-034QkFsRkLkoZqHkFaRHhA"
type: "task"
status: "done"
title: "plugin-core: param_write contract types + coalescing + same-offset docs"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
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
    - "docs/plans/issue-186-param-control-events.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-186-param-control-events.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-186-param-control-events.md"
  references: []
  defers: []
  deferred-by: []
---

# plugin-core: param_write contract types + coalescing + same-offset docs

## Work

`ruviform-plugin-core` に `param_write` モジュールを追加する。`ParamWriteMeans`（`TimedEvent`/`BlockBoundary`/`Immediate`/`PerNote`、#[non_exhaustive]）、`ParamWriteRejection`（`ReadOnly`/`NotAutomatable`/`UnsupportedMeans`/`UnsupportedPrecision`/`UnknownParam`/`NonFiniteValue`、#[non_exhaustive]）、`check_param_write(&ParamInfo, ParamWriteMeans) -> Result<(), ParamWriteRejection>`、opt-in `coalesce_param_events_latest`。`event.rs` に同一 offset 同一 param の安定順+last-wins 適用規約の doc を明文化する。`lib.rs` で公開。公開 API は doctest 付き。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-186-param-control-events.md`
- `docs/designs/issue-186-param-control-events-detailed.md`

## Dependencies

- なし（plugin-core 内部の追加のみ）

## Affected Files

- `crates/ruviform-plugin-core/src/param_write.rs`（新設）
- `crates/ruviform-plugin-core/src/event.rs`（doc 追加）
- `crates/ruviform-plugin-core/src/lib.rs`（公開）

## Done When

- [ ] `check_param_write` が design §1 の行列通りの判定を返す
- [ ] `coalesce_param_events_latest` が同一 param 同一 position 連続 ParamValue のみ latest-wins 圧縮する
- [ ] 同一 offset 規約が event.rs doc に記述される
- [ ] 全公開 enum が `#[non_exhaustive]`、公開 API に doctest

## Verification

- [ ] `mise run check` / `lint` / `test` green（param_write 単体テスト含む）
