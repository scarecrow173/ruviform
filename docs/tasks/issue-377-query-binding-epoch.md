---
id: "TASK-1EfMLBvEIXGocxCFFgixSj"
type: "task"
status: "done"
title: "Issue #377 — QueryBinding adopt-on-join 実装と恒久テスト"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/plans/issue-377-query-binding-epoch.md"
    - "https://github.com/scarecrow173/ruviform/issues/377"
  changes:
    added: []
    modified:
      - type: "file"
        path: "ui/binding/src/link.ts"
      - type: "file"
        path: "ui/binding/src/query.ts"
      - type: "file"
        path: "ui/binding/src/contract.ts"
      - type: "file"
        path: "ui/binding/test/link.test.ts"
      - type: "file"
        path: "docs/specs/viewlink-authoritative-generation.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-377-query-binding-epoch.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-377-query-binding-epoch.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-377-query-binding-epoch.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-377-query-binding-epoch.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-377-query-binding-epoch.md"
  verified-by:
    - "docs/test-specs/issue-377-query-binding-epoch.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #377 — QueryBinding adopt-on-join

## Work

`ViewLink.registerQuery` に `if (this.attached_) query.attach(this.generation_)`
を追加し、attach 済み link への新規 query 登録が `registerBinding` と同じ
adopt-on-join 規則で現行 view epoch を継承するようにする。join = epoch 遷移
（in-flight は旧 epoch で `cancelAll()`）と unregister = membership 解除のみ
の意味論を doc comment に明文化し、`link.ts` / `query.ts` / `contract.ts` の
docstring を整合させる。wire 型・公開シグネチャ・echo/stale guard は不変。
`ui/guitest/src/oracle.ts` の `addQuery` 同型乖離は #378 管轄のため触らず、
PR 本文に記録する。

## Acceptance

- [x] test-spec 全ケースの恒久テストが RED→GREEN で検証される
      （RED: 10 fail / GREEN: `test-ui-binding` 99 pass）
- [x] `mise run check-ui-binding` / `test-ui-binding` / `build-ui-binding` green
- [x] `mise run check-ui-guitest` / `test-ui-guitest` green（attach 前作成の
      既存 caller 経路に回帰なし — 135 pass）
- [x] `mise run check-ui-showcase` / `test-ui-showcase` green（73 pass）
- [x] `mise run fmt-check` green
- [x] review-protocol レビューで blocker なし（ACCEPT — P0/P1 指摘なし）
