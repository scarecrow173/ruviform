---
id: "TSPEC-qAlLq79jiKf7NgITUKo6as"
type: "test-spec"
status: "approved"
title: "Issue #377 QueryBinding adopt-on-join テスト仕様"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/designs/issue-377-query-binding-epoch.md"
    - "docs/plans/issue-377-query-binding-epoch.md"
  changes:
    added: []
    modified:
      - type: "file"
        path: "ui/binding/test/link.test.ts"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/designs/issue-377-query-binding-epoch.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-377-query-binding-epoch.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-377-query-binding-epoch.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-377-query-binding-epoch.md"
    - "docs/designs/issue-377-query-binding-epoch.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #377 QueryBinding adopt-on-join テスト仕様

`ui/binding/test/link.test.ts` に追加する恒久テスト
（node --test + `MockAnalysisBackend`）。既存の `lateCases` パターンに倣い、
attach タイミングをパラメタ化する。

## Feature: late-created query の epoch 継承

attach ケース（`attach` 関数・期待 generation・期待 windowId）:

- 初回 attach: `attach("window-A")` → generation 1
- Dock 移動後: `attach("window-A")` → `attach("window-B")` → generation 2
- detach/reattach 後: `attach("window-A")` → `detach("unmount")` →
  `attach("window-A")` → generation 2

各ケースで:

- `link.createQuery()` が返す query の `generation` と
  `diagnostics.generation` が link の現行 epoch と一致する。
- `link.registerQuery(new QueryBinding(...))` の外部登録経路でも同じ
  epoch を採用する。
- `request()` が mint する `ViewportQuery.generation` が link epoch と一致し、
  正しい echo の返信が `ok: true` で解決する（`diagnostics.resolved` 加算）。
- 返信 echo の generation が要求と異なる場合 `stale` で discard される
  （`diagnostics.stale` 加算）。既存 guard の非弱化を固定。

## Feature: authoritative epoch

- `attach("window-A", 7)` 後の `createQuery` / `registerQuery` が
  generation 7 を採用し、request の `generation` も 7 を mint する。

## Feature: epoch 遷移と join の cancel 意味論

- late-created query の in-flight request 中に `link.attach` で epoch が
  遷移すると、旧 epoch の request は cancelled になり
  `backend.cancelledQueries` に記録される。遷移後の新規 request は
  新 epoch を mint して解決できる。
- attach 済み link へ in-flight を抱えた外部 `QueryBinding` を
  `registerQuery` すると、join = epoch 遷移として in-flight が旧 epoch で
  cancel され、query は link epoch を採用する。
- detached link への `registerQuery`（deferred join）では採用は次回
  `attach()` まで延期され、attach 時に link epoch が採用される。

## Feature: 対照の維持（attach 前作成）

- 既存テスト（`queries mint and replay replies under the adopted epoch` 等）
  が attach 前作成経路を引き続き検証する。本変更でそれらが壊れないこと。
