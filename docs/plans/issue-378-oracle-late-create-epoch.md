---
id: "PLAN-034UhlBN58rtCJqfZMTYgt"
type: "plan"
status: "approved"
title: "Issue #378 oracle late-create binding epoch 実装計画"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "docs/specs/issue-378-oracle-late-create-epoch.md"
    - "docs/designs/issue-378-oracle-late-create-epoch.md"
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
    - "docs/tasks/issue-378-t1-oracle-addbinding-epoch.md"
  depends-on:
    - "docs/designs/issue-378-oracle-late-create-epoch.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/viewlink-authoritative-generation.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-378-oracle-late-create-epoch.md"
    - "docs/designs/issue-378-oracle-late-create-epoch.md"
  derived-by:
    - "docs/tasks/issue-378-t1-oracle-addbinding-epoch.md"
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

# Issue #378 oracle late-create binding epoch 実装計画 Implementation Plan

## Goal

`ContractModel.addBinding` を修復済み SDK `ViewLink.registerBinding` と同じ epoch 採用順序へ整合させ、late-create binding の恒久 unit test を追加する。

## Scope

単一の実装ストリーム。`ui/guitest` 内の oracle 修正とその直接 unit test のみで完結し、SDK（`ui/binding/`）・replay driver・scenario 語彙・fixture/golden には触れない。分割不要。

## Upstream Documents

- `docs/specs/issue-378-oracle-late-create-epoch.md`（approved）
- `docs/designs/issue-378-oracle-late-create-epoch.md`（approved）
- `docs/specs/viewlink-authoritative-generation.md` / `docs/designs/viewlink-authoritative-generation.md`（上流契約。#378 はその非目標として切り出された範囲）

## Design Inputs

- `docs/designs/issue-378-oracle-late-create-epoch.md`
- SDK 側の正本実装: `ui/binding/src/link.ts` `registerBinding`（`setContext` 無条件 → attached なら `attach(generation_)` → `subscribeBinding`）
- oracle 側の既存の正しい順序: `ContractModel.attach`（`oracle.ts` 内、全 binding へ `setContext` → `attach(linkGeneration)` → `subscribe`）
- テスト移植元: `ui/binding/test/link.test.ts`「ViewLink late-created bindings」3 ケース

## File Map

- `ui/guitest/src/oracle.ts` - `ContractModel.addBinding` の epoch 採用順序修復とヘッダ契約記述の更新。唯一のプロダクションコード変更。
- `ui/guitest/test/oracle.test.ts` - `ContractModel` 直接 unit test（新規）。late-create 3 ケース + attach 前登録の既存経路が不変であることの確認。
- `docs/specs|designs|plans|test-specs|tasks/issue-378-*.md` - 本チェーンの文書。`docs/*/README.md` index は作成 script が更新済み。

## Implementation Sequence

- [ ] `ui/guitest/test/oracle.test.ts` を作成し、late-create 3 ケース（初回 attach 後 / Dock 後 / reattach 後）で link epoch 採用を検証するテストを書く。
- [ ] `mise run test-ui-guitest` で期待どおり RED（late binding の generation が link epoch を採用しない）を確認する。
- [ ] `oracle.ts` の `ContractModel.addBinding` を `setContext` 無条件 → `linkAttached` 時に `attach(linkGeneration)` → `subscribe` の順へ修正し、ヘッダ 39-41 行の旧契約記述を更新する。
- [ ] `mise run test-ui-guitest` で GREEN を確認し、`mise run check-ui-guitest`（tsc --noEmit）を通す。
- [ ] `git fetch origin main && git merge origin/main` で最新 main を取り込み、docs index 等の生成物を再生成して再検証する。
- [ ] review-protocol レビュー（blocker なしを確認）→ 論理単位 commit → push → 日本語 PR 作成。

## Dependencies and Risks

| # | Risk | Likelihood | Impact | Mitigation |
|---|------|-----------|--------|------------|
| 1 | `setContext` 無条件化が既存 replay の観測 trace を変える | low | med | replay では `model.attach` が ops 前に必ず呼ばれ dispatch は attach 後のみ。attach 時に同値で上書きされるため差分なし。`test-ui-guitest` 全件で確認。 |
| 2 | late-create binding が `attach(epoch)` で意図せず emit / 状態遷移する | low | low | `BindingModel.attach` は既に attached なら generation 採用後 early return（emit なし）。SDK `CommandBinding.attach` と同形であることをコードで確認済み。 |
| 3 | Issue #377 merge 前に `addQuery` を触って oracle 先行状態になる | — | — | `addQuery` / `QueryModel` は本 plan の対象外。PR 本文に #377 merge 後の整合要否を記録する。 |
| 4 | oracle.test.ts が SDK ではなく oracle 実装の写しを検証してしまう | low | med | assertion は「link epoch を採用する」という契約条件（finalStates/dispatched の generation が linkGeneration と一致）に限定し、内部手順には依存しない。 |

Rollback: 変更は `oracle.ts` 1 関数 + 新規 test + docs のみで、revert で完全に戻る。永続データ・公開 API・fixture への影響なし。

## Task Breakdown

- T1（`docs/tasks/issue-378-t1-oracle-addbinding-epoch.md`）: oracle.test.ts 新設（RED）→ `addBinding` 修復 + ヘッダ更新（GREEN）→ 検証一式。単一タスクに収まる粒度。

## Verification

| Step | Verification command or criteria | Pass condition |
|------|----------------------------------|----------------|
| テスト追加 | `mise run test-ui-guitest` | 新規 oracle.test.ts が late-create 3 ケースで FAIL（RED）— epoch 未採用を示す |
| oracle 修復 | `mise run test-ui-guitest` | 全テスト PASS（GREEN）。既存 scenario replay の trace 一致を維持 |
| 型検査 | `mise run check-ui-guitest` | tsc --noEmit exit 0 |
| フォーマット | `mise run fmt-check` | cargo fmt --check exit 0（Rust 非対象だが共通 gate として実行） |
| main 取込 | `git fetch origin main && git merge origin/main` 後に上記を再実行 | conflict なし・再検証 green |

## Open Questions / Gaps

- なし。`addQuery` の扱いは Issue #378 AC-4 と spec の Deferred Design Concerns で確定済み。

## Self-Review

- [x] Every spec requirement maps to a task. — spec の In Scope 3 項目（addBinding 修復・ヘッダ更新・unit test 新設）はすべて T1 に写像。Out of Scope（addQuery・ui/binding・replay/schema・fixture/golden・guards 緩和）は変更対象に含めない。
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（単一タスク・単一ファイル変更のため delegation 不要）
- Independent work streams: none
- User approval required before dispatch: 本 dispatch では review-protocol による blocker なし確認を承認として扱う
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- 前提: SDK `registerBinding`（#376 / PR #379 merge 済み）の形状が正本。#377 は未 merge のため query 側は触れない。
- reviewer focus: `addBinding` の順序が `registerBinding` と同形か、`setContext` 無条件化の観測影響がないか、テストが契約条件（epoch 採用）を検証しているか。
