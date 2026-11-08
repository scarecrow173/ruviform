---
id: "SPEC-BXaMmK5e7X2mLZAG4xFeZv"
type: "spec"
status: "approved"
title: "ViewLink authoritative generation 採用 — backend attach応答epochのSDK seam"
created: "2026-11-07"
updated: "2026-11-07"
owners: []
relations:
  source:
    - "docs/superpowers/specs/2026-11-07-viewlink-authoritative-generation-design.md"
    - "https://github.com/scarecrow173/synvelis/pull/171"
  changes:
    added: []
    modified:
      - type: "file"
        path: "ui/binding/src/link.ts"
      - type: "file"
        path: "ui/binding/test/link.test.ts"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "docs/designs/viewlink-authoritative-generation.md"
    - "docs/plans/viewlink-authoritative-generation.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-181-multi-window.md"
    - "docs/test-specs/issue-181-multi-window.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/superpowers/specs/2026-11-07-viewlink-authoritative-generation-design.md"
  derived-by:
    - "docs/designs/viewlink-authoritative-generation.md"
    - "docs/plans/viewlink-authoritative-generation.md"
    - "docs/specs/issue-377-query-binding-epoch.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/376"
    - "https://github.com/scarecrow173/ruviform/issues/377"
    - "https://github.com/scarecrow173/ruviform/issues/378"
  defers: []
  deferred-by: []
---

# ViewLink authoritative generation 採用

## 背景

Synvelisのview remount後、frontendがlink-localなgeneration 1をdispatchへ載せる一方、backendは同じ`viewId`のattach epoch 2以上を要求し、操作をreceiptなしで破棄するP1。view generationのauthoritative sourceはbackendのattach応答であり、その値をRuviform `ViewLink`へ渡すSDK seamが必要。

## 要求

- `ViewLink.attach(windowId, generation?)` でbackend attach応答のauthoritative generationを採用できる。
- 指定されたgenerationがlinkのgeneration getter、CommandBinding/QueryBindingのattach epoch、subscription contextへ一貫して伝わる。
- 同window・異epochのattachは新epoch遷移（cancel・再購読・binding再attach）、同window・同値は冪等no-op。
- generation省略の既存callerはlocal increment動作を維持（source compatibility）。
- 無効なgeneration（非整数・負数）は受理せず、link状態を部分変更しない。

## 非目標

- backend stale-generation guardの緩和。
- Issue #377（QueryBinding late-create）、#378（guitest oracle変更）の無条件混入。
- Synvelis consumer側の変更（別repo・別dispatch、本計画Task 2）。

## 受入条件

- attach応答generationがwire `ctx.generation`（dispatch/subscribe/query）へ一致して届く。
- 同window異epoch遷移で旧subscription/listenerが新epochへprojectionを漏らさない。
- 無効epochでlink状態が原子性を保つ。
- Ruviform側の変更が独立レビュー/Validator/Judgeを通過する。
