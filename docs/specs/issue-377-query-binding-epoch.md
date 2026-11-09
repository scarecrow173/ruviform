---
id: "SPEC-jXnvZdJj5kDVqJywcaAgiy"
type: "spec"
status: "approved"
title: "Issue #377 — live ViewLink の新規 QueryBinding epoch 契約整合"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/377"
    - "docs/specs/viewlink-authoritative-generation.md"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-377-query-binding-epoch.md"
      - type: "file"
        path: "docs/designs/issue-377-query-binding-epoch.md"
      - type: "file"
        path: "docs/plans/issue-377-query-binding-epoch.md"
      - type: "file"
        path: "docs/test-specs/issue-377-query-binding-epoch.md"
      - type: "file"
        path: "docs/tasks/issue-377-query-binding-epoch.md"
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
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "docs/designs/issue-377-query-binding-epoch.md"
    - "docs/plans/issue-377-query-binding-epoch.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-181-multi-window.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/viewlink-authoritative-generation.md"
  derived-by:
    - "docs/designs/issue-377-query-binding-epoch.md"
    - "docs/plans/issue-377-query-binding-epoch.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-377-query-binding-epoch.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/376"
    - "https://github.com/scarecrow173/ruviform/issues/378"
  defers: []
  deferred-by: []
---

# Issue #377 — live ViewLink の新規 QueryBinding epoch 契約整合

## 背景

`ViewLink.createQuery()` / `registerQuery()` は、link が既に attach 済みでも
`QueryBinding` へ現行 view epoch を採用させない。attach 前に作成された query は
`attach()` の `for (const query of this.queries) query.attach(this.generation_)`
で正しく採用されるが、attach 後に作成・登録された query の `generation` は 0 の
まま残り、request が旧 epoch token を mint する。

対照的に `registerBinding` は #376 / #379 で adopt-on-join が修復済み:
`if (this.attached_) { binding.attach(this.generation_); off = this.subscribeBinding(binding); }`。
本 Issue は query 側の同型乖離を、同じ責務境界（`ViewLink` が view epoch の
owner）で整合させる。基礎契約は #261、authoritative epoch seam は
`docs/specs/viewlink-authoritative-generation.md`（#383）に由来する。

## 要求

- attach 前作成の対照動作を維持しつつ、初回 attach 後・Dock 移動後・
  detach/reattach 後に新規作成・登録された query が意図した view epoch を
  継承する契約を明確化する。
- `registerBinding` の adopt-on-join 先例に倣う:
  `if (this.attached_) query.attach(this.generation_)`。
  join は epoch 遷移であり、query が抱える in-flight request は旧 epoch で
  `cancelAll()` される意味論を doc comment に明文化する。
- request generation・query diagnostics・返信 echo/correlation・
  cancel/stale reply を恒久テストで RED→GREEN 検証する。
- `registerQuery` の membership/deferred-join との関係を整理する
  （unregister は membership 解除のみ。query に subscription は存在しない）。
- 変更は SDK 境界（`ui/binding`）に置き、既存 reply echo・stale/revision
  guard を弱めない。wire 型の変更は不要。

## 非目標

- `ui/guitest/src/oracle.ts` の `addQuery()` にある同型の epoch 不採用
  （#378 と同型の oracle 乖離）。本 Issue では実装せず、PR 本文に
  「#378 で扱うべき副次的発見」として記録するのみ。
- Consumer 側の恒久回避策、未設計の wire field 追加。
- 即時 subscription 等、query に無関係な契約の追加。
- `crates/`（Rust 側）、`Cargo.*`、`.github/` の変更。

## 受入条件（Issue #377 本文に従う）

| ID | 条件 |
| --- | --- |
| A1 | attach 前作成の対照を維持し、初回 attach 後・Dock 移動後・detach/reattach 後の新規 query が意図した view epoch を継承する契約を明確化する |
| A2 | request generation・query diagnostics・返信 echo/correlation・cancel/stale reply を恒久テストで RED→GREEN 検証する（初回 attach / Dock / reattach のパラメタ化 + authoritative epoch + createQuery/registerQuery 両経路 + echo 解決 + epoch 遷移時 cancel/stale） |
| A3 | 変更は SDK 境界に置き、既存 reply echo・stale/revision guard を弱めない。wire 型は変更不要 |
| A4 | `registerQuery` の membership/deferred-join との関係を整理する。無関係な契約は追加しない |
