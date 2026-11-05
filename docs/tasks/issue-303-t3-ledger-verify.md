---
id: "TASK-9zUBxcJWsJbiJzI8S143vV"
type: "task"
status: "done"
title: "T3 crate doc/README・台帳・main 取込・最終検証 (Issue #303)"
created: "2026-11-05"
updated: "2026-11-05"
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
    - "docs/plans/issue-303-io-archive-tar.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-303-io-archive-tar.md"
    - "docs/tasks/issue-303-t1-tar-reader.md"
    - "docs/tasks/issue-303-t2-tar-writer.md"
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
    - "docs/test-specs/issue-303-io-archive-tar.md"
  references: []
  defers: []
  deferred-by: []
---

# T3 crate doc/README・台帳・main 取込・最終検証 (Issue #303)

## Work

crate doc（`src/lib.rs` の TAR 節・脅威境界・2 層 EOF 分離）・
`crates/ruviform-io-archive/README.md`・ルート `crates/README.md`
台帳を更新する。`git fetch origin` → `git merge origin/main` で最新
main を取り込み、`mise run fmt` / `check` / `lint` / `test` を
workspace 全体で緑にする。ZIP 側既存テストの非退行と `default=[]`
・`unsafe` なし・backend 型非露出を確認し、docs/impl の IR を記録
してコミットする。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-303-io-archive-tar.md`
- `docs/designs/issue-303-io-archive-tar.md`

## Acceptance

- rustdoc/README に TAR の状態機械・Pending/discard・`NotPresent`
  意味・`tar.gz` 2 層分離・脅威境界が明記されている。
- 最終検証 4 lane が緑で、ZIP 既存テストに退行がない。
