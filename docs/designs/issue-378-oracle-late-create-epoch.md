---
id: "DESIGN-034UhiZJ7MAi32Sa1sVTWR"
type: "design"
status: "approved"
title: "Issue #378 oracle late-create binding epoch 整合設計"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "docs/specs/issue-378-oracle-late-create-epoch.md"
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
  implements:
    - "docs/specs/issue-378-oracle-late-create-epoch.md"
  implemented-by:
    - "docs/plans/issue-378-oracle-late-create-epoch.md"
  depends-on:
    - "docs/specs/issue-378-oracle-late-create-epoch.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/viewlink-authoritative-generation.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-378-oracle-late-create-epoch.md"
    - "docs/designs/viewlink-authoritative-generation.md"
  derived-by:
    - "docs/plans/issue-378-oracle-late-create-epoch.md"
    - "docs/test-specs/issue-378-oracle-late-create-epoch.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-378-oracle-late-create-epoch.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/376"
    - "https://github.com/scarecrow173/ruviform/issues/377"
    - "https://github.com/scarecrow173/ruviform/issues/378"
  defers: []
  deferred-by: []
---

# Issue #378 oracle late-create binding epoch 整合設計

## Context

`ContractModel`（`ui/guitest/src/oracle.ts`）は `ui/binding` の文書化された契約を独立に encode する replay oracle である。SDK 側では Issue #376 / PR #379 で `ViewLink.registerBinding` が修復され、attached link へ後から登録された binding は `setContext`（無条件）→ `attach(generation_)`（live epoch 採用）→ `subscribeBinding` の順で処理される。一方 oracle の `ContractModel.addBinding` は `linkAttached` 時に `setContext` + `subscribe` のみを行い、`attach(this.linkGeneration)` による epoch 採用が欠落しており、ヘッダコメントも「does NOT adopt the view epoch」という旧契約を記述したままである。

replay の op 語彙には mid-scenario の binding 生成 op が存在しないため、この分岐は現行の全 scenario fixture replay では未到達の dead code である。それでも oracle は SDK 契約の正しい写しでなければならず、本設計は model の authoring を正しい契約へ揃える最小の整合的変更を定める。

## Scope

### In Scope

- `ContractModel.addBinding` の処理順序を SDK `registerBinding` と同形にする。
- `oracle.ts` ヘッダの `ViewLink.attach` 契約記述の更新。
- `ContractModel` 直接 unit test（`ui/guitest/test/oracle.test.ts`）の新設。

### Out of Scope

- `ContractModel.addQuery` / `QueryModel`（Issue #377 merge 待ち。oracle 先行を避けるため本 dispatch では変更しない）。
- `ui/binding/`、`replay.ts`、scenario schema / op 語彙、fixture / golden。
- receipt / revision / stale guard の意味変更。

## Components and Boundaries

- `ContractModel.addBinding`（`ui/guitest/src/oracle.ts`）: 唯一の変更点。`BindingModel` 生成・登録後、`b.setContext(this.viewId, this.linkWindow)` を無条件に呼び、`this.linkAttached` の場合のみ `b.attach(this.linkGeneration)` → `this.subscribe(b)` をこの順で実行する。
- `BindingModel.attach(epoch)`（同ファイル）: 既存実装がそのまま使える。`epoch` 指定時に `generation` を採用し、既に `attached`（late-create 直後の初期状態）なら early return するため emit も発生しない — SDK `CommandBinding.attach` と同じ shape。
- `ContractModel.attach`（同ファイル 886-890 行付近）: 既存 binding への `setContext` → `attach(linkGeneration)` → `subscribe` の順序が既に正しく、本変更は late-create 経路をこの順序へ揃えるもの。
- `ui/guitest/test/oracle.test.ts`（新規）: `ContractModel` を直接駆動する unit test。`link.test.ts` の late-created 3 ケース構成を oracle の観測面（`traceResult()` の `subscriptions` / `dispatched` / `finalStates` / `bindingDiagnostics` / `link`）へ移植する。

## Data and Control Flow

- late-create 経路（修復後）: `addBinding` → `BindingModel` 生成・`bindings`/`bindingsByTarget`/`store` 登録 → `setContext(viewId, linkWindow)`（無条件）→ `linkAttached` なら `attach(linkGeneration)` で `generation` を live epoch に採用 → `subscribe`（subscription trace に現行 `linkGeneration` を記録し、daw consumer では store の現行 revision を即時 `observe`）。
- 既存経路（不変）: attach 前に登録された binding は `ContractModel.attach` が全 binding に `setContext` → `attach(linkGeneration)` → `subscribe` を適用し、以後の detach/reattach も同じ経路を通る。`addBinding` の `setContext` 無条件化は attach 前登録では `linkWindow === undefined` の context を先行設定するだけで、`attach` 時に同じ値で上書きされるため観測結果は変わらない。
- `addQuery` / `QueryModel`: 変更しない。`QueryModel.generation` は link attach のたびに `attach(linkGeneration)` で同期される既存経路のみを維持する（#377 対応後に late-create query の epoch 整合を別途行う）。

## Non-Functional Constraints

- oracle は SDK 実装とコードを共有しない独立モデルであり続ける（型 import のみ）。
- `node --test` の headless・deterministic lane で完結し、DOM / renderer / 実機依存を持ち込まない。
- 既存の全 scenario replay（`ui/guitest/fixtures/scenarios/`）と `expect` pins の結果を変えない。

## Risks and Trade-offs

- リスク: `addBinding` の `setContext` を無条件化すると、attach 前に登録された binding の `viewId` が早期に設定される。`dispatchNow` の trace rec は `viewId !== undefined` のときのみ `viewId` を出力するが、replay では `model.attach` が ops 開始前に必ず呼ばれ、dispatch は attach 後にしか発生しないため観測差分は生じない。単体 test で attach 前 dispatch を行う場合も SDK `registerBinding` が同じく無条件 `setContext` を行うため契約上一致する。
- リスク: late-create binding を同一 target で複数登録すると `bindingsByTarget` / `store` が上書きされる。これは既存の model 設計（target → binding は 1:1）の範囲であり、本変更では触れない。テストは区別可能な target を使う。
- トレードオフ: `addQuery` を今回整合させないことで oracle 内に一時的な非対称（binding は late-create epoch 採用、query は未対応）が残る。#377 merge を待つ方が、SDK 未実装の契約を oracle が先行 encode するより安全である。

## Deferred Design Concerns

- `ContractModel.addQuery` / `QueryModel` の late-create epoch 整合 | Issue #377（`QueryBinding` late-create）が未 merge であり、現時点で model 側だけ epoch adopt すると oracle が SDK 未実装の契約を予測する oracle 先行状態になる | Issue #377 merge 後 | oracle と SDK の query 契約が再び乖離する

## Verification Notes

- [ ] `ui/guitest/test/oracle.test.ts` が RED（late-create binding の generation が link epoch を採用しない）→ GREEN（採用する）を記録する。
- [ ] `mise run check-ui-guitest`（tsc --noEmit）と `mise run test-ui-guitest`（node --test、既存 scenario replay を含む）が green。
- [ ] `oracle.ts` ヘッダの契約記述が `registerBinding` / `attach` の現行動作と一致する。
