---
id: "TASK-PUEIOL8kRSg2p5kKdPYMUK"
type: "task"
status: "done"
title: "T3 end-to-end 否定行列・doc/台帳・最終検証 (Issue #304)"
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
    - "docs/plans/issue-304-io-archive-extract.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-304-io-archive-extract.md"
    - "docs/tasks/issue-304-t1-path-policy.md"
    - "docs/tasks/issue-304-t2-staging-backend.md"
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
    - "docs/test-specs/issue-304-io-archive-extract.md"
  references: []
  defers: []
  deferred-by: []
---

# T3 end-to-end 否定行列・doc/台帳・最終検証 (Issue #304)

## Work

`tests/extract_secure.rs` で crafted ZIP/TAR の end-to-end 試験
（Zip Slip・collision・kind/budget・取消・writer error・round-trip・
`tar.gz` 2 層分離）を実装し、crate doc（`src/lib.rs`・`extract`
module doc の脅威モデル）・`crates/ruviform-io-archive/README.md`・
ルート `crates/README.md` 台帳を更新する。`git fetch origin` →
`git merge origin/main` で最新 main を取り込み、`mise run fmt`/
`check`/`lint`/`test` を workspace 全体で緑にする。既存 ZIP/TAR
テストの非退行・`default=[]`・`unsafe` 局所許可のみ・backend 型
非露出を確認し、docs/impl の IR を記録してコミットする。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-304-io-archive-extract.md`
- `docs/designs/issue-304-io-archive-extract.md`

## Acceptance

- `extract_secure` の否定行列（slip・collision・kind・budget・取消・
  writer error）と正常系 round-trip が tests/ で緑。
- rustdoc/README に Path Policy・脅威モデル・OS 別保証・
  `ExtractedTree` 非不変性・Source 改変 race の保証外が明記。
- 最終検証 4 lane が緑で、既存 ZIP/TAR テストに退行がない。
