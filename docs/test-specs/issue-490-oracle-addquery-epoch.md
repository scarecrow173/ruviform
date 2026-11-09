---
id: "TSPEC-hojj4XgrGAgMW3wK6Tr5Ok"
type: "test-spec"
status: "approved"
title: "Issue #490 oracle late-create query epoch テスト仕様"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "docs/specs/issue-490-oracle-addquery-epoch.md"
    - "docs/designs/issue-490-oracle-addquery-epoch.md"
  changes:
    added: []
    modified:
      - type: "file"
        path: "ui/guitest/test/oracle.test.ts"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/designs/issue-490-oracle-addquery-epoch.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-377-query-binding-epoch.md"
    - "docs/test-specs/issue-378-oracle-late-create-epoch.md"
    - "docs/test-specs/viewlink-authoritative-generation-tests.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-490-oracle-addquery-epoch.md"
    - "docs/specs/issue-490-oracle-addquery-epoch.md"
  derived-by: []
  verifies:
    - "docs/designs/issue-490-oracle-addquery-epoch.md"
    - "docs/specs/issue-490-oracle-addquery-epoch.md"
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/377"
    - "https://github.com/scarecrow173/ruviform/issues/378"
    - "https://github.com/scarecrow173/ruviform/issues/490"
  defers: []
  deferred-by: []
---

# Issue #490 oracle late-create query epoch テスト仕様

## Purpose

`ContractModel.addQuery` が SDK `ViewLink.registerQuery`（Issue #377 / PR #489）と同じ epoch 採用契約を encode していることを、scenario 語彙に依存しない `ContractModel` 直接 unit test（`ui/guitest/test/oracle.test.ts`、node --test）で恒久的に保証する。replay op に mid-scenario query 生成がない現状でも、oracle が正しい契約を保持していることを検証でき、将来 op が追加された際の oracle/SDK 乖離を早期に検出する。SDK 側の同契約は `ui/binding/test/link.test.ts` の「ViewLink late-created queries」が既に保証しており、本 test-spec は oracle 側の写しの正しさを対象とする。SDK 契約が再度変更された場合は本 spec を見直す。

## Feature

ContractModel late-created query の view epoch 採用

## Rules

- Rule: attached な link へ後から `addQuery` された query model は、現在の link generation（view epoch）を採用する（`queryDiagnostics.generation` が link generation と一致）。
- Rule: epoch 採用は query 発行より先に行われ、発行される query の `generation`（`trace.queries` rec）は採用した link epoch を記録し、`viewId` は link の view id を持つ。
- Rule: late-create query への reply は採用 epoch を echo すれば `stale` に分類されず解決する（`queryOutcomes` が `ok`、`resolved` が増え、`stale` が増えない）。foreign epoch を echo する reply は `stale` に分類される guard を維持する。
- Rule: attach 前に登録された query model の既存経路（`ContractModel.attach` が `queryModel.attach(linkGeneration)` で epoch を配る）は不変であり、late-create 経路と同じ最終状態に収束する。
- Rule: `detach` は late-create query model の in-flight を cancel（`cancelled` 計数・`inflight` 0）し generation を bump し、次の `attach` で新しい link epoch を再採用する。

## Examples

- Example（初回 attach 後の late-create）: `attach("window-A")`（generation 1）→ `addQuery()`。Then `queryDiagnostics["view-1"].generation === 1`、発行した query の rec が `generation === 1`・`viewId === "view-1"`、`{generation: 1, sourceRevision: ...}` の reply で outcome `ok`。
- Example（Dock 後の late-create）: `attach("window-A")` → `attach("window-B")`（dock-transfer、generation 2）→ `addQuery()`。Then query model は generation 2 を採用して発行する。
- Example（reattach 後の late-create）: `attach("window-A")` → `detach("unmount")` → `attach("window-A")`（generation 2）→ `addQuery()`。Then query model は generation 2 を採用する。
- Example（detach の波及）: 上記各ケースで発行済み query がある状態で `detach` → outcome `cancelled`・`inflight === 0`。その後 `attach` で新 epoch を採用し、以後の発行は新 epoch で mint される。

## Guarantees

- late-create query model が live view epoch を採用し、既存経路（attach 前登録）と同一 epoch で発行・解決・diagnostics を記録する。
- 上記 3 ケース（初回 attach 後 / Dock 後 / reattach 後）で契約が成立する。
- 既存の全 scenario replay（`runScenario` の actual/expected trace deep-equal）と `expect` pins が変わらない — oracle の他経路・guard に回帰がない。

## Non-goals

- `ui/binding` の `QueryBinding` / `ViewLink` 自体の契約 — `ui/binding/test/link.test.ts` と `issue-377-query-binding-epoch` テスト仕様が担当。
- mid-scenario create-query op など scenario schema / op 語彙の拡張。
- fixture / golden / 既存 scenario の更新。
- binding 側の late-create（`addBinding`）— `issue-378-oracle-late-create-epoch` テスト仕様で保証済み。

## Risk

- この保証が落ちると、将来 replay op が mid-scenario query 生成を追加した時点で oracle が旧契約（epoch 未採用）のまま残り、実装（修復済み SDK）との trace 乖離による false failure、あるいは oracle 緩和による false pass を招く。
- query には subscription が存在しないため、binding テストの「subscription generation」に相当する観測は発行 rec・diagnostics・outcomes で代替する。これは SDK 側でも query の epoch 観測が request/diagnostics 経由であることと一致する。
