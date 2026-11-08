---
id: "TSPEC-034UhmzL4r0lLg4T5tUy74"
type: "test-spec"
status: "approved"
title: "Issue #378 oracle late-create binding epoch テスト仕様"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "docs/specs/issue-378-oracle-late-create-epoch.md"
    - "docs/designs/issue-378-oracle-late-create-epoch.md"
  changes:
    added:
      - type: "file"
        path: "ui/guitest/test/oracle.test.ts"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/designs/issue-378-oracle-late-create-epoch.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/viewlink-authoritative-generation-tests.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-378-oracle-late-create-epoch.md"
    - "docs/specs/issue-378-oracle-late-create-epoch.md"
  derived-by: []
  verifies:
    - "docs/designs/issue-378-oracle-late-create-epoch.md"
    - "docs/specs/issue-378-oracle-late-create-epoch.md"
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/378"
  defers: []
  deferred-by: []
---

# Issue #378 oracle late-create binding epoch テスト仕様

## Purpose

`ContractModel.addBinding` が SDK `ViewLink.registerBinding`（Issue #376 / PR #379）と同じ epoch 採用契約を encode していることを、scenario 語彙に依存しない `ContractModel` 直接 unit test（`ui/guitest/test/oracle.test.ts`、node --test）で恒久的に保証する。replay op に mid-scenario binding 生成がない現状でも、oracle が正しい契約を保持していることを検証でき、将来 op が追加された際の oracle/SDK 乖離を早期に検出する。SDK 側の同契約は `ui/binding/test/link.test.ts` の late-created 3 ケースが既に保証しており、本 test-spec は oracle 側の写しの正しさを対象とする。SDK 契約が再度変更された場合は本 spec を見直す。

## Feature

ContractModel late-created binding の view epoch 採用

## Rules

- Rule: attached な link へ後から `addBinding` された binding は、現在の link generation（view epoch）を採用する。
- Rule: epoch 採用は subscription 記録より先に行われ、subscription の `generation` は link generation を記録する（`ContractModel.subscribe` の既存契約）。
- Rule: late-create binding が dispatch する command の `generation` は採用した link epoch と一致し、context の `viewId` / `windowId` は link の現在値を持つ。
- Rule: late-create binding の receipt は採用 epoch で相関し、live epoch の receipt は stale に分類されない（`staleReceipts` が増えない）。
- Rule: attach 前に登録された binding の既存経路（`ContractModel.attach` が全 binding へ epoch を配る）は不変であり、late-create 経路と同じ最終状態に収束する。
- Rule: `detach` は late-create binding を含む全 binding の subscription を解放し、detach 後の push は detached intent として計数され dispatch されない。

## Examples

- Example（初回 attach 後の late-create）: `addBinding("f1", ...)` → `attach("window-A")`（generation 1）→ `addBinding("f2", ...)`。Then `finalStates["f2"].generation === 1`、最後の subscription rec の `generation === 1`・`windowId === "window-A"`、f2 の gesture commit の dispatched rec が `generation === 1`・`viewId`/`windowId` を持ち、`ack` で `staleReceipts === 0`。
- Example（Dock 後の late-create）: `attach("window-A")` → `attach("window-B")`（dock-transfer、generation 2）→ `addBinding("f2", ...)`。Then f2 は generation 2・window-B の context で subscribe・dispatch される。
- Example（reattach 後の late-create）: `attach("window-A")` → `detach("unmount")` → `attach("window-A")`（generation 2）→ `addBinding("f2", ...)`。Then f2 は generation 2 を採用する。
- Example（detach の波及）: 上記各ケースで `detach("unmount")` → 全 binding の `attached === false`、unsubscription が全件記録され、double detach は no-op。

## Guarantees

- late-create binding が live view epoch を採用し、既存 binding と同一 epoch で dispatch / subscribe / finalState を記録する。
- 上記 3 ケース（初回 attach 後 / Dock 後 / reattach 後）で契約が成立する。
- 既存の全 scenario replay（`runScenario` の actual/expected trace deep-equal）と `expect` pins が変わらない — oracle の他経路・guard に回帰がない。

## Non-goals

- `ContractModel.addQuery` / `QueryModel` の late-create epoch — Issue #377（`QueryBinding` late-create）merge 後の別 dispatch で扱う。oracle 先行を避けるため本 spec では検証しない。
- mid-scenario create-binding op など scenario schema / op 語彙の拡張。
- SDK `CommandBinding` / `ViewLink` 自体の契約 — `ui/binding/test/link.test.ts` と `viewlink-authoritative-generation-tests` が担当。
- fixture / golden / 既存 scenario の更新。

## Risk

- この保証が落ちると、将来 replay op が mid-scenario binding 生成を追加した時点で oracle が旧契約（epoch 未採用）のまま残り、実装（修復済み SDK）との trace 乖離による false failure、あるいは oracle 緩和による false pass を招く。
- `setContext` 無条件化による attach 前登録 binding の早期 `viewId` 設定は、replay 経路では attach が ops 前に必ず走るため観測差分を生じないが、attach 前 dispatch を行う直接利用では SDK と同じ context を持つことが本 spec の前提となる。
