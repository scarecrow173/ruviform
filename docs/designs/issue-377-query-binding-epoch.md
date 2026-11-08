---
id: "DESIGN-imYFBiewK6Cdfwg9TUbsx9"
type: "design"
status: "approved"
title: "Issue #377 設計 — registerQuery adopt-on-join"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-377-query-binding-epoch.md"
    - "https://github.com/scarecrow173/ruviform/issues/377"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-377-query-binding-epoch.md"
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
    - "docs/specs/issue-377-query-binding-epoch.md"
  implemented-by:
    - "docs/plans/issue-377-query-binding-epoch.md"
  depends-on:
    - "docs/specs/issue-377-query-binding-epoch.md"
    - "docs/specs/viewlink-authoritative-generation.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/viewlink-authoritative-generation.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-377-query-binding-epoch.md"
  derived-by:
    - "docs/plans/issue-377-query-binding-epoch.md"
    - "docs/test-specs/issue-377-query-binding-epoch.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-377-query-binding-epoch.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/376"
    - "https://github.com/scarecrow173/ruviform/issues/378"
  defers: []
  deferred-by: []
---

# Issue #377 設計 — registerQuery adopt-on-join

## Overview

`ViewLink.registerQuery` に adopt-on-join を追加し、attach 済み link へ
join する `QueryBinding` が `registerBinding` と同じ規則で現行 view epoch を
採用する。変更は `ui/binding/src/link.ts` の 1 分岐と doc comment、および
`contract.ts` / `query.ts` の契約 docstring の明確化に限定する。wire 型・
公開シグネチャ・reply echo / stale / revision guard は一切変更しない。

## 詳細

### 採用規則（`registerQuery`）

```ts
registerQuery(query: QueryBinding): () => void {
  this.queries.add(query);
  if (this.attached_) query.attach(this.generation_);
  // ... unregister は membership 解除のみ（従来どおり）
}
```

- `QueryBinding.attach(epoch)` は `generation_ = epoch` を代入してから
  `cancelAll()` する既存実装をそのまま使う。新規 query では `cancelAll` は
  no-op で、generation だけが link の epoch を採用する。
- attach 済み link へ **in-flight を抱えた外部 query** を登録した場合、
  join はその query にとって epoch 遷移であり、旧 epoch で mint された
  in-flight request は `cancelAll()` で cancelled になる。この意味論を
  doc comment に明文化する（`registerBinding` のコメントと同じ構造）。
- `createQuery` は `registerQuery` を経由するため、factory 経路・外部
  `registerQuery` 経路の両方が同一規則でカバーされる。

### 変更しない経路（対照の維持）

- **attach 前作成**: detached link への `registerQuery` では
  `attached_ === false` のため何も起きず、従来どおり次回 `attach()` の
  `for (const query of this.queries) query.attach(this.generation_)` で
  採用される（deferred join）。既存 caller（`replay.ts`・`consumers.ts`）は
  全てこの経路であり、挙動は不変。
- **detach/reattach**: `attach()` の既存伝搬経路が registered 全 query に
  新 epoch を採用させる（現行どおり）。
- **`unregister`**: `this.queries` からの membership 解除のみ。
  query には subscription が存在しないため解放対象は無い（binding の
  unregister が subscription `off` を畳むのとは構造が異なる）。
  この非対称を doc comment に明記する。
- **`viewId` の不整合**: `registerQuery` は query の `viewId` を link の
  ものへ書き換えない（`QueryBinding.viewId` は constructor 確定の
  readonly）。binding の `setContext` に相当する機構は query に存在せず、
  本 Issue の範囲外とする（既存挙動のまま）。

### docstring の明確化

- `link.ts`: `registerQuery` / `createQuery` の doc comment に adopt-on-join
  と「join = epoch 遷移（in-flight は旧 epoch で cancelAll）」の意味論を記す。
  module header の per-attach contract 記述は query 側も adopt-on-join を
  含む旨に整合させる。
- `contract.ts`: `ViewportQuery.generation` の doc に「link 管理下の query
  は所有 `ViewLink` の view epoch を継承する」旨を追記（wire 形状は不変）。
- `query.ts`: module header の generation 記述を adopt-on-join 前提に整合
  （挙動変更なし、記述の正確化のみ）。

## 検討した方法

1. **`registerQuery` で adopt-on-join（採用）** — `registerBinding` の
   #379 修復と同じ責務境界・同じ先例。`attached_` 判定→`attach(epoch)` の
   2 行で済み、epoch 遷移時の cancel/stale 意味論は `QueryBinding.attach`
   の既存実装がそのまま担保する。
2. `request()` 発行時に lazy adopt — 不採用。diagnostics の generation・
   in-flight の minted epoch が join と request の間で乖離し、遷移が
   観測不能な副作用になる。join 時点で epoch を確定する方が契約として
   明確。
3. Consumer 側 workaround / wire field 追加 — 不採用。Issue が明示的に
   禁止。SDK 境界内で完結する。

## Verification

- `ui/binding/test/link.test.ts` に恒久テストを追加し、
  `mise run test-ui-binding` で RED→GREEN を確認。
- `mise run check-ui-binding` / `build-ui-binding` green。
- `mise run test-ui-guitest` で既存 caller（`replay.ts` の attach 前作成
  経路）に回帰が無いことを確認。
- `docs/test-specs/issue-377-query-binding-epoch.md` の全ケースを満たす。
