---
id: "DESIGN-eBMdc3IAyT8cNugckC1k2Q"
type: "design"
status: "approved"
title: "Issue #490 oracle late-create query epoch 整合設計"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "docs/specs/issue-490-oracle-addquery-epoch.md"
    - "https://github.com/scarecrow173/ruviform/issues/490"
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
  implements:
    - "docs/specs/issue-490-oracle-addquery-epoch.md"
  implemented-by:
    - "docs/plans/issue-490-oracle-addquery-epoch.md"
  depends-on:
    - "docs/specs/issue-490-oracle-addquery-epoch.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-377-query-binding-epoch.md"
    - "docs/designs/issue-378-oracle-late-create-epoch.md"
    - "docs/designs/viewlink-authoritative-generation.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-490-oracle-addquery-epoch.md"
    - "docs/designs/issue-377-query-binding-epoch.md"
  derived-by:
    - "docs/plans/issue-490-oracle-addquery-epoch.md"
    - "docs/test-specs/issue-490-oracle-addquery-epoch.md"
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

# Issue #490 oracle late-create query epoch 整合設計

## Context

`ContractModel`（`ui/guitest/src/oracle.ts`）は `ui/binding` の文書化された契約を独立に encode する replay oracle である。SDK 側では Issue #377 / PR #489 で `ViewLink.registerQuery` が修復され、attach 済み link へ後から登録された query は `query.attach(this.generation_)` で live view epoch を採用する（join は epoch 遷移であり、foreign epoch で mint された in-flight request は `cancelAll` される）。一方 oracle の `ContractModel.addQuery` は `QueryModel` を `generation = 0` で生成するだけで、`linkAttached` 時の epoch 採用が欠落しており、ヘッダコメントの `QueryBinding` 記述も late-create adopt を含まない旧契約のままである。

binding 側の同型乖離は Issue #378 / PR #488 で `addBinding`（`setContext` 無条件 → attached なら `attach(linkGeneration)` → `subscribe`）として修復済みであり、本設計は query 側を同じ責務境界・同じ形状で整合させる最小の整合的変更を定める。

replay の op 語彙には mid-scenario の binding/query 生成 op が存在しないため、この分岐は現行の全 scenario fixture replay では未到達の dead code である。それでも oracle は SDK 契約の正しい写しでなければならず、本設計は model の authoring を正しい契約へ揃える。

## Scope

### In Scope

- `ContractModel.addQuery` への live epoch 採用の追加（`linkAttached` 時に `attach(linkGeneration)`）。
- `oracle.ts` ヘッダの `QueryBinding` 契約記述の更新。
- `ui/guitest/test/oracle.test.ts` への late-create query ケース追加（#378 の `addBinding` 3 ケース構成を query 観測面へ移植）。

### Out of Scope

- `ui/binding/`（#377 で完了済み）、`replay.ts`、scenario schema / op 語彙、fixture / golden。
- receipt / revision / stale guard の意味変更、oracle 判定の緩和。

## Components and Boundaries

- `ContractModel.addQuery`（`ui/guitest/src/oracle.ts`）: 唯一のプロダクションコード変更点。`QueryModel` 生成・登録後、`this.linkAttached` の場合のみ `this.queryModel.attach(this.linkGeneration)` を実行する。SDK `registerQuery` の `queries.add` → `if (attached_) query.attach(generation_)` と同じ「登録してから adopt」の順序。
- `QueryModel.attach(epoch)`（同ファイル）: 既存実装がそのまま使える。`epoch` 指定時に `generation` を採用し `cancelAll()` する。`addQuery` 直後は in-flight が空のため `cancelAll` は no-op であり、SDK `QueryBinding.attach` と同じ shape。
- `ContractModel.attach`（同ファイル）: 既存経路は `this.queryModel?.attach(this.linkGeneration)` で attach ごとに全 query model の epoch を再同期しており不変。late-create 経路はこの経路が担う状態（attached link 上の query が live epoch を持つ）へ join 時に揃えるもの。
- `ContractModel.detach`: `queryModel?.detach()` → `attach()`（generation+1・cancelAll）の既存経路は不変。late-create query も通常経路と同じく detach で in-flight cancel・epoch bump の対象になる。
- `ui/guitest/test/oracle.test.ts`（既存・#378 新設）: `ContractModel` を直接駆動する unit test へ late-create query の describe を追加。query 側の観測面は `traceResult()` の `queries`（発行 rec の `generation`/`viewId`）・`queryOutcomes`・`queryDiagnostics`（`generation`/`resolved`/`stale`/`cancelled`/`inflight`）・`link`。binding の `subscriptions`/`finalStates` に相当する query 側の「subscription generation」は存在しない（query は subscription を持たない — SDK `registerQuery` の unregister が membership-only であるのと同じ契約）ため、発行 rec の generation がその代替観測面となる。

## Data and Control Flow

- late-create 経路（修復後）: `addQuery` → `QueryModel` 生成 → `linkAttached` なら `attach(linkGeneration)` で `generation` を live epoch に採用（in-flight 空のため cancel は no-op）→ 以後の `query()` が `generation = linkGeneration` で `q-<viewId>-<n>` を mint し、`trace.queries` に同 generation を記録。
- 既存経路（不変）: attach 前に登録された query model は `ContractModel.attach` が `attach(linkGeneration)` で epoch を配り、detach は `detach()`（generation+1・cancelAll）、reattach は再度 `attach(linkGeneration)`。`runScenario` は常に `addQuery` → `attach` の順で呼ぶため、replay 経路の観測結果は変わらない。
- SDK との差異の解消: 旧実装では attach 後 `addQuery` すると `generation = 0` のまま query が発行され、live epoch を echo する reply が `stale` に分類されていた。修復後は SDK `registerQuery` と同じく join 時に live epoch を採用する。

## Non-Functional Constraints

- oracle は SDK 実装とコードを共有しない独立モデルであり続ける（型 import のみ）。
- `node --test` の headless・deterministic lane で完結し、DOM / renderer / 実機依存を持ち込まない。
- 既存の全 scenario replay（`ui/guitest/fixtures/scenarios/`）と `expect` pins の結果を変えない。

## Risks and Trade-offs

- リスク: `addQuery` の adopt 追加が既存 replay の観測 trace を変える。`runScenario` は `addQuery`（attach 前）→ `attach` の順で呼ぶため `linkAttached === false` の分岐は replay では未到達。`ContractModel.attach` の `queryModel?.attach(linkGeneration)` が既に正しい epoch を配るため、replay の `queryDiagnostics.generation`・発行 rec の generation は不変。
- リスク: `QueryModel.attach(epoch)` が `cancelAll` を伴うため、仮に in-flight がある状態で adopt すると cancel が走る。`addQuery` は model 生成と同時にのみ呼ばれ in-flight は必ず空のため、実害はない。SDK `registerQuery` の「join は epoch 遷移で in-flight を cancel する」契約とも整合する。
- トレードオフ: query には subscription が存在しないため、binding テストの「subscription generation」相当の観測は発行 rec（`trace.queries[].generation`）と `queryDiagnostics.generation` で代替する。これは SDK 側でも query の epoch 観測が request/diagnostics 経由であることと一致する。

## Deferred Design Concerns

- mid-scenario の query/binding 生成 op | replay op 語彙に生成 op がなく late-create 経路は unit test のみで到達可能 | 必要になった時点で別 Issue | oracle が正しい契約を encode していても scenario lane からは未到達のまま

## Verification Notes

- [ ] `ui/guitest/test/oracle.test.ts` の late-create query 3 ケースが RED（発行 generation が link epoch を採用せず、live-epoch echo reply が stale 判定）→ GREEN を記録する。
- [ ] `mise run check-ui-guitest`（tsc --noEmit）と `mise run test-ui-guitest`（node --test、既存 scenario replay を含む）が green。
- [ ] `oracle.ts` ヘッダの `QueryBinding` 契約記述が `registerQuery` / `attach` の現行動作と一致する。
