---
id: "SPEC-1ZxAql6tPbxsqXhvmGvphr"
type: "spec"
status: "approved"
title: "Issue #490 oracle late-create query epoch 整合"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/490"
    - "docs/specs/issue-377-query-binding-epoch.md"
  changes:
    added: []
    modified:
      - type: "file"
        path: "ui/guitest/src/oracle.ts"
      - type: "file"
        path: "ui/guitest/test/oracle.test.ts"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "docs/designs/issue-490-oracle-addquery-epoch.md"
    - "docs/plans/issue-490-oracle-addquery-epoch.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-377-query-binding-epoch.md"
    - "docs/designs/issue-377-query-binding-epoch.md"
    - "docs/test-specs/issue-377-query-binding-epoch.md"
    - "docs/specs/issue-378-oracle-late-create-epoch.md"
    - "docs/specs/viewlink-authoritative-generation.md"
    - "docs/designs/viewlink-authoritative-generation.md"
    - "docs/test-specs/viewlink-authoritative-generation-tests.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "docs/designs/issue-490-oracle-addquery-epoch.md"
    - "docs/plans/issue-490-oracle-addquery-epoch.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-490-oracle-addquery-epoch.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/377"
    - "https://github.com/scarecrow173/ruviform/issues/378"
    - "https://github.com/scarecrow173/ruviform/issues/490"
  defers: []
  deferred-by: []
---

# Issue #490 oracle late-create query epoch 整合

## What Should Be Built

`ui/guitest/src/oracle.ts` の `ContractModel.addQuery` を、修復済み SDK `ViewLink.registerQuery`（`ui/binding/src/link.ts`、Issue #377 / PR #489）と同じ epoch 採用契約へ整合させる。具体的には `QueryModel` 生成後、link が attached の場合に限り `attach(this.linkGeneration)` で live view epoch を採用させる（SDK `registerQuery` の `if (this.attached_) query.attach(this.generation_)` と同形）。併せてファイルヘッダの `QueryBinding` 契約記述を、late-create query が live epoch を採用する現行契約へ更新する。

あわせて `ContractModel` を直接対象とする恒久 unit test（`ui/guitest/test/oracle.test.ts`、#378 で新設済み）へ late-create query のケースを追加し、初回 attach 後・Dock（別 window への attach）後・detach→reattach 後に `addQuery` した各ケースで、発行される query の generation が link epoch を採用し、live epoch を echo する reply が `stale` ではなく解決されること、detach が in-flight を cancel し reattach で新 epoch を採用することを検証する。

## Why Now

Issue #377（PR #489 `7dc0fdf3`）で SDK 側 `registerQuery` に adopt-on-join（attach 済み link への join 時 `query.attach(generation_)`）が実装された。これにより replay oracle の `ContractModel.addQuery` は `generation = 0` のまま linkAttached 時に epoch adopt しない旧契約のまま残り、修復後 SDK と乖離する。#378（PR #488 `41b24fac`）で同型の `addBinding` は既に整合済みであり、本 Issue はその完了時記録で指名された query 側の後続である。oracle は SDK 契約の独立した写しであるべきものであり、誤った契約を encode したままでは将来の scenario/op 拡張時に oracle と実装が静かに乖離する。

## Users and Value

- gui-test replay 基盤（`ui/guitest`）の保守者: oracle が SDK の現行 query 契約を正しく写していることを、scenario 拡張とは独立に検証できる。
- 将来 mid-scenario create-query op を追加する実装者: oracle 側の期待値が既に正しい epoch 採用を encode しているため、op 追加時に oracle 修正を再度行う必要がない。

## Scope

### In Scope

- `ContractModel.addQuery` の epoch 採用（`linkAttached` 時に `queryModel.attach(this.linkGeneration)` — SDK `registerQuery` と同形の順序）。
- `oracle.ts` ヘッダの `QueryBinding` 契約記述の整合。
- `ContractModel` 直接 unit test（`ui/guitest/test/oracle.test.ts`）への late-create query ケース追加 — 初回 attach 後 / Dock 後 / reattach 後の 3 ケース構成（`addBinding` ケース・SDK `link.test.ts` late-created queries を移植）。

### Out of Scope

- `ui/binding/` 配下の変更（#377 で完了済み・SDK 側は変更しない）。
- `replay.ts`・scenario schema / op 語彙の拡張（mid-scenario create-query op は追加しない）。
- fixture / golden / 既存 scenario の更新。
- receipt / revision / stale guard の緩和や oracle 判定の変更。

## Requirements

- `addQuery` は SDK `registerQuery` と同形に、link attached 時に生成した `QueryModel` へ live view epoch（`linkGeneration`）を採用させる。attach 前登録の既存経路（`ContractModel.attach` が `queryModel.attach(linkGeneration)` を呼ぶ）は不変。
- late-create された query model は live view epoch を採用し、以後に発行する query の `generation`（`trace.queries` rec・`queryDiagnostics.generation`）が link epoch と一致する。
- 現行 fixture / replay の振る舞い、receipt / revision / stale guards を保持する（oracle 緩和や golden 更新で済ませない）。
- scenario schema / op 語彙は拡張しない。
- model の authoring は正しい契約から行う。

## Acceptance Criteria

- [ ] `ContractModel.addQuery` が attach 済み link で `linkGeneration` を adopt する（SDK `registerQuery`・`addBinding` と同形の順序）。
- [ ] ヘッダ `oracle.ts` の QueryBinding late-create 記述を新契約に整合。
- [ ] `oracle.test.ts` に late-create query のケースを追加し RED→GREEN を確認（generation adopt・subscription generation・detach 後の振る舞い — `addBinding` 3 ケース構成を移植）。
- [ ] `check-ui-guitest`・`test-ui-guitest`・`fmt-check` が exit 0。
- [ ] 既存 fixture/replay の振る舞い・receipt/revision/stale guards を保持（op 語彙拡張なし）。

## Deferred Design Concerns

- mid-scenario の query 生成 op（scenario schema / op 語彙への追加） | replay op に binding/query の mid-scenario 生成がなく、late-create 経路は `ContractModel` 直接 unit test でのみ到達可能 | 必要になった時点で別 Issue | oracle が正しい契約を encode していても、scenario lane からは依然として未到達のまま

## Sources and Relations

- Issue #490（本 spec の source）。関連 Issue: #377（SDK `registerQuery` の adopt-on-join、merge 済み PR #489）、#378（oracle `addBinding` の同型整合、merge 済み PR #488）。
- 上流契約の正本: `docs/specs/issue-377-query-binding-epoch.md` 系チェーン、`docs/specs/viewlink-authoritative-generation.md` 系チェーン。
- SDK 側の修復形状: `ui/binding/src/link.ts` `registerQuery`（`if (this.attached_) query.attach(this.generation_)`）。
- oracle 側の対応実装: `ui/guitest/src/oracle.ts` `ContractModel.addQuery` / `attach` / `addBinding`（#378 で整合済みの先例）。
- テスト移植元: `ui/guitest/test/oracle.test.ts` の「ContractModel late-created bindings」3 ケース、`ui/binding/test/link.test.ts` の「ViewLink late-created queries」。
