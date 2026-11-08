---
id: "SPEC-034UhhQycxrrbYudGthyQi"
type: "spec"
status: "approved"
title: "Issue #378 oracle late-create binding epoch 整合"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/378"
  changes:
    added:
      - type: "file"
        path: "ui/guitest/test/oracle.test.ts"
    modified:
      - type: "file"
        path: "ui/guitest/src/oracle.ts"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/viewlink-authoritative-generation.md"
    - "docs/designs/viewlink-authoritative-generation.md"
    - "docs/test-specs/viewlink-authoritative-generation-tests.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/376"
    - "https://github.com/scarecrow173/ruviform/issues/377"
    - "https://github.com/scarecrow173/ruviform/issues/378"
  defers: []
  deferred-by: []
---

# Issue #378 oracle late-create binding epoch 整合

## What Should Be Built

`ui/guitest/src/oracle.ts` の `ContractModel.addBinding` を、修復済み SDK `ViewLink.registerBinding`（`ui/binding/src/link.ts`、Issue #376 / PR #379）と同じ epoch 採用順序へ整合させる。具体的には `setContext` を無条件に行い、link が attached の場合に限り `b.attach(this.linkGeneration)` → `subscribe` の順で処理する。併せてファイルヘッダの旧契約記述（「already-attached link 上で作成された binding は view epoch を採用しない」）を現行契約へ更新する。

あわせて `ContractModel` を直接対象とする恒久 unit test（`ui/guitest/test/oracle.test.ts`）を新設し、初回 attach 後・Dock（別 window への attach）後・detach→reattach 後に model へ binding を追加した各ケースで、late-create binding が link generation を採用し dispatch/subscribe/finalState の generation が一致することを検証する。

## Why Now

Issue #376 で SDK 側は修復済み（`registerBinding` が live view epoch を採用してから subscribe する）だが、replay oracle の `ContractModel.addBinding` は旧契約 — context 設定と subscribe のみで `attach` による epoch 採用が欠落 — のまま残っている。現行の replay op 語彙には mid-scenario の binding 生成 op がなく、この分岐は全 scenario fixture の replay では未到達の dead code である。しかし oracle は SDK 契約の独立した写しであるべきものであり、誤った契約を encode したままでは、将来 op が追加された時点で oracle と実装が静かに乖離する。model の authoring は正しい契約から行う必要がある。

## Users and Value

- gui-test replay 基盤（`ui/guitest`）の保守者: oracle が SDK の現行契約を正しく写していることを、scenario 拡張とは独立に検証できる。
- 将来 mid-scenario create-binding op を追加する実装者: oracle 側の期待値が既に正しい epoch 採用を encode しているため、op 追加時に oracle 修正を再度行う必要がない。

## Scope

### In Scope

- `ContractModel.addBinding` の epoch 採用順序修復（`setContext` 無条件 → attached 時に `attach(linkGeneration)` → `subscribe`）。
- `oracle.ts` ヘッダコメントの旧契約記述の整合。
- `ContractModel` 直接 unit test の新設（`ui/guitest/test/oracle.test.ts`）— late-create 3 ケース（初回 attach 後 / Dock 後 / reattach 後）。

### Out of Scope

- `ContractModel.addQuery` / `QueryModel` の epoch 整合 — Issue #377（`QueryBinding` late-create）が未 merge の現時点で model だけ epoch adopt すると、oracle が SDK 未実装の契約を予測する「oracle 先行」状態になるため。本 dispatch の対象外とし、#377 merge 後のフォローアップとして記録する。
- `ui/binding/` 配下の変更（兄弟 Issue #377 が別 Worker で実装中）。
- `replay.ts`・scenario schema / op 語彙の拡張（mid-scenario create-binding op は追加しない）。
- fixture / golden / 既存 scenario の更新。
- receipt / revision / stale guard の緩和や oracle 判定の変更。

## Requirements

- `addBinding` は SDK `registerBinding` と同形の順序で処理する: `setContext` は無条件、`linkAttached` 時に `b.attach(this.linkGeneration)` → `subscribe`。
- late-create された binding は live view epoch を採用し、以後の dispatch・subscribe context・`finalStates` の generation が link epoch と一致する。
- 現行 fixture / replay の振る舞い、receipt / revision / stale guards を保持する（oracle 緩和や golden 更新で済ませない）。
- scenario schema / op 語彙は拡張しない。
- model の authoring は正しい契約から行う。

## Acceptance Criteria

- [ ] `ContractModel.addBinding` が修復後 SDK の epoch 採用順序（`setContext` 無条件 → attached 時 `attach(linkGeneration)` → `subscribe`）に一致し、ヘッダ `oracle.ts` の旧契約記述が整合している。
- [ ] 初回 attach 後 / Dock 後 / reattach 後に model へ binding を加えるケースを恒久テストで検証する（`ContractModel` 直接 unit test。`link.test.ts` の late-created 3 ケース構成を移植）。scenario schema / op 拡張なし。RED → GREEN を記録する。
- [ ] 現行 fixture / replay の振る舞い・receipt / revision / stale guards を保持する（oracle 緩和や golden 更新で済ませない）。
- [ ] `addQuery`（`oracle.ts`）は本 dispatch の対象外であること、および #377 merge 後に `addQuery` + ヘッダの query 契約記述の整合が必要であることを、PR 本文または Issue #378 へのコメント案として記録する。

## Deferred Design Concerns

- `ContractModel.addQuery` / `QueryModel` の late-create epoch 整合 | Issue #377（`QueryBinding` late-create）が未 merge であり、現時点で model 側だけ epoch adopt すると oracle が SDK 未実装の契約を予測する oracle 先行状態になる | Issue #377 merge 後 | oracle と SDK の query 契約が再び乖離する

## Sources and Relations

- Issue #378（本 spec の source）。関連 Issue: #376（SDK `registerBinding` の epoch 修復、merge 済み PR #379）、#377（`QueryBinding` late-create、別 Worker が実装中）。
- 上流契約の正本: `docs/specs/viewlink-authoritative-generation.md`、`docs/designs/viewlink-authoritative-generation.md`、`docs/test-specs/viewlink-authoritative-generation-tests.md`（#376 系の spec/design/test-spec チェーン。#378 はその非目標として明示的に切り出された範囲）。
- SDK 側の修復形状: `ui/binding/src/link.ts` `registerBinding`（`setContext` 無条件 → attached なら `attach(generation_)` → `subscribeBinding`）。
- oracle 側の対応実装: `ui/guitest/src/oracle.ts` `ContractModel.addBinding` / `attach`。
- テスト移植元: `ui/binding/test/link.test.ts` の「ViewLink late-created bindings」3 ケース。
